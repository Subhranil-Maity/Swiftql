//! TLS with rustls, following libpq's `sslmode`:
//!
//! - disable: no TLS.
//! - prefer: TLS if the server offers it, without checking the certificate;
//!   plain otherwise.
//! - require: TLS, without checking the certificate, unless a CA file is
//!   given (then like verify-ca, as libpq does).
//! - verify-ca: the certificate must chain to a trusted root; the host name
//!   isn't checked.
//! - verify-full: the chain and the host name are checked.
//!
//! Trusted roots are the CA file when given, otherwise the OS store plus the
//! Mozilla roots. `Auth::ClientCert` adds a client certificate (PEM files).

use std::{path::Path, sync::Arc};

use driver_core::{
    connect::{Auth, SslMode, TlsConfig},
    dbresults::DbResult,
    error::{DbError, ErrorKind},
};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, Error as TlsError, RootCertStore,
    SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto::{WebPkiSupportedAlgorithms, ring},
};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime, pem::PemObject};
use tokio_postgres_rustls::MakeRustlsConnect;

#[derive(Clone)]
pub enum PgTls {
    None,
    Rustls(MakeRustlsConnect),
}

/// SslMode, CA file and client certificate → connector.
pub fn make_tls(config: &TlsConfig, auth: &Auth) -> DbResult<PgTls> {
    if config.mode == SslMode::Disable {
        return Ok(PgTls::None);
    }
    let provider = Arc::new(ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;
    let verify_ca = match config.mode {
        SslMode::Prefer => false,
        SslMode::Require => config.ca_cert.is_some(),
        _ => true,
    };
    let verifier: Arc<dyn ServerCertVerifier> = if !verify_ca {
        Arc::new(NoVerification { algorithms })
    } else {
        let roots = Arc::new(roots(config.ca_cert.as_deref())?);
        let webpki = WebPkiServerVerifier::builder_with_provider(roots, Arc::clone(&provider))
            .build()
            .map_err(|err| tls_error(format!("couldn't set up certificate checks: {err}")))?;
        if config.mode == SslMode::VerifyFull {
            webpki
        } else {
            Arc::new(IgnoreHostName { inner: webpki })
        }
    };

    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| tls_error(err.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(verifier);
    let client = match auth {
        Auth::ClientCert { cert, key, .. } => {
            let certs = load_certs(cert)?;
            let key = PrivateKeyDer::from_pem_file(key).map_err(|err| {
                tls_error(format!(
                    "couldn't read the client key {}: {err}",
                    key.display()
                ))
            })?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|err| tls_error(format!("invalid client certificate: {err}")))?
        }
        _ => builder.with_no_client_auth(),
    };
    Ok(PgTls::Rustls(MakeRustlsConnect::new(client)))
}

fn tls_error(message: impl Into<String>) -> DbError {
    DbError::new(ErrorKind::Network, message)
}

fn load_certs(path: &Path) -> DbResult<Vec<CertificateDer<'static>>> {
    let certs: Vec<_> = CertificateDer::pem_file_iter(path)
        .and_then(|iter| iter.collect::<Result<Vec<_>, _>>())
        .map_err(|err| tls_error(format!("couldn't read {}: {err}", path.display())))?;
    if certs.is_empty() {
        return Err(tls_error(format!(
            "no certificate found in {}",
            path.display()
        )));
    }
    Ok(certs)
}

/// The CA file's certificates, or the OS store plus the Mozilla roots.
fn roots(ca_file: Option<&Path>) -> DbResult<RootCertStore> {
    let mut store = RootCertStore::empty();
    if let Some(path) = ca_file {
        for cert in load_certs(path)? {
            store.add(cert).map_err(|err| {
                tls_error(format!(
                    "invalid CA certificate in {}: {err}",
                    path.display()
                ))
            })?;
        }
        return Ok(store);
    }
    let native = rustls_native_certs::load_native_certs();
    for err in &native.errors {
        tracing::debug!(error = %err, "reading an OS root certificate failed");
    }
    let (added, _ignored) = store.add_parsable_certificates(native.certs);
    tracing::debug!(added, "OS root certificates");
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    Ok(store)
}

/// prefer / require: encrypt, but accept any certificate (libpq does the same).
#[derive(Debug)]
struct NoVerification {
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

/// verify-ca: the full WebPKI check, except that a host name mismatch passes.
#[derive(Debug)]
struct IgnoreHostName {
    inner: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for IgnoreHostName {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        match self
            .inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp, now)
        {
            Err(TlsError::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_build_connectors() {
        let auth = Auth::None;
        assert!(matches!(
            make_tls(
                &TlsConfig {
                    mode: SslMode::Disable,
                    ca_cert: None
                },
                &auth
            )
            .unwrap(),
            PgTls::None
        ));
        for mode in [
            SslMode::Prefer,
            SslMode::Require,
            SslMode::VerifyCa,
            SslMode::VerifyFull,
        ] {
            let tls = make_tls(
                &TlsConfig {
                    mode,
                    ca_cert: None,
                },
                &auth,
            )
            .unwrap();
            assert!(matches!(tls, PgTls::Rustls(_)), "{mode:?}");
        }
    }

    #[test]
    fn missing_ca_file_is_an_error() {
        let config = TlsConfig {
            mode: SslMode::VerifyFull,
            ca_cert: Some("no/such/ca.pem".into()),
        };
        let err = make_tls(&config, &Auth::None).err().unwrap();
        assert!(err.message.contains("ca.pem"), "{}", err.message);
    }
}
