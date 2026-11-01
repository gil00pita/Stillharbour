# ADR-0074: Conservative comparison conflicts and full-report review

Date: 2026-11-01

Status: implemented M4A increment; executed platform evidence recorded separately.

## Decision

Comparison policy 2 retains native paths and observed file/directory/unsupported kinds separately. Inventory failures cannot turn an observed object into absence. Block file/directory and ancestor conflicts, unsupported objects, non-Unicode native names and potential case/Unicode aliases, including descendants of ambiguous directories. Alias screening uses component-wise NFC/lowercase/NFC keys only to require review. These keys do not establish native filesystem equivalence, cover every filesystem naming rule or authorize publication. Keep all distinct native names; never round-trip display strings as execution paths.

Blocked rows have no copy proposal, are excluded from payload comparison and make the report incomplete without a digest. Inventory issues identify their source or destination. Complete unaffected pairs may still report their own byte evidence. Comparison remains read-only, bounded and revision-checked, without a destination capability or transfer approval.

Compute category totals and optional filtered pages in Rust over the complete bounded report. Filter offsets apply to matching rows, with 200 rows per page. React retires prior pages and entry details on filter/page/run changes. The inspector renders existing observations as escaped text; it introduces no new file read, arbitrary-path command or image fidelity claim.

## Consequences

No dependency or catalog migration is required; saved profile format 1 and schema 11 remain unchanged. Policy-2 review digests also bind the optional profile ID and remain session-only. Conservative warnings may block names that coexist on a particular filesystem. Native destination-specific name preparation, historical verified manifests, companion completeness, visual media review and remaining platform/failure gates stay open before M4A acceptance. See [M4 scope](../M4_BACKUP_SYNC.md).
