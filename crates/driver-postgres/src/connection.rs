use async_trait::async_trait;
use driver_core::{
    capabilities::Capabilities,
    connection::{CancelHandle, Connection, ExecOutcome, ServerVersion, TxStatus},
    dbresults::DbResult,
    dialect::Dialect,
    introspec::{IntrospectScope, SchemaSnapshot},
};
use tokio_postgres::Client;

use crate::{cancel::PgCancel, dialect::PgDialect, error::map_err, introspect, tls::PgTls};

pub struct PgConnection {
    pub(crate) client: Client,
    pub(crate) dialect: PgDialect,
    pub(crate) version: ServerVersion,
    pub(crate) capabilities: Capabilities, // version-dependent
    pub(crate) tls: PgTls,                 // reused for cancel requests
}

#[async_trait]
impl Connection for PgConnection {
    fn dialect(&self) -> &dyn Dialect {
        &self.dialect
    }

    fn server_version(&self) -> &ServerVersion {
        &self.version
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn tx_status(&self) -> TxStatus {
        todo!("track BEGIN / COMMIT / ROLLBACK and failed statements")
    }

    async fn execute(&self, _sql: &str) -> DbResult<ExecOutcome> {
        // TODO: prepare; has columns → Rows (PgRowStream), else Affected
        todo!()
    }

    fn cancel_handle(&self) -> Box<dyn CancelHandle> {
        Box::new(PgCancel {
            token: self.client.cancel_token(),
            tls: self.tls.clone(),
        })
    }

    async fn introspect(&self, scope: IntrospectScope) -> DbResult<SchemaSnapshot> {
        introspect::introspect(&self.client, &self.version, scope).await
    }

    async fn ping(&self) -> DbResult<()> {
        self.client
            .simple_query("SELECT 1")
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn close(self: Box<Self>) {
        // dropping the client ends the spawned connection task
    }
}
