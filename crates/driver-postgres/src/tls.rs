use driver_core::{
    connect::{SslMode, TlsConfig},
    dbresults::DbResult,
    error::{DbError, ErrorKind},
};
use postgres_native_tls::MakeTlsConnector;

#[derive(Clone)]
pub enum PgTls {
    None,
    Native(MakeTlsConnector), // TODO: built by make_tls once TLS is supported
}

/// SslMode + CA cert → connector.
///
/// Only plain connections for now. TODO, when TLS is needed:
/// - Prefer: try TLS without verifying the certificate (libpq behaviour), fall back
///   to plain. Today it is always plain, because `NoTls` makes tokio-postgres skip TLS.
/// - Require: TLS, but accept any certificate and hostname.
/// - VerifyCa: verify the chain, not the hostname (`danger_accept_invalid_hostnames`).
/// - VerifyFull: verify the chain and the hostname (native-tls default).
/// - `ca_cert`: load the PEM and add it with `add_root_certificate`.
/// - Auth::ClientCert: load cert + key as an `Identity` (`Identity::from_pkcs8`).
///   make_tls will need the auth for this, not just `TlsConfig`.
pub fn make_tls(config: &TlsConfig) -> DbResult<PgTls> {
    match config.mode {
        SslMode::Disable | SslMode::Prefer => Ok(PgTls::None),
        SslMode::Require | SslMode::VerifyCa | SslMode::VerifyFull => Err(DbError::new(
            ErrorKind::Unsupported,
            "TODO: TLS connections are not supported yet",
        )),
    }
}
