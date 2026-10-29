# Stillharbour - Library feature sources

Checked: 2026-10-26. Primary sources only. Provider and dependency behavior must be rechecked during implementation; this register is not proof of integration, account eligibility or runtime testing.

## LF01 - Google: Download your Google data

https://support.google.com/accounts/answer/3024190?hl=en

Use: Archive downloads, multiple parts, supplementary photo JSON, and scheduled Photos exports of new/updated data. Availability and fidelity still need per-account/export validation.

## LF02 - Google: Updates to the Google Photos APIs

https://developers.google.com/photos/support/updates

Use: Library API scope restrictions effective March 31, 2025; Picker for user-selected existing-library media.

## LF03 - Google: List and retrieve picked media

https://developers.google.com/photos/picker/guides/media-items

Use: Picker URL lifetime/authorization; image download omits location metadata; video download is a transcoded rendition; motion-photo resources.

## LF04 - Apple: Download iCloud photos and videos

https://support.apple.com/en-gb/111762

Use: Download originals to Mac, unmodified-original options, and iCloud.com selection limits.

## LF05 - Apple: Export photos, videos, slideshows, and memories on Mac

https://support.apple.com/guide/photos/export-photos-videos-slideshows-and-memories-pht6e157c5f/mac

Use: Export unmodified originals, separate Live Photo still/video files and optional IPTC-as-XMP export.

## LF06 - Apple: Keep photos/videos downloaded in iCloud for Windows

https://support.apple.com/en-euro/guide/icloud-windows/icw524278130/icloud

Use: Pinned files can still be deleted through cloud sync; copy to another folder for an independent copy.

## LF07 - Apple Developer: PHAssetResourceManager

https://developer.apple.com/documentation/photos/phassetresourcemanager

Use: Access to asset resources including original/edited representations and requests to write resource data locally.

## LF08 - SQLite: Online Backup API

https://sqlite.org/backup.html

Use: Consistent live-database snapshots; examples do not replace application collision/protection requirements.

## LF09 - SQLite: Write-Ahead Logging

https://sqlite.org/wal.html

Use: Network-filesystem constraints, WAL dependencies and documented WAL-reset corruption fix in 3.51.3 or specified backports; recheck current advisories.

## LF10 - SQLite: Security guidance

https://sqlite.org/security.html

Use: Untrusted database handling, defensive/trusted-schema settings, resource limits and integrity checks.

## LF11 - ExifTool: Metadata Sidecar Files

https://exiftool.org/metafiles.html

Use: XMP and other metadata sidecars; specific examples can overwrite files, so the app must enforce its own protected export workflow.

## LF12 - Google Data Portability API: Available OAuth Scopes

https://developers.google.com/data-portability/user-guide/scopes

Use: The checked list does not establish a Google Photos library scope. Maps/Street View photo scopes must not be mistaken for Photos access.

## Interpretation boundary

The addendum contains proposed application policies and acceptance criteria, not claims that those capabilities exist in Stillharbour. Google Takeout is the initial large-library path; Picker is deliberately a limited selected-import option. Apple exports/local hydrated copies are the initial route; PhotoKit is a gated native follow-on. No unsupported private endpoint, generic portability entitlement, trademark clearance or installed runtime has been assumed.
