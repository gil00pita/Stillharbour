//! M4B append planning. A review is neither approval nor an executable manifest.
use super::comparison::{
    CompareMode, ComparisonCategory as Category, ComparisonEntry, ComparisonState,
};
use crate::domain::{AppError, ErrorCode, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const APPEND_POLICY_VERSION: u32 = 1;

/// Explicit decisions for differing destination content. Replacement is M4C.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppendAction {
    CopyNew,
    KeepDestination,
    LeaveUnchanged,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendPlanItem {
    pub entry_id: String,
    pub display_path: String,
    pub action: AppendAction,
    pub source_bytes: Option<String>,
}

/// Display-only response; no Deserialize and no start/approval API accepts it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendPlan {
    pub run_id: String,
    pub comparison_digest: String,
    pub policy_version: u32,
    pub digest: String,
    pub items: Vec<AppendPlanItem>,
    pub copy_files: u32,
    pub copy_bytes: String,
    pub retained_destination_files: u32,
    pub uncopied_source_revisions: u32,
    pub execution_available: bool,
}

pub(crate) struct AppendReview<'a> {
    pub run_id: &'a str,
    pub comparison_digest: Option<&'a str>,
    pub native_scope_digest: &'a str,
    pub mode: CompareMode,
    pub state: ComparisonState,
    pub rows: &'a [ComparisonEntry],
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}

/// All report rows participate, irrespective of the currently visible page.
/// The native caller supplies an already revalidated scope fingerprint.
pub(crate) fn plan(review: AppendReview<'_>, keep_destination: &[String]) -> Result<AppendPlan> {
    let comparison_digest = review.comparison_digest.filter(|d| valid_digest(d));
    if review.mode != CompareMode::Verified
        || review.state != ComparisonState::NeedsReview
        || comparison_digest.is_none()
        || !valid_digest(review.native_scope_digest)
        || review.rows.len() > 20_000
        || keep_destination.len() > review.rows.len()
    {
        return Err(invalid(
            "Append review requires a complete verified comparison and fresh native scope.",
        ));
    }
    let mut decisions = BTreeSet::new();
    for id in keep_destination {
        if !decisions.insert(id.as_str()) {
            return Err(invalid("A keep-destination decision is repeated."));
        }
    }
    let mut rows = BTreeMap::new();
    for row in review.rows {
        if !valid_digest(&row.id) || rows.insert(row.id.as_str(), row).is_some() {
            return Err(invalid(
                "Comparison entry identities are invalid or repeated.",
            ));
        }
    }
    if decisions.iter().any(|id| !rows.contains_key(id)) {
        return Err(invalid(
            "A keep-destination decision belongs to another comparison.",
        ));
    }
    let mut items = Vec::with_capacity(rows.len());
    let mut copy_files = 0;
    let mut copy_bytes = 0u128;
    let mut retained_destination_files = 0;
    let mut uncopied_source_revisions = 0;
    for (id, row) in rows {
        use super::comparison::ComparisonEntryKind::File;
        let explicit_keep = decisions.contains(id);
        let action = match row.category {
            Category::SourceOnly
                if row.source_kind == Some(File)
                    && row.destination_kind.is_none()
                    && !explicit_keep =>
            {
                let bytes = row
                    .source_bytes
                    .as_deref()
                    .and_then(|s| s.parse::<u64>().ok())
                    .ok_or_else(|| invalid("A source-only file has no valid length."))?;
                copy_files += 1;
                copy_bytes += u128::from(bytes);
                AppendAction::CopyNew
            }
            Category::DestinationOnly
                if row.source_kind.is_none()
                    && row.destination_kind == Some(File)
                    && !explicit_keep =>
            {
                retained_destination_files += 1;
                AppendAction::KeepDestination
            }
            Category::VerifiedEqual
                if row.source_kind == Some(File)
                    && row.destination_kind == Some(File)
                    && !explicit_keep =>
            {
                AppendAction::LeaveUnchanged
            }
            Category::DifferentContent | Category::DestinationEditedSinceLastRun
                if explicit_keep
                    && row.source_kind == Some(File)
                    && row.destination_kind == Some(File) =>
            {
                retained_destination_files += 1;
                uncopied_source_revisions += 1;
                AppendAction::KeepDestination
            }
            _ => {
                return Err(invalid(
                    "Resolve every difference explicitly. Uncertain, blocked or unverified entries cannot enter an append plan.",
                ));
            }
        };
        items.push(AppendPlanItem {
            entry_id: row.id.clone(),
            display_path: row.display_path.clone(),
            action,
            source_bytes: row.source_bytes.clone(),
        });
    }
    // JSON framing prevents ambiguous concatenation. Native evidence stays private.
    let encoded = serde_json::to_vec(&(
        "stillharbour-m4b-append",
        APPEND_POLICY_VERSION,
        review.run_id,
        comparison_digest,
        review.native_scope_digest,
        &items,
    ))
    .map_err(|e| invalid("Append plan could not be fingerprinted.").caused_by(e))?;
    Ok(AppendPlan {
        run_id: review.run_id.into(),
        comparison_digest: comparison_digest.unwrap().into(),
        policy_version: APPEND_POLICY_VERSION,
        digest: blake3::hash(&encoded).to_hex().to_string(),
        items,
        copy_files,
        copy_bytes: copy_bytes.to_string(),
        retained_destination_files,
        uncopied_source_revisions,
        execution_available: false,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests;
