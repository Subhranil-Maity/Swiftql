use std::pin::Pin;

use async_trait::async_trait;
use driver_core::{
    connection::{CancelHandle, RowBatch, RowStream},
    dbresults::DbResult,
};
use futures::TryStreamExt;
use tokio_postgres::Row;

use crate::{
    cancel::PgCancel,
    error::map_err,
    tx::{self, TxCommand, TxState},
    types::decode_row,
};

/// Rows of one query, read as the server sends them. Rows that arrive
/// faster than they are read are buffered by tokio-postgres; the app reads
/// them as fast as they come into a capped result buffer (src-tauri/src/results),
/// and closing the stream at the cap cancels the query on the server.
pub struct PgRowStream {
    rows: Pin<Box<tokio_postgres::RowStream>>,
    peeked: Option<Row>, // read ahead one row so `has_more` is exact
    done: bool,
    cancel: PgCancel,
    tx: TxState,
}

impl PgRowStream {
    pub fn new(rows: tokio_postgres::RowStream, cancel: PgCancel, tx: TxState) -> Self {
        Self {
            rows: Box::pin(rows),
            peeked: None,
            done: false,
            cancel,
            tx,
        }
    }

    async fn next_row(&mut self) -> DbResult<Option<Row>> {
        if self.done {
            return Ok(None);
        }
        match self.rows.as_mut().try_next().await {
            Ok(Some(row)) => Ok(Some(row)),
            Ok(None) => {
                self.done = true;
                Ok(None)
            }
            Err(err) => {
                // e.g. division by zero halfway through: aborts an open transaction
                self.done = true;
                tx::after_error(&self.tx, TxCommand::Other);
                Err(map_err(err))
            }
        }
    }
}

#[async_trait]
impl RowStream for PgRowStream {
    async fn next_batch(&mut self, max: usize) -> DbResult<Option<RowBatch>> {
        let max = max.max(1);
        let mut rows = Vec::with_capacity(max);
        if let Some(row) = self.peeked.take() {
            rows.push(decode_row(&row));
        }
        while rows.len() < max {
            match self.next_row().await? {
                Some(row) => rows.push(decode_row(&row)),
                None => break,
            }
        }
        if rows.is_empty() {
            return Ok(None);
        }

        let has_more = match self.next_row().await? {
            Some(row) => {
                self.peeked = Some(row);
                true
            }
            None => false,
        };
        Ok(Some(RowBatch { rows, has_more }))
    }

    async fn close(mut self: Box<Self>) -> DbResult<()> {
        if self.done {
            return Ok(());
        }
        // Stop the server sending the rest, then read to the end so the
        // connection is free and a cancel inside a transaction marks it failed.
        let _ = self.cancel.cancel().await;
        while let Ok(Some(_)) = self.next_row().await {}
        Ok(())
    }
}
