use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use driver_core::{
    capabilities::Capabilities,
    connect::{AuthKind, ConnectConfig, FieldSpec, TargetKind},
    connection::Connection,
    dbresults::DbResult,
    driver::Driver,
};

use crate::{capabilities, config, connection::PgConnection, tls};

const IMAGE_BYTES: &[u8] = include_bytes!("../../../assets/postgresql.svg");

pub struct PgDriver;

#[async_trait]
impl Driver for PgDriver {
    fn name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn id(&self) -> &'static str {
        "postgres"
    }

    fn image(&self) -> Option<String> {
        Some(STANDARD.encode(IMAGE_BYTES))
    }

    fn capabilities(&self) -> Capabilities {
        capabilities::static_capabilities()
    }

    fn supported_targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Url, TargetKind::Host, TargetKind::Socket]
    }

    fn supported_auth(&self) -> &'static [AuthKind] {
        &[
            AuthKind::Password,
            AuthKind::PasswordPrompt,
            // AuthKind::PgPass,
            // AuthKind::ClientCert, // Not needed for now
        ]
    }

    fn extra_options(&self) -> Vec<FieldSpec> {
        // todo!("application_name, search_path, connect_timeout, ...")
        vec![]
    }

    fn parse_url(&self, url: &str) -> DbResult<ConnectConfig> {
        config::parse_url(url)
    }

    async fn test_conn(&self, config: &ConnectConfig) -> DbResult<()> {
        let conn = self.connect(config).await?;
        let result = conn.ping().await;
        conn.close().await;
        result
    }

    async fn connect(&self, config: &ConnectConfig) -> DbResult<Box<dyn Connection>> {
        let pg_config = config::to_pg_config(config)?;
        let tls = tls::make_tls(&config.tls)?;
        Ok(Box::new(PgConnection::open(&pg_config, tls).await?))
    }
}
