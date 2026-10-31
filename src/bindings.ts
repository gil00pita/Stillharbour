// Generated from crates/core/src/{contract,domain,preferences,organization,library/{model,original},rendition}.rs. Run npm run bindings.
export const FOUNDATION_COMMAND = "foundation_request";
export const LIBRARY_COMMAND = "library_request";
export const THUMBNAIL_POLICY = "p2-v3-tall-jpeg-srgb8-preview";
export const ORIGINAL_IMAGE_POLICY = "original-v2-static-heic-32mib-32mp";
export const DEFAULT_PHOTO_EXTENSIONS: string[] = ["jpg","jpeg","jfif","heic","heif","hif","png","tif","tiff","webp","avif","gif","bmp","dng","cr3","cr2","crw","nef","nrw","arw","sr2","srf","raf","orf","rw2","raw","pef","srw","x3f","3fr","fff","iiq","rwl"];
export const RENDITION_EDGES = [32, 256, 512, 1024, 2048] as const;
export const GALLERY_MAX_EDGE = 1024;
export const VIEWER_EDGE = 2048;
export const GALLERY_DECODED_BYTES = 134217728;
export type ErrorCode = "IO" | "CATALOG" | "CATALOG_IN_USE" | "CATALOG_SCHEMA_NEWER" | "UNSUPPORTED" | "CONFLICT" | "INVALID_INPUT" | "WORKER_FAILED" | "WORKER_TIMEOUT" | "BUSY" | "MUTATION_UNAVAILABLE";
export type AppError = { code: ErrorCode, message: string, };
export type MissingDatePolicy = "undated" | "block";
export type OrganizationTemplate = { pattern: string, event: string, missingDate: MissingDatePolicy, };
export type OrganizationIssue = "missing_capture_date" | "missing_value" | "invalid_path" | "source_unavailable" | "non_unicode_path" | "collision" | "possible_case_collision";
export type OrganizationRow = { assetId: string, rootId: string, sourcePath: string, sourceRevision: string,
/**
 * Display-only components derived by the backend from the lossless native
 * relative path. Old immutable rows omit this field and retain their digest.
 */
sourceComponents?: Array<string>, targetRelativePath: string | null, sequence: number, bytes: string, issues: Array<OrganizationIssue>, usedUndated: boolean,
/**
 * Same-stem indexed candidates are not confirmed bundles.
 */
companionCandidates: number, };
export type OrganizationPreview = { policy: string, fingerprint: string, catalogRevision: string, template: OrganizationTemplate, rows: Array<OrganizationRow>, proposed: number, blocked: number, selectedBytes: string, executionAvailable: boolean, };
export type OrganizationCompanion = { assetId: string, sourcePath: string, sourceRevision: string, bytes: string, presence: string, rootAvailability: string, isMedia: boolean, filenameSidecar: boolean, };
export type OrganizationCompanionReview = { assetId: string, catalogRevision: string, fingerprint: string, candidates: Array<OrganizationCompanion>, truncated: boolean, inventoryComplete: boolean, executionAvailable: boolean, };
export type OrganizationDestinationReview = { previewFingerprint: string, catalogRevision: string, destinationPath: string, fingerprint: string, executionAvailable: boolean, persisted: boolean, targets: Array<OrganizationTargetReview>, availableBytes: string, totalBytes: string, };
export type OrganizationTargetState = "unoccupied_observed" | "occupied" | "ancestor_not_directory" | "unsupported_ancestor" | "preview_blocked";
export type OrganizationTargetReview = { assetId: string, targetRelativePath: string | null, state: OrganizationTargetState, blockingComponent: string | null, };
export type TemplateRef = { id: string, version: number, };
export type TemplateSample = { relativePath: string, capture: string | null, sequence: number, expectedTarget: string, };
export type TemplateSave = { previous: TemplateRef | null, name: string, description: string, template: OrganizationTemplate, samples: Array<TemplateSample>, };
export type PlanDraftInput = { assetIds: Array<string>, template: OrganizationTemplate, previewFingerprint: string, templateReference: TemplateRef | null, };
export type TemplatePolicies = { schemaVersion: number, enabledTokens: Array<string>, datePolicy: string, filenameCasePolicy: string, collisionPolicy: string, sequencePolicy: string, sampleInputs: Array<string>, sampleOutputs: Array<string>, };
export type SavedTemplate = { reference: TemplateRef, name: string, description: string, createdAt: string, template: OrganizationTemplate, policies: TemplatePolicies, };
export type PlanState = "draft" | "ready" | "approved" | "running" | "completed" | "completed_with_errors" | "canceled" | "needs_recovery" | "invalidated";
export type PlanBlocker = "native_target_checks" | "companion_inventory" | "capacity_and_fidelity" | "execution_and_recovery";
export type OrganizationPlan = { id: string, policy: string, createdAt: string, digest: string, state: PlanState, templateReference: TemplateRef | null, preview: OrganizationPreview, destinationPath: string, destinationFingerprint: string, blockers: Array<PlanBlocker>, executionAvailable: boolean, };
export type OrganizationPlanDetail = { plan: OrganizationPlan, stale: boolean, };
export type OrganizationPlanSummary = { id: string, createdAt: string, digest: string, state: PlanState, destinationPath: string, proposed: number, blocked: number, selectedBytes: string, };
export type OrganizationPlanPage = { plans: Array<OrganizationPlanSummary>, next: string | null, };
export type PlanExportFormat = "json" | "csv";
export type PlanExport = { filename: string, mediaType: string, text: string, };
export type PlanExportSaved = { path: string, bytes: string, digest: string, };
export type BundleEntryKind = "file" | "directory" | "link_or_special" | "placeholder" | "unreadable";
export type BundleRelation = "primary" | "same_stem" | "filename_sidecar" | "apple_double" | "additional";
export type BundleInventoryEntry = { id: string, name: string, kind: BundleEntryKind, relation: BundleRelation, bytes: string | null, catalogAssetId: string | null, catalogPresence: string | null, nameRepresentable: boolean, selectable: boolean, error: string | null, };
export type BundleInventory = { assetId: string, sourceRevision: string, catalogRevision: string, fingerprint: string, entries: Array<BundleInventoryEntry>, inventoryComplete: boolean, truncated: boolean, indexedLimitExceeded: boolean, unobservedCandidates: Array<string>, executionAvailable: boolean, };
export type BundleConfirmationInput = { planId: string, assetId: string, sourceRevision: string, inventoryFingerprint: string, companionIds: Array<string>, reviewedAllEntries: boolean, };
export type ConfirmedBundleFile = { entryId: string, sourceName: string, targetRelativePath: string, relation: BundleRelation, bytes: string, };
export type BundleConfirmation = { id: string, planId: string, assetId: string, inventoryFingerprint: string, fingerprint: string, files: Array<ConfirmedBundleFile>, totalBytes: string, executionAvailable: boolean, };
export type AppPreferences = { version: number, defaultRootId: string | null, rootMode: RootMode, scanFilter: ScanFilter, galleryView: GalleryView, thumbnailSize: ThumbnailSize, gridFit: GridFit, sort: SortOrder, navigationCollapsed: boolean, };
export type GalleryView = "gallery" | "grid" | "list";
export type ThumbnailSize = "compact" | "comfortable" | "large";
export type GridFit = "contain" | "cover";
export type SortOrder = "capture" | "name" | "size" | "added" | "modified";
export type FoundationRequest = { "kind": "status", } | { "kind": "pick_directory", } | { "kind": "get_preferences", } | { "kind": "save_preferences", preferences: AppPreferences, };
export type FoundationStatus = { name: string, version: string, milestone: string, sqliteVersion: string, catalogId: string, catalogSchema: number, originalMutationsEnabled: boolean, };
export type DirectorySelection = { id: string,
/**
 * Display only. Native PathBuf and identity stay in the backend.
 */
displayPath: string, identityAvailable: boolean, };
export type FoundationResponse = { "kind": "status", "data": FoundationStatus } | { "kind": "directory", "data": DirectorySelection | null } | { "kind": "preferences", "data": AppPreferences };
export type RootMode = "referenced" | "managed";
export type ScanFilter = { version: number, includeGlobs: Array<string>, excludeGlobs: Array<string>, excludedFolders: Array<string>, extensions: Array<string>, includeHiddenFiles: boolean, includeHiddenFolders: boolean, minBytes: string | null, maxBytes: string | null, fromDate: string | null, throughDate: string | null, };
export type LibraryRoot = { preparingPreviews?: boolean, id: string, label: string, displayPath: string, mode: RootMode, isBase: boolean, availability: string, filter: ScanFilter, lastScan: string | null, fileCount: number, };
export type JobKind = "scan" | "thumbnails";
export type ScanJob = { kind?: JobKind, id: string, rootId: string, state: string, discovered: number, indexed: number, skipped: number, errors: number, message: string, startedAt: string, };
export type LibraryStatus = { monitoring?: Array<RootMonitor>, roots: Array<LibraryRoot>, jobs: Array<ScanJob>, revision: string, previewGeneration: string, metadataAvailable: boolean, previewAvailable: boolean, };
export type RootMonitor = { rootId: string, mode: string, message: string, };
export type LibraryChangeKind = "added" | "moved" | "updated" | "removed" | "reappeared";
export type LibraryChange = { version: number, sequence: string, rootId: string, rootLabel: string, assetId: string | null, entryKind: string, kind: LibraryChangeKind, previousNative: string | null, nativePath: string | null, previousPath: string | null, path: string | null, previousRevision: string | null, revision: string | null, observedAt: string, scanId: string, };
export type LibraryChangePage = { changes: Array<LibraryChange>, nextBefore: string | null, };
export type AssetQuery = { deferPendingPreviews?: boolean, directoryId?: string, rootId: string | null, folder: string, name: string, fromDate: string, throughDate: string, minBytes: string, maxBytes: string, extension: string, favoritesOnly: boolean, tag: string, sort: string, includeMissing: boolean, };
export type AssetSummary = { id: string, rootId: string, name: string, relativePath: string, folder: string, extension: string, detectedFormat: string | null, bytes: string, date: string, dateSource: string, rawDate: string | null, offset: string | null, modifiedAt: string | null, bornAt: string | null, addedAt: string, revision: string, presence: string, width: number | null, height: number | null, camera: string | null, metadataState: string, metadataError: string | null, previewState: string, previewError: string | null, favorite: boolean, rating: number, tags: Array<string>, };
export type AssetDetail = { asset: AssetSummary, root: LibraryRoot, rawMetadata: string | null, companions: Array<string>, };
export type AssetNeighbors = { previous: AssetSummary | null, next: AssetSummary | null, };
export type AssetPage = { assets: Array<AssetSummary>, nextCursor: string | null, total: number, revision: string, };
export type PackedAsset = [string, string, string, string, string, string, string | null, string, string, string, string | null, string | null, string | null, string | null, string, string, string, number | null, number | null, string | null, string, string | null, string, string | null, boolean, number, Array<string>];
export type PackedAssetPage = { format: number, rows: Array<PackedAsset>, nextCursor: string | null, total: number, revision: string, };
export type DateBucket = { year: string, count: number, months: Array<MonthBucket>, };
export type MonthBucket = { period: string, count: number, };
export type SelectionToken = { id: string, count: number, revision: string, };
export type AnnotationPatch = { favorite: boolean | null, rating: number | null, tags: Array<string> | null, };
export type AssetSelection = { "kind": "ids", ids: Array<string>, } | { "kind": "token", id: string, };
export type ThumbnailKey = { assetId: string, revision: string, edge: number, };
export type ThumbnailResult = { fallbackEdge?: number, assetId: string, revision: string, edge: number, state: string, error: string | null, };
export type JobDetail = { job: ScanJob, issues: Array<string>, };
export type FolderEntry = { path: string, count: number, };
export type AssetSeekPosition = "before" | "at";
export type MaintenanceTarget = { "kind": "library", root_id: string, } | { "kind": "preview_cache" };
export type MaintenancePreview = { id: string, target: MaintenanceTarget, rootLabel: string | null, rootDisplayPath: string | null, assetCount: number, cachedPreviews: number, cacheBytes: string, };
export type MaintenanceResult = { removedRootId: string | null, clearedEntries: number, };
export type CatalogDirectory = { id: string, name: string, state: string, };
export type DirectoryEntry = { "kind": "folder", directory: CatalogDirectory, } | { "kind": "file", asset: AssetSummary, media: boolean, };
export type DirectoryPage = { directory: CatalogDirectory, ancestors: Array<CatalogDirectory>, entries: Array<DirectoryEntry>, total: number, previousCursor: string | null, nextCursor: string | null, revision: string, };
export type LibraryRequest = { "kind": "organization_templates", } | { "kind": "organization_template_save", save: TemplateSave, } | { "kind": "organization_plans", before: string | null, } | { "kind": "organization_plan", plan_id: string, } | { "kind": "organization_plan_create", selection_id: string, draft: PlanDraftInput, } | { "kind": "organization_plan_export", plan_id: string, format: PlanExportFormat, } | { "kind": "organization_plan_save_export", selection_id: string, plan_id: string, format: PlanExportFormat, } | { "kind": "organization_destination", selection_id: string, asset_ids: Array<string>, template: OrganizationTemplate, preview_fingerprint: string, } | { "kind": "organization_companions", asset_id: string, } | { "kind": "organization_bundle_inventory", asset_id: string, source_revision: string, } | { "kind": "organization_bundle_confirm", input: BundleConfirmationInput, } | { "kind": "organization_bundle_reviews", plan_id: string, asset_id: string, } | { "kind": "organization_preview", asset_ids: Array<string>, template: OrganizationTemplate, } | { "kind": "changes", before: string | null, limit: number, } | { "kind": "browse_directory", root_id: string, directory_id: string | null, name: string, cursor: string | null, limit: number, } | { "kind": "prepare_maintenance", target: MaintenanceTarget, } | { "kind": "apply_maintenance", plan_id: string, } | { "kind": "seek_asset", query: AssetQuery, asset_id: string, position: AssetSeekPosition, limit: number, } | { "kind": "status", } | { "kind": "save_and_scan", selection_id: string, label: string, mode: RootMode, filter: ScanFilter, } | { "kind": "scan", root_id: string, filter: ScanFilter, } | { "kind": "generate_thumbnails", root_id: string, } | { "kind": "cancel", job_id: string, } | { "kind": "job_details", job_id: string, } | { "kind": "folders", root_id: string, } | { "kind": "tags", } | { "kind": "relink", root_id: string, selection_id: string, } | { "kind": "update_mode", root_id: string, mode: RootMode, } | { "kind": "query", query: AssetQuery, cursor: string | null, limit: number, } | { "kind": "detail", asset_id: string, } | { "kind": "neighbors", query: AssetQuery, asset_id: string, } | { "kind": "seek", query: AssetQuery, year: string, limit: number, } | { "kind": "seek_date", query: AssetQuery, period: string, limit: number, } | { "kind": "date_buckets", query: AssetQuery, } | { "kind": "freeze_selection", query: AssetQuery, } | { "kind": "annotate", selection: AssetSelection, patch: AnnotationPatch, } | { "kind": "add_tags", selection: AssetSelection, tags: Array<string>, } | { "kind": "thumbnails", keys: Array<ThumbnailKey>, generation: string, cache_only?: boolean, } | { "kind": "filter_preview", filter: ScanFilter, paths: Array<string>, };
export type LibraryResponse = { "kind": "organization_templates", "data": Array<SavedTemplate> } | { "kind": "organization_template", "data": SavedTemplate } | { "kind": "organization_plans", "data": OrganizationPlanPage } | { "kind": "organization_plan", "data": OrganizationPlanDetail } | { "kind": "organization_plan_export", "data": PlanExport } | { "kind": "organization_plan_export_saved", "data": PlanExportSaved } | { "kind": "organization_bundle_inventory", "data": BundleInventory } | { "kind": "organization_bundle_confirmation", "data": BundleConfirmation } | { "kind": "organization_bundle_reviews", "data": Array<BundleConfirmation> } | { "kind": "organization_destination", "data": OrganizationDestinationReview } | { "kind": "organization_companions", "data": OrganizationCompanionReview } | { "kind": "organization_preview", "data": OrganizationPreview } | { "kind": "changes", "data": LibraryChangePage } | { "kind": "directory", "data": DirectoryPage } | { "kind": "maintenance_preview", "data": MaintenancePreview } | { "kind": "maintenance_done", "data": MaintenanceResult } | { "kind": "status", "data": LibraryStatus } | { "kind": "job", "data": ScanJob } | { "kind": "page", "data": PackedAssetPage } | { "kind": "detail", "data": AssetDetail } | { "kind": "neighbors", "data": AssetNeighbors } | { "kind": "buckets", "data": Array<DateBucket> } | { "kind": "selection", "data": SelectionToken } | { "kind": "updated", "data": number } | { "kind": "thumbnails", "data": Array<ThumbnailResult> } | { "kind": "filter_preview", "data": Array<string> } | { "kind": "job_details", "data": JobDetail } | { "kind": "folders", "data": Array<FolderEntry> } | { "kind": "tags", "data": Array<string> } | { "kind": "done" };
