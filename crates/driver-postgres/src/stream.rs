use std::pin::Pin;

use async_trait::async_trait;
use driver_core::{
    connection::{RowBatch, RowStream},
    dbresults::DbResult,
};

pub struct PgRowStream {
    pub(crate) rows: Pin<Box<tokio_postgres::RowStream>>,
}

#[async_trait]
impl RowStream for PgRowStream {
    async fn next_batch(&mut self, _max: usize) -> DbResult<Option<RowBatch>> {
        // TODO: pull up to `max` rows from self.rows, decode with types::decode_row
        todo!()
    }

    async fn close(self: Box<Self>) -> DbResult<()> {
        // TODO: close the portal/cursor on the server if one is used
        Ok(())
    }
}
