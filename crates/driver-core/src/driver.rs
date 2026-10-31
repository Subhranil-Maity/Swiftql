use async_trait::async_trait;

use crate::{
    capabilities::Capabilities,
    connect::{AuthKind, ConnectConfig, FieldSpec, TargetKind},
    connection::Connection,
    dbresults::DbResult,
};

#[async_trait]
pub trait Driver: Send + Sync {
    fn name(&self) -> &'static str; // "PostgreSQL"
    fn id(&self) -> &'static str; // "postgres": stable key for registry / saved connections
    fn image(&self) -> Option<String>; // base64 encoded image

    /// Static capabilities, known before connecting (drive the connection form).
    /// Version-dependent ones live on `Connection::capabilities`.
    fn capabilities(&self) -> Capabilities;

    fn supported_targets(&self) -> &'static [TargetKind]; // PG: Url, Host, Socket   SQLite: File
    fn supported_auth(&self) -> &'static [AuthKind]; // SQLite: None
    fn extra_options(&self) -> Vec<FieldSpec>; // describes ConnectConfig::options

    /// Turn a pasted URL into a config (also used to prefill the form).
    fn parse_url(&self, url: &str) -> DbResult<ConnectConfig>;

    /// Unsupported target/auth → Err(ErrorKind::Unsupported).
    async fn test_conn(&self, config: &ConnectConfig) -> DbResult<()>;
    async fn connect(&self, config: &ConnectConfig) -> DbResult<Box<dyn Connection>>;
}
