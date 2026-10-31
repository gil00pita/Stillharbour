//! Planning contracts and deterministic export. No filesystem, SQL or Tauri.
use crate::{
    domain::{AppError, ErrorCode, Result},
    organization::{OrganizationPreview, OrganizationTemplate},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PLAN_POLICY: &str = "organization-plan-v1";
pub const MAX_TEMPLATES: usize = 100;
pub const MAX_TEMPLATE_VERSIONS: u32 = 100;
pub const MAX_PLAN_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BUNDLE_DIRECTORY_ENTRIES: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleEntryKind {
    File,
    Directory,
    LinkOrSpecial,
    Placeholder,
    Unreadable,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleRelation {
    Primary,
    SameStem,
    FilenameSidecar,
    AppleDouble,
    Additional,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BundleInventoryEntry {
    pub id: String,
    pub name: String,
    pub kind: BundleEntryKind,
    pub relation: BundleRelation,
    pub bytes: Option<String>,
    pub catalog_asset_id: Option<String>,
    pub catalog_presence: Option<String>,
    pub name_representable: bool,
    pub selectable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BundleInventory {
    pub asset_id: String,
    pub source_revision: String,
    pub catalog_revision: String,
    pub fingerprint: String,
    pub entries: Vec<BundleInventoryEntry>,
    pub inventory_complete: bool,
    pub truncated: bool,
    pub indexed_limit_exceeded: bool,
    pub unobserved_candidates: Vec<String>,
    pub execution_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleConfirmationInput {
    pub plan_id: String,
    pub asset_id: String,
    pub source_revision: String,
    pub inventory_fingerprint: String,
    pub companion_ids: Vec<String>,
    pub reviewed_all_entries: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedBundleFile {
    pub entry_id: String,
    pub source_name: String,
    pub target_relative_path: String,
    pub relation: BundleRelation,
    pub bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BundleConfirmation {
    pub id: String,
    pub plan_id: String,
    pub asset_id: String,
    pub inventory_fingerprint: String,
    pub fingerprint: String,
    pub files: Vec<ConfirmedBundleFile>,
    pub total_bytes: String,
    pub execution_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateRef {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanDraftInput {
    pub asset_ids: Vec<String>,
    pub template: OrganizationTemplate,
    pub preview_fingerprint: String,
    pub template_reference: Option<TemplateRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateSave {
    pub previous: Option<TemplateRef>,
    pub name: String,
    pub description: String,
    pub template: OrganizationTemplate,
    #[serde(default)]
    pub samples: Vec<crate::organization::TemplateSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplatePolicies {
    pub schema_version: u32,
    pub enabled_tokens: Vec<String>,
    pub date_policy: String,
    pub filename_case_policy: String,
    pub collision_policy: String,
    pub sequence_policy: String,
    pub sample_inputs: Vec<String>,
    pub sample_outputs: Vec<String>,
}
impl Default for TemplatePolicies {
    fn default() -> Self {
        Self {
            schema_version: 1,
            enabled_tokens: [
                "original_stem",
                "original_folder",
                "ext",
                "event",
                "capture",
                "sequence",
            ]
            .map(str::to_owned)
            .to_vec(),
            date_policy: "embedded_capture_only".into(),
            filename_case_policy: "preserve".into(),
            collision_policy: "block".into(),
            sequence_policy: "root_native_path_then_id".into(),
            sample_inputs: Vec::new(),
            sample_outputs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedTemplate {
    pub reference: TemplateRef,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub template: OrganizationTemplate,
    pub policies: TemplatePolicies,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Draft,
    Ready,
    Approved,
    Running,
    Completed,
    CompletedWithErrors,
    Canceled,
    NeedsRecovery,
    Invalidated,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanBlocker {
    NativeTargetChecks,
    CompanionInventory,
    CapacityAndFidelity,
    ExecutionAndRecovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizationPlan {
    pub id: String,
    pub policy: String,
    pub created_at: String,
    pub digest: String,
    pub state: PlanState,
    pub template_reference: Option<TemplateRef>,
    pub preview: OrganizationPreview,
    pub destination_path: String,
    pub destination_fingerprint: String,
    pub blockers: Vec<PlanBlocker>,
    pub execution_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlanDetail {
    pub plan: OrganizationPlan,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlanSummary {
    pub id: String,
    pub created_at: String,
    pub digest: String,
    pub state: PlanState,
    pub destination_path: String,
    pub proposed: u32,
    pub blocked: u32,
    pub selected_bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlanPage {
    pub plans: Vec<OrganizationPlanSummary>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PlanExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanExport {
    pub filename: String,
    pub media_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanExportSaved {
    pub path: String,
    pub bytes: String,
    pub digest: String,
}

pub fn validate_template_save(save: &TemplateSave) -> Result<()> {
    let valid_text = |s: &str, max: usize| {
        !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
    };
    if !valid_text(&save.name, 128)
        || save.description.len() > 1024
        || save.description.chars().any(char::is_control)
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Use a name of 1–128 bytes and a description of at most 1,024 bytes without control characters.",
        ));
    }
    crate::organization::validate_template(&save.template)?;
    crate::organization::validate_samples(&save.template, &save.samples)?;
    if let Some(reference) = &save.previous {
        validate_id(&reference.id)?;
        if reference.version == 0 || reference.version >= MAX_TEMPLATE_VERSIONS {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "This template version cannot be extended.",
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_id(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid catalog identifier."))
}

pub fn export(plan: &OrganizationPlan, format: PlanExportFormat) -> Result<PlanExport> {
    let (extension, media_type, text) = match format {
        PlanExportFormat::Json => (
            "json",
            "application/json",
            serde_json::to_string_pretty(plan).map_err(|e| {
                AppError::new(ErrorCode::Catalog, "Could not export the plan.").caused_by(e)
            })?,
        ),
        PlanExportFormat::Csv => {
            let mut text = String::from(
                "\"asset_id\",\"source_path\",\"target_relative_path\",\"bytes\",\"issues\"\r\n",
            );
            for row in &plan.preview.rows {
                let issues = serde_json::to_string(&row.issues).map_err(|e| {
                    AppError::new(ErrorCode::Catalog, "Could not export review issues.")
                        .caused_by(e)
                })?;
                let fields = [
                    row.asset_id.as_str(),
                    row.source_path.as_str(),
                    row.target_relative_path.as_deref().unwrap_or(""),
                    row.bytes.as_str(),
                    issues.as_str(),
                ];
                text.push_str(&fields.map(csv_cell).join(","));
                text.push_str("\r\n");
            }
            ("csv", "text/csv", text)
        }
    };
    Ok(PlanExport {
        filename: format!("stillharbour-plan-{}.{}", plan.id, extension),
        media_type: media_type.into(),
        text,
    })
}
fn csv_cell(value: &str) -> String {
    // CSV quoting alone does not prevent spreadsheet formulas. Protect leading
    // whitespace/control prefixes as well; JSON retains the exact source text.
    let significant = value.trim_start_matches(char::is_whitespace);
    let protect =
        significant.starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r', '\n']);
    format!(
        "\"{}{}\"",
        if protect { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spreadsheet_cells_are_quoted_and_formula_safe_without_losing_json_text() {
        for value in [
            "=SUM(A1)", "+1", "-1", "@call", "  =1", "\t=1", "\r=1", "\n+1",
        ] {
            assert!(csv_cell(value).starts_with("\"'"));
            assert_eq!(
                serde_json::from_str::<String>(&serde_json::to_string(value).unwrap()).unwrap(),
                value
            );
        }
        assert_eq!(
            csv_cell("photo,\"name\"\n.JPG"),
            "\"photo,\"\"name\"\"\n.JPG\""
        );
    }
}
