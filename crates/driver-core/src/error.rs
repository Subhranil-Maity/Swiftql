use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Syntax,
    Auth,
    Network,
    Permission,
    Constraint,
    Cancelled,
    Timeout,
    NotFound,
    Unsupported,
    Internal,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct DbError {
    pub kind: ErrorKind, // Syntax, Auth, Network, Permission, Constraint, Cancelled, ...
    pub message: String,
    pub code: Option<String>,    // SQLSTATE or MySQL error code
    pub position: Option<usize>, // character offset for editor highlighting
    pub hint: Option<String>,
    pub detail: Option<String>,
}

impl DbError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            code: None,
            position: None,
            hint: None,
            detail: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_position(mut self, position: usize) -> Self {
        self.position = Some(position);
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
