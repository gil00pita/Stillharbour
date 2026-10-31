//! Pure, bounded template previews. These are catalog observations, never
//! executable plans, approvals, destination reservations or recovery evidence.
use crate::domain::{AppError, ErrorCode, Result};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub const PREVIEW_LIMIT: usize = 240;
pub const COMPANION_REVIEW_LIMIT: usize = 64;
const POLICY: &str = "organization-preview-v1";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationCompanion {
    pub asset_id: String,
    pub source_path: String,
    pub source_revision: String,
    pub bytes: String,
    pub presence: String,
    pub root_availability: String,
    pub is_media: bool,
    pub filename_sidecar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationCompanionReview {
    pub asset_id: String,
    pub catalog_revision: String,
    pub fingerprint: String,
    pub candidates: Vec<OrganizationCompanion>,
    pub truncated: bool,
    pub inventory_complete: bool,
    pub execution_available: bool,
}

/// Read-only observation, never an approval or a persisted execution plan.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationDestinationReview {
    pub preview_fingerprint: String,
    pub catalog_revision: String,
    pub destination_path: String,
    pub fingerprint: String,
    pub execution_available: bool,
    pub persisted: bool,
    pub targets: Vec<OrganizationTargetReview>,
    pub available_bytes: String,
    pub total_bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationTargetState {
    UnoccupiedObserved,
    Occupied,
    AncestorNotDirectory,
    UnsupportedAncestor,
    PreviewBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationTargetReview {
    pub asset_id: String,
    pub target_relative_path: Option<String>,
    pub state: OrganizationTargetState,
    pub blocking_component: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingDatePolicy {
    Undated,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationTemplate {
    pub pattern: String,
    pub event: String,
    pub missing_date: MissingDatePolicy,
}

/// Synthetic examples only: slash-separated portable names, never native locators.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateSample {
    pub relative_path: String,
    pub capture: Option<String>,
    pub sequence: u32,
    pub expected_target: String,
}

pub(crate) fn validate_samples(
    template: &OrganizationTemplate,
    samples: &[TemplateSample],
) -> Result<()> {
    if samples.len() > 4 {
        return Err(invalid("Use at most four template examples."));
    }
    let components = parse(&template.pattern)?;
    for (index, sample) in samples.iter().enumerate() {
        if sample.relative_path.len() > 256
            || sample.relative_path.split('/').count() > 32
            || !sample.relative_path.split('/').all(valid_component)
            || sample.expected_target.is_empty()
            || sample.expected_target.len() > 1024
            || !(1..=PREVIEW_LIMIT as u32).contains(&sample.sequence)
            || sample.capture.as_ref().is_some_and(|date| {
                date.len() > 32
                    || NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.f").is_err()
            })
        {
            return Err(invalid(&format!(
                "Example {} needs a portable relative source name (at most 256 bytes), a valid capture-local date or no date, a sequence from 1–240 and an expected target (at most 1,024 bytes).",
                index + 1
            )));
        }
        let source = PreviewSource {
            id: String::new(),
            root_id: String::new(),
            root_identity: String::new(),
            native_path: Vec::new(),
            relative_path: Some(sample.relative_path.clone()),
            display_path: sample.relative_path.clone(),
            revision: String::new(),
            bytes: 0,
            capture: sample.capture.clone(),
            available: true,
            companion_candidates: 0,
        };
        match resolve(&components, template, &source, sample.sequence) {
            Ok((target, _)) if target == sample.expected_target => {}
            Ok((target, _)) => {
                return Err(invalid(&format!(
                    "Example {} does not match. The template produces: {target}",
                    index + 1
                )));
            }
            Err(issue) => {
                let reason = match issue {
                    OrganizationIssue::MissingCaptureDate => "a capture date is required",
                    OrganizationIssue::MissingValue => "a required token value is missing",
                    _ => "the resolved name violates the portable naming or extension policy",
                };
                return Err(invalid(&format!(
                    "Example {} is blocked: {reason}. Fix the example or template before saving.",
                    index + 1
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationIssue {
    MissingCaptureDate,
    MissingValue,
    InvalidPath,
    SourceUnavailable,
    NonUnicodePath,
    Collision,
    PossibleCaseCollision,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationRow {
    pub asset_id: String,
    pub root_id: String,
    pub source_path: String,
    pub source_revision: String,
    /// Display-only components derived by the backend from the lossless native
    /// relative path. Old immutable rows omit this field and retain their digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_components: Option<Vec<String>>,
    pub target_relative_path: Option<String>,
    pub sequence: u32,
    pub bytes: String,
    pub issues: Vec<OrganizationIssue>,
    pub used_undated: bool,
    /// Same-stem indexed candidates are not confirmed bundles.
    pub companion_candidates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPreview {
    pub policy: String,
    pub fingerprint: String,
    pub catalog_revision: String,
    pub template: OrganizationTemplate,
    pub rows: Vec<OrganizationRow>,
    pub proposed: u32,
    pub blocked: u32,
    pub selected_bytes: String,
    pub execution_available: bool,
}

/// Private native locators and identities participate in the fingerprint but
/// never become renderer execution inputs. All values come from one DB snapshot.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PreviewSource {
    pub id: String,
    pub root_id: String,
    pub root_identity: String,
    pub native_path: Vec<u8>,
    pub relative_path: Option<String>,
    pub display_path: String,
    pub revision: String,
    pub bytes: u64,
    pub capture: Option<String>,
    pub available: bool,
    pub companion_candidates: u32,
}

enum Piece {
    Literal(String),
    Stem,
    Folder,
    Extension,
    Event,
    Capture(String),
    Sequence(usize),
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}

fn parse(pattern: &str) -> Result<Vec<Vec<Piece>>> {
    if pattern.is_empty() || pattern.len() > 1024 || pattern.split('/').count() > 32 {
        return Err(invalid(
            "Use a template of 1–1024 bytes and at most 32 path components.",
        ));
    }
    pattern.split('/').map(parse_component).collect()
}

fn parse_component(mut rest: &str) -> Result<Vec<Piece>> {
    if rest.is_empty() || matches!(rest, "." | "..") {
        return Err(invalid(
            "Template paths must be relative, without traversal or empty components.",
        ));
    }
    let mut pieces = Vec::new();
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix('{') {
            let (token, remaining) = tail
                .split_once('}')
                .ok_or_else(|| invalid("Close every template token with }."))?;
            pieces.push(parse_token(token)?);
            rest = remaining;
        } else {
            let end = rest.find('{').unwrap_or(rest.len());
            let literal = &rest[..end];
            if literal.contains('}') || literal.chars().any(illegal) {
                return Err(invalid(
                    "Template literals contain an unmatched brace or a forbidden path character.",
                ));
            }
            pieces.push(Piece::Literal(literal.into()));
            rest = &rest[end..];
        }
    }
    Ok(pieces)
}

fn parse_token(token: &str) -> Result<Piece> {
    Ok(match token {
        "original_stem" => Piece::Stem,
        "original_folder" => Piece::Folder,
        "ext" => Piece::Extension,
        "event" => Piece::Event,
        _ if token.starts_with("capture:") => {
            let format = &token[8..];
            validate_date_format(format)?;
            Piece::Capture(format.into())
        }
        _ if token.starts_with("sequence:") => {
            let width = match &token[9..] {
                "01" => 1,
                "02" => 2,
                "03" => 3,
                "04" => 4,
                "05" => 5,
                "06" => 6,
                _ => return Err(invalid("Sequence width must be 01 through 06.")),
            };
            Piece::Sequence(width)
        }
        _ => {
            return Err(invalid(
                "Unknown template token. Use capture, original_stem, original_folder, ext, event or sequence.",
            ));
        }
    })
}

fn validate_date_format(format: &str) -> Result<()> {
    if format.is_empty() {
        return Err(invalid("Capture tokens need a date format."));
    }
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if !chars.next().is_some_and(|c| "YmdHMS".contains(c)) {
                return Err(invalid(
                    "Allowed capture directives: %Y, %m, %d, %H, %M, %S.",
                ));
            }
        } else if !matches!(c, '-' | '_' | '.') {
            return Err(invalid(
                "Date formats allow only date directives, hyphens, underscores and periods.",
            ));
        }
    }
    Ok(())
}

fn illegal(c: char) -> bool {
    c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
}

fn valid_component(value: &str) -> bool {
    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    let device = matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        base.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    !value.is_empty()
        && !matches!(value, "." | "..")
        && !value.ends_with(['.', ' '])
        && value.len() <= 255
        && value.encode_utf16().count() <= 255
        && !value.chars().any(illegal)
        && !device
}

fn resolve(
    components: &[Vec<Piece>],
    template: &OrganizationTemplate,
    source: &PreviewSource,
    sequence: u32,
) -> std::result::Result<(String, bool), OrganizationIssue> {
    use OrganizationIssue::*;
    let path = std::path::Path::new(source.relative_path.as_deref().ok_or(NonUnicodePath)?);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or(MissingValue)?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or(MissingValue)?;
    let date = source
        .capture
        .as_deref()
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").ok());
    let uses_date = components
        .iter()
        .flatten()
        .any(|p| matches!(p, Piece::Capture(_)));
    let undated = uses_date && date.is_none();
    if undated && template.missing_date == MissingDatePolicy::Block {
        return Err(MissingCaptureDate);
    }
    let target = if undated {
        // The documented fallback replaces the entire pattern, not only one token.
        if !valid_component(stem) || !valid_component(ext) {
            return Err(InvalidPath);
        }
        format!("Undated/{stem}.{ext}")
    } else {
        let mut rendered = Vec::with_capacity(components.len());
        for component in components {
            let mut value = String::new();
            for piece in component {
                let token = match piece {
                    Piece::Literal(s) => {
                        value.push_str(s);
                        continue;
                    }
                    Piece::Stem => stem.to_owned(),
                    Piece::Extension => ext.to_owned(),
                    Piece::Folder => path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .ok_or(MissingValue)?
                        .to_owned(),
                    Piece::Event => {
                        if template.event.is_empty() {
                            return Err(MissingValue);
                        }
                        template.event.clone()
                    }
                    Piece::Capture(format) => {
                        date.ok_or(MissingCaptureDate)?.format(format).to_string()
                    }
                    Piece::Sequence(width) => format!("{sequence:0width$}"),
                };
                // Validate token values before concatenation: a slash cannot
                // inject a new component or turn a rejected value into a path.
                if !valid_component(&token) {
                    return Err(InvalidPath);
                }
                value.push_str(&token);
            }
            if !valid_component(&value) {
                return Err(InvalidPath);
            }
            rendered.push(value);
        }
        rendered.join("/")
    };
    if target.len() > 1024
        || target.split('/').any(|c| !valid_component(c))
        || std::path::Path::new(&target)
            .extension()
            .and_then(|s| s.to_str())
            != Some(ext)
    {
        return Err(InvalidPath);
    }
    Ok((target, undated))
}

pub(crate) fn preview(
    catalog_revision: String,
    template: OrganizationTemplate,
    mut sources: Vec<PreviewSource>,
) -> Result<OrganizationPreview> {
    if sources.is_empty() || sources.len() > PREVIEW_LIMIT || template.event.len() > 512 {
        return Err(invalid(
            "Choose 1–240 individual photos and an event of at most 512 bytes.",
        ));
    }
    let components = parse(&template.pattern)?;
    sources.sort_by(|a, b| {
        (&a.root_id, &a.native_path, &a.id).cmp(&(&b.root_id, &b.native_path, &b.id))
    });
    let identity = serde_json::to_vec(&(POLICY, &catalog_revision, &template, &sources))
        .map_err(|e| invalid("Could not fingerprint the preview.").caused_by(e))?;
    let fingerprint = blake3::hash(&identity).to_hex().to_string();
    let total: u128 = sources.iter().map(|s| u128::from(s.bytes)).sum();
    let mut rows = Vec::with_capacity(sources.len());
    for (index, source) in sources.iter().enumerate() {
        let sequence = index as u32 + 1;
        let (target_relative_path, used_undated, mut issues) =
            match resolve(&components, &template, source, sequence) {
                Ok((path, undated)) => (Some(path), undated, Vec::new()),
                Err(issue) => (None, false, vec![issue]),
            };
        if !source.available {
            issues.push(OrganizationIssue::SourceUnavailable);
        }
        rows.push(OrganizationRow {
            asset_id: source.id.clone(),
            root_id: source.root_id.clone(),
            source_path: source.display_path.clone(),
            source_revision: source.revision.clone(),
            source_components: source.relative_path.as_deref().and_then(|path| {
                std::path::Path::new(path)
                    .components()
                    .map(|component| match component {
                        std::path::Component::Normal(name) => name.to_str().map(str::to_owned),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()
                    .filter(|parts| !parts.is_empty() && parts.len() <= 65)
            }),
            target_relative_path,
            sequence,
            bytes: source.bytes.to_string(),
            issues,
            used_undated,
            companion_candidates: source.companion_candidates,
        });
    }
    // One proposed output namespace. Equal names and file/directory conflicts
    // block every participant, including participants with another issue.
    let mut targets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, row) in rows.iter().enumerate() {
        if let Some(path) = &row.target_relative_path {
            targets.entry(path.clone()).or_default().push(i);
        }
    }
    for (path, indices) in &targets {
        let mut conflicting = indices.clone();
        for (prefix, parent_indices) in &targets {
            if path
                .strip_prefix(prefix)
                .is_some_and(|tail| tail.starts_with('/'))
            {
                conflicting.extend(parent_indices);
            }
        }
        if conflicting.len() > indices.len() || indices.len() > 1 {
            for index in conflicting {
                if !rows[index].issues.contains(&OrganizationIssue::Collision) {
                    rows[index].issues.push(OrganizationIssue::Collision);
                }
            }
        }
    }
    // Advisory ASCII case conflicts only. This is deliberately not an emulation
    // of native Unicode normalization/case semantics or an execution preflight.
    for left in 0..rows.len() {
        for right in left + 1..rows.len() {
            if let (Some(a), Some(b)) = (
                &rows[left].target_relative_path,
                &rows[right].target_relative_path,
            ) && a != b
                && a.eq_ignore_ascii_case(b)
            {
                for index in [left, right] {
                    if !rows[index]
                        .issues
                        .contains(&OrganizationIssue::PossibleCaseCollision)
                    {
                        rows[index]
                            .issues
                            .push(OrganizationIssue::PossibleCaseCollision);
                    }
                }
            }
        }
    }
    let blocked = rows.iter().filter(|r| !r.issues.is_empty()).count() as u32;
    Ok(OrganizationPreview {
        policy: POLICY.into(),
        fingerprint,
        catalog_revision,
        template,
        proposed: rows.len() as u32 - blocked,
        blocked,
        rows,
        selected_bytes: total.to_string(),
        execution_available: false,
    })
}

pub(crate) fn validate_template(template: &OrganizationTemplate) -> Result<()> {
    if template.event.len() > 512 {
        return Err(invalid("Use an event of at most 512 bytes."));
    }
    parse(&template.pattern)?;
    Ok(())
}

#[cfg(test)]
mod tests;
