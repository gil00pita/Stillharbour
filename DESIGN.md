---
name: Stillharbour
description: Quiet native photo workspace — photo canvas, indexed folders and cached viewer
colors:
  background: '#16191b'
  sidebar: '#121516'
  surface: '#1d2123'
  text: '#e7e9e8'
  border: '#303638'
  muted: '#a9b3b2'
  secondary: '#c7cecb'
  accent: '#b6d6c8'
  action-text: '#162b22'
  action-hover: '#cee5da'
  navigation: '#26322e'
  navigation-text: '#e2ece7'
  library-navigation: '#263a30'
  control: '#1d2123'
  control-border: '#3c4643'
  control-hover: '#232827'
  pressed: '#3c5147'
  selected: '#2e4237'
  field: '#161a1c'
  field-border: '#46504f'
  switch-border: '#75847d'
  tag: '#2d4036'
  preview: '#232827'
  availability-background: '#352c23'
  availability-text: '#ead3b8'
  sidebar-version: '#a9b3b2'
  error-border: '#826052'
  error-background: '#30231e'
  error-text: '#f2c5b4'
typography:
  headline:
    {
      fontFamily: '-apple-system, BlinkMacSystemFont, Segoe UI, sans-serif',
      fontSize: 'clamp(24px, 3vw, 31px)',
      fontWeight: 500,
      lineHeight: 1.2,
      letterSpacing: '-0.02em',
    }
  title: { fontSize: '18px', fontWeight: 500 }
  panel-title: { fontSize: '17px', letterSpacing: '0' }
  filter-title: { fontSize: '18px' }
  control: { fontSize: '13px', lineHeight: '20px', minHeight: '36px' }
  body: { fontSize: '14px', lineHeight: 1.5 }
  label: { fontSize: '12px', lineHeight: '16px' }
rounded:
  small: '4px'
  photo-container: '5px'
  control: '8px'
  action: '8px'
  navigation: '8px'
  container: '8px'
  switch: '12px'
spacing:
  action: '12px 24px'
  selected-folder: '20px'
  workspace: '32px 48px'
  library: '20px 24px 0'
  photo-gutter: '4px'
  control: '7px 12px'
  field: '7px 12px'
components:
  button-primary:
    {
      backgroundColor: '{colors.accent}',
      textColor: '{colors.action-text}',
      rounded: '{rounded.action}',
      padding: '{spacing.action}',
    }
  button-primary-hover: { backgroundColor: '{colors.action-hover}' }
  navigation-current:
    {
      backgroundColor: '{colors.navigation}',
      textColor: '{colors.navigation-text}',
      rounded: '{rounded.navigation}',
      padding: '12px',
    }
  library-navigation-current:
    {
      backgroundColor: '{colors.library-navigation}',
      textColor: '{colors.navigation-text}',
      rounded: '{rounded.control}',
      padding: '12px',
    }
  button-secondary:
    {
      backgroundColor: '{colors.control}',
      textColor: '{colors.navigation-text}',
      rounded: '{rounded.control}',
      padding: '{spacing.control}',
    }
  button-secondary-hover: { backgroundColor: '{colors.control-hover}' }
  button-pressed: { backgroundColor: '{colors.pressed}' }
  field:
    {
      backgroundColor: '{colors.field}',
      textColor: '{colors.text}',
      rounded: '{rounded.control}',
      padding: '{spacing.field}',
    }
  asset-selected: { backgroundColor: '{colors.selected}', rounded: '{rounded.control}' }
  explorer-entry: { textColor: '{colors.text}', rounded: '{rounded.small}', padding: '8px' }
  explorer-glyph: { textColor: '{colors.accent}', height: '132px' }
  selected-folder: { rounded: '{rounded.container}', padding: '{spacing.selected-folder}' }
  switch-off:
    {
      backgroundColor: '{colors.control-border}',
      rounded: '{rounded.switch}',
      width: '36px',
      height: '22px',
    }
  switch-on: { backgroundColor: '{colors.accent}' }
  switch-thumb-off: { backgroundColor: '{colors.text}', size: '14px' }
  switch-thumb-on: { backgroundColor: '{colors.action-text}' }
  tag-editor:
    {
      backgroundColor: '{colors.field}',
      textColor: '{colors.text}',
      rounded: '{rounded.control}',
      padding: '4px 5px',
    }
  tag:
    {
      backgroundColor: '{colors.tag}',
      textColor: '{colors.navigation-text}',
      rounded: '{rounded.small}',
      padding: '0 0 0 8px',
    }
  filter-panel:
    {
      backgroundColor: '{colors.surface}',
      textColor: '{colors.text}',
      width: 'min(380px, 92vw)',
      height: '100vh',
    }
  job-progress-track:
    { backgroundColor: '{colors.border}', rounded: '{rounded.small}', height: '6px' }
  job-progress-fill: { backgroundColor: '{colors.accent}' }
---

# Design System: Stillharbour

## Overview

**Creative North Star: "Photo canvas inside the native workspace"**

The working library lets photographs lead inside Stillharbour’s restrained native shell. Graphite surfaces, warm white system text and sparse sea-green state treatments carry the controls; aspect-preserving rows and small day headings carry the collection. The spacious first-run folder choice gives way to compact browsing, explicit selection and an immersive cached-preview viewer. Settings extends the same flat native workspace with divided folder rows, shared controls and explicit defaults. Folders uses that shell for named libraries, immediate child folders and indexed files, with an image-free Details view beside Icons.

React renders inside Tauri; the backend owns the catalog, filesystem access and preview work. The owner confirmed the gallery/viewer scope on 2026-10-28 and kept similarity grouping unavailable. Compare supports visual inspection of explicit selections only.

**Key Characteristics:**

- Photographs lead; compact controls remain quiet and reachable.
- Four-pixel gutters, real date context and left-aligned incomplete rows preserve the image rhythm.
- Availability, selection and catalog-only edits have explicit words and semantics.
- Settings separates indexed-folder actions, selected-library scan settings and saved browsing/new-folder defaults.
- Folders preserves directory context through Back/Up, breadcrumbs and named entries; Photos retains its chronological canvas.

Use Stillharbour, the descriptor “Photo & Video Library”, and the unchanged owner aperture SVG at `assets/branding/stillharbour-logo.svg`. The mark renders at 32 × 34px beside the wordmark. Reference screenshots are inspiration, not application artwork. Images in the supplied M1 evidence are generated test fixtures, not a bundled sample library.

Preservation stays visible through specific states: choosing is separate from saving and scanning, catalog notes leave originals and sidecars unchanged, and unavailable previews remain explained entries. M2 original-file operations, map and backup remain unavailable. “Albums / Tags” currently leads to tags; it does not establish saved albums.

## Colors

Sea green identifies the primary action, folder outline, selection, pressed view controls and keyboard focus. Dark neutral layers separate the identity rail, workspace, fields and preview wells; thin border lines define regions. The frontmatter retains the initial Library-link color and adds the delivered library navigation and control states.

Warm white carries primary content; muted grey carries provenance, counts, paths and explanations. Availability notices use a warm brown surface and pale warm text. Library removal reuses the pale warm text for its secondary action label; retain explicit warning copy alongside that color. Errors use the existing warm border, surface and text tokens together with explicit messages and an alert role. The multicolour owner logo retains its original fills.

**The State Has Words Rule.** Selection, availability, job progress and errors must have textual or semantic states alongside their color treatment.

## Typography

All interface text uses the system UI stack in the frontmatter; there is no downloaded or separate display font. The empty-state headline is balanced and centred, with no supporting paragraph. It reads “Start by selecting your Photo library folder.” The wordmark is 18px/600 with −0.01em tracking, its descriptor 11px/400 with ordinary tracking and 16px line height. At widths ≤850px the wordmark becomes 16px while the descriptor remains 11px.

The shell toolbar uses the title role. The working-photo toolbar has a compact 18px title, hidden at widths ≤760px to give filename search room. Inspector and Jobs headings use the panel-title role; the Filters dialog uses the filter-title role. Folder and asset headings are compact (13px/550); shared buttons and fields use 13px text; dense photo filenames and scan status retain their compact roles. The primary folder action remains 13px/600. Settings uses a standalone 24px heading, 16px section headings and 14px folder names, with shared 13px controls and help text beneath them. Its heading carries the surface without an eyebrow or duplicated toolbar title. Paths use 13px/1.6, favorite labels 10px, and timeline month/year bubbles and the muted sidebar version 12px (11px when collapsed). Text list headers and cells use 12px text, with muted headers and an accent active sort. Timeline omits filename captions at rest; a small SVG favorite badge can overlay the image. Uniform grid retains filenames, and Text list presents file data. Date headings use 13px/550 with a quiet 11px shown-count; sparse heading labels truncate within the images they describe. Timeline year photo counts belong in accessible button labels rather than visible text.

Metadata labels are quieter than their values. Keep recorded dates and their source distinguishable: Text list defaults to the actual Modified Date without a repeated provenance subtitle; optional Capture Date and Date Source are separate columns. Inspector separates displayed date, date source, original date and time-zone knowledge. Long paths and raw metadata wrap; collection filenames truncate within their row or tile. Treat hostile names as displayed data and preserve bidirectional isolation.

## Layout

The native window defaults to 1180 × 780 and has a 660 × 560 minimum. The shell fills the viewport without scrolling the document. A 228px identity rail borders a flexible main column. The main pane scrolls independently; inside the rail, only the primary navigation region scrolls when space is constrained. Identity and the collapse control stay at the top, while a non-shrinking workspace-actions footer stays at the bottom. A thin divider and 16px top padding separate that footer from primary navigation. Settings sits above the runtime version; Jobs belongs in each screen’s header. The toolbar uses a 64px minimum height with 12px 24px padding; Photos retains its compact header and existing narrow-window behavior. At widths ≤850px the rail is 184px. Expanded and collapsed navigation retain the same footer anchoring at the native minimum.

**First run and root setup.** The workspace is at most 920px wide, including padding, and centres the welcome region below the toolbar. The welcome is at most 570px wide; a 40px folder outline precedes its headline with 24px separation. The primary action sits 28px below. Selecting a folder replaces the entire welcome and chooser with the selected-folder form; Cancel returns to the chooser. The form, Advanced accordion and saved-root settings expand to their natural height without internal scrollbars. The main column scrolls vertically to keep all content and actions reachable. The two-column field grid uses 16px row and column gaps. Paths wrap anywhere while preserving whitespace.

Hidden-file and hidden-folder switch rows sit side by side with a 16px gap, stacking at widths ≤760px. Exclude patterns and Excluded folder names keep the two-column form grid at the native minimum width; File Extensions spans both columns. Chips and draft input wrap within each field. Long chip values wrap rather than widening the form. Expanded setup remains reachable through the main pane at the minimum window size.

Minimum size and Maximum size retain that two-column Advanced grid. Each field places its label 8px above a row containing a flexible amount input and a 72px native unit select, separated by 8px. The amount input can shrink within its grid cell; inline errors expand the field vertically. At the ≤600px fallback, the fields follow the existing single-column form layout.

**Settings.** A standalone page scrolls inside the main pane while the identity rail remains separate. Settings explicitly gives that pane vertical auto overflow, including after a library is registered; opening folder setup keeps the setup pane’s scrolling behavior. Its content is at most 1040px wide including 32px 36px 48px padding; the heading has 24px separation below it. Library locations, Preview cache, Scan settings, Browsing defaults and New folder defaults use 24px vertical padding and thin bottom dividers, with no enclosing cards or shadows. Indexed-folder rows have a flexible information column and an action column, separated by 16px; the root label, wrapping native display path, availability, indexed count and mode stay together. Add photo folder sits beside the section heading. At widths ≤760px, page padding becomes 20px 16px 32px, folder actions move beneath the folder information and the section-heading row can wrap. Scan settings sits between Library locations and Browsing defaults. Its labelled library selector is at most 420px wide; the selected path and shared setup fields sit directly in the section without selected-folder card framing. The inline form expands naturally through Advanced and its own scan actions, with the main pane owning scrolling. Browser controls wrap and keep their shared pressed states. New folder defaults reuses the naturally expanding Advanced form, tag editors and size grid; the main pane scrolls through the bottom Save defaults and Discard changes actions rather than adding an inner scroll region.

**Working library.** The separate identity rail can collapse to a 72px icon column, retaining the owner mark, navigation semantics and version. The working-photo toolbar stays above the internal collection scroll region, with a compact title, fixed filename search, Filter button and Jobs icon. It uses a 64px minimum height and 12px 24px padding. Photos has no Add folder action; indexed-folder setup remains in Settings. The canvas begins below the toolbar without resting query, count, view-control or filter-chip rows, or a pagination footer. Contextual batch actions appear only when a selection exists. Opening Scan settings from Photos navigates to Settings and focuses the inline group for that root, within the naturally scrolling main pane.

**Folders explorer.** The fixed Folders toolbar has filename filtering and pressed Icons / Details controls. The separate flex-column explorer body uses 16px 24px 12px padding and owns a flexible, internally scrolling virtual canvas. Back and Up precede a single-line breadcrumb trail beginning at Libraries. Breadcrumbs scroll horizontally; ancestor buttons truncate at 200px and the current segment at 260px, with bidirectionally isolated display names. At widths ≤760px the toolbar wraps, body padding becomes 12px 16px and breadcrumbs take a full row. This preserves the navigation and bottom Previous items / Next items controls at the native minimum.

The initial board shows registered libraries. Opening a library or folder shows its immediate catalog children, folders first and then files in native-byte order. Icons adapts its columns to the available width using a 180px target; rows are 210px high with 12px gaps. Named entries have 8px padding, a 132px preview/glyph region, a 12px filename and 11px muted kind/status. Media previews preserve the whole cached image; folders and non-media files use outline SVG glyphs. Details uses image-free 64px rows with the name on the left and kind/size/status on the right. One directory page retains at most 240 entries; only visible rows and two rows of overscan mount. Shown/indexed counts and explicit Previous items / Next items navigation belong to Folders. Photos keeps its separate scroll feed.

**Filter panel.** Filter opens a native modal dialog anchored to the right edge, filling the window height at the width in the frontmatter. A surface fill and left divider separate it from the dimmed canvas. The header uses 18px 20px padding; the independently scrolling body uses 16px 20px; the persistent bottom actions use 14px 20px and a top divider. Reset filters and Apply filters align vertically. Fields use a single column with 14px gaps; view buttons and library controls wrap within the panel. The same panel layout keeps its header, scroll body and footer reachable at the 660 × 560 native minimum.

**Photos scroll loading.** Approaching the collection edges loads earlier or later assets through native query/seek requests. The feed retains at most three 240-asset pages (720 summaries), while only visible rows and overscan mount in the document. Appending, prepending and trimming preserve the visible asset and row offset. Query changes and date jumps restart the viewport; view/density changes preserve the visible-item anchor. A compact collection overlay reports loading or a failed edge request with Retry loading rather than adding permanent controls below the images. Preview submissions remain batches of at most 64.

**Photo canvas.** The visible Timeline view uses justified, aspect-preserving rows with 4px horizontal and vertical gutters. Compact, Comfortable and Large target 150px, 220px and 300px image heights. Rows fit the available width where the bounded stretch/aspect geometry permits; exceptional rows can leave whitespace. The final incomplete row stays left-aligned at no more than its target height. Images use contain fit. Day changes add a 36px heading band above the relevant part of a row, with Today, Yesterday, a full or compact date, or Unknown date; the number of shown photos appears when the heading has room. Sparse dates share a row rather than creating a column of large blank blocks; heading text truncates to its available span. Day headings follow capture or added-date sorting and disappear for filename/size sorting. Timeline hides its scrollbar while retaining scrolling. Uniform grid uses equal preview wells with the same 4px gutter and filename space, offering Fit whole image or Crop to fill. Text list uses 54px rows without images.

**Date rail.** A 72px year/month rail sits to the right of the collection, separated by 16px. It appears only for capture-date and recently-added sorting when real filtered date buckets exist. Year labels adapt to the available height and the current year; the adjacent scrubber indexes the actual matching months or undated bucket. This is a filtered jump control, not a decorative distribution. At widths ≤760px it becomes 56px with an 8px separation and remains available at the native minimum.

**Viewer.** A full-window native modal dialog leaves the collection mounted underneath. Its 64px minimum toolbar precedes the preview-status line; the remaining area contains the image, compact zoom controls and optional filmstrip. Fit-relative zoom ranges from 100% to 400% in 25% steps; zoomed content scrolls and supports pointer pan. Info opens a 320px independently scrolling details region. At widths ≤760px, Info overlays the right side of the viewer body at up to 90vw; toolbar and zoom controls wrap. The filmstrip uses 64px buttons and 4px gaps. Compare uses a two-column image area with 16px gaps and contained previews.

At the supplied 660 × 560 minimum, controls wrap and the collection keeps its own scroll region. Library content padding becomes 14px 16px at ≤760px; the working toolbar uses 12px 16px. Text list keeps every chosen column at all widths: its header and rows share one horizontal scroll surface. The default table has an 826px minimum width; a 36px selection track precedes a flexible Name track of at least 180px and the chosen fixed-width data columns. The fallback below 600px collapses the rail to a header and forms to one column; it does not establish mobile support.

**Header Jobs.** A compact outline-icon button, at least 36px square, opens a right-aligned nonmodal dropdown below the header control. An accent count badge appears only for running or cancellation-requested jobs; the button retains the accessible name Jobs and an associated textual running count. The portalled native dialog escapes page overflow, uses the shared surface, field border and container radius, and has 20px padding without a shadow or dimmed backdrop. Its width is the smaller of 390px and the viewport minus 32px; positioning keeps at least 16px of viewport space and updates on resize or outer scrolling. Available height is bounded below the opener, with vertical scrolling inside the dropdown. Plain divided job rows have 20px vertical padding, wrapping labels/status and content-width progress tracks. Actions sit together with 8px gaps; Recent jobs is a native disclosure below active rows. The same layout fits the native minimum window.

**Library maintenance review.** Removal and all-library preview clearing open an inline review between Preview cache and Scan settings, within Settings’ main scroll pane. The review has 22px vertical margins, 20px padding, a uniform 1px neutral border. It retains the workspace background and square corners, without a backdrop, shadow, nested scroll region or new card treatment. Review paragraphs use 1.6 line height and a maximum 72-character measure. Native display paths wrap and remain bidirectionally isolated. Shared action buttons wrap with 8px gaps, 18px below the review text; the same layout keeps the review reachable at the native minimum.

## Elevation & Depth

The workspace stays flat at rest. Tonal surfaces and 1px dividers provide hierarchy; selected assets use an inset accent outline. Viewer and Compare use an opaque sidebar-toned surface that gives the photographs protected focus. The right-side Filters dialog uses a surface fill and left divider with a translucent black backdrop; it adds no shadow.

Keyboard focus uses a 2px accent outline, offset 5px on buttons and links and 2px on fields, summaries and the collection viewport. Gallery photo buttons use an inset −3px offset so their focus remains visible within image tiles; timeline year buttons use −2px. Switches retain the field focus outline. A tag editor outlines its whole field when focus is within; its draft input suppresses a second outline, while chip removal buttons have an inset focus outline (−2px offset). Focus is distinct from selected and pressed states. Retain purposeful scan/preparation, viewer and loading feedback, honoring Reduced Motion and occluded-window behavior. Shared buttons use 140ms neutral color feedback; confirmed favorite feedback uses 180ms ease-out and a restrained 4% press, without overshoot. There are no decorative interface gradients. Fixture image content does not define a decorative UI treatment.

## Shapes

Keep modest corners: primary actions use the action radius, secondary buttons and fields use the 8px control radius; photo Grid asset containers retain their existing 5px corners, Grid preview wells, explorer entries and scan-list chips use the small radius, and standalone root setup uses the container radius. Embedded scan settings stays flat within its divided Settings section. Timeline tiles and preview wells have square corners, separated by the photo gutter. The timeline month/year bubble, scrubber thumb and image-state badges use the small radius. The initial Library anchor and error container retain the navigation radius. Controls have visible borders rather than heavy shadows. Standard checkboxes are compact (16px); switches use a pill track and circular thumb. The header Jobs count uses a small accent badge with dark text; it accompanies the icon rather than replacing textual job states. Folder and initial navigation icons use outline SVGs with 1.5px strokes.

## Components

**Navigation.** The primary region is named “Main navigation”: before saving a root it contains Library; a saved library exposes Photos, Folders and Albums / Tags, followed by disabled Map and Utilities with visible “Later” copy and a reason. A separate “Workspace actions” region in the bottom footer contains Settings at all times; the runtime version follows it. Jobs is available from each screen’s header, including onboarding, without a saved-library prerequisite. Destination buttons use outline SVG icons and `aria-current="page"`; their default treatment stays flat and borderless, with the deeper green surface marking the current destination. Collapse navigation reduces both regions to icons, retaining accessible names, destination titles, active states and the Expand navigation control; disabled destinations are hidden. Do not introduce successful placeholders for unfinished destinations.

**Sidebar version.** Beneath the workspace actions, show “Version” followed by the native runtime version (currently 0.1.0), supplied through `FoundationStatus.version` from the Cargo package version. It uses muted text and the accessible label “Application version”; omit only this text until that value is available, keeping Settings reachable. There is no footer tagline.

**Buttons and fields.** “Choose a folder” remains the clear first-run primary action, minimum height 44px; pending text reads “Please wait…”. Connection readiness and busy state gate it. Secondary library actions use the compact bordered control treatment, minimum height 34px. Disabled controls have 55% opacity; hover changes fill and pressed view controls use the pressed token. Inputs, selects and textareas have persistent labels, a dark field fill and visible focus. Retain native input semantics, including the styled checkbox inputs exposed as switches in root setup, and disclose advanced scan options.

**Root setup.** A chosen native folder displays its path and “Not scanned” before “Save & Start scanning”. Once the picker finishes, focus moves to Library name once for each new selection; later Save activity does not refocus it. Saving stores the root and begins a real background scan; subsequent roots use “Add folder”. Factory defaults use Managed with the Read only switch off; switching it on saves referenced mode. New folders inherit the saved mode and scan defaults from Settings. Existing saved roots retain their own mode and filters. The visible folder-mode label and help explain the current setting; Managed permission still only reads originals in M1. Scan settings expose independent Include hidden files and Include hidden folders switches, exclusion lists, File Extensions, byte-size restrictions and a path-based filter preview. Include patterns and scan date controls are absent; Save/Rescan and Preview clear their legacy restrictions. “Save & Rescan” applies settings to an existing root. A picker cancellation keeps the previous selection and announces the outcome. Identity-unavailable folders cannot be saved.

**Settings and indexed locations.** Library locations means the photo folders indexed in place; it does not mean catalog or cache storage. Each location offers Change location and Remove library with root-specific accessible names; scan controls live in the dedicated inline Scan settings group. Change location opens the native picker to reconnect the same moved folder through identity validation, then reconciliation; cancellation preserves the current root. A different folder uses Add photo folder and the existing explicit setup and Save & Start scanning flow. These actions do not move originals. A missing status has “Connecting to your libraries…”; a successful empty root list has “No photo folders added yet.” Busy states disable conflicting actions.

**Remove library and clear preview cache.** Remove library prepares a review for that registered folder; the separate Preview cache section offers Clear preview cache for every library. The review names the folder and native display path, or All libraries for cache clearing, then shows backend-prepared catalog-record, indexed-preview and byte counts as applicable. Library removal explicitly warns that indexed records, catalog favorites, ratings, tags and scan history are forgotten and catalog notes will not return when the folder is added again. Both reviews state that original photos and sidecars stay unchanged, only indexed previews are cleared and unexpected files are retained. Cache clearing preserves libraries and catalog notes; help explains that accessible photos regenerate previews and offline photos need their originals reconnected.

Confirmation uses the specific labels Remove library and its previews or Clear all cached previews, alongside Cancel. Preparing and applying use “Please wait…” and disable review actions; missing or failed preparation cannot enable confirmation. Failed application clears the stale review and offers Review again. A completed action shows an explicit outcome and Done. If refreshing Settings fails after commitment, keep the completed result and explain the refresh failure rather than inviting the action to run again. Opening focuses the review heading; Cancel, Escape and Done return to the opener, with the Settings heading as the fallback after a removed row disappears. Conflicting Settings controls remain disabled while review is open. Removal is disabled for a scanning root, and clearing all previews is disabled while a scan is running or stopping.

**Inline scan settings.** A labelled Library to configure selector chooses a registered root and loads its saved mode and filter, separate from application defaults. The embedded setup shows its native display path, read-only library name, folder mode and shared Advanced controls. Save & Rescan applies this root’s scan draft; Discard scan changes restores its saved controls, including pending tags, size amount/unit/validity and filter preview, and collapses Advanced. Switching roots resets the scan draft and feedback. Starting shows “Starting scan…” and disables conflicting controls; an active root scan disables resubmission with “Scan running…”. Failures retain the draft with an alert; successful startup announces “Scan started. Originals stay unchanged.” When there are no roots, explain “Add a photo folder to configure its scan settings.” The Photos Scan settings action opens Settings for the requested root and focuses this section heading. The group has its own form and actions beside the separate Application defaults form; there are no nested forms and Save defaults does not submit scan changes.

**Settings drafts and defaults.** Browsing defaults shares the Timeline, Uniform grid and Text list buttons, density and grid-fit controls, together with a registered-library/all-libraries selector, sort and Start with navigation collapsed switch. New folder defaults adds Read only by default and the same Advanced scan controls used by setup. Browsing defaults seed Photos when it opens; temporary browsing controls remain separate overrides. New-folder defaults seed the next folder setup and do not rewrite existing roots or start a scan. Save defaults is the explicit primary submit action; view, preview and Discard changes buttons do not submit. Saving reads pending tag input, respects size validity, shows “Saving…” and disables the draft fieldset. Save failure keeps the draft with an alert; success announces “Defaults saved. Existing photo folders are unchanged.” Explicit Discard changes restores the saved draft, resets amount/unit input and validity, pending tags and relative-path sample/preview, clears feedback and collapses Advanced. Ordinary edits leave the disclosure and field-local state intact. Native preferences loading and failed loading have explicit states and a Retry settings action. Settings focuses its page heading on ordinary entry and the Scan settings heading when opened for a root. The small versioned preferences record is native-owned and separate from the catalog; corrupt, linked or unsupported records stay preserved and surface an error. Catalog/cache relocation remains outside this surface, and Windows preferences publication remains gated by native durability validation.

**Advanced accordion.** Use a full-width native details/summary disclosure, initially collapsed. The summary has a visible “Advanced” label, a right-aligned chevron, a minimum height of 46px and 12px 16px padding. A thin border and 6px corners define the accordion; opening adds a divider below the summary and turns the chevron upward. Hover uses the control fill. Preserve native keyboard disclosure behavior and visible focus; expansion has no animation or internal scrolling.

**Switches.** Keep the label and native checkbox input with `role="switch"` in one clickable row, at least 38px high. The off track uses the control-border fill and switch-border outline with a light thumb; the on track and outline use accent with a dark thumb. The thumb shifts 14px to indicate on without motion. Disabled switches have 55% opacity and the default cursor. Forced-colors mode uses Canvas/CanvasText and Highlight for the checked track. Retain native keyboard and checked-state behavior; do not replace the input with a painted button.

**Scan-list tags.** Exclude patterns, Excluded folder names and File Extensions use labelled tag editors, at least 38px high, with a field-border outline and 4px gaps. Chips use the tag surface, navigation text and a labelled 28px removal button. Enter, Tab or blur commits a trimmed value; multiline paste adds one value per line, and Backspace in an empty draft returns the last chip to editing. Duplicate values collapse, while commas remain literal content. File Extensions displays dotted suffixes and preloads the shared 33-photo-suffix preset for new folders; existing saved custom lists, including an empty scope, remain intact. Extension input strips the leading dot, lowercases and deduplicates values for case-insensitive matching. This discovery scope does not expand preview/codec support. Save and Preview filters include an unfinished draft even without blur. Disabled editors use 55% opacity and disable their inputs and removal controls. This pattern applies to scan lists; Inspector retains its separate comma-separated catalog-tag field.

**Scan-size fields.** Advanced labels the independent thresholds “Minimum size” and “Maximum size”. Each amount uses a decimal keyboard hint and a “No limit” placeholder, paired with a native KB/MB/GB select whose accessible name includes its field label and “unit”. Empty new fields default to MB. Help states “Size units use 1 KB = 1,000 bytes. Leave a size empty for no limit.” All units are decimal; blank means unrestricted and zero remains an explicit threshold. Existing saved limits display exactly in an appropriate unit. Changing the unit keeps the displayed amount and applies the new scale. Keep conversion exact through integer-byte strings; do not round existing values through floating-point arithmetic. Invalid, negative, fractional-byte or oversized amounts show a 12px error-text message beneath the controls, associated with the amount through `aria-describedby` and `aria-invalid`. Native custom validity blocks both Save and Preview filters until corrected. Busy state disables both amount and unit. This pattern applies to Advanced scan settings; browsing Filters keeps its byte inputs.

**Queries and facets.** Search filenames in the fixed working toolbar; its Filter button names the number of active query restrictions and exposes dialog/expanded semantics. The right-side Filters dialog contains the current matching count, removable active-filter chips, library, sort, date, byte-size, extension, tag, favorite and missing-file criteria, plus Display controls for Timeline, Uniform grid, Text list, thumbnail size and grid fit. The legacy Folder (relative) input, catalog tag facets and Library availability share this panel. Its folder text filter retains recursive compatibility behavior; it does not define the Folders destination. Similar groups remains visibly unavailable: no filename or cached-preview inference stands in for a similarity engine. Tags exposes catalog tag buttons after the drafted root is applied. A selected root shows its availability/mode and Scan settings; unavailable roots offer Relink folder. The panel notice distinguishes inaccessible originals from retained catalog details/cache.

**Folders navigation and states.** Folders is a catalog-only hierarchy, not live filesystem enumeration. Opening a named library starts at its indexed root; Back restores the preceding location, scroll position and entry focus from at most 64 history positions. Up and breadcrumb actions enter the parent or Libraries board. Filtering matches direct filenames while leaving child folders visible; moving into another directory clears the filename filter. Root availability, retained previous-scan folders, partial indexing, blocked entries and cached-preview availability remain distinct and labelled. Blocked folder entries are disabled. Explain scan coverage beside the contents. Loading, a successful empty page, no filter matches and a failed directory request each have explicit copy; a failed request offers Retry folder. Preview failures offer Retry previews without disguising the directory as empty.

Activating media opens the existing cached-preview viewer with an exact-directory query, filename filter and filename ordering. Previous/Next stays inside that directory rather than including descendants. Closing restores the explorer pixel offset and originating entry focus. Activating an indexed non-media file opens a native File details dialog with its filename, relative display path, library, size, modification date, catalog presence and root availability; Close or Escape returns to the entry. It has no image, external application launch or original-file edit action. Details mode mounts no images and submits no nonempty preview batch. Display paths remain data; opaque native directory IDs carry catalog scope.

**Filter drafts.** Opening starts from the applied query and display settings. Editing fields, chips, sort, view, density or fit changes the draft; Apply filters validates, commits and closes. Close and Escape discard pending edits and return to the canvas. Reset filters resets the query draft, leaving display choices separate. The displayed matching count describes the applied query until changes are applied. Select all results lives in the panel and requires the current query draft to be applied first; successful creation closes the panel. Selection creation disables conflicting dialog controls and Escape until complete, and failures retain an explicit retryable error. Filename search in the fixed header applies directly.

**Text list columns and sorting.** A compact Sort select and Columns disclosure sit above the table, separated by 12px from it. The select reuses the five native orders shared with Filters and Settings: Capture Date newest first, Filename A–Z, Size largest first, Recently added newest first and Modified Date newest first. Name, Modified Date, Size and the optional Capture Date/Added Date headers provide the supported shortcuts; other headers remain labels. Sort direction is fixed rather than toggled. The active header has an accent chevron and `aria-sort`; sorting is disabled during loading or batch work. Changes query the matching catalog and retain the existing query-change selection reset.

Defaults show Name, Modified Date, Size, Dimensions, Folder and Status. Modified Date uses the indexed filesystem modification timestamp and shows Unknown when absent. Optional Capture Date is the effective capture date, including its documented fallbacks; Date Source exposes provenance as its own field. Added Date, File Format, Camera, Rating and Tags are also available. Name has a checked, disabled “always shown” checkbox; Reset columns restores the default set. Choices apply immediately and survive view/filter changes while the same LibraryScreen stays mounted; they do not change saved defaults or the catalog schema.

Columns uses native details/summary semantics and the shared restrained borders and field focus. Its right-aligned surface menu is 240px wide, limited to the viewport, with 12px padding, 5px corners and a vertically scrolling checkbox list capped at the smaller of 350px or 55vh. Escape closes it and returns focus to Columns; an outside pointer closes it. The table exposes table, row, column-header and cell semantics. Its header is at least 40px high; image-free rows remain 54px high with an independently named selection checkbox and Name open button. Long cell values truncate; full indexed date timestamps remain in tooltips.

**Collection and selection.** Timeline, Uniform grid and Text list use pressed view controls and preserve stable-ID selection across views. Timeline checkboxes appear on tile hover, keyboard focus or selection; day-heading checkboxes appear on hover/focus or when checked. Day selection names its scope as shown photos from that date, not the whole library. Grid and Text list keep separate checkboxes and open controls. Normal photo activation opens the viewer; an active selection or Shift activation selects instead. Shift ranges and keyboard arrows use the current ordered loaded window. An inset accent outline marks selection, and Escape clears it. View/density changes retain a visible-item anchor; query changes and date jumps restart the viewport.

“Select all results” freezes matching membership in a backend token. Its scope is distinct from explicit IDs and shown-date/loaded-window ranges; individual selection is disabled until cleared. A contextual action row shows the count and scope, Add tags, Favorite selected, Compare and Clear selection. Add tags unions trimmed, deduplicated values with existing catalog tags; its help states that originals and sidecars stay unchanged. Compare is enabled only for 2–4 explicitly selected photos. Album and Export actions remain disabled with reasons.

**Image states and text-only mode.** Timeline and the viewer preserve the whole cached image; Uniform grid allows an explicit display-only crop. Pending, unavailable, failed-load and offline-original states have visible copy. A failed preview keeps the filename and reason; a ready cached preview may still carry an Offline original badge. Decoder failure never claims damage or removes the catalog entry. Folders Details and its non-media File details dialog are also image-free. Text list defaults include file status; date provenance is available through the optional Date Source column and Inspector. It mounts no image elements and requests no nonempty preview batches; an empty batch may cancel prior preview work. Cached thumbnails do not establish original availability or integrity.

**Photo timeline.** Year buttons use real filtered counts in their accessible labels, with the current year marked by accent and `aria-current="date"`. A native range control supports pointer and keyboard month/year jumps; its value text names the period. The dotted track and small accent thumb show the control position, while a quiet month/year bubble appears during scrolling, hover, focus and dragging. Hover previews a period; releasing the pointer or a navigation key commits the jump. The actually visible first row supplies the current date rather than overscan. Undated remains explicit. Indexed seeks do not load preceding pages or fabricate absent months.

**Immersive viewer and Compare.** The viewer uses native `showModal`, visible Back to gallery, Previous/Next under the original filters/sort, Favorite, optional Info and optional Filmstrip. Arrow keys navigate outside editable fields. Escape closes Info first, then the viewer. Closing restores the original gallery pixel offset and originating photo focus, with a collection fallback. Gallery preview requests pause while either modal owns the shared preview queue; explorer previews pause while its viewer or File details is open.

The image is a read-only cached preview up to 512px; the optional filmstrip uses 256px previews. The status line discloses this limit and distinguishes offline/inaccessible originals from cache readiness. Zoom is a percentage of fit, not evidence of original resolution. Preparing, unavailable, image-load failure, navigation failure and annotation failure have explicit states; retry actions accompany preview and navigation errors. Info retains catalog-only notes and metadata provenance. Edit stays disabled with a reason. Compare shows only 2–4 explicit selections, filename, catalog presence and the original library’s current availability, and states that visual comparison does not establish exact duplicates. Similarity groups, saved albums, editing/export and full-resolution decoding remain behind their separate gates.

**Query, empty and error states.** Loading has its own “Reading your catalog…” state. A successful zero-result query uses “No matching photos yet”, with scan-progress or filter guidance as appropriate. Photo-query failure has a visible error and Retry action; failed results must not appear as empty success or stale successful rows. Scroll loading and selection controls must not operate on a failed query; failed edge loads retain the loaded rows and offer Retry loading. Global connection errors use an alert with Reconnect when no foundation status is available and Dismiss otherwise.

**Inspector.** Viewer Info opens “Photo details”, a labelled side region with a Close control. It shows filename/path, root, displayed and original dates with provenance/time-zone knowledge, dimensions, format/size, camera, catalog-added and filesystem-modified dates, availability and metadata/preview state. Catalog notes provides Favorite, rating and comma-separated tags, explicitly stating that originals and sidecars stay unchanged. Possible companions and raw metadata are disclosures; matching names are not proof of grouping. A failed detail request shows an error and Retry rather than an endless loading state or another asset’s details. Closing Info returns focus to its toolbar button; closing the viewer restores the collection position and focus.

**Scan progress and Jobs.** The header Jobs control replaces the separate Jobs destination and scan banner on onboarding, Photos, Tags, Folders and Settings. It exposes expanded/dialog semantics and a textual running count; the badge includes both running and cancellation-requested jobs. Opening focuses the Jobs heading without navigating or changing the underlying collection. Escape or Close jobs dismisses it and restores opener focus; an outside pointer or focus move dismisses it. The dialog is nonmodal and does not trap focus or dim the page. Missing status shows “Connecting to jobs…”; a ready idle state shows “No running jobs.” Active rows lead; completed, partial, canceled, interrupted and failed jobs remain under the initially collapsed Recent jobs disclosure. Rows show the root label, actual job state, found/indexed/skipped/issue counts and backend message. Beneath each root label, a 6px border-color track with the small radius carries an accent fill. Running and cancellation-requested jobs show a 32%-wide segment sweeping across the track with a 1.8s ease-in-out alternating animation. Reduced-motion preferences replace the sweep with a static centred segment. The accompanying text reads “Scanning · total files not yet known” or “Stopping scan…”. Discovery has no fixed total: this activity treatment must not imply a percentage or estimated finish time.

Active tracks have a root-labelled progressbar role, an associated description and no current numeric value. Only backend-confirmed `completed` jobs receive a full track and a value of 100 with “Scan complete”. Partial, canceled, interrupted and failed jobs retain an empty neutral track without progress semantics and use explicit outcome text: “Scan finished with issues”, “Scan canceled”, “Scan interrupted” or “Scan failed”. Counts, backend messages and the existing polling remain authoritative. Details loads the native issue list on demand, showing “Loading scan details…” before the issue list or “No file issues recorded.” Detail and cancellation failures remain explicit alerts. Cancel scan uses the real job ID; while submitting or cancellation is requested, it becomes disabled “Stopping…”. Terminal rows have no cancellation action. Partial results can appear during scanning; cancellation and unavailable roots must not imply that the library is empty.

**Evidence boundary.** Current source is `src/library/`, `src/settings/SettingsScreen.tsx`, `src/settings/ScanSettingsGroup.tsx`, `src/settings/LibraryMaintenance.tsx`, `src/App.tsx`, `src/useFoundation.ts`, `src/usePreferences.ts`, `src/styles.css` and the native `crates/core/src/preferences.rs` adapter. Header Jobs captures `jobs-dropdown-running-desktop.png`, `jobs-dropdown-running-minimum.png` and `jobs-dropdown-recent-desktop.png` under `.context/screenshots/` show generated-fixture running and recent-job states in the incumbent shell; they do not establish release performance or additional job types. The Filter panel/canvas captures `filter-canvas-desktop.png`, `filter-canvas-minimum.png`, `filter-panel-desktop.png`, `filter-panel-minimum.png` and `filter-panel-minimum-bottom.png` under `.context/screenshots/` document the header, resting canvas and dialog layout using generated fixtures. Library-removal captures `remove-library-review-desktop.png`, `remove-library-review-minimum.png` and `remove-library-empty.png` under `.context/screenshots/` record the generated-fixture inline review and empty state; source remains authoritative for review copy and path display. Folders captures `folder-explorer-libraries.png`, `folder-explorer-root.png`, `folder-explorer-coast-desktop.png`, `folder-explorer-coast-minimum.png` and `folder-explorer-coast-minimum-details.png` under `.context/screenshots/` show the generated-fixture library board, immediate-child navigation, contained media previews and image-free Details at desktop/minimum sizes. These are indexed catalog views, not evidence of live filesystem browsing or untested native filename support. Current Settings captures `settings-inline-scan-desktop.png`, `settings-inline-scan-minimum.png` and `settings-inline-scan-minimum-actions.png` under `.context/screenshots/` show the inline selected-library group, Advanced controls and reachable scan actions using generated-library data; earlier `settings-desktop.png`, `settings-minimum.png` and `settings-minimum-defaults.png` retain evidence for the unchanged defaults surface; they do not establish real-library mutations, catalog relocation, physical wheel gestures or OS picker presentation. The photo-canvas overview, timeline, viewer and Compare captures under `.context/screenshots/` show generated media fixtures; their gradients are test image content, not shippable sample assets or decorative interface gradients. Earlier native and advanced-setup captures remain evidence for the unchanged setup/scan patterns. Native WKWebView checks use generated fixtures and backend-issued folder selection rather than automating the OS picker. Check outcomes, capture dates and limitations belong in `docs/M1_VALIDATION.md` and `docs/IMPLEMENTATION_LOG.md`; this system record creates no new runtime evidence. Windows, VoiceOver, broad manual accessibility review, full-resolution decoding and large-catalog performance remain unverified gates.

Isolated label/pressed-text color literals and text-glyph removal marks in active-filter chips are deliberately not promoted into new component rules. The owner’s explicit system-typography/native-shell commitment governs this interface; it does not establish a separate system-font display style for other surfaces.

## Do's and Don'ts

- **Do** preserve the owner logo, British spelling, graphite surfaces, system typography and restrained sea-green state treatments.
- **Do** keep the spacious first-run action and the denser working-library layout appropriate to their different tasks.
- **Do** keep Photos search, Filter and Jobs in the fixed header, place browsing controls in the right-side panel and show batch actions only for an active selection.
- **Do** keep Folders as immediate-child catalog navigation with named entries, horizontal breadcrumbs and bounded pages; retain Photos as the chronological scroll feed.
- **Do** keep indexed-folder actions, the inline selected-library scan draft and saved defaults distinct, and reuse the shared scan and browsing controls in Settings.
- **Do** keep selected, scanned, loading, failed, missing, offline and preview-unavailable states explicit and distinct.
- **Do** make catalog-only notes and their preservation boundary clear at the point of editing.
- **Do** name the library and preview scope before maintenance confirmation, disclose catalog-note loss, and state that originals and sidecars remain unchanged.
- **Do** preserve visible keyboard focus, distinct selection scopes and exact gallery position/focus when returning from the viewer.
- **Don't** imply that selecting a folder already saves, scans, imports or changes its contents.
- **Don't** present Change location as moving photos or relocating the catalog/cache, or let saved defaults rewrite existing roots.
- **Don't** request thumbnails in Text list or Folders Details or treat cached previews as surviving originals.
- **Don't** describe library removal as deleting photos, cache clearing as clearing every app file, or a completed action’s refresh failure as an unapplied action.
- **Don't** replace exact Folders navigation with recursive display-path filtering, or describe indexed coverage as every file on disk.
- **Don't** turn query, directory or Inspector failures into empty results, stale success or perpetual loading.
- **Don't** present similarity groups, saved albums, edit/export, full-resolution decoding, original-file mutations, map, backup, mobile or Windows scanning as delivered capabilities.

## Gallery row balancing — 2026-10-28

The existing justified canvas now chooses balanced row breaks within the retained photo window, rather than filling each row greedily. Preserve4px gutters and chronology; ordinary rows may stretch up to125% of their150/220/300px target, while sparse final rows remain at or below target. Extreme aspect ratios can leave whitespace to avoid excessive row heights. Narrow day headings use compact month/day/year text with the complete date as accessible name/title; show the secondary count only at heading widths of180px or more. Hover/focus selection, exact shown-group scope, timeline placement and all shell tokens remain unchanged. Resize notifications coalesce per frame using the latest width and the existing anchor. See docs/GALLERY_REFINEMENT.md for implementation and actual validation evidence.

## Date sections and preview continuity — 2026-10-29

Timeline dates start on separate justified rows with one full-width heading aligned to the canvas left edge. Each section retains full extent around virtual rows; its opaque 36 px sticky heading is replaced naturally by the next section. Use the existing background/text/selection tokens. Keep the naturally soft tiny preview, with a 160 ms opacity-only clear reveal and immediate reduced-motion/occluded-window reveal; gallery layers have no extra blur filter. Preserve already-loaded pixels through request-status changes. Grid/Text list and all selected-photo, keyboard, unknown/offline/error semantics remain. Behavioral requirements and native evidence live in docs/GALLERY_REFINEMENT.md.

## Approved restrained polish — 2026-10-30

[INTERFACE_POLISH.md](docs/INTERFACE_POLISH.md) records the approved light-polish direction, passes, native checks and unchanged milestone boundaries. Standard native fields/selects align at 36px; primary actions/viewer tools use 44px. Chip editors, range controls and checkboxes retain their compact roles. Organization previews show source-library names with IDs available in a tooltip, preserve literal path whitespace and format exact byte totals with grouped digits. The contact-sheet geometry, original SVG, system type and photo-first composition remain the visual authority.

The initial-preparation banner is a single row with remaining count, inline 6px progress bar and a 36px stop/pause action. Use 8px vertical and 16px horizontal padding, a 12px gap and the canvas background. Hide repeated progress prose visually while retaining its accessible description. Unknown totals read “Remaining unknown”; paused messages, unavailable outcomes and action errors remain visible when needed. Header Jobs retains full counts/history.
