# ADR-0055: Explicit filename stem case in organization planning

Date: 2026-11-01
Status: implemented with local full/Windows native validation; macOS checkpoint CI pending; M2 remains open

## Decision

Add an explicit Preserve/Lowercase/Uppercase choice for proposed filename stems. Apply Unicode default casing to the resolved final filename stem, including an Undated fallback, before validating final length/portability and detecting collisions. Directory components and extension case remain exact. This is an output naming policy, not a guess about native filesystem equivalence or an original-file rewrite.

The single Rust IPC schema adds optional `OrganizationTemplate.filenameStemCase`. Missing and explicit Preserve values serialize identically to legacy templates, preserving their preview/draft/effect identity. Nondefault choices are retained with immutable template versions and participate in fingerprints. Saved-template policy metadata reports schema 2 and lowercase_stem/uppercase_stem for nondefault choices; legacy Preserve metadata remains schema 1/preserve. The application catalog schema, identifiers and original paths are unchanged. Older readers that do not understand the new policy must reject it rather than silently execute a different naming rule.

The preview form resets its old results and saved-template reference when the policy changes. Returning to Preserve removes the optional property. Saved definitions load the selected policy. Existing companion derivation follows the resolved primary stem while keeping RAW/sidecar/filename-sidecar suffix case. No new execution, destination overwrite or original-mutation grant is introduced.

## Validation and gate

Test legacy serialization/fingerprint identity, lower/uppercase output and fallback, unchanged source/folder/extension strings, collisions after transformation, Unicode length expansion, unsupported enum input, saved-version metadata/CAS, companion suffixes and strict generated IPC. Component checks cover policy invalidation and Preserve normalization. Native generated-library tests compare actual UI targets with real backend previews, retain independent unchanged-source audits and capture the uppercase preview, then restore Preserve before existing workflows. Embedded select automation uses the existing DOM change helper; this is not native menu automation evidence.

Full and native results belong in the implementation log. M2/M3, native name reservation, capacity/recovery and production original-mutation gates remain separate.
