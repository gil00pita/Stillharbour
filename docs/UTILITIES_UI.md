# Utilities and duplicate review

Owner-requested interface revision, 2026-11-05. The attached Duplicate Studio prototype supplies interaction ideas; its branding, images, sample measurements and deletion behaviour are not application assets or safety evidence.

## Navigation and search

Secondary buttons use flat neutral fills without visible outlines; focus outlines and selected states remain visible. Utilities groups Duplicates, Integrity, Storage and Backup & Sync. Map remains library navigation. Jobs opens its existing nonmodal disclosure from the sidebar footer immediately above Settings, in expanded and collapsed navigation. Opening Jobs preserves the current workspace.

Photos, Albums / Tags and Map use one search field for filenames, calendar days and offline city/country suggestions. Typing continues ordinary filename/date searching. Choosing an explicit place clears the lookup text and applies its versioned GPS membership ID; subsequent text can narrow that place. Clear removes the place filter. Suggestions distinguish same-name cities, disclose GPS coverage, handle errors/retries, and support keyboard entry, Down to suggestions and Escape dismissal. The suggestion panel closes when a filename/date has no matching places, keeping the photo results clear. No separate Places button remains. Dates and location filters can still be combined through the filter panel.

## Different tools for different questions

- Duplicates presents a cluster queue and an evidence inspector, with file counts, potential duplicate counts, exact/candidate separation, observed-name search, page-bounded group selection and keeper choices.
- Integrity presents presence, content-check and baseline states. Needs attention includes unavailable entries, decoder warnings, read changes and changed baselines. A changed checksum is not proven damage; no repair or baseline reset is offered.
- Storage retains volume capacity, recursive folder totals, treemap navigation, logical/allocated size accounting and largest-file observations. It no longer repeats health and checksum columns.

Inspection setup remains an expandable section. Existing start/cancel, retained-job selection, issue paging, coverage and native root checks remain connected. All observations are historical until rechecked. Search uses file names and paths available in the current findings page; groups without loaded names can still be found by their ID or evidence label. Group paging retires the previous page's selection.

## Comparison and keeper review

Choose two indexed media members from the selected group. Side by side, the draggable/keyboard range slider and a difference heatmap render the actual cached previews, up to 512 px with smaller-tier fallback. The heatmap colours a difference blend of unaligned rendered previews: black is unchanged; warm/pale pixels show increasing differences. It includes scaling, colour and framing differences and is not original-pixel equality, damage analysis or cleanup authority. Missing/failed previews show a reason and Retry; overlays require two available previews.

The evidence inspector shows the backend grouping explanation, exact-byte status and available dimensions, capture provenance and catalog annotation differences. It explicitly identifies unmeasured audio fingerprints, framing percentages and export lineage. RAW recognition is a filename-based keeper preference, not proof of camera-original provenance.

Resolution, RAW and largest-file suggestions operate only after complete member loading, retain ties and keep every member when resolution is unknown. Size comparisons retain integer precision. Manual keep-multiple and backend-retained folder protection/preference reviews remain available. Filesystem-date policies in that backend review retain their explicit names; they are never relabelled as capture or edit history.

## Cleanup gates and unavailable signals

The rules drawer closes with its Close action or Escape and restores the triggering control. Its keeper preference is shared with the selected cluster and survives reopening within the page. The rules drawer previews the selected groups (or the current page), a session keeper preference, protection requirements and a visual threshold. Visual scores are not emitted by the current inspection engine: every threshold therefore has zero scored eligible groups. Exact-byte evidence is never presented as a fabricated visual percentage. Newest edit, oldest capture and album-count preferences remain visibly unavailable because this inspection interface does not supply their authoritative ranking data.

Favourites, RAW files, video masters, shared albums and edited derivatives are presented as protection requirements for a future native cleanup plan. These drawer choices are session preview requirements, not saved/enforced removal policy. Unknown lineage/membership cannot be bypassed by unchecking them. Existing native folder protection is retained separately in the keeper review.

Recoverable physical storage remains unknown. Logical duplicate bytes are reported only for complete exact-byte groups whose entries have distinct known objects, one link each and equal recorded sizes. They do not imply reclaimable disk blocks. Same-volume quarantine frees zero bytes.

Can automate explains why no groups qualify in this build. Preview cleanup opens explicit blocked confirmation; Move to Recently Deleted and Undo remain disabled. Recently Deleted explains that no files have been moved by this workflow. There is no simulated success, automatic expiry, purge, renderer mutation or new production permission. Real removal/restore still requires an immutable native plan, fresh full-byte survivor/bundle checks, independent exact-version recovery and the existing native failure/recovery gates. The prototype's 30-day expiry is intentionally not adopted.

## Verification

Run `npm run check`, `npm run desktop:test:build`, then `npm run desktop:test` on generated fixtures only. Component coverage includes cluster search/filter/selection, conservative keeper choices and accounting, blocked cleanup/Undo, preview modes/retry and integrity distinctions. Native coverage updates existing toolbar/footer assertions, actual offline place selection, inspection table checks, rules confirmation, comparison screenshots when an indexed media pair exists, and minimum-window bounds. The native generated media scope may contain no matching photo pair; a separate visibly labelled browser fixture checks the filled queue, comparison pixels and narrow composition without claiming native pair evidence. A browser render does not establish native integration. Record actual outcomes and remaining platform/release limits in IMPLEMENTATION_LOG.md. Restore production frontend output with `npm run build` after instrumented native tests.

The v3 anchored search composition follows [Chakra Popover](https://chakra-ui.com/docs/components/popover); the heatmap is display-only and never attempts to bypass the WebView's image security boundary.
