use std::{error::Error, io};

use driver_core::error::{DbError, ErrorKind};
use tokio_postgres::error::ErrorPosition;

/// tokio-postgres error → DbError.
pub fn map_err(err: tokio_postgres::Error) -> DbError {
    if let Some(db) = err.as_db_error() {
        let code = db.code().code();
        let mut out =
            DbError::new(kind_for_sqlstate(code, db.message()), db.message()).with_code(code);
        // Postgres counts characters from 1; DbError counts from 0.
        // Internal positions point into a server-generated query, not the user's SQL.
        if let Some(ErrorPosition::Original(pos)) = db.position() {
            out = out.with_position(pos.saturating_sub(1) as usize);
        }
        if let Some(hint) = db.hint() {
            out = out.with_hint(hint);
        }
        if let Some(detail) = db.detail() {
            out = out.with_detail(detail);
        }
        return out;
    }

    let io_error = err.source().and_then(|e| e.downcast_ref::<io::Error>());
    let kind = match io_error {
        Some(e) if e.kind() == io::ErrorKind::TimedOut => ErrorKind::Timeout,
        Some(_) => ErrorKind::Network,
        None if err.is_closed() => ErrorKind::Network,
        None => ErrorKind::Internal,
    };
    DbError::new(kind, err.to_string())
}

/// SQLSTATE → ErrorKind. Specific codes first, then the two-character class.
fn kind_for_sqlstate(code: &str, message: &str) -> ErrorKind {
    match code {
        "42501" => ErrorKind::Permission, // insufficient_privilege
        // undefined table / column / function / object, schema, database
        "42P01" | "42703" | "42883" | "42704" | "3F000" | "3D000" => ErrorKind::NotFound,
        // statement_timeout and a user cancel share 57014. TODO: the message is
        // localized (lc_messages); track timeouts on our side instead.
        "57014" if message.contains("statement timeout") => ErrorKind::Timeout,
        "57014" => ErrorKind::Cancelled,
        // lock_timeout, idle_in_transaction_session_timeout
        "55P03" | "25P03" => ErrorKind::Timeout,
        "0A000" => ErrorKind::Unsupported, // feature_not_supported
        _ => match code.get(..2) {
            Some("28") => ErrorKind::Auth,                 // invalid authorization
            Some("08") | Some("57") => ErrorKind::Network, // connection exception, server shutdown
            Some("23") => ErrorKind::Constraint,           // integrity constraint violation
            // syntax error / access rule; data exception (e.g. 'abc'::int).
            // No dedicated kind for bad input; Syntax is the closest.
            Some("42") | Some("22") => ErrorKind::Syntax,
            _ => ErrorKind::Internal,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlstate_mapping() {
        assert_eq!(kind_for_sqlstate("42601", ""), ErrorKind::Syntax);
        assert_eq!(kind_for_sqlstate("42P01", ""), ErrorKind::NotFound);
        assert_eq!(kind_for_sqlstate("42501", ""), ErrorKind::Permission);
        assert_eq!(kind_for_sqlstate("23505", ""), ErrorKind::Constraint);
        assert_eq!(kind_for_sqlstate("28P01", ""), ErrorKind::Auth);
        assert_eq!(kind_for_sqlstate("08006", ""), ErrorKind::Network);
        assert_eq!(
            kind_for_sqlstate("57014", "canceling statement due to user request"),
            ErrorKind::Cancelled
        );
        assert_eq!(
            kind_for_sqlstate("57014", "canceling statement due to statement timeout"),
            ErrorKind::Timeout
        );
        assert_eq!(kind_for_sqlstate("XX000", ""), ErrorKind::Internal);
    }
}
