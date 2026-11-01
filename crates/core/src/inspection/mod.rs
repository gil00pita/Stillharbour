//! Read-only duplicate, health and storage evidence. Reports never grant mutation.
pub(crate) mod candidates;
pub mod decoder;
pub(crate) mod hashing;
pub(crate) mod keepers;
pub mod model;
pub(crate) mod rules;
pub(crate) mod storage;
pub use keepers::{KeeperBlock, KeeperBlockReason, KeeperOrder, KeeperReview, KeeperReviewInput};
pub use storage::{FolderSize, InspectionScope, InventoryCoverage, StorageReport};

use crate::domain::{AppError, ErrorCode};
pub(crate) fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
pub(crate) fn changed(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}
