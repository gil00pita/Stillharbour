use stillharbour_core::{
    contract::*,
    domain::{AppError, ErrorCode},
    library::model::*,
    operations::*,
    organization::*,
    preferences::*,
};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ts_rs::Config::default();
    let declarations = [
        ErrorCode::decl(&config),
        AppError::decl(&config),
        MissingDatePolicy::decl(&config),
        OrganizationTemplate::decl(&config),
        OrganizationIssue::decl(&config),
        OrganizationRow::decl(&config),
        OrganizationPreview::decl(&config),
        OrganizationCompanion::decl(&config),
        OrganizationCompanionReview::decl(&config),
        OrganizationDestinationReview::decl(&config),
        OrganizationTargetState::decl(&config),
        OrganizationTargetReview::decl(&config),
        TemplateRef::decl(&config),
        TemplateSample::decl(&config),
        TemplateSave::decl(&config),
        PlanDraftInput::decl(&config),
        TemplatePolicies::decl(&config),
        SavedTemplate::decl(&config),
        PlanState::decl(&config),
        PlanBlocker::decl(&config),
        OrganizationPlan::decl(&config),
        OrganizationPlanDetail::decl(&config),
        OrganizationPlanSummary::decl(&config),
        OrganizationPlanPage::decl(&config),
        PlanExportFormat::decl(&config),
        PlanExport::decl(&config),
        PlanExportSaved::decl(&config),
        AppPreferences::decl(&config),
        GalleryView::decl(&config),
        ThumbnailSize::decl(&config),
        GridFit::decl(&config),
        SortOrder::decl(&config),
        FoundationRequest::decl(&config),
        FoundationStatus::decl(&config),
        DirectorySelection::decl(&config),
        FoundationResponse::decl(&config),
        RootMode::decl(&config),
        ScanFilter::decl(&config),
        LibraryRoot::decl(&config),
        JobKind::decl(&config),
        ScanJob::decl(&config),
        LibraryStatus::decl(&config),
        RootMonitor::decl(&config),
        LibraryChangeKind::decl(&config),
        LibraryChange::decl(&config),
        LibraryChangePage::decl(&config),
        AssetQuery::decl(&config),
        AssetSummary::decl(&config),
        AssetDetail::decl(&config),
        AssetNeighbors::decl(&config),
        AssetPage::decl(&config),
        PackedAsset::decl(&config),
        PackedAssetPage::decl(&config),
        DateBucket::decl(&config),
        MonthBucket::decl(&config),
        SelectionToken::decl(&config),
        AnnotationPatch::decl(&config),
        AssetSelection::decl(&config),
        ThumbnailKey::decl(&config),
        ThumbnailResult::decl(&config),
        JobDetail::decl(&config),
        FolderEntry::decl(&config),
        AssetSeekPosition::decl(&config),
        MaintenanceTarget::decl(&config),
        MaintenancePreview::decl(&config),
        MaintenanceResult::decl(&config),
        CatalogDirectory::decl(&config),
        DirectoryEntry::decl(&config),
        DirectoryPage::decl(&config),
        LibraryRequest::decl(&config),
        LibraryResponse::decl(&config),
    ];
    let mut output = String::from(
        "// Generated from crates/core/src/{contract,domain,preferences,organization,library/{model,original},rendition}.rs. Run npm run bindings.\n",
    );
    output.push_str(&format!(
        "export const FOUNDATION_COMMAND = {FOUNDATION_COMMAND:?};\n"
    ));
    output.push_str(&format!("export const LIBRARY_COMMAND = {LIBRARY_COMMAND:?};\nexport const THUMBNAIL_POLICY = {:?};\n",stillharbour_core::library::THUMBNAIL_POLICY));
    output.push_str(&format!(
        "export const ORIGINAL_IMAGE_POLICY = {:?};\n",
        stillharbour_core::library::ORIGINAL_IMAGE_POLICY
    ));
    output.push_str(&format!(
        "export const DEFAULT_PHOTO_EXTENSIONS: string[] = {};\n",
        serde_json::to_string(DEFAULT_PHOTO_EXTENSIONS)?
    ));
    output.push_str(&format!("export const RENDITION_EDGES = {:?} as const;\nexport const GALLERY_MAX_EDGE = {};\nexport const VIEWER_EDGE = {};\nexport const GALLERY_DECODED_BYTES = {};\n", stillharbour_core::rendition::EDGES, stillharbour_core::rendition::GALLERY_MAX_EDGE, stillharbour_core::rendition::VIEWER_EDGE, stillharbour_core::rendition::GALLERY_DECODED_BYTES));
    for declaration in declarations {
        output.push_str("export ");
        output.push_str(&declaration);
        output.push('\n');
    }
    let output = output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/bindings.ts");
    if std::env::args().any(|arg| arg == "--check") {
        if std::fs::read_to_string(&path)? != output {
            return Err("IPC bindings are stale; run npm run bindings".into());
        }
    } else {
        std::fs::write(path, output)?;
    }
    Ok(())
}
