use serde::Serialize;

/// A message the server sent alongside a statement that is not an error,
/// e.g. Postgres `RAISE NOTICE` or the warning from `VACUUM` on a missing table.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub severity: NoticeSeverity,
    pub message: String,
    pub code: Option<String>, // SQLSTATE or MySQL warning code
    pub detail: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum NoticeSeverity {
    Warning,
    Notice,
    Info,
    Log,
    Debug,
}
