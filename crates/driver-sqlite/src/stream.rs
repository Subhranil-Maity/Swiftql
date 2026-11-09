use std::sync::mpsc;

use async_trait::async_trait;
use driver_core::{
    connection::{RowBatch, RowStream},
    dbresults::DbResult,
};
use tokio::sync::oneshot;

use crate::worker::{Job, gone};

/// Rows of one statement that stays open on the worker thread; each batch
/// is read there when asked for.
pub struct SqliteRowStream {
    pub(crate) id: u64,
    pub(crate) jobs: mpsc::Sender<Job>,
    pub(crate) done: bool,
}

#[async_trait]
impl RowStream for SqliteRowStream {
    async fn next_batch(&mut self, max: usize) -> DbResult<Option<RowBatch>> {
        if self.done {
            return Ok(None);
        }
        let (reply, rx) = oneshot::channel();
        self.jobs
            .send(Job::Fetch {
                stream_id: self.id,
                max,
                reply,
            })
            .map_err(|_| gone())?;
        let result = rx.await.map_err(|_| gone())?;
        match &result {
            Ok(Some(batch)) if batch.has_more => {}
            _ => self.done = true,
        }
        result
    }

    async fn close(mut self: Box<Self>) -> DbResult<()> {
        if self.done {
            return Ok(());
        }
        self.done = true;
        let (reply, rx) = oneshot::channel();
        if self
            .jobs
            .send(Job::CloseStream {
                stream_id: self.id,
                reply: Some(reply),
            })
            .is_ok()
        {
            let _ = rx.await;
        }
        Ok(())
    }
}

impl Drop for SqliteRowStream {
    /// Dropped without `close`: still free the statement, or the worker
    /// would wait for fetches that never come.
    fn drop(&mut self) {
        if !self.done {
            let _ = self.jobs.send(Job::CloseStream {
                stream_id: self.id,
                reply: None,
            });
        }
    }
}
