# Use Stillharbour

Stillharbour indexes photo folders and stores organization in a local catalog.
Your originals stay in place. A catalog, thumbnail or inspection report is not a
backup of the photographs. Read [format support](FORMAT_SUPPORT.md) and
[release status](RELEASE_STATUS.md) for the build's limits.

## Add a photo folder

Choose a folder, review its name and scan settings, then select
**Save & Start scanning**. Additional folders must not overlap registered roots.
Selection authorizes the reviewed scan, not moving or deleting originals.
Use generated disposable folders for development tests.

New folders default to **Managed**. **Read only** saves a referenced root. Both
modes read originals in the default build; Managed is not a blanket write grant.
Settings lets you add folders, change scan scope or reconnect a moved folder
through native identity checks. A different disk at the same path is not
automatically trusted.

Advanced settings include extensions, hidden files/folders, exclusions, dates and
size limits. Minimum and maximum sizes each contain an amount and unit selector.
Discovery of a video or RAW extension does not imply preview/playback support.
Scan filters do not establish complete backup coverage.

Follow discovery and preview preparation in the progress banner and **Jobs**.
Discovery can have no known total. Pause, resume, cancellation and retry use the
actual job state. Offline roots retain earlier observations and available cached
previews; they are not empty libraries.

## Browse and select photos

Use **Photos** for the chronological canvas and **Folders** for indexed immediate
children. **Display options** contains Timeline, Uniform grid, Text list,
thumbnail size and grid fit. **Apply display** commits a draft; Cancel, Close and
Escape discard it. Text list mounts no photo previews.

The **Filter** icon opens query choices and shows their active count. Applying
filters is independent of display choices. Search accepts filenames and full
dates as `DD-MM-YYYY` or `YYYY-MM-DD`, using the catalog's capture-date policy and
existing fallback. Other active constraints still apply.

Hover or focus a thumbnail to reveal its checkbox and favourite heart. The heart
fills only after the catalog confirms the save. The **Select** checkbox icon
reveals selection controls; **Done selecting** clears selection and returns to
browsing. Right-click or press Shift+F10 on a focused photo for **Select** and
**Add to album**. Group selection covers displayed members; explicit all-results
selection can include unloaded matches. Batch actions appear above the canvas.

Appearance follows the system on first use. The header's sun/moon button saves a
light or dark choice on this device, taking precedence over system appearance.

## View and annotate

Open a photo for zoom, pan, filmstrip and **Info**. The viewer uses admitted
original bytes when supported, otherwise a labelled cached preview or explicit
unavailable state. Closing returns to the invoking photo and its browsing scope.
Visual comparison of selected photos does not establish duplicate equality.

Favourites, ratings, tags, captions and reject flags save to the catalog, without
rewriting originals or source sidecars. Save caption/review drafts explicitly;
unsaved bounded session drafts do not survive app closure or catalog switching.

Open **Catalog** and **Saved plans** from the **Albums / Tags** header. Catalog
provides reviewed tag changes, retained undo history and observed-keyword import.
An edit made against an older catalog version needs a fresh review.

## Organize albums

**Albums / Tags** shows custom albums and tag-derived collections together.
**From tags** marks collections gathered from observed photo tags. A photo can
belong to several albums; membership never moves it.

Choose **New album**, enter a unique name, select photos and choose **Create album**.
Empty albums are allowed. In a custom album, **Add photos** adds up to 500 selected
members per save and retains existing members. Picker checkboxes stay visible
even when no photo is selected. Tag-derived albums follow their tags rather than
manual membership.

## Explore locations

**Map** opens a bundled offline world overview. Local city/country search works
without online geocoding. City filters mean within 25 km of the listed center;
country boundaries are approximate. Only indexed GPS can match. Missing GPS is
different from valid zero coordinates. Old observations may require an explicit
rescan to acquire previously unextracted GPS.

Markers load representative previews. Select a marker or explicitly request
photos to load the photo strip. **Show in photos** transfers the chosen map scope
to Photos, where its tag can be removed independently of filename/date filters.
Ordinary sidebar Photos navigation clears a previous map handoff.

**Satellite** explicitly loads NASA GIBS overview imagery over the internet. It
sends viewed tile areas to NASA, not your originals, private filenames or catalog
records. Attribution, loading/failure state and limited-detail disclosure remain
visible. Return to **Map** for bundled offline geometry. A photo-list alternative
is available when map graphics cannot run.

## Inspect and compare

**Duplicates**, **Integrity**, **Storage** and **Sync** start with setup and retain
earlier reports. Reopening a scanned utility can run a read-only background check;
it does not continuously monitor every original or start a copy/backup operation.
History keeps the selected earlier report visible. Offline roots need reconnecting
before new inspections.

Duplicates distinguishes exact bytes, property candidates and its declared
normalized static-pixel policy. Normalized pixels require manual review and do
not prove identical files. Keeper suggestions, cached hashes and offline images
cannot authorize removal. Cleanup and quarantine execution remain disabled.

Integrity distinguishes quick checks, bounded deep decodes and recorded checksum
baselines. A checksum mismatch proves a change, not damage. Failed runs offer
explicit retry; establishing a baseline requires acknowledgement. There is no
automatic repair.

Storage distinguishes logical size, unique filesystem objects and reported
allocation. Hard links, clones, sparse files, snapshots and out-of-scope links
prevent simple claims about reclaimable space. Partial and unreadable coverage
remains visible.

Sync compares the bound base library and a separately chosen destination.
Quick equality is unverified; full-byte comparison has stronger evidence.
Destination-only files are retained. Saved profiles preserve their native
bindings; historical paths grant no new access. Comparison creates no backup.

## Preview folder structures

**Structure Formater** previews the whole catalog independently of Photos'
selection. Date, indexed location, catalog-tag, camera and custom rules propose
paths without creating folders. Prefix icons identify layout choices. Location
uses indexed metadata rather than inventing a trip or online place label.

**Illustrative preview** uses four invented examples. **Preview restructure**
builds the actual paged catalog report. Review global counts, before/after paths,
search, attention and skipped rows. Missing-date Undated/Skip policies apply only
when the rule needs a date. Session presets survive navigation within the open
catalog but clear on reload; durable catalog templates are separate.

**Simulate plan** revalidates the preview and simulates completion. It changes no
files and starts no backup or Sync job. Selected-photo template and destination
reviews, saved drafts and JSON/CSV review exports also grant no execution authority.
Actual copy, rename, move and restructure execution remain disabled.

## Export catalog metadata

Portable catalog export creates a consistent SQLite copy in a separately chosen
folder. It contains catalog organization, not original media, credentials,
schedules or live root permissions. Keep the writable catalog local; placing it
on network/cloud-sync storage is unsupported. General import/open of arbitrary
portable catalogs is not an enabled workflow.

XMP export previews exact new outputs and requires approval. Existing source
sidecars and destination files are not overwritten. Unsupported or ambiguous
packets block export. Changed sources or destinations require a new preview.

## Handle errors and closed capabilities

Global errors remain visible until dismissed. Retry controls are explicit;
notifications do not automatically replay an operation. If completion could not
be confirmed after a lost catalog reply, reopen and check the result before
repeating an edit. A stopped catalog worker needs reopening, not a catalog reset.

For a macOS Keychain startup error, **Authorize** permits one OS request for the
catalog activation record. macOS may ask for your Mac login password, not a backup
repository password; Stillharbour does not receive it. Trust only builds you
recognize. Unlock a locked login keychain in Keychain Access before retrying.
Do not delete credentials or catalogs to bypass an error.

The default build does not execute backup snapshots, restore, schedules or
original-file mutations. Retain sources, catalogs, repository objects and any
staging/history after an interruption. Never force-unlock, repair or delete
unfamiliar files to clear a warning. See [release status](RELEASE_STATUS.md) and
the [preservation contract](PRESERVATION_CONTRACT.md).
