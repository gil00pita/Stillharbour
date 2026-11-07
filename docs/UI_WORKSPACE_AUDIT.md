# Workspace consistency audit

Owner request, 2026-11-07. Retain the existing dark photographic studio identity;
improve interaction design, field composition and alignment. The latest owner
direction supersedes the attachment's preserve-layout instruction. Base includes
`feat/collapsed-sidenav-alignment-fix` at `d1ddf6f`.

## Migration checklist

- Inspect all React surfaces and the existing Chakra 3.37.0 system before editing.
- Consolidate title, search and action geometry in the shared PageHeader.
- Reuse labelled search/field components; keep native input semantics and focus.
- Give Backup and Sync separate navigation destinations and existing workflows.
- Open Structure Formater directly on rules for the entire indexed library.
- Reserve timeline label space and keep the navigation rail visible at rest.
- Migrate applicable layout, typography, lists and tables across React components.
- Retain native elements needed for SVG, native forms/dialogs, range interaction,
  bidirectional filenames and third-party integrations; explain exceptions.
- Verify all page families at desktop and minimum window widths, including
  loading, empty, errors, selected photos and existing gated actions.
- Run frontend types, lint, tests, build, formatting and whitespace checks; record
  native evidence separately and retain both-platform CI as the merge gate.

## Findings and direction

| Surface          | Finding                                                                                           | Revision                                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Shared shell     | Page-specific toolbar rules compete with shared chrome; search widths and action alignment drift. | One 64 px title row, consistent horizontal inset, 36 px header controls and a bounded search lane.        |
| Photos           | Date output extends above the rail and beyond its allocated width; the rail fades away at rest.   | Contain labels inside the rail at both endpoints; keep years, position and date visible.                  |
| Albums / Tags    | Search has different composition from Photos/Map; primary action has a different height.          | Reuse the shared search frame and standard header control geometry.                                       |
| Folders          | Search is a plain field with different alignment; local navigation mixes scopes.                  | Shared search frame; keep Back/Up/breadcrumbs in the content toolbar.                                     |
| Duplicates       | The first-use introduction, real setup and retained results are a useful task model.              | Preserve the task model and conservative evidence/keeper controls.                                        |
| Integrity        | Long explanatory content competes with field groups.                                              | Shared field rhythm, divided setup sections and readable line lengths.                                    |
| Storage          | Measurement settings are already separated from results.                                          | Preserve details dialog, result canvas and incomplete-measurement status.                                 |
| Backup / Sync    | Versioned recovery and one-way file comparison share a mode switch.                               | Separate pages: Backup owns snapshots/recovery; Sync owns comparison and reviewed one-way copying.        |
| Organization     | A complex folder/filename workflow is buried in a photo-selection dialog.                         | Structure Formater opens on whole-library rules, paged global preview and saved plans; no selection step. |
| Map              | Search and filters should align with Photos without changing geographic membership.               | Preserve the shared place search and explicit Map → Photos handoff.                                       |
| Jobs             | Distinct status/history content already exists.                                                   | Retain mounted-page return behavior; align header actions.                                                |
| Settings / setup | Repeated labels, controls and help have uneven spacing and alignment.                             | Consistent label/control/help rhythm, bounded forms and responsive field groups.                          |
| Viewer / Compare | Specialized canvas controls need their existing geometry.                                         | Retain original/preview distinction, 44 px viewer controls, bounded decoding and focus return.            |

## Priority and evidence boundaries

The clipping chronology and inconsistent compact controls are P1 usability
issues. Combined Backup/Sync navigation, buried organization and uneven field
hierarchy are P2 discovery/composition issues. Retain the existing efficient
photo canvas, visible preservation limits and explicit review/approval model.

Review perspectives are a first-time library owner, a frequent batch organizer
and a keyboard user. Their main concerns are finding the right task, retaining
selection/context, and having named, reachable controls. This is one integrated
implementation review using source inspection, primary market references and
native screenshots; it is not an independent two-reviewer critique or a claim
that every accessibility/performance gate has passed.

## Reference rationale

[Lightroom workspace basics](https://helpx.adobe.com/lightroom-classic/desktop/workspace/workspace-basics.html)
separates tasks into workspaces with stable image and tool regions.
[Capture One interface overview](https://support.captureone.com/hc/en-us/articles/360002468797-User-interface-overview)
distinguishes browser, viewer and related tools.
[Apple Photos](https://support.apple.com/en-ie/guide/photos/pht23b129fed/12.0/mac/27)
uses persistent sidebar navigation and contextual toolbar controls.
These inform consistency and task separation; no competitor assets are shipped.

Use the existing warm charcoal, system type, sage actions and unchanged owner
logo. Avoid decorative cards around every field. Explanations belong beside the
decision they support; advanced evidence stays available through disclosures.
Original-file behavior, typed IPC, immutable previews, approvals, recovery,
helper restrictions and milestone enablement gates remain authoritative.

Structure Formater uses the entire catalog regardless of Map or album scope. App regressions verify direct rules entry from both origins without source selection. Ordinary Photos navigation retains its existing Map scope rules.

## Migration coverage and intentional exceptions

Ordinary containers, text/headings, lists, tables, images and buttons throughout
the component tree use Chakra v3. Unstyled recipe components preserve existing
CSS where it owns native or photographic geometry. New search/utility composition
uses Chakra props; existing recipes, virtual positions, map integration and
viewer animations keep their specialized styles. No dependency or theme-token
change is needed.

Native checkbox/range controls preserve direct checked/value and pointer/keyboard
semantics. Native labels/spans and the two small container adapters under asChild
are deliberate Chakra composition children. Native dialogs/disclosures preserve
showModal, cancel, focus return and summary behavior. SVG/image-canvas markup,
output/progress, definition lists, code, strong/emphasis, bdi/bdo and line breaks
retain their precise semantics. The ordered retained-copy list retains its
numbering offset through List.Root asChild. No ordinary raw button, text input,
textarea, select, table or image remains outside these composition boundaries.

Changed component implementations and colocated tests:

- `src/components/AlbumEditor/AlbumEditor.tsx`
- `src/components/AlbumsPage/AlbumsPage.tsx`
- `src/components/App/App.test.tsx`
- `src/components/App/App.tsx`
- `src/components/AssetBrowser/AssetBrowser.tsx`
- `src/components/AssetNotesPanel/AssetNotesPanel.tsx`
- `src/components/AssetTile/AssetTile.tsx`
- `src/components/BackupScreen/BackupScreen.test.tsx`
- `src/components/BackupScreen/BackupScreen.tsx`
- `src/components/Basemap/Basemap.tsx`
- `src/components/BatchToolbar/BatchToolbar.tsx`
- `src/components/BrowserControls/BrowserControls.tsx`
- `src/components/BundleInventoryReview/BundleInventoryReview.tsx`
- `src/components/CachedPreviewImage/CachedPreviewImage.tsx`
- `src/components/CatalogActions/CatalogActions.tsx`
- `src/components/CatalogOrganizer/CatalogOrganizer.tsx`
- `src/components/CatalogRestore/CatalogRestore.tsx`
- `src/components/CatalogSwitchStopped/CatalogSwitchStopped.test.tsx`
- `src/components/CatalogSwitchStopped/CatalogSwitchStopped.tsx`
- `src/components/ComparisonPreviews/ComparisonPreviews.tsx`
- `src/components/ComparisonViewer/ComparisonViewer.tsx`
- `src/components/CopyAssessmentReview/CopyAssessmentReview.tsx`
- `src/components/CopyContinuationWorkflow/CopyContinuationWorkflow.tsx`
- `src/components/CopyJobProgress/CopyJobProgress.tsx`
- `src/components/CopyManifestReview/CopyManifestReview.tsx`
- `src/components/DateHeading/DateHeading.tsx`
- `src/components/DateTimeline/DateTimeline.tsx`
- `src/components/DuplicateComparison/DuplicateComparison.tsx`
- `src/components/DuplicateMatchOptions/DuplicateMatchOptions.tsx`
- `src/components/DuplicateStudio/DuplicateStudio.tsx`
- `src/components/DuplicateStudio/components/CleanupRules.tsx`
- `src/components/DuplicateStudio/components/DuplicateDetail.tsx`
- `src/components/DuplicateStudio/components/ReviewDialog.tsx`
- `src/components/ExplorerCanvas/ExplorerCanvas.tsx`
- `src/components/ExportHistory/ExportHistory.tsx`
- `src/components/FavoriteHeart/FavoriteHeart.tsx`
- `src/components/FileDetails/FileDetails.tsx`
- `src/components/FileSizeField/FileSizeField.tsx`
- `src/components/FilterFields/FilterFields.tsx`
- `src/components/FilterPanel/FilterPanel.tsx`
- `src/components/FolderExplorer/FolderExplorer.tsx`
- `src/components/GeneratedRenameLab/GeneratedRenameLab.tsx`
- `src/components/ImportComparisonReview/ImportComparisonReview.tsx`
- `src/components/ImportDestinationSettings/ImportDestinationSettings.tsx`
- `src/components/ImportOutputIndexReview/ImportOutputIndexReview.tsx`
- `src/components/ImportPlanReview/ImportPlanReview.tsx`
- `src/components/InspectionChoiceCards/InspectionChoiceCards.tsx`
- `src/components/InspectionDialog/InspectionDialog.tsx`
- `src/components/InspectionIssues/InspectionIssues.tsx`
- `src/components/InspectionJobProgress/InspectionJobProgress.tsx`
- `src/components/InspectionScreen/InspectionScreen.tsx`
- `src/components/Inspector/Inspector.tsx`
- `src/components/IntegrityFindings/IntegrityFindings.tsx`
- `src/components/JobsButton/JobsButton.tsx`
- `src/components/JobsPage/JobsPage.tsx`
- `src/components/KeeperControls/KeeperControls.tsx`
- `src/components/KeywordImport/KeywordImport.tsx`
- `src/components/LibraryChanges/LibraryChanges.tsx`
- `src/components/LibraryHeader/LibraryHeader.tsx`
- `src/components/LibraryMaintenance/LibraryMaintenance.tsx`
- `src/components/LibraryScreen/LibraryScreen.tsx`
- `src/components/MapFilters/MapFilters.tsx`
- `src/components/MapScreen/MapScreen.tsx`
- `src/components/MetadataExportPanel/MetadataExportPanel.tsx`
- `src/components/OldDestinationBackup/OldDestinationBackup.tsx`
- `src/components/OrganizationCompanions/OrganizationCompanions.tsx`
- `src/components/OrganizationDestination/OrganizationDestination.tsx`
- `src/components/OrganizationPreview/OrganizationPreview.tsx`
- `src/components/OrganizationPreview/components/OrganizationTemplateFields.tsx`
- `src/components/OrganizationTemplates/OrganizationTemplates.tsx`
- `src/components/OrganizationTrees/OrganizationTrees.tsx`
- `src/components/OriginalHistoryReview/OriginalHistoryReview.tsx`
- `src/components/OriginalJobProgress/OriginalJobProgress.tsx`
- `src/components/OriginalPhoto/OriginalPhoto.tsx`
- `src/components/PhotoAlbumActions/PhotoAlbumActions.tsx`
- `src/components/PhotoCollection/PhotoCollection.tsx`
- `src/components/PhotoViewer/PhotoViewer.tsx`
- `src/components/PreparationBanner/PreparationBanner.tsx`
- `src/components/RetainedCopyInspection/RetainedCopyInspection.tsx`
- `src/components/RootPartitionCopyWorkflow/RootPartitionCopyWorkflow.tsx`
- `src/components/RootSetup/RootSetup.tsx`
- `src/components/SavedImportPlans/SavedImportPlans.tsx`
- `src/components/SavedOrganizationPlans/SavedOrganizationPlans.tsx`
- `src/components/SavedSearchReview/SavedSearchReview.tsx`
- `src/components/ScanJobProgress/ScanJobProgress.tsx`
- `src/components/ScanOptions/ScanOptions.tsx`
- `src/components/ScanSettingsGroup/ScanSettingsGroup.tsx`
- `src/components/SearchField/SearchField.tsx`
- `src/components/SettingsScreen/SettingsScreen.tsx`
- `src/components/SettingsScreen/components/BrowsingDefaults.tsx`
- `src/components/SettingsScreen/components/FolderDefaults.tsx`
- `src/components/SettingsScreen/components/LibraryLocations.tsx`
- `src/components/SettingsScreen/components/PreviewCacheSettings.tsx`
- `src/components/SettingsScreen/components/SettingsHeading.tsx`
- `src/components/SnapshotProfileForm/SnapshotProfileForm.tsx`
- `src/components/SnapshotRecoveryInspection/SnapshotRecoveryInspection.tsx`
- `src/components/SnapshotWorkflow/SnapshotWorkflow.tsx`
- `src/components/StorageExplorer/StorageExplorer.tsx`
- `src/components/StorageExplorer/components/StorageMap.tsx`
- `src/components/StorageExplorer/components/StorageOverview.tsx`
- `src/components/StorageReview/StorageReview.tsx`
- `src/components/StructureFormatter/StructureFormatter.test.tsx`
- `src/components/StructureFormatter/StructureFormatter.tsx`
- `src/components/Switch/Switch.tsx`
- `src/components/SyncScreen/SyncScreen.test.tsx`
- `src/components/SyncScreen/SyncScreen.tsx`
- `src/components/SyncWorkflow/SyncWorkflow.tsx`
- `src/components/TagInput/TagInput.tsx`
- `src/components/TagTools/TagTools.tsx`
- `src/components/TemplateExamples/TemplateExamples.tsx`
- `src/components/TextList/TextList.tsx`
- `src/components/UtilityWelcome/UtilityWelcome.tsx`
- `src/components/VerifiedCopyWorkflow/VerifiedCopyWorkflow.tsx`
- `src/components/ViewerControls/ViewerControls.tsx`
- `src/components/ViewerImage/ViewerImage.tsx`

Other changed files:

- `DESIGN.md`
- `PRODUCT.md`
- `docs/ACCEPTANCE_TESTS.md`
- `docs/CODEX_SPEC.md`
- `docs/DECISIONS.md`
- `docs/DEVELOPMENT.md`
- `docs/IMPLEMENTATION_LOG.md`
- `docs/M2_ORGANIZATION_PREVIEWS.md`
- `docs/REACT_DOCTOR_REVIEW.md`
- `docs/TIMELINE_NAVIGATION.md`
- `docs/TYPESCRIPT_STRUCTURE.md`
- `docs/UI_WORKSPACE_AUDIT.md`
- `docs/USER_GUIDE.md`
- `docs/UTILITIES_UI.md`
- `src/backup/backup.css`
- `src/components/SearchField/index.ts`
- `src/components/StructureFormatter/index.ts`
- `src/components/SyncScreen/index.ts`
- `src/map/map.css`
- `src/styles.css`
- `tests/desktop/foundation.e2e.mjs`
- `tests/desktop/m4-application.e2e.mjs`
- `tests/desktop/page-chrome.e2e.mjs`
- `tests/desktop/structure-formatter.e2e.mjs`
- `tests/desktop/timeline-navigation.e2e.mjs`
- `tests/desktop/utilities-scans.e2e.mjs`

## Whole-library scope correction — 2026-11-07

The owner's follow-up supersedes the earlier selected-photo handoff. The page
opens immediately on rules and uses every indexed media item, independent of
Photos/Map/album filters. Progress/cancel, bounded result paging, global sequence
and cross-page conflicts replace the source chooser. Native TEMP state is
revision-bound; no original-file operation is enabled. The separate explicit
name-preview/copy-plan workflow retains its existing bounds. See ADR-0095 and
the implementation log for fresh checks. Earlier screenshot/test receipts
reflect the earlier design and are not current evidence for this page.

Additional scope implementation: `crates/core/src/library/organization_library.rs`,
its module/typed request/response registration, the shared pure renderer and
exported bindings, the native command dispatcher, shared rule/issue-label exports,
and [ADR-0095](decisions/ADR-0095-whole-library-structure-preview.md). The original
migration list above describes the initial presentation pass; this addition
changes read-only native preview behavior without enabling original operations.

## Illustrated structure rules — 2026-11-07

Date-based presets now use three illustrated RadioCard choices. OrganizationPresets
owns their patterns, SVG examples and recipe; SegmentedControl owns the two small
policy groups. Event naming is removed from this editor. Custom paths and
historical saved rules remain compatible. The native test captures responsive
rules/results under `.context/ui-redesign/structure-rule-cards-*`; whole-library
scope and the original-file execution gate are unchanged. See the implementation
log for exact component/native checks and instrumented keyboard-input limits.

## Expanded structure layouts — 2026-11-07

The latest owner revision expands the date/location/tag/camera choices and adds
an input with predefined folder chips for Custom. OrganizationPresets holds the
25 grouped choices; StructurePathBuilder owns adding/removing/reordering levels.
Native organization metadata projection, optional sample data and policy-v2
resolution make the options actual read-only previews. Existing rules and
original-file gates remain. ADR-0096 documents field/reader limitations. Native
screenshots and count/prefix receipts are under `.context/ui-redesign/structure-layouts-*`.

## Rules-to-review refinement — 2026-11-07

The owner-supplied local Stillharbour.html prototype informs the editing/review
composition; its identity assets and simulated file operations are not shipped
as native capabilities. The existing studio palette and selected SVG remain.
Separate folder cards from naming settings, pair editing with explicitly
illustrative native tree/path examples, and move actual library results to a
focused review with global counts, full-scope search/filters and missing-date
inspection. Native cached TEMP summaries avoid shipping the entire report to
React. Session presets are transient and distinct from catalog templates/plans.

Confirmation rechecks a native read-only preview before explicitly simulated
completion. Native automation verifies no organization mutation requests and
unchanged generated sources; actual execution stays gated. Screenshots/receipts
use `.context/ui-redesign/structure-flow-*`. ADR-0097 and the implementation log
record bounds, tests and qualification limits, including missing indexed metadata
and unqualified physical-input/Windows behavior.
