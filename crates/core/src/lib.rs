//! Domain rules and owned adapters. No Tauri or renderer authority lives here.
pub mod backup;
pub mod catalog;
pub mod catalog_restore;
pub mod contract;
pub mod domain;
pub mod executor;
pub mod filesystem;
pub mod inspection;
pub mod jobs;
pub mod library;
pub mod media;
#[cfg(target_os = "macos")]
mod native_preview;
pub mod operations;
pub mod organization;
pub mod rendition;

#[cfg(feature = "fixture-probes")]
pub mod fixture_copy;

#[cfg(feature = "fixture-probes")]
pub mod library_fixture;

pub mod preferences;

pub mod performance;
#[cfg(test)]
mod test_process;
