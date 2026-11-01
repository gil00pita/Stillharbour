#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(all(feature = "desktop-test", not(debug_assertions)))]
compile_error!("desktop-test instrumentation must never be compiled into a release build");

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use stillharbour_core::{
    catalog::Catalog,
    contract::{DirectorySelection, FoundationRequest, FoundationResponse, FoundationStatus},
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
    library::{Library, LibraryOptions, WorkerPaths, model::*},
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone)]
struct NativeSelection {
    id: String,
    path: PathBuf,
    identity: Option<FileIdentity>,
    volume: Option<String>,
}
#[cfg(feature = "desktop-test")]
struct MultiSourceFixture {
    sources: Vec<stillharbour_core::library_fixture::LibraryFixture>,
    destination: tempfile::TempDir,
    registered: usize,
    keeper: Option<stillharbour_core::library_fixture::LibraryFixture>,
    keeper_root: Option<String>,
}

struct Foundation {
    #[cfg(feature = "desktop-test")]
    rename_fixture: stillharbour_core::executor::GeneratedRenameController,
    #[cfg(feature = "desktop-test")]
    _test_data: tempfile::TempDir,
    #[cfg(feature = "desktop-test")]
    fixture: Mutex<Option<stillharbour_core::library_fixture::LibraryFixture>>,
    #[cfg(feature = "desktop-test")]
    destination_fixture: Mutex<Option<tempfile::TempDir>>,
    #[cfg(feature = "desktop-test")]
    managed_destination_fixture: Mutex<Option<(tempfile::TempDir, Option<String>)>>,
    #[cfg(feature = "desktop-test")]
    multisource_fixture: Mutex<Option<MultiSourceFixture>>,
    #[cfg(feature = "desktop-test")]
    comparison_fixture: Mutex<
        Option<(
            stillharbour_core::library_fixture::LibraryFixture,
            Option<String>,
        )>,
    >,
    #[cfg(feature = "desktop-test")]
    import_skip_fixture: Mutex<
        Option<(
            stillharbour_core::library_fixture::LibraryFixture,
            Option<String>,
        )>,
    >,
    #[cfg(feature = "desktop-test")]
    export_fixture: Mutex<Option<tempfile::TempDir>>,
    _catalog: Arc<Catalog>,
    preferences: Arc<stillharbour_core::preferences::PreferencesStore>,
    library: Arc<Library>,
    status: FoundationStatus,
    selection: Mutex<Option<NativeSelection>>,
    picker: tokio::sync::Mutex<()>,
}

#[tauri::command]
async fn foundation_request(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    request: FoundationRequest,
) -> Result<FoundationResponse> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "This window has no foundation authority.",
        ));
    }
    match request {
        FoundationRequest::GetPreferences {} => {
            let preferences = state.preferences.clone();
            let library = state.library.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let mut value = preferences.get()?;
                if let Some(id) = &value.default_root_id
                    && !library.root_exists(id.clone())?
                {
                    value.default_root_id = None;
                }
                Ok(FoundationResponse::Preferences(value))
            })
            .await
            .map_err(|e| {
                AppError::new(ErrorCode::WorkerFailed, "Settings read stopped.").caused_by(e)
            })?
        }
        FoundationRequest::SavePreferences { preferences } => {
            let store = state.preferences.clone();
            let library = state.library.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Some(id) = &preferences.default_root_id {
                    library.validate_root_id(id.clone())?;
                }
                store
                    .save(*preferences)
                    .map(FoundationResponse::Preferences)
            })
            .await
            .map_err(|e| {
                AppError::new(ErrorCode::WorkerFailed, "Settings save stopped.").caused_by(e)
            })?
        }
        FoundationRequest::Status {} => Ok(FoundationResponse::Status(state.status.clone())),
        FoundationRequest::PickDirectory {} => {
            let _picker = state
                .picker
                .try_lock()
                .map_err(|_| AppError::new(ErrorCode::Busy, "A folder picker is already open."))?;
            let (sender, receiver) = tokio::sync::oneshot::channel();
            app.dialog()
                .file()
                .set_parent(&window)
                .set_title("Select a folder — selection only, no scan")
                .set_can_create_directories(false)
                .pick_folder(move |selection| {
                    let _ = sender.send(selection);
                });
            let selected = receiver.await.map_err(|_| {
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "The folder picker closed unexpectedly.",
                )
            })?;
            let Some(selected) = selected else {
                return Ok(FoundationResponse::Directory(None));
            };
            let path = selected.into_path().map_err(|error| {
                AppError::new(ErrorCode::InvalidInput, "Select a local folder.").caused_by(error)
            })?;
            let (response, native) = tauri::async_runtime::spawn_blocking(move || -> Result<_> {
                let handle = filesystem::open_directory_readonly(&path)?;
                let (identity, volume) = match filesystem::identity(&handle) {
                    Ok(identity) => match stillharbour_core::library::native::volume_token(&handle)
                    {
                        Ok(volume) => (Some(identity), Some(volume)),
                        Err(error) if error.code == ErrorCode::Unsupported => (None, None),
                        Err(error) => return Err(error),
                    },
                    Err(error) if error.code == ErrorCode::Unsupported => (None, None),
                    Err(error) => return Err(error),
                };
                let response = DirectorySelection {
                    id: uuid::Uuid::new_v4().to_string(),
                    display_path: path.to_string_lossy().into_owned(),
                    identity_available: identity.is_some(),
                };
                let id = response.id.clone();
                Ok((
                    response,
                    NativeSelection {
                        id,
                        path,
                        identity,
                        volume,
                    },
                ))
            })
            .await
            .map_err(|error| {
                AppError::new(ErrorCode::WorkerFailed, "Folder validation stopped.")
                    .caused_by(error)
            })??;
            *state.selection.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Selection state is unavailable.")
            })? = Some(native);
            Ok(FoundationResponse::Directory(Some(response)))
        }
    }
}

fn selected(state: &Foundation, id: &str) -> Result<NativeSelection> {
    state
        .selection
        .lock()
        .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Selection state is unavailable."))?
        .as_ref()
        .filter(|selected| selected.id == id)
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidInput,
                "Choose this folder again with the native picker before saving or relinking.",
            )
        })
}

fn selected_identity(selection: &NativeSelection) -> Result<(FileIdentity, String)> {
    match (&selection.identity, &selection.volume) {
        (Some(identity), Some(volume)) => Ok((identity.clone(), volume.clone())),
        _ => Err(AppError::new(
            ErrorCode::Unsupported,
            "This folder's identity and volume could not be verified. Select a supported local folder.",
        )),
    }
}

#[tauri::command]
async fn library_request(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    request: LibraryRequest,
) -> Result<LibraryResponse> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "This window has no library authority.",
        ));
    }
    let selection = match &request {
        LibraryRequest::SaveAndScan { selection_id, .. }
        | LibraryRequest::OrganizationPlanSaveExport { selection_id, .. }
        | LibraryRequest::OrganizationPlanCreate { selection_id, .. }
        | LibraryRequest::OrganizationDestination { selection_id, .. }
        | LibraryRequest::OrganizationCopyPreparationReopen { selection_id, .. }
        | LibraryRequest::OrganizationCopyEffectMatch { selection_id, .. }
        | LibraryRequest::OrganizationCopyReviewCreate { selection_id, .. }
        | LibraryRequest::OrganizationImportCopyReview { selection_id, .. }
        | LibraryRequest::OrganizationImportIndexOutput { selection_id, .. }
        | LibraryRequest::OrganizationCopyReview { selection_id, .. }
        | LibraryRequest::OrganizationCopyApprove { selection_id, .. }
        | LibraryRequest::OrganizationCopyStart { selection_id, .. }
        | LibraryRequest::OrganizationCopyInspect { selection_id, .. }
        | LibraryRequest::OrganizationCopyRecoveryInspect { selection_id, .. }
        | LibraryRequest::OrganizationCopyContinuationReview { selection_id, .. }
        | LibraryRequest::OrganizationCopyContinuationApprove { selection_id, .. }
        | LibraryRequest::OrganizationCopyContinuationStart { selection_id, .. }
        | LibraryRequest::Relink { selection_id, .. } => Some(selected(&state, selection_id)?),
        _ => None,
    };
    let library = state.library.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<LibraryResponse> {
        Ok(match request {
            LibraryRequest::OrganizationImportDestinationSet {
                catalog_id,
                root_id,
            } => LibraryResponse::OrganizationImportDestination(
                library.set_import_destination(catalog_id, root_id)?,
            ),
            LibraryRequest::OrganizationImportIndexOutput { job_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the original import destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationImportOutputIndex(Box::new(
                    library.organization_import_index_output(
                        job_id,
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationImportCopyReceipt { job_id } => {
                LibraryResponse::OrganizationImportCopyReceipt(
                    library
                        .organization_import_copy_receipt(job_id)?
                        .map(Box::new),
                )
            }
            LibraryRequest::OrganizationImportCopyReview {
                plan_id,
                import_plan_id,
                ..
            } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved import destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyReview(Box::new(
                    library.organization_import_copy_review(
                        plan_id,
                        import_plan_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationImportSavedPlans { plan_id } => {
                LibraryResponse::OrganizationImportSavedPlans(
                    library.organization_import_saved_plans(plan_id)?,
                )
            }
            LibraryRequest::OrganizationImportPlanRelease {
                plan_id,
                import_plan_id,
            } => {
                library.organization_import_plan_release(plan_id, import_plan_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::OrganizationImportHistory {
                plan_id,
                manifest_id,
            } => LibraryResponse::OrganizationImportHistory(Box::new(
                library.organization_import_history(plan_id, manifest_id)?,
            )),
            LibraryRequest::OrganizationImportSkipApprove { input } => {
                LibraryResponse::OrganizationImportSkipApproval(
                    library.organization_import_skip_approve(input)?,
                )
            }
            LibraryRequest::OrganizationImportSkipStart { input } => {
                LibraryResponse::OrganizationImportSession(Box::new(
                    library.organization_import_skip_start(input)?,
                ))
            }
            LibraryRequest::OrganizationImportSession {
                plan_id,
                session_id,
            } => LibraryResponse::OrganizationImportSession(Box::new(
                library.organization_import_session(plan_id, session_id)?,
            )),
            LibraryRequest::OrganizationImportSessionCancel {
                plan_id,
                session_id,
            } => LibraryResponse::OrganizationImportSession(Box::new(
                library.organization_import_session_cancel(plan_id, session_id)?,
            )),
            LibraryRequest::OrganizationImportSessionDiscard {
                plan_id,
                session_id,
            } => {
                library.organization_import_session_discard(plan_id, session_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::OrganizationImportPlanCreate { input } => {
                LibraryResponse::OrganizationImportPlan(Box::new(
                    library.organization_import_plan_create(input)?,
                ))
            }
            LibraryRequest::OrganizationImportPlanOpen {
                plan_id,
                import_plan_id,
            } => LibraryResponse::OrganizationImportPlan(Box::new(
                library.organization_import_plan_open(plan_id, import_plan_id)?,
            )),
            LibraryRequest::OrganizationImportComparisonStart { input } => {
                LibraryResponse::OrganizationImportComparisonJob(Box::new(
                    library.organization_import_comparison_start(input)?,
                ))
            }
            LibraryRequest::OrganizationImportComparisonJob { job_id } => {
                LibraryResponse::OrganizationImportComparisonJob(Box::new(
                    library.organization_import_comparison_job(job_id)?,
                ))
            }
            LibraryRequest::OrganizationImportComparisonCancel { job_id } => {
                LibraryResponse::OrganizationImportComparisonJob(Box::new(
                    library.organization_import_comparison_cancel(job_id)?,
                ))
            }
            LibraryRequest::OrganizationImportComparisonDiscard { job_id } => {
                library.organization_import_comparison_discard(job_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::OrganizationCopyCapabilities {} => {
                LibraryResponse::OrganizationCopyCapabilities(
                    library.organization_copy_capabilities(),
                )
            }
            LibraryRequest::OrganizationCopyPreparationReopen { job_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyPreparationReopened(Box::new(
                    library.organization_copy_preparation_reopen(
                        job_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationCopyEffectMatch {
                plan_id,
                manifest_id,
                source_root_id,
                ..
            } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyEffectMatch(
                    library.organization_copy_effect_match_for_root(
                        plan_id,
                        manifest_id,
                        source_root_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                )
            }
            LibraryRequest::OrganizationCopyReviewCreate {
                plan_id,
                manifest_id,
                source_root_id,
                ..
            } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyReview(Box::new(
                    library.organization_copy_review_create_for_root(
                        plan_id,
                        manifest_id,
                        source_root_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationCopyReview { review_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyReview(Box::new(
                    library.organization_copy_review(
                        review_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationCopyApprove { input, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyApproval(library.organization_copy_approve(
                    input,
                    chosen.id.clone(),
                    chosen.path.clone(),
                    selected_identity(chosen)?,
                )?)
            }
            LibraryRequest::OrganizationCopyStart { input, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyJob(library.organization_copy_start(
                    input,
                    chosen.id.clone(),
                    chosen.path.clone(),
                    selected_identity(chosen)?,
                )?)
            }
            LibraryRequest::OrganizationCopyJob { job_id } => {
                LibraryResponse::OrganizationCopyJob(library.organization_copy_job(job_id)?)
            }
            LibraryRequest::OrganizationOriginalHistory { job_id } => {
                LibraryResponse::OrganizationOriginalHistory(
                    library.organization_original_history(job_id)?,
                )
            }
            LibraryRequest::OrganizationCopyCancel { job_id } => {
                LibraryResponse::OrganizationCopyJob(library.organization_copy_cancel(job_id)?)
            }
            LibraryRequest::OrganizationCopyIo {} => {
                LibraryResponse::OrganizationCopyIo(library.organization_copy_io()?)
            }
            LibraryRequest::OrganizationCopySetIo {
                aggregate_bytes_per_second,
                byte_io_paused,
            } => LibraryResponse::OrganizationCopyIo(
                library.organization_copy_set_io(aggregate_bytes_per_second, byte_io_paused)?,
            ),
            LibraryRequest::OrganizationCopyDiscard { review_id } => {
                library.organization_copy_discard(review_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::OrganizationCopyContinuationReview { job_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the retained destination again.",
                    )
                })?;
                LibraryResponse::OrganizationCopyContinuationReview(Box::new(
                    library.organization_copy_continuation_review(
                        job_id,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                ))
            }
            LibraryRequest::OrganizationCopyContinuationApprove { input, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the retained destination again.",
                    )
                })?;
                LibraryResponse::OrganizationCopyApproval(
                    library.organization_copy_continuation_approve(
                        input,
                        chosen.id.clone(),
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                )
            }
            LibraryRequest::OrganizationCopyContinuationStart { input, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the retained destination again.",
                    )
                })?;
                LibraryResponse::OrganizationCopyJob(library.organization_copy_continuation_start(
                    input,
                    chosen.id.clone(),
                    chosen.path.clone(),
                    selected_identity(chosen)?,
                )?)
            }
            LibraryRequest::OrganizationCopyContinuationDiscard { review_id } => {
                library.organization_copy_continuation_discard(review_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::OrganizationCopyRecoveryInspect { job_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the retained destination with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyRecoveryInspection(
                    library.organization_copy_recovery_inspect(
                        job_id,
                        chosen.path.clone(),
                        selected_identity(chosen)?,
                    )?,
                )
            }
            LibraryRequest::OrganizationCopyInspect { review_id, .. } => {
                let chosen = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose the saved destination again with the native picker.",
                    )
                })?;
                LibraryResponse::OrganizationCopyInspection(library.organization_copy_inspect(
                    review_id,
                    chosen.id.clone(),
                    chosen.path.clone(),
                    selected_identity(chosen)?,
                )?)
            }
            LibraryRequest::OrganizationTemplates {} => {
                LibraryResponse::OrganizationTemplates(library.organization_templates()?)
            }
            LibraryRequest::OrganizationBundleInventory {
                asset_id,
                source_revision,
            } => LibraryResponse::OrganizationBundleInventory(
                library.organization_bundle_inventory(asset_id, source_revision)?,
            ),
            LibraryRequest::OrganizationTemplateSave { save } => {
                LibraryResponse::OrganizationTemplate(library.organization_template_save(save)?)
            }
            LibraryRequest::OrganizationBundleConfirm { input } => {
                LibraryResponse::OrganizationBundleConfirmation(
                    library.organization_bundle_confirm(input)?,
                )
            }
            LibraryRequest::OrganizationCopyManifest { input } => {
                LibraryResponse::OrganizationCopyManifest(
                    library.organization_copy_manifest(input)?,
                )
            }
            LibraryRequest::OrganizationCopyManifestOpen {
                plan_id,
                manifest_id,
            } => LibraryResponse::OrganizationCopyManifest(
                library.organization_copy_manifest_open(plan_id, manifest_id)?,
            ),
            LibraryRequest::OrganizationCopyAssessment {
                plan_id,
                manifest_id,
            } => LibraryResponse::OrganizationCopyAssessment(
                library.organization_copy_assessment(plan_id, manifest_id)?,
            ),
            LibraryRequest::OrganizationPlans { before } => {
                LibraryResponse::OrganizationPlans(library.organization_plans(before)?)
            }
            LibraryRequest::OrganizationBundleReviews { plan_id, asset_id } => {
                LibraryResponse::OrganizationBundleReviews(
                    library.organization_bundle_reviews(plan_id, asset_id)?,
                )
            }
            LibraryRequest::OrganizationPlan { plan_id } => {
                LibraryResponse::OrganizationPlan(Box::new(library.organization_plan(plan_id)?))
            }
            LibraryRequest::OrganizationPlanExport { plan_id, format } => {
                LibraryResponse::OrganizationPlanExport(
                    library.organization_plan_export(plan_id, format)?,
                )
            }
            LibraryRequest::OrganizationPlanSaveExport {
                plan_id, format, ..
            } => {
                let selection = selection.as_ref().ok_or_else(|| {
                    AppError::new(ErrorCode::InvalidInput, "Choose an export folder again.")
                })?;
                LibraryResponse::OrganizationPlanExportSaved(
                    library.organization_plan_save_export(
                        plan_id,
                        format,
                        selection.path.clone(),
                        selected_identity(selection)?,
                    )?,
                )
            }
            LibraryRequest::OrganizationPlanCreate { draft, .. } => {
                let selection = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose a destination folder again.",
                    )
                })?;
                LibraryResponse::OrganizationPlan(Box::new(library.organization_plan_create(
                    draft,
                    selection.path.clone(),
                    selected_identity(selection)?,
                )?))
            }
            LibraryRequest::OrganizationDestination {
                destination_root_id,
                asset_ids,
                template,
                preview_fingerprint,
                ..
            } => {
                let selection = selection.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Choose a destination folder again.",
                    )
                })?;
                LibraryResponse::OrganizationDestination(
                    library.organization_destination_for_root(
                        asset_ids,
                        template,
                        preview_fingerprint,
                        selection.path.clone(),
                        selected_identity(selection)?,
                        destination_root_id,
                    )?,
                )
            }
            LibraryRequest::OrganizationCompanions { asset_id } => {
                LibraryResponse::OrganizationCompanions(library.organization_companions(asset_id)?)
            }
            LibraryRequest::OrganizationPreview {
                asset_ids,
                template,
            } => LibraryResponse::OrganizationPreview(
                library.organization_preview(asset_ids, template)?,
            ),
            LibraryRequest::Changes { before, limit } => {
                LibraryResponse::Changes(library.changes(before, limit)?)
            }
            LibraryRequest::BrowseDirectory {
                root_id,
                directory_id,
                name,
                cursor,
                limit,
            } => LibraryResponse::Directory(library.browse_directory(
                root_id,
                directory_id,
                name,
                cursor,
                limit,
            )?),
            LibraryRequest::PrepareMaintenance { target } => {
                LibraryResponse::MaintenancePreview(library.prepare_maintenance(target)?)
            }
            LibraryRequest::ApplyMaintenance { plan_id } => {
                LibraryResponse::MaintenanceDone(library.apply_maintenance(plan_id)?)
            }
            LibraryRequest::SeekAsset {
                query,
                asset_id,
                position,
                limit,
            } => {
                LibraryResponse::Page(library.seek_asset(query, asset_id, position, limit)?.into())
            }
            LibraryRequest::InspectionIssues { job_id, after } => {
                LibraryResponse::InspectionIssues(library.inspection_issues(job_id, after)?)
            }
            LibraryRequest::InspectionKeeperLatest { job_id, group_id } => {
                LibraryResponse::InspectionKeeperLatest(
                    library
                        .inspection_keeper_latest(job_id, group_id)?
                        .map(Box::new),
                )
            }
            LibraryRequest::InspectionKeeperReview { input } => {
                LibraryResponse::InspectionKeeperReview(library.inspection_keeper_review(input)?)
            }
            LibraryRequest::InspectionStart { input } => {
                LibraryResponse::InspectionJob(Box::new(library.inspection_start(input)?))
            }
            LibraryRequest::InspectionJob { job_id } => {
                LibraryResponse::InspectionJob(Box::new(library.inspection_job(job_id)?))
            }
            LibraryRequest::InspectionJobs {} => {
                LibraryResponse::InspectionJobs(library.inspection_jobs()?)
            }
            LibraryRequest::InspectionCancel { job_id } => {
                LibraryResponse::InspectionJob(Box::new(library.inspection_cancel(job_id)?))
            }
            LibraryRequest::InspectionFindings { job_id, after } => {
                LibraryResponse::InspectionFindings(library.inspection_findings(job_id, after)?)
            }
            LibraryRequest::InspectionGroups { job_id, after } => {
                LibraryResponse::InspectionGroups(library.inspection_groups(job_id, after)?)
            }
            LibraryRequest::InspectionGroupMembers {
                job_id,
                group_id,
                after,
            } => LibraryResponse::InspectionFindings(
                library.inspection_group_members(job_id, group_id, after)?,
            ),
            LibraryRequest::InspectionReport { job_id } => {
                LibraryResponse::InspectionReport(library.inspection_report(job_id)?.map(Box::new))
            }
            LibraryRequest::InspectionFolders {
                job_id,
                parent_id,
                after,
            } => LibraryResponse::InspectionFolders(
                library.inspection_folders(job_id, parent_id, after)?,
            ),
            LibraryRequest::Status {} => LibraryResponse::Status(library.status()?),
            LibraryRequest::SaveAndScan {
                label,
                mode,
                filter,
                ..
            } => {
                let selection = selection.ok_or_else(|| {
                    AppError::new(ErrorCode::InvalidInput, "Missing native selection.")
                })?;
                let expected = selected_identity(&selection)?;
                LibraryResponse::Job(library.register_verified(
                    selection.path,
                    expected,
                    label,
                    mode,
                    filter,
                )?)
            }
            LibraryRequest::Scan { root_id, filter } => {
                LibraryResponse::Job(library.scan(root_id, filter)?)
            }
            LibraryRequest::GenerateThumbnails { root_id } => {
                LibraryResponse::Job(library.generate_thumbnails(root_id)?)
            }
            LibraryRequest::Cancel { job_id } => {
                library.cancel(job_id)?;
                LibraryResponse::Done
            }
            LibraryRequest::Relink { root_id, .. } => {
                let selection = selection.ok_or_else(|| {
                    AppError::new(ErrorCode::InvalidInput, "Missing native selection.")
                })?;
                let expected = selected_identity(&selection)?;
                LibraryResponse::Job(library.relink_verified(root_id, selection.path, expected)?)
            }
            LibraryRequest::UpdateMode { root_id, mode } => {
                library.update_mode(root_id, mode)?;
                LibraryResponse::Done
            }
            LibraryRequest::Query {
                query,
                cursor,
                limit,
            } => LibraryResponse::Page(library.query(query, cursor, limit)?.into()),
            LibraryRequest::Detail { asset_id } => {
                LibraryResponse::Detail(Box::new(library.detail(asset_id)?))
            }
            LibraryRequest::Neighbors { query, asset_id } => {
                LibraryResponse::Neighbors(Box::new(library.neighbors(query, asset_id)?))
            }
            LibraryRequest::Seek { query, year, limit } => {
                LibraryResponse::Page(library.seek(query, year, limit)?.into())
            }
            LibraryRequest::SeekDate {
                query,
                period,
                limit,
            } => LibraryResponse::Page(library.seek_date(query, period, limit)?.into()),
            LibraryRequest::DateBuckets { query } => {
                LibraryResponse::Buckets(library.buckets(query)?)
            }
            LibraryRequest::AddTags { selection, tags } => {
                LibraryResponse::Updated(library.add_tags(selection, tags)?)
            }
            LibraryRequest::FreezeSelection { query } => {
                LibraryResponse::Selection(library.freeze(query)?)
            }
            LibraryRequest::Annotate { selection, patch } => {
                LibraryResponse::Updated(library.annotate(selection, patch)?)
            }
            LibraryRequest::Thumbnails {
                keys,
                generation,
                cache_only,
            } => LibraryResponse::Thumbnails(if cache_only.unwrap_or(false) {
                library.cached_thumbnails(keys, generation)?
            } else {
                library.thumbnails(keys, generation)?
            }),
            LibraryRequest::FilterPreview { filter, paths } => {
                LibraryResponse::FilterPreview(library.filter_preview(filter, paths)?)
            }
            LibraryRequest::JobDetails { job_id } => {
                LibraryResponse::JobDetails(library.job_details(job_id)?)
            }
            LibraryRequest::Folders { root_id } => {
                LibraryResponse::Folders(library.folders(root_id)?)
            }
            LibraryRequest::Tags {} => LibraryResponse::Tags(library.tags()?),
        })
    })
    .await
    .map_err(|error| {
        AppError::new(ErrorCode::WorkerFailed, "The library request stopped.").caused_by(error)
    })?
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
fn test_scroll_catalog(window: tauri::WebviewWindow, state: State<'_, Foundation>) -> Result<u32> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid test window.",
        ));
    }
    let fixture = state
        .fixture
        .lock()
        .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?;
    let fixture = fixture
        .as_ref()
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Generate fixture first."))?;
    stillharbour_core::library_fixture::seed_scroll_catalog(&state._catalog, fixture)
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
fn test_library_fixture(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    verify: bool,
    native_formats: Option<bool>,
) -> Result<serde_json::Value> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid test window.",
        ));
    }
    let mut fixture = state
        .fixture
        .lock()
        .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Test fixture unavailable."))?;
    if verify {
        let count = fixture
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Generate the fixture first."))?
            .verify_unchanged()?;
        return Ok(serde_json::json!({"verifiedFiles":count}));
    }
    if fixture.is_some() {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "A generated fixture already exists.",
        ));
    }
    let generated = if native_formats.unwrap_or(false) {
        stillharbour_core::library_fixture::LibraryFixture::generate_native_formats()?
    } else {
        stillharbour_core::library_fixture::LibraryFixture::generate()?
    };
    let path = generated.path().to_owned();
    let handle = filesystem::open_readonly(&path)?;
    let identity = filesystem::identity(&handle)?;
    let volume = stillharbour_core::library::native::volume_token(&handle)?;
    let id = uuid::Uuid::new_v4().to_string();
    *state
        .selection
        .lock()
        .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Test selection unavailable."))? =
        Some(NativeSelection {
            id: id.clone(),
            path,
            identity: Some(identity),
            volume: Some(volume),
        });
    *fixture = Some(generated);
    serde_json::to_value(DirectorySelection {
        id,
        display_path: "Generated test library · 18 images and safety fixtures".into(),
        identity_available: true,
    })
    .map_err(|e| AppError::new(ErrorCode::WorkerFailed, "Test response failed.").caused_by(e))
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
async fn test_generated_rename(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    request: stillharbour_core::operations::generated_rename::GeneratedRenameRequest,
) -> Result<stillharbour_core::operations::generated_rename::GeneratedRenameReview> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid generated-fixture window.",
        ));
    }
    let controller = state.rename_fixture.clone();
    tokio::task::spawn_blocking(move || controller.request(request))
        .await
        .map_err(|error| {
            AppError::new(ErrorCode::WorkerFailed, "Generated review worker stopped.")
                .caused_by(error)
        })?
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
async fn test_generated_rename_history(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    job_id: String,
) -> Result<stillharbour_core::operations::original_jobs::OriginalOperationHistory> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid generated-history window.",
        ));
    }
    let controller = state.rename_fixture.clone();
    tokio::task::spawn_blocking(move || controller.history(job_id))
        .await
        .map_err(|error| {
            AppError::new(ErrorCode::WorkerFailed, "Generated history review failed.")
                .caused_by(error)
        })?
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
async fn test_generated_rename_worker(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    request: stillharbour_core::operations::generated_rename::GeneratedWorkerRequest,
) -> Result<stillharbour_core::operations::generated_rename::GeneratedWorkerStatus> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid generated-worker window.",
        ));
    }
    let controller = state.rename_fixture.clone();
    tokio::task::spawn_blocking(move || controller.worker(request))
        .await
        .map_err(|error| {
            AppError::new(ErrorCode::WorkerFailed, "Generated worker status failed.")
                .caused_by(error)
        })?
}

#[cfg(feature = "desktop-test")]
#[tauri::command]
async fn test_performance(
    window: tauri::WebviewWindow,
    state: State<'_, Foundation>,
    action: String,
    rows: Option<u32>,
) -> Result<serde_json::Value> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid diagnostic window.",
        ));
    }
    match action.as_str() {
        "organization_rename_arm_hold" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "The generated hold accepts no supplied scope.",
                ));
            }
            state.rename_fixture.arm_native_hold()?;
            Ok(
                serde_json::json!({"scope":"generated-rename-only","afterMoves":1,"timeoutSeconds":15}),
            )
        }
        "organization_destination_reselect" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Owned destination selection accepts no supplied scope.",
                ));
            }
            let owned = state.destination_fixture.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Destination fixture unavailable.")
            })?;
            let directory = owned.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate a destination fixture first.",
                )
            })?;
            let path = directory.path().to_owned();
            let file = filesystem::open_directory_readonly(&path)?;
            let identity = filesystem::identity(&file)?;
            let volume = stillharbour_core::library::native::volume_token(&file)?;
            let id = uuid::Uuid::new_v4().to_string();
            *state
                .selection
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Selection unavailable."))? =
                Some(NativeSelection {
                    id: id.clone(),
                    path,
                    identity: Some(identity),
                    volume: Some(volume),
                });
            Ok(
                serde_json::json!({"id":id,"displayPath":"Generated destination fixture","identityAvailable":true}),
            )
        }
        "organization_destination_audit" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Owned destination audit accepts no supplied scope.",
                ));
            }
            let fixture = state
                .fixture
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?;
            let fixture = fixture.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate an owned source fixture first.",
                )
            })?;
            let destination = state.destination_fixture.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Destination fixture unavailable.")
            })?;
            fixture.audit_generated_outputs(
                destination
                    .as_ref()
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorCode::InvalidInput,
                            "Generate an owned destination fixture first.",
                        )
                    })?
                    .path(),
            )
        }
        "organization_name_preparation_fixture" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "The owned name-preparation fixture accepts no supplied scope.",
                ));
            }
            tokio::task::spawn_blocking(stillharbour_core::executor::run_preparation_fixture_probe)
                .await
                .map_err(|error| {
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "The owned name-preparation worker failed.",
                    )
                    .caused_by(error)
                })?
        }
        "organization_copy_executor_fixture" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "The owned executor fixture accepts no supplied scope.",
                ));
            }
            tokio::task::spawn_blocking(stillharbour_core::executor::run_fixture_probe)
                .await
                .map_err(|error| {
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "The owned executor fixture worker failed.",
                    )
                    .caused_by(error)
                })?
        }
        "organization_multisource_fixture"
        | "organization_multisource_next"
        | "organization_multisource_keeper"
        | "organization_multisource_audit" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Generated multi-source fixture accepts no supplied scope.",
                ));
            }
            let mut owned = state.multisource_fixture.lock().map_err(|_| {
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "Generated multi-source fixture unavailable.",
                )
            })?;
            if action == "organization_multisource_audit" {
                let fixture = owned.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Create the generated multi-source fixture first.",
                    )
                })?;
                let audits = fixture
                    .sources
                    .iter()
                    .map(|source| source.audit_generated_outputs(fixture.destination.path()))
                    .collect::<Result<Vec<_>>>()?;
                let keeper = fixture
                    .keeper
                    .as_ref()
                    .map(|keeper| keeper.verify_unchanged())
                    .transpose()?;
                return Ok(
                    serde_json::json!({"scope":"constructor-owned-generated-only","audits":audits,"keeperVerifiedFiles":keeper,"productionAuthorization":false}),
                );
            }
            if action == "organization_multisource_fixture" && owned.is_some() {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated multi-source fixture already exists.",
                ));
            }
            if action == "organization_multisource_fixture" {
                let sources = (0..2)
                    .map(|_| stillharbour_core::library_fixture::LibraryFixture::generate())
                    .collect::<Result<Vec<_>>>()?;
                let destination = tempfile::Builder::new()
                    .prefix("stillharbour-multisource-destination-test-")
                    .tempdir()?;
                std::fs::write(
                    destination.path().join("destination-only.txt"),
                    b"retained destination",
                )?;
                *owned = Some(MultiSourceFixture {
                    sources,
                    destination,
                    registered: 0,
                    keeper: None,
                    keeper_root: None,
                });
            }
            let fixture = owned.as_mut().ok_or_else(|| {
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "Generated multi-source fixture unavailable.",
                )
            })?;
            let mut jobs = Vec::new();
            if action == "organization_multisource_keeper" {
                if fixture.registered != 2 || fixture.keeper_root.is_some() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Register both generated sources once before creating their comparison keeper.",
                    ));
                }
                if fixture.keeper.is_none() {
                    fixture.keeper = Some(fixture.sources[0].coherent_import_copy()?);
                }
                let keeper = fixture.keeper.as_ref().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "Generated comparison keeper unavailable.",
                    )
                })?;
                let handle = filesystem::open_directory_readonly(keeper.path())?;
                let identity = filesystem::identity(&handle)?;
                let volume = stillharbour_core::library::native::volume_token(&handle)?;
                let job = state.library.register_verified(
                    keeper.path().to_owned(),
                    (identity, volume),
                    "Generated multi-source import keeper".into(),
                    RootMode::Referenced,
                    ScanFilter::default(),
                )?;
                fixture.keeper_root = Some(job.root_id.clone());
                jobs.push(job);
            }
            if action == "organization_multisource_next" {
                let index = fixture.registered;
                let source = fixture.sources.get(index).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::Conflict,
                        "Both generated source libraries are already registered.",
                    )
                })?;
                source.verify_unchanged()?;
                let handle = filesystem::open_directory_readonly(source.path())?;
                let identity = filesystem::identity(&handle)?;
                let volume = stillharbour_core::library::native::volume_token(&handle)?;
                let job = state.library.register_verified(
                    source.path().to_owned(),
                    (identity, volume),
                    format!("Generated source partition {}", index + 1),
                    RootMode::Managed,
                    ScanFilter::default(),
                )?;
                fixture.registered += 1;
                jobs.push(job);
            }
            let path = fixture.destination.path().to_owned();
            let handle = filesystem::open_directory_readonly(&path)?;
            let identity = filesystem::identity(&handle)?;
            let volume = stillharbour_core::library::native::volume_token(&handle)?;
            let id = uuid::Uuid::new_v4().to_string();
            *state.selection.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Generated selection unavailable.")
            })? = Some(NativeSelection {
                id: id.clone(),
                path,
                identity: Some(identity),
                volume: Some(volume),
            });
            Ok(
                serde_json::json!({"jobs":jobs,"selection":{"id":id,"displayPath":"Generated multi-source destination","identityAvailable":true}}),
            )
        }
        "organization_managed_destination_fixture" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Generated destination accepts no supplied scope.",
                ));
            }
            let mut owned = state.managed_destination_fixture.lock().map_err(|_| {
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "Generated managed destination unavailable.",
                )
            })?;
            if owned.is_some() {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated managed destination already exists.",
                ));
            }
            let directory = tempfile::Builder::new()
                .prefix("stillharbour-managed-destination-test-")
                .tempdir()?;
            std::fs::write(
                directory.path().join("destination-only.txt"),
                b"retained managed destination",
            )?;
            let path = directory.path().to_owned();
            let handle = filesystem::open_directory_readonly(&path)?;
            let identity = filesystem::identity(&handle)?;
            let volume = stillharbour_core::library::native::volume_token(&handle)?;
            drop(handle);
            let job = state.library.register_verified(
                path.clone(),
                (identity.clone(), volume.clone()),
                "Generated managed copies destination".into(),
                RootMode::Managed,
                ScanFilter {
                    extensions: DEFAULT_PHOTO_EXTENSIONS
                        .iter()
                        .map(|value| (*value).into())
                        .collect(),
                    ..ScanFilter::default()
                },
            )?;
            let id = uuid::Uuid::new_v4().to_string();
            *state.selection.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Generated selection unavailable.")
            })? = Some(NativeSelection {
                id: id.clone(),
                path,
                identity: Some(identity),
                volume: Some(volume),
            });
            *owned = Some((directory, Some(job.root_id.clone())));
            Ok(
                serde_json::json!({"rootId":job.root_id,"jobId":job.id,"selection":{"id":id,"displayPath":"Generated managed copies destination","identityAvailable":true}}),
            )
        }
        "organization_managed_destination_audit_remove" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Generated destination accepts no supplied scope.",
                ));
            }
            let mut owned = state.managed_destination_fixture.lock().map_err(|_| {
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "Generated managed destination unavailable.",
                )
            })?;
            let (directory, id) = owned.as_mut().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Create the generated managed destination first.",
                )
            })?;
            let root_id = id.clone().ok_or_else(|| {
                AppError::new(
                    ErrorCode::Conflict,
                    "Generated destination was already removed.",
                )
            })?;
            if std::fs::read(directory.path().join("destination-only.txt"))?
                != b"retained managed destination"
                || std::fs::read_dir(directory.path())?
                    .take(2)
                    .collect::<std::io::Result<Vec<_>>>()?
                    .len()
                    != 1
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated destination changed during read-only review.",
                ));
            }
            stillharbour_core::library::destination_fixture::remove_empty_registration(
                &state.library,
                root_id,
                directory.path(),
            )?;
            *id = None;
            if std::fs::read(directory.path().join("destination-only.txt"))?
                != b"retained managed destination"
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated destination neighbor was not retained.",
                ));
            }
            Ok(
                serde_json::json!({"rootRemoved":true,"neighborPreserved":true,"destinationFilesChanged":false}),
            )
        }
        "organization_destination_fixture" | "organization_export_fixture" => {
            if state
                .fixture
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?
                .is_none()
            {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate an owned library fixture first.",
                ));
            }
            let is_export = action == "organization_export_fixture";
            let mut owned = if is_export {
                &state.export_fixture
            } else {
                &state.destination_fixture
            }
            .lock()
            .map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Destination fixture unavailable.")
            })?;
            if owned.is_some() {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Destination fixture already exists.",
                ));
            }
            let directory = tempfile::Builder::new()
                .prefix("stillharbour-destination-test-")
                .tempdir()?;
            std::fs::write(
                directory.path().join("destination-only.txt"),
                b"retained destination",
            )?;
            let path = directory.path().to_owned();
            let handle = filesystem::open_directory_readonly(&path)?;
            let identity = filesystem::identity(&handle)?;
            let volume = stillharbour_core::library::native::volume_token(&handle)?;
            let id = uuid::Uuid::new_v4().to_string();
            *state
                .selection
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Selection unavailable."))? =
                Some(NativeSelection {
                    id: id.clone(),
                    path,
                    identity: Some(identity),
                    volume: Some(volume),
                });
            *owned = Some(directory);
            Ok(
                serde_json::json!({"id":id, "displayPath":"Generated destination fixture", "identityAvailable":true}),
            )
        }
        "organization_destination_verify" | "organization_export_verify" => {
            let is_export = action == "organization_export_verify";
            let owned = if is_export {
                &state.export_fixture
            } else {
                &state.destination_fixture
            }
            .lock()
            .map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Destination fixture unavailable.")
            })?;
            let directory = owned.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate a destination fixture first.",
                )
            })?;
            let unchanged = std::fs::read(directory.path().join("destination-only.txt"))?
                == b"retained destination";
            if is_export {
                if !unchanged {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "The generated export neighbor changed.",
                    ));
                }
                let mut exports = Vec::new();
                for entry in std::fs::read_dir(directory.path())? {
                    let entry = entry?;
                    if entry.file_name() == "destination-only.txt" {
                        continue;
                    }
                    if !entry.file_type()?.is_file() || entry.metadata()?.len() > 16 * 1024 * 1024 {
                        return Err(AppError::new(
                            ErrorCode::Conflict,
                            "Unexpected generated export entry.",
                        ));
                    }
                    let text = std::fs::read_to_string(entry.path())?;
                    exports.push(serde_json::json!({"filename":entry.file_name().to_string_lossy(),"bytes":text.len().to_string(),"text":text}));
                }
                return Ok(serde_json::json!({"neighborUnchanged":true,"exports":exports}));
            }
            let unchanged = unchanged && std::fs::read_dir(directory.path())?.count() == 1;
            if !unchanged {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Generated destination changed.",
                ));
            }
            Ok(serde_json::json!({"unchanged":true, "entries":1}))
        }
        "initial_previews_hold" | "initial_previews_release" => {
            let fixture = state
                .fixture
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?;
            if fixture.is_none() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate an owned fixture first.",
                ));
            }
            state
                .library
                .hold_initial_previews_for_probe(action == "initial_previews_hold");
            Ok(serde_json::json!({"held":action=="initial_previews_hold"}))
        }
        "start" => {
            stillharbour_core::performance::enable();
            Ok(serde_json::json!({"pid":std::process::id()}))
        }
        "stop" => {
            stillharbour_core::performance::disable();
            Ok(serde_json::json!({"stopped":true}))
        }
        "read" => {
            let catalog = state._catalog.clone();
            tauri::async_runtime::spawn_blocking(move || -> Result<_> { Ok(serde_json::json!({"trace":stillharbour_core::performance::snapshot(),"sqlite":stillharbour_core::library::performance_probe::sqlite_info(&catalog)?})) }).await.map_err(|e|AppError::new(ErrorCode::WorkerFailed,"Diagnostic read stopped.").caused_by(e))?
        }
        "drain" => Ok(serde_json::json!({"trace":stillharbour_core::performance::drain()})),
        "monitor_stop" => {
            state.library.cancel_all();
            Ok(serde_json::json!({"stopped":true}))
        }
        "monitor" => {
            state.library.start_monitoring()?;
            Ok(serde_json::json!({"started":true}))
        }
        "import_comparison_fixture"
        | "import_comparison_verify"
        | "import_comparison_finish"
        | "import_skip_fixture"
        | "import_skip_verify"
        | "import_skip_finish" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Owned comparison fixtures accept no supplied scope.",
                ));
            }
            let source = state.fixture.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Owned source fixture unavailable.")
            })?;
            let source = source.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Generate the source fixture first.",
                )
            })?;
            let source_count = source.verify_unchanged()?;
            let skip_fixture = action.starts_with("import_skip_");
            let mut comparison = if skip_fixture {
                &state.import_skip_fixture
            } else {
                &state.comparison_fixture
            }
            .lock()
            .map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Comparison fixture unavailable.")
            })?;
            if action == "import_comparison_fixture" || action == "import_skip_fixture" {
                if comparison.is_some() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Comparison fixture already exists.",
                    ));
                }
                *comparison = Some((
                    if skip_fixture {
                        source.coherent_import_copy()?
                    } else {
                        source.renamed_comparison_copy()?
                    },
                    None,
                ));
                let (fixture, root_id) = comparison.as_mut().ok_or_else(|| {
                    AppError::new(ErrorCode::WorkerFailed, "Comparison fixture unavailable.")
                })?;
                let path = fixture.path().to_owned();
                let handle = stillharbour_core::library::native::open_root(&path, None)?;
                let identity = filesystem::identity(&handle)?;
                let volume = stillharbour_core::library::native::volume_token(&handle)?;
                drop(handle);
                let job = state.library.register_verified(
                    path,
                    (identity, volume),
                    if skip_fixture {
                        "Complete existing import bundle fixture".into()
                    } else {
                        "Existing import comparison fixture".into()
                    },
                    stillharbour_core::library::model::RootMode::Referenced,
                    stillharbour_core::library::model::ScanFilter::default(),
                )?;
                *root_id = Some(job.root_id.clone());
                return Ok(
                    serde_json::json!({"job":job,"sourceVerifiedEntries":source_count,"comparisonVerifiedEntries":fixture.verify_unchanged()?}),
                );
            }
            let (fixture, root_id) = comparison.as_mut().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Create the owned comparison fixture first.",
                )
            })?;
            let compared = fixture.verify_unchanged()?;
            if action == "import_comparison_finish" || action == "import_skip_finish" {
                let root = root_id
                    .as_ref()
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorCode::InvalidInput,
                            "No registered comparison root remains.",
                        )
                    })?
                    .clone();
                #[cfg(windows)]
                {
                    return Ok(
                        serde_json::json!({"retainedRoot":root,"catalogRegistrationRetained":true,"reason":"Windows cache cleanup still requires its native validation gate.","sourceVerifiedEntries":source_count,"comparisonVerifiedEntries":compared,"sourceFilesRemoved":false}),
                    );
                }
                #[cfg(not(windows))]
                {
                    let preview = state.library.prepare_maintenance(
                        stillharbour_core::library::model::MaintenanceTarget::Library {
                            root_id: root.clone(),
                        },
                    )?;
                    state.library.apply_maintenance(preview.id)?;
                    *root_id = None;
                    fixture.verify_unchanged()?;
                    source.verify_unchanged()?;
                    return Ok(
                        serde_json::json!({"removedRoot":root,"sourceVerifiedEntries":source_count,"comparisonVerifiedEntries":compared,"sourceFilesRemoved":false}),
                    );
                }
            }
            Ok(
                serde_json::json!({"sourceVerifiedEntries":source_count,"comparisonVerifiedEntries":compared,"rootId":root_id}),
            )
        }
        "fixture_reselect" => {
            if rows.is_some() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Owned source selection accepts no supplied scope.",
                ));
            }
            let fixture = state
                .fixture
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?;
            let fixture = fixture
                .as_ref()
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Generate fixture first."))?;
            fixture.verify_unchanged()?;
            let path = fixture.path().to_owned();
            let file = filesystem::open_directory_readonly(&path)?;
            let identity = filesystem::identity(&file)?;
            let volume = stillharbour_core::library::native::volume_token(&file)?;
            let id = uuid::Uuid::new_v4().to_string();
            *state.selection.lock().map_err(|_| {
                AppError::new(ErrorCode::WorkerFailed, "Fixture selection unavailable.")
            })? = Some(NativeSelection {
                id: id.clone(),
                path,
                identity: Some(identity),
                volume: Some(volume),
            });
            Ok(
                serde_json::json!({"id":id,"displayPath":"Generated source fixture","identityAvailable":true}),
            )
        }
        "fixture_location" => {
            let fixture = state
                .fixture
                .lock()
                .map_err(|_| AppError::new(ErrorCode::WorkerFailed, "Fixture unavailable."))?;
            let fixture = fixture
                .as_ref()
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Generate fixture first."))?;
            // Private harness channel only, never included in exported observations.
            let selection_id = state
                .selection
                .lock()
                .map_err(|_| {
                    AppError::new(ErrorCode::WorkerFailed, "Fixture selection unavailable.")
                })?
                .as_ref()
                .filter(|selection| selection.path == fixture.path())
                .map(|selection| selection.id.clone());
            Ok(serde_json::json!({"fixtureRoot":fixture.path(),"selectionId":selection_id}))
        }
        "seed" => {
            let rows = rows
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Fixture size required."))?;
            let catalog = state._catalog.clone();
            let directory = state._test_data.path().to_owned();
            tauri::async_runtime::spawn_blocking(move || {
                stillharbour_core::library::performance_probe::seed_catalog(
                    &catalog, &directory, rows,
                )
            })
            .await
            .map_err(|e| {
                AppError::new(ErrorCode::WorkerFailed, "Fixture seed stopped.").caused_by(e)
            })?
        }
        _ => Err(AppError::new(
            ErrorCode::InvalidInput,
            "Unknown diagnostic action.",
        )),
    }
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "--inspection-worker") {
        let result = if args.len() == 5 {
            stillharbour_core::inspection::decoder::worker(&args[2], &args[3], &args[4])
        } else {
            Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid inspection worker arguments.",
            ))
        };
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if args
        .get(1)
        .is_some_and(|arg| arg == "--media-worker-file-v2" || arg == "--media-worker-stdin-v2")
    {
        let result = if (3..=4).contains(&args.len()) {
            stillharbour_core::media::worker_v2_arguments(
                &args[2],
                args.get(3).map(|s| s.as_os_str()),
                args[1] == "--media-worker-file-v2",
            )
        } else {
            Err(stillharbour_core::domain::AppError::new(
                stillharbour_core::domain::ErrorCode::InvalidInput,
                "Invalid rendition worker arguments.",
            ))
        };
        if result.is_err() {
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--media-worker-file") {
        let result = if (3..=4).contains(&args.len()) {
            stillharbour_core::media::worker_file_arguments(
                &args[2],
                args.get(3).map(|s| s.as_os_str()),
            )
        } else {
            Err(stillharbour_core::domain::AppError::new(
                stillharbour_core::domain::ErrorCode::InvalidInput,
                "Invalid preview worker arguments.",
            ))
        };
        if result.is_err() {
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--diagnostic-sqlite") {
        if args.len() != 2 {
            eprintln!("SQLite diagnostics accepts no catalog or path arguments.");
            std::process::exit(2);
        }
        match stillharbour_core::catalog::sqlite_diagnostics() {
            Ok(info) => println!("{info}"),
            Err(_) => {
                eprintln!("SQLite build diagnostics could not be read.");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--media-worker-stdin") {
        let result = if (3..=4).contains(&args.len()) {
            stillharbour_core::media::worker_stdin_arguments(
                &args[2],
                args.get(3).map(|s| s.as_os_str()),
            )
        } else {
            Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid preview worker arguments.",
            ))
        };
        if result.is_err() {
            eprintln!("Preview unavailable; original unchanged.");
            std::process::exit(1);
        }
        return;
    }
    if args
        .get(1)
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| arg.starts_with("--media-worker"))
    {
        eprintln!("Invalid media worker invocation; app startup was not attempted.");
        std::process::exit(2);
    }
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(feature = "desktop-test")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    #[cfg(feature = "desktop-test")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        foundation_request,
        library_request,
        test_library_fixture,
        test_scroll_catalog,
        test_generated_rename,
        test_generated_rename_history,
        test_generated_rename_worker,
        test_performance
    ]);
    #[cfg(not(feature = "desktop-test"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        foundation_request,
        library_request
    ]);
    builder
        .register_asynchronous_uri_scheme_protocol("original", |context, request, responder| {
            let library = context.app_handle().state::<Foundation>().library.clone();
            let allowed =
                context.webview_label() == "main" && request.method() == tauri::http::Method::GET;
            tauri::async_runtime::spawn_blocking(move || {
                let result = (|| {
                    if !allowed {
                        return Err(AppError::new(
                            ErrorCode::InvalidInput,
                            "Unauthorized original-image request.",
                        ));
                    }
                    let parts: Vec<_> = request
                        .uri()
                        .path()
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if parts.len() != 3 {
                        return Err(AppError::new(
                            ErrorCode::InvalidInput,
                            "Invalid original-image URL.",
                        ));
                    }
                    library.original_image(parts[0].into(), parts[1].into(), parts[2])
                })();
                let response = match result {
                    Ok(image) => tauri::http::Response::builder()
                        .status(200)
                        .header("Content-Type", image.mime)
                        .header("X-Content-Type-Options", "nosniff")
                        .header("Cache-Control", "no-store")
                        .header("Content-Security-Policy", "default-src 'none'; sandbox")
                        .body(image.bytes),
                    Err(_) => tauri::http::Response::builder()
                        .status(404)
                        .header("Content-Type", "text/plain")
                        .header("Cache-Control", "no-store")
                        .body(Vec::new()),
                };
                if let Ok(response) = response {
                    responder.respond(response);
                }
            });
        })
        .register_asynchronous_uri_scheme_protocol("thumbnail", |context, request, responder| {
            let library = context.app_handle().state::<Foundation>().library.clone();
            let allowed = context.webview_label() == "main";
            tauri::async_runtime::spawn_blocking(move || {
                let result = (|| -> Result<Vec<u8>> {
                    if !allowed {
                        return Err(AppError::new(
                            ErrorCode::InvalidInput,
                            "Unauthorized preview request.",
                        ));
                    }
                    let parts: Vec<_> = request
                        .uri()
                        .path()
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if parts.len() != 4 {
                        return Err(AppError::new(
                            ErrorCode::InvalidInput,
                            "Invalid preview URL.",
                        ));
                    }
                    let edge = parts[2].parse().map_err(|_| {
                        AppError::new(ErrorCode::InvalidInput, "Invalid preview size.")
                    })?;
                    library.cached_thumbnail(
                        ThumbnailKey {
                            asset_id: parts[0].into(),
                            revision: parts[1].into(),
                            edge,
                        },
                        parts[3],
                    )
                })();
                let response = match result {
                    Ok(bytes) => tauri::http::Response::builder()
                        .status(200)
                        .header("Content-Type", "image/png")
                        .header("X-Content-Type-Options", "nosniff")
                        .header("Cache-Control", "private, max-age=31536000, immutable")
                        .body(bytes),
                    Err(_) => tauri::http::Response::builder()
                        .status(404)
                        .header("Cache-Control", "no-store")
                        .header("Content-Type", "text/plain")
                        .body(Vec::new()),
                };
                if let Ok(response) = response {
                    responder.respond(response);
                }
            });
        })
        .setup(|app| {
            // Debug instances have worktree-local catalogs. Production never uses
            // a cloud-synced checkout as a live catalog location.
            #[cfg(all(debug_assertions, not(feature = "desktop-test")))]
            let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.context/native-data");
            #[cfg(not(debug_assertions))]
            let data = app.path().app_local_data_dir()?;
            #[cfg(feature = "desktop-test")]
            let test_data = tempfile::Builder::new()
                .prefix("stillharbour-native-test-")
                .tempdir()?;
            #[cfg(feature = "desktop-test")]
            let data = test_data.path().to_owned();
            std::fs::create_dir_all(&data)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o700))?;
            }
            let catalog = Arc::new(Catalog::open(data.join("Foundation.stillharbour"))?);
            let summary = catalog.summary()?;
            let packaged = app.path().resolve(
                "helpers/exiftool/exiftool",
                tauri::path::BaseDirectory::Resource,
            )?;
            #[cfg(debug_assertions)]
            let packaged = if packaged.is_file() {
                packaged
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vendor/exiftool/exiftool")
            };
            let library = Library::new(
                catalog.clone(),
                LibraryOptions {
                    data_directory: data.clone(),
                    workers: WorkerPaths {
                        executable: std::env::current_exe()?,
                        exiftool: cfg!(target_os = "macos").then_some(packaged),
                        working_directory: data.join("worker"),
                    },
                },
            )?;
            #[cfg(not(feature = "desktop-test"))]
            library.start_monitoring()?;
            app.manage(Foundation {
                #[cfg(feature = "desktop-test")]
                _test_data: test_data,
                #[cfg(feature = "desktop-test")]
                rename_fixture: stillharbour_core::executor::GeneratedRenameController::default(),
                #[cfg(feature = "desktop-test")]
                fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                destination_fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                managed_destination_fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                multisource_fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                comparison_fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                import_skip_fixture: Mutex::new(None),
                #[cfg(feature = "desktop-test")]
                export_fixture: Mutex::new(None),
                _catalog: catalog,
                preferences: Arc::new(stillharbour_core::preferences::PreferencesStore::new(
                    &data,
                )?),
                library,
                status: FoundationStatus {
                    name: "Stillharbour".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    milestone: "M1".into(),
                    sqlite_version: summary.sqlite_version,
                    catalog_id: summary.id,
                    catalog_schema: summary.schema,
                    original_mutations_enabled: false,
                },
                selection: Mutex::new(None),
                picker: tokio::sync::Mutex::new(()),
            });
            let dev_origin = app
                .config()
                .build
                .dev_url
                .as_ref()
                .map(|url| url.origin().ascii_serialization());
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .on_navigation(move |url| {
                    (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                        || matches!(
                            url.origin().ascii_serialization().as_str(),
                            "http://tauri.localhost" | "https://tauri.localhost"
                        )
                        || (cfg!(debug_assertions)
                            && dev_origin.as_ref().is_some_and(|origin| {
                                *origin == url.origin().ascii_serialization()
                            }))
                })
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.state::<Foundation>().library.cancel_all();
            }
        })
        .run(tauri::generate_context!())
        .expect("Stillharbour could not start; catalog or native runtime is unavailable");
}
