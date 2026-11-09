use driver_core::error::{DbError, ErrorKind};
use rusqlite::{Error, ErrorCode};

/// rusqlite error → DbError. `timed_out` says whether an interrupt came from
/// the query timeout rather than a user cancel.
pub fn map_err(err: Error, timed_out: bool) -> DbError {
    match err {
        Error::SqliteFailure(e, msg) => {
            let message = msg.unwrap_or_else(|| e.to_string());
            failure(e.code, message, timed_out)
        }
        Error::SqlInputError {
            error,
            msg,
            sql,
            offset,
        } => {
            let mut out = failure(error.code, msg, timed_out);
            // SQLite gives a byte offset; DbError counts characters from 0.
            if let Ok(offset) = usize::try_from(offset)
                && let Some(before) = sql.get(..offset)
            {
                out = out.with_position(before.chars().count());
            }
            out
        }
        Error::MultipleStatement => DbError::new(
            ErrorKind::Unsupported,
            "multiple statements are not supported yet; run one statement at a time",
        ),
        Error::InvalidQuery => DbError::new(ErrorKind::Syntax, "the statement is empty"),
        other => DbError::new(ErrorKind::Internal, other.to_string()),
    }
}

fn failure(code: ErrorCode, message: String, timed_out: bool) -> DbError {
    let kind = match code {
        ErrorCode::OperationInterrupted if timed_out => {
            return DbError::new(ErrorKind::Timeout, "Query timed out").with_code(code_name(code));
        }
        ErrorCode::OperationInterrupted => ErrorKind::Cancelled,
        ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => ErrorKind::Busy,
        ErrorCode::ConstraintViolation => ErrorKind::Constraint,
        ErrorCode::PermissionDenied
        | ErrorCode::AuthorizationForStatementDenied
        | ErrorCode::ReadOnly => ErrorKind::Permission,
        ErrorCode::CannotOpen | ErrorCode::NotFound => ErrorKind::NotFound,
        ErrorCode::TypeMismatch => ErrorKind::Syntax,
        // SQLITE_ERROR covers both bad SQL and missing objects. SQLite's
        // messages aren't translated, so reading them is safe.
        ErrorCode::Unknown if message.starts_with("no such ") => ErrorKind::NotFound,
        ErrorCode::Unknown => ErrorKind::Syntax,
        _ => ErrorKind::Internal,
    };
    DbError::new(kind, message).with_code(code_name(code))
}

fn code_name(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InternalMalfunction => "SQLITE_INTERNAL",
        ErrorCode::PermissionDenied => "SQLITE_PERM",
        ErrorCode::OperationAborted => "SQLITE_ABORT",
        ErrorCode::DatabaseBusy => "SQLITE_BUSY",
        ErrorCode::DatabaseLocked => "SQLITE_LOCKED",
        ErrorCode::OutOfMemory => "SQLITE_NOMEM",
        ErrorCode::ReadOnly => "SQLITE_READONLY",
        ErrorCode::OperationInterrupted => "SQLITE_INTERRUPT",
        ErrorCode::SystemIoFailure => "SQLITE_IOERR",
        ErrorCode::DatabaseCorrupt => "SQLITE_CORRUPT",
        ErrorCode::NotFound => "SQLITE_NOTFOUND",
        ErrorCode::DiskFull => "SQLITE_FULL",
        ErrorCode::CannotOpen => "SQLITE_CANTOPEN",
        ErrorCode::FileLockingProtocolFailed => "SQLITE_PROTOCOL",
        ErrorCode::SchemaChanged => "SQLITE_SCHEMA",
        ErrorCode::TooBig => "SQLITE_TOOBIG",
        ErrorCode::ConstraintViolation => "SQLITE_CONSTRAINT",
        ErrorCode::TypeMismatch => "SQLITE_MISMATCH",
        ErrorCode::ApiMisuse => "SQLITE_MISUSE",
        ErrorCode::NoLargeFileSupport => "SQLITE_NOLFS",
        ErrorCode::AuthorizationForStatementDenied => "SQLITE_AUTH",
        ErrorCode::ParameterOutOfRange => "SQLITE_RANGE",
        ErrorCode::NotADatabase => "SQLITE_NOTADB",
        _ => "SQLITE_ERROR",
    }
}
