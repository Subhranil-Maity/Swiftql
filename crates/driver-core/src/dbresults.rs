use crate::error::DbError;

pub type DbResult<T> = Result<T, DbError>;
