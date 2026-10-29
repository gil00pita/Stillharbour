use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Io,
    Catalog,
    CatalogInUse,
    CatalogSchemaNewer,
    Unsupported,
    Conflict,
    InvalidInput,
    WorkerFailed,
    WorkerTimeout,
    Busy,
    MutationUnavailable,
}

/// Safe IPC error. The underlying diagnostic remains in the backend and is not
/// automatically logged (OS/SQLite errors can contain private paths).
#[derive(Debug, Clone, Serialize, Deserialize, TS, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip)]
    #[ts(skip)]
    pub cause: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            cause: None,
        }
    }

    pub fn caused_by(mut self, cause: impl std::fmt::Display) -> Self {
        self.cause = Some(cause.to_string());
        self
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, "The local filesystem operation failed.").caused_by(error)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(ErrorCode::Catalog, "The catalog operation failed.").caused_by(error)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

/// M0 has no production executor. This backend guard is independent of UI state.
pub fn require_original_mutation() -> Result<()> {
    Err(AppError::new(
        ErrorCode::MutationUnavailable,
        "Original-file operations require the reviewed plan, executor and recovery gates.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutations_fail_closed_and_private_causes_do_not_cross_ipc() {
        assert_eq!(
            require_original_mutation().unwrap_err().code,
            ErrorCode::MutationUnavailable
        );
        let error = AppError::new(ErrorCode::Io, "Cannot read the selected folder.")
            .caused_by("/private/family/photo.jpg");
        assert!(!serde_json::to_string(&error).unwrap().contains("family"));
        assert!(error.cause.is_some());
    }
}
