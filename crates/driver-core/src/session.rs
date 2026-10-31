use std::time::Instant;

use uuid::Uuid;

use crate::connection::{CancelHandle, Connection};

pub struct Session {
    id: Uuid,
    connection: Box<dyn Connection>,
    current_query: Option<RunningQuery>,
}

pub struct RunningQuery {
    id: Uuid,
    started_at: Instant,
    cancel: Box<dyn CancelHandle>,
    // cursor/portal for fetching more pages
}

// in app state
// sessions: HashMap<Uuid, Session>
