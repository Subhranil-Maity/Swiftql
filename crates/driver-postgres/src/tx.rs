use std::sync::{Arc, Mutex};

use driver_core::connection::TxStatus;

use crate::sql::leading_keywords;

/// Shared between the connection and its open row streams, since a SELECT
/// can fail (and abort the transaction) while its rows are being read.
pub type TxState = Arc<Mutex<TxStatus>>;

/// tokio-postgres doesn't expose the server's transaction status, so it is
/// tracked from the statements we run.
/// TODO: procedures can COMMIT / ROLLBACK inside CALL, which isn't seen here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxCommand {
    Begin,      // BEGIN, START TRANSACTION, COMMIT/ROLLBACK AND CHAIN
    End,        // COMMIT, END, ROLLBACK, ABORT, PREPARE TRANSACTION
    RollbackTo, // ROLLBACK TO SAVEPOINT: leaves a failed transaction usable again
    Other,
}

impl TxCommand {
    pub fn of(sql: &str) -> Self {
        let words = leading_keywords(sql, 5);
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        let chain = words.contains(&"CHAIN") && !words.contains(&"NO");
        match words.as_slice() {
            ["BEGIN", ..] | ["START", "TRANSACTION", ..] => Self::Begin,
            // the *_PREPARED forms act on a prepared transaction, not this session's
            ["COMMIT" | "ROLLBACK", "PREPARED", ..] => Self::Other,
            ["ROLLBACK", "TO", ..] | ["ROLLBACK", "WORK" | "TRANSACTION", "TO", ..] => {
                Self::RollbackTo
            }
            ["COMMIT" | "END" | "ROLLBACK" | "ABORT", ..] if chain => Self::Begin,
            ["COMMIT" | "END" | "ROLLBACK" | "ABORT", ..] => Self::End,
            ["PREPARE", "TRANSACTION", ..] => Self::End,
            _ => Self::Other,
        }
    }
}

pub fn get(state: &TxState) -> TxStatus {
    *state.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn after_success(state: &TxState, command: TxCommand) {
    let mut status = state.lock().unwrap_or_else(|e| e.into_inner());
    *status = match command {
        TxCommand::Begin | TxCommand::RollbackTo => TxStatus::InTransaction,
        TxCommand::End => TxStatus::Idle, // COMMIT of a failed transaction rolls back
        TxCommand::Other => *status,
    };
}

/// Any server error inside a transaction aborts it.
pub fn after_error(state: &TxState, command: TxCommand) {
    let mut status = state.lock().unwrap_or_else(|e| e.into_inner());
    *status = match (command, *status) {
        (TxCommand::End, _) => TxStatus::Idle, // a failed COMMIT still ends the transaction
        (_, TxStatus::Idle) => TxStatus::Idle,
        _ => TxStatus::Failed,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        assert_eq!(TxCommand::of("begin"), TxCommand::Begin);
        assert_eq!(
            TxCommand::of("START TRANSACTION ISOLATION LEVEL SERIALIZABLE"),
            TxCommand::Begin
        );
        assert_eq!(TxCommand::of("commit"), TxCommand::End);
        assert_eq!(TxCommand::of("rollback"), TxCommand::End);
        assert_eq!(
            TxCommand::of("ROLLBACK TO SAVEPOINT a"),
            TxCommand::RollbackTo
        );
        assert_eq!(TxCommand::of("rollback work to a"), TxCommand::RollbackTo);
        assert_eq!(TxCommand::of("commit and chain"), TxCommand::Begin);
        assert_eq!(TxCommand::of("commit and no chain"), TxCommand::End);
        assert_eq!(TxCommand::of("COMMIT PREPARED 'x'"), TxCommand::Other);
        assert_eq!(TxCommand::of("select 1"), TxCommand::Other);
    }

    #[test]
    fn transitions() {
        let state: TxState = Arc::new(Mutex::new(TxStatus::Idle));
        after_error(&state, TxCommand::Other);
        assert_eq!(get(&state), TxStatus::Idle); // errors outside a transaction don't matter

        after_success(&state, TxCommand::Begin);
        after_success(&state, TxCommand::Other);
        assert_eq!(get(&state), TxStatus::InTransaction);

        after_error(&state, TxCommand::Other);
        assert_eq!(get(&state), TxStatus::Failed);
        after_error(&state, TxCommand::Other); // 25P02 while aborted
        assert_eq!(get(&state), TxStatus::Failed);

        after_success(&state, TxCommand::RollbackTo);
        assert_eq!(get(&state), TxStatus::InTransaction);

        after_success(&state, TxCommand::End);
        assert_eq!(get(&state), TxStatus::Idle);
    }
}
