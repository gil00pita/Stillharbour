# React Doctor review — 2026-11-06

This is a source refactor and diagnostic review, not a declaration that every
scanner finding is a defect or that all remaining findings are resolved.

## Reproducible scan

Use React Doctor **0.9.22**, JSON schema **3**, project **stillharbour**:

```sh
npx --yes react-doctor@0.9.22 --verbose --scope full --json --blocking none --yes --no-telemetry --no-supply-chain
npx --yes react-doctor@0.9.22 --verbose --scope changed --base origin/main --include-untracked --json --blocking none --yes --no-telemetry --no-supply-chain
```

Both full scans detect React 19.3.1 and report complete lint coverage with no
skipped checks. The baseline analyzes 329 files and reports **1 error / 152
warnings**. The final scan analyzes 436 files, including the new barrels and
private components, and reports **1 error / 150 warnings**. These are the same
full-project category selection. Telemetry, the remote score service and
supply-chain queries are deliberately disabled; there is no numeric health score
or dependency/performance claim.

The changed-scope scan reports no errors and two warnings. One is the existing
editable TemplateExamples index key. The other is an existing five-file repeated
header group whose representative moved from SnapshotWorkflow to
CopyContinuationWorkflow after file reorganization. Comparing full reports with
the old-to-new path mapping confirms that these are preexisting findings, not two
new failures. Do not compare changed-scope counts with full-scope counts.

Raw reports, the path map, per-rule guidance and logs are retained under the
ignored `.context/chakra-refactor/` directory. The published rule pages identify
plugin rule set 0.9.3; exact parity with the CLI's bundled plugin was not
established. Matching local `rules explain` output was also checked for the
cleanup rule. Source, types and tests decide applicability; a fetched recipe
alone does not prove a failure.

## Reviewed changes and exceptions

| Finding                                        | Outcome and evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SettingsScreen complexity and component length | Confirmed maintainability issue, fixed. Library locations, cache review, browsing defaults, folder defaults and heading now have private components. Draft, save, busy and focus ownership stays in the parent. Both findings disappear from the full scan; existing Settings tests pass.                                                                                                                                                                                                                                   |
| Basemap effect cleanup                         | The setup catch could leave a constructed map behind when later setup failed. Fixed by one disposal function that unsubscribes every retained subscription, disconnects observation and removes the map on both setup failure and unmount. Regression tests exercise normal teardown and a canvas-setup exception. The final scanner still reports the effect: its matcher does not follow this helper/array ownership. The current occurrence is rejected with source and regression evidence, without disabling the rule. |
| FileSizeField and TagInput labels              | Field.Root/Label retain the original explicit label association. FileSizeField uses Field.ErrorText plus native custom validity; the independent unit field does not inherit an amount error. Test assertions now wait for Chakra's automatic error association rather than reconstructing ARIA IDs.                                                                                                                                                                                                                        |
| Ordinary field composition                     | 73 existing labeled layouts now compose Field.Root/Label through `asChild`, preserving the outer native label and existing layout CSS. OrganizationTemplateFields is a private child so this composition does not introduce a new oversized OrganizationPreview.                                                                                                                                                                                                                                                            |
| StorageExplorer post-await state               | The camera worker checks its effect-local `active` flag after the awaited request and cleanup sets it false. This occurrence is rejected on that explicit cancellation evidence; the worker count stays bounded at two.                                                                                                                                                                                                                                                                                                     |
| CatalogOrganizer loading flag                  | The reviewed `change` function resets busy in `finally`, guarded by the same generation token as success/error setters. This occurrence is rejected on that source evidence. Other loading warnings are not automatically rejected by analogy.                                                                                                                                                                                                                                                                              |

MapLibre's documented [Subscription API](https://maplibre.org/maplibre-gl-js/docs/API/interfaces/Subscription/)
provides the unsubscribe operation used by the cleanup. Chakra's
[Field](https://chakra-ui.com/docs/components/field),
[native select](https://chakra-ui.com/docs/components/native-select) and
[recipe](https://chakra-ui.com/docs/theming/recipes) APIs inform the control
composition. Installed declarations were checked against the pinned versions.

## Remaining diagnostic work

| Rule group                                           | Final occurrences | Review state                                                                                                                                                       |
| ---------------------------------------------------- | ----------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| High-complexity functions / large components         |           41 / 17 | Needs evidence per component; Settings is fixed. Moving a component does not resolve its control-flow complexity.                                                  |
| Loading-flag reset                                   |                22 | Needs evidence for unreviewed callbacks; CatalogOrganizer's finally is a proven exception. Preserve cancellation/generation ownership when investigating the rest. |
| Repeated includes / find lookups                     |            17 / 1 | Performance observations. No trace or benchmark establishes a current bottleneck, so no speedup is claimed.                                                        |
| Parent state/data/callback effects                   |       14 / 5 / 10 | Needs evidence. Several propagate workflow activity to approval/navigation guards. Removing them solely to clear diagnostics could change those guards.            |
| Array-index keys                                     |                 8 | Needs evidence and stable UI identity design, especially editable template rows. No protocol or persisted identity change is introduced here.                      |
| Await placement / awaits in loops                    |             4 / 3 | Observations pending dependency/order evidence. Preserve bounded I/O, cancellation and recovery ordering.                                                          |
| Duplicate JSX groups                                 |                 2 | Maintainability observations; the five-file group's representative changed after relocation.                                                                       |
| Post-await effect state                              |                 2 | Storage's active guard is proven; InspectionScreen's result-epoch and disposal ownership needs a separate complete review.                                         |
| Derived initial state                                |                 1 | Needs evidence for the retained first-page contract; the parent keys StorageExplorer by viewed job ID.                                                             |
| Default-array identity / state used only by handlers |             1 / 1 | Observations pending consumer identity and render evidence; no performance improvement inferred.                                                                   |
| Interactive row focus                                |                 1 | Needs an accessibility review of the TextList row and its existing keyboard-reachable photo controls before changing tab order.                                    |
| Effect cleanup                                       |                 1 | Current Basemap diagnostic rejected with the explicit disposal and regression evidence above.                                                                      |

No diagnostic suppression or scanner configuration was added. The refactor does
not certify Windows, screen-reader coverage, release signing or original-file
recovery. Validation and any remaining native limitations are recorded in the
implementation log; milestone and preservation gates remain in force.

## Jobs page follow-up — 2026-11-06

The subsequent owner-requested navigation change replaces JobsDropdown with
JobsPage and a separate sidebar JobsButton. Its changed-scope regression scan
uses the same pinned command, schema and telemetry policy: zero errors and
three warnings. JobEntry's existing complexity finding moves with its file and
retains the identical cyclomatic 20 / cognitive 27 metrics; the page container's
former JobsDropdown complexity finding is removed. The other two warnings are
the existing repeated-header group and editable template index key described
above. No suppression is added. Reports for this follow-up live in
`.context/jobs-page/doctor.json`; the preceding full-scan counts belong to the
refactor checkpoint, not a new full scan of the page change.

## Header and photo-menu follow-up — 2026-11-06

The same pinned changed-scope command reports zero errors and the same three
warnings as the Jobs checkpoint: existing JobEntry complexity, repeated table
headers and the editable template index key. The header/menu change adds no
new diagnostic class. The scan uses local schema-3 JSON with telemetry,
remote scoring and supply-chain requests disabled; no numerical health score
is claimed. Receipt: `.context/chrome-selection/react-doctor.json`.

## Workspace consistency migration — 2026-11-07

The documented pinned changed-scope command reports zero errors and ten warnings
in four files after this broad JSX migration, with telemetry, supply-chain
queries and remote scoring still disabled. Seven warnings move with the existing
BackupWorkspace implementation into SyncScreen (complexity/length, activity
callbacks and existing lookup/await/index-key patterns). The remaining findings
are the re-identified table header subtree, formatting length in
SavedOrganizationPlans and OrganizationPreview's existing template-load reset:
that synchronous callback retires a pending preview when loading a template,
not a rejected await lacking cleanup. No suppression or unsafe change to activity
guards is introduced to satisfy the scanner. App's added source binding is
encapsulated by a catalog-scope helper; its intermediate complexity warning is
absent from the final scan. Raw reports: `.context/ui-react-doctor*.json`.

The design detector reports only seventeen advisory palette/radius differences in
existing CSS, including the owner-selected reference timeline. No new color
literal or decorative effect is added. The stale Impeccable sidecar is left
unchanged; a separate document refresh can reconcile it. These scans are source
evidence, not native, accessibility or performance certification.

## Whole-library formatter follow-up — 2026-11-07

The same pinned changed-scope scan reports zero errors and eleven warnings
(`.context/structure-react-doctor.json`). Ten match the preceding presentation
pass. The additional StructureFormatter loading-reset finding points directly
into its generation-guarded `finally` block. Rejection/cancellation/unmount tests
verify that it releases busy state or retires that generation; no suppression or
control-flow weakening is warranted. The obsolete selected-source App binding
has been removed; whole-library preview scope belongs to the native adapter.

Illustrated rule controls (2026-11-07): the same pinned changed-scope command
reports zero errors/eleven existing reviewed warnings after the new radio cards
and segmented controls. Neither new control nor the shared rule field introduces
a diagnostic. Receipt: `.context/rule-cards-react-doctor.json`; no suppression or
numeric remote score is used. Focused component checks pass 44 cases.

Expanded layouts (2026-11-07): the first scan adds one chip-builder complexity
warning. Extract its pure filtering/literal/path-label helpers; the final pinned
changed-scope scan returns to zero errors/eleven existing reviewed warnings with
no suppression (`.context/layouts-react-doctor-final.json`). Component behavior
and the actual native Custom request are rechecked after extraction. Scanner
coverage is source evidence, not physical input, metadata extraction or release
qualification.
