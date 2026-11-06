use async_trait::async_trait;
use driver_core::{connection::CancelHandle, dbresults::DbResult};
use tokio_postgres::{CancelToken, NoTls, tls::MakeTlsConnect};

use crate::{
    error::map_err,
    tls::PgTls,
    tunnel::{Route, ssh_error},
};

/// Sends a cancel request on a new connection to the server, through the
/// same route (SSH tunnel) and TLS as the session.
pub struct PgCancel {
    pub(crate) token: CancelToken,
    pub(crate) tls: PgTls,
    pub(crate) route: Route,
}

#[async_trait]
impl CancelHandle for PgCancel {
    async fn cancel(&self) -> DbResult<()> {
        match (&self.route, &self.tls) {
            (Route::Direct, PgTls::None) => self.token.cancel_query(NoTls).await,
            (Route::Direct, PgTls::Rustls(tls)) => self.token.cancel_query(tls.clone()).await,
            (Route::Tunnel { tunnel, host, port }, tls) => {
                let stream = tunnel.dial(host, *port).await.map_err(ssh_error)?;
                match tls {
                    PgTls::None => self.token.cancel_query_raw(stream, NoTls).await,
                    PgTls::Rustls(make) => {
                        let connect = MakeTlsConnect::<ssh_tunnel::TunnelStream>::make_tls_connect(
                            &mut make.clone(),
                            host,
                        )
                        .map_err(|err| {
                            driver_core::error::DbError::new(
                                driver_core::error::ErrorKind::Network,
                                err.to_string(),
                            )
                        })?;
                        self.token.cancel_query_raw(stream, connect).await
                    }
                }
            }
        }
        .map_err(map_err)
    }
}
