# Stillharbour - Product identity

Specification naming revision: 1.2.1  
Date: 2026-10-26  
Status: owner-selected working product identity; no public brand clearance claimed

## Canonical name

**Stillharbour**

Use one word, capital S, and the British spelling **harbour**. Use this form for the application title, installer display name, About screen, documentation and product-facing copy. The lowercase technical slug is `stillharbour`. Do not substitute Stillharbor or introduce another working brand.

| Element | Approved working convention |
| --- | --- |
| Product name | Stillharbour |
| Descriptor | Photo & Video Library |
| Tagline | A home for the moments you keep. |
| Technical slug | stillharbour |
| Catalog filename example | Family.stillharbour |
| Logo source | assets/branding/stillharbour-logo.svg |

## Product description

A local-first photo and video library for organising your collection, finding similar images, and managing verified backups - with the preservation of your originals at its core.

This describes the planned product, not an assertion that any feature has been implemented or tested. Existing milestones and acceptance gates determine what can be advertised at each release. Visual similarity does not prove byte equality and never grants automatic removal authority.

## Selected logo

The file [stillharbour-logo.svg](../assets/branding/stillharbour-logo.svg) contains the exact SVG provided by the owner: six coloured aperture paths, original fills, width 483, height 507 and viewBox `0 0 483 507`. No path, colour, geometry or viewBox has been changed. It contains no font, text, scripts or external resources.

The selected SVG is distinct from the inspiration screenshots under docs/reference/, which must not be bundled as application artwork. Native app-icon exports, small-size rendering and platform packaging remain implementation tasks; this package does not claim they are already completed.

## Catalog naming and safe adoption

`Family.stillharbour` is the current specification's catalog filename example. A catalog is a versioned SQLite application file containing references and application data, not a container holding the original photos and videos. A catalog copy alone is not a media backup. The detailed save/open, snapshot, locking, validation and recovery requirements remain in [Library features](LIBRARY_FEATURES_ADDENDUM.md).

A branding update must never rename a user's catalog, original files, library root or backup repository without a separate, explicitly approved safe operation. Do not silently change catalog UUIDs, root identities, stored credential identifiers or an existing application/bundle identifier. Inspect any existing deployment before proposing compatibility handling or migrations; none are performed by this document revision.

A file extension is not proof of file type or safety. Preserve the existing defensive catalog-validation requirements. The proposed extension and technical slug are conventions, not claims of worldwide exclusivity or registered associations. Production package/bundle identifiers must be chosen under an owner-controlled namespace rather than an invented domain.

## Tone and safety language

Prefer clear actions such as **Compare**, **Preview changes**, **Keep both**, **Verify backup** and **Restore a copy**. Describe preservation and verification precisely; do not promise that hardware failure or every possible loss is impossible. The name and tagline never override [the preservation contract](PRESERVATION_CONTRACT.md).

## Availability and scope

The owner selected the name. Trademark, domain and app-store availability have not been established in this update. This is a naming-only revision of the existing v1.2 specification: no feature scope, implementation milestone or acceptance gate is changed.
