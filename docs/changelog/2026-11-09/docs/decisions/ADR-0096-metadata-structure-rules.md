> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0096: Metadata-based structure rules and a folder chip builder

Date: 2026-11-07
Status: Accepted for read-only previews; existing execution gates remain binding

The owner expanded Structure Formater to date, embedded location, catalog tags,
camera metadata and Custom layouts. They explicitly chose embedded location
metadata only and removed the proposed Trip hierarchy. Camera model → Event is
separately requested and accepts one explicit event name for this whole-library
rule. No travel grouping or reverse geocoding is introduced.

The UI groups 25 presets: eight date layouts (the five requested plus the three
existing choices), four location layouts, six tag layouts and seven camera
layouts. Examples are illustrative folder trees. Year → Month → Location uses
Country; Tag → Location uses City. Custom lets users search/add predefined folder
chips or explicit literal names, remove/reorder them and edit the filename
pattern. The actual pattern remains visible/editable. There are at most 31 folder
levels plus a filename. Every change invalidates the previous preview.

Extend the pure parser with quarter, country/region/city/place, tag/subcategory/
subject, camera type/brand/model/combined camera and file-format tokens. Quarters
use capture-local calendar months. Filenames retain the original extension;
portable component checks, collisions and global sequence ordering still apply.
A required naming value missing from a new metadata pattern cannot be hidden by
the legacy Undated fallback. Once required metadata is present, the existing
explicit missing-date policy still applies. No original is renamed or moved.

Add optional `OrganizationMetadata` to native preview sources and synthetic
TemplateSample data. Omit absent fields when serializing; historical source,
example, template and saved-plan JSON/digests retain their previous form. Legacy
patterns keep organization-preview-v1 and omit metadata context. New metadata or
quarter patterns use organization-preview-v2. Context is captured consistently
for preview, destination review and plan creation. Arbitrary renderer metadata
cannot replace catalog source observations; metadata accepted with samples is
synthetic example data only. Loading retained examples validates and preserves
these optional values instead of dropping them.

Native snapshots read bounded catalog metadata only: at most 240 media records,
1 MiB raw metadata and 16 KiB tag JSON per row, with the existing validated
512-node/16-level category limit. JSON is projected and dropped per record. Use
explicit flat embedded naming observations, never GPS, filenames, nearby map
search areas, captions, marketing camera databases or guessed camera classes.
Conflicting aliases remain missing. Camera make/model remain their observed
strings; combined Camera joins make/model without duplicating an already branded
model. Explicit camera type can be unavailable even when make/model are present.
Format folders use the observed indexed format, falling back to the indexed
extension; known RAW suffixes form a RAW group. This is naming classification,
not file-format verification.

Tag hierarchy follows the existing catalog category parent graph. An assigned
ancestor is not an extra competing leaf. A requested level must have the same
prefix in every assigned leaf path. Shared roots can resolve while conflicting
lower branches remain missing; unrelated roots never receive arbitrary priority.
Flat uncategorized tags are one level. Untagged/ambiguous entries remain blocked
in the whole-library preview rather than being dropped or copied into several
outputs. Country/camera/tag naming context comes from the same revision-bound
snapshot as the original source identity.

The pinned macOS metadata helper's read-only allowlist now requests the naming
fields, including actual IPTC country/province aliases, for future metadata
observations. Existing catalogs are not silently rescanned or rewritten. A field
that was not collected remains missing. The existing Windows bounded reader does
not gain new IPTC/XMP naming-label extraction in this increment; it continues to
provide its current make/model and indexed formats. No claim is made that all
location/camera-type fields are extracted on all platforms or from all formats.
The preview reports unavailable required fields explicitly. Source metadata
refresh/reader coverage is a separate qualification task, not inferred from
catalog-only tests.

Regression checks cover date/quarter boundaries, explicit naming values,
missing/ambiguous data, path injection, source fingerprint compatibility, template
sample retention and whole-library global conflicts. Instrumented desktop tests
exercise the grouped UI, custom chip input and real native catalog projections
using owned synthetic observations that still remain unavailable originals.
Those checks do not certify embedded-label extraction or physical keyboard input;
actual outcomes and limits are recorded in IMPLEMENTATION_LOG. Production
restructuring remains disabled pending reviewed whole-library execution,
destination/companion checks, recovery evidence and both-platform qualification.
