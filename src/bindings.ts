// Generated from crates/core/src/{contract,domain,preferences,library/model}.rs. Run npm run bindings.
export const FOUNDATION_COMMAND = "foundation_request";
export const LIBRARY_COMMAND = "library_request";
export const THUMBNAIL_POLICY = "m5-v2-oriented-native-srgb8-preview";
export const DEFAULT_PHOTO_EXTENSIONS: string[] = ["jpg","jpeg","jfif","heic","heif","hif","png","tif","tiff","webp","avif","gif","bmp","dng","cr3","cr2","crw","nef","nrw","arw","sr2","srf","raf","orf","rw2","raw","pef","srw","x3f","3fr","fff","iiq","rwl"];
export type ErrorCode = "IO" | "CATALOG" | "CATALOG_IN_USE" | "CATALOG_SCHEMA_NEWER" | "UNSUPPORTED" | "CONFLICT" | "INVALID_INPUT" | "WORKER_FAILED" | "WORKER_TIMEOUT" | "BUSY" | "MUTATION_UNAVAILABLE";
export type AppError = { code: ErrorCode, message: string, };
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
export type LibraryRoot = { id: string, label: string, displayPath: string, mode: RootMode, isBase: boolean, availability: string, filter: ScanFilter, lastScan: string | null, fileCount: number, };
export type ScanJob = { id: string, rootId: string, state: string, discovered: number, indexed: number, skipped: number, errors: number, message: string, startedAt: string, };
export type LibraryStatus = { roots: Array<LibraryRoot>, jobs: Array<ScanJob>, revision: string, previewGeneration: string, metadataAvailable: boolean, previewAvailable: boolean, };
export type AssetQuery = { directoryId?: string, rootId: string | null, folder: string, name: string, fromDate: string, throughDate: string, minBytes: string, maxBytes: string, extension: string, favoritesOnly: boolean, tag: string, sort: string, includeMissing: boolean, };
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
export type ThumbnailResult = { assetId: string, revision: string, edge: number, state: string, error: string | null, };
export type JobDetail = { job: ScanJob, issues: Array<string>, };
export type FolderEntry = { path: string, count: number, };
export type AssetSeekPosition = "before" | "at";
export type MaintenanceTarget = { "kind": "library", root_id: string, } | { "kind": "preview_cache" };
export type MaintenancePreview = { id: string, target: MaintenanceTarget, rootLabel: string | null, rootDisplayPath: string | null, assetCount: number, cachedPreviews: number, cacheBytes: string, };
export type MaintenanceResult = { removedRootId: string | null, clearedEntries: number, };
export type CatalogDirectory = { id: string, name: string, state: string, };
export type DirectoryEntry = { "kind": "folder", directory: CatalogDirectory, } | { "kind": "file", asset: AssetSummary, media: boolean, };
export type DirectoryPage = { directory: CatalogDirectory, ancestors: Array<CatalogDirectory>, entries: Array<DirectoryEntry>, total: number, previousCursor: string | null, nextCursor: string | null, revision: string, };
export type LibraryRequest = { "kind": "browse_directory", root_id: string, directory_id: string | null, name: string, cursor: string | null, limit: number, } | { "kind": "prepare_maintenance", target: MaintenanceTarget, } | { "kind": "apply_maintenance", plan_id: string, } | { "kind": "seek_asset", query: AssetQuery, asset_id: string, position: AssetSeekPosition, limit: number, } | { "kind": "status", } | { "kind": "save_and_scan", selection_id: string, label: string, mode: RootMode, filter: ScanFilter, } | { "kind": "scan", root_id: string, filter: ScanFilter, } | { "kind": "cancel", job_id: string, } | { "kind": "job_details", job_id: string, } | { "kind": "folders", root_id: string, } | { "kind": "tags", } | { "kind": "relink", root_id: string, selection_id: string, } | { "kind": "update_mode", root_id: string, mode: RootMode, } | { "kind": "query", query: AssetQuery, cursor: string | null, limit: number, } | { "kind": "detail", asset_id: string, } | { "kind": "neighbors", query: AssetQuery, asset_id: string, } | { "kind": "seek", query: AssetQuery, year: string, limit: number, } | { "kind": "seek_date", query: AssetQuery, period: string, limit: number, } | { "kind": "date_buckets", query: AssetQuery, } | { "kind": "freeze_selection", query: AssetQuery, } | { "kind": "annotate", selection: AssetSelection, patch: AnnotationPatch, } | { "kind": "add_tags", selection: AssetSelection, tags: Array<string>, } | { "kind": "thumbnails", keys: Array<ThumbnailKey>, generation: string, } | { "kind": "filter_preview", filter: ScanFilter, paths: Array<string>, };
export type LibraryResponse = { "kind": "directory", "data": DirectoryPage } | { "kind": "maintenance_preview", "data": MaintenancePreview } | { "kind": "maintenance_done", "data": MaintenanceResult } | { "kind": "status", "data": LibraryStatus } | { "kind": "job", "data": ScanJob } | { "kind": "page", "data": PackedAssetPage } | { "kind": "detail", "data": AssetDetail } | { "kind": "neighbors", "data": AssetNeighbors } | { "kind": "buckets", "data": Array<DateBucket> } | { "kind": "selection", "data": SelectionToken } | { "kind": "updated", "data": number } | { "kind": "thumbnails", "data": Array<ThumbnailResult> } | { "kind": "filter_preview", "data": Array<string> } | { "kind": "job_details", "data": JobDetail } | { "kind": "folders", "data": Array<FolderEntry> } | { "kind": "tags", "data": Array<string> } | { "kind": "done" };
