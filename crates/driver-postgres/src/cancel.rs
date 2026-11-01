use async_trait::async_trait;
use driver_core::{connection::CancelHandle, dbresults::DbResult};
use tokio_postgres::{CancelToken, NoTls};

use crate::{error::map_err, tls::PgTls};

pub struct PgCancel {
    pub(crate) token: CancelToken,
    pub(crate) tls: PgTls,
}

#[async_trait]
impl CancelHandle for PgCancel {
    async fn cancel(&self) -> DbResult<()> {
        match &self.tls {
            PgTls::None => self.token.cancel_query(NoTls).await,
            PgTls::Native(tls) => self.token.cancel_query(tls.clone()).await,
        }
        .map_err(map_err)
    }
}
