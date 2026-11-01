import { useEffect, useRef, useState } from 'react';
import type {
  CopyAssessment,
  CopyFidelityAspect,
  CopyFidelityStatus,
  CopyManifest,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';

const aspects: Record<CopyFidelityAspect, string> = {
  file_bytes: 'File bytes',
  confirmed_companions: 'Confirmed companions',
  timestamps: 'Timestamps',
  permissions_and_ownership: 'Permissions and ownership',
  extended_attributes_and_resource_forks: 'Extended attributes and resource forks',
  alternate_data_streams: 'Alternate data streams',
  sparse_and_compression_layout: 'Sparse and compression layout',
};
const statuses: Record<CopyFidelityStatus, string> = {
  verification_required: 'Full byte verification required',
  separate_files_selected: 'Selected as separate files',
  not_preserved_or_verified: 'Not preserved or verified',
};
export function CopyAssessmentReview({ manifest }: { manifest: CopyManifest }) {
  const [report, setReport] = useState<CopyAssessment | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  useEffect(() => {
    if (report) heading.current?.focus();
  }, [report]);
  async function check() {
    if (pending) return;
    const token = ++request.current;
    setPending(true);
    setReport(null);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_assessment',
          plan_id: manifest.planId,
          manifest_id: manifest.id,
        },
        'organization_copy_assessment',
      );
      if (request.current !== token) return;
      if (
        value.planId !== manifest.planId ||
        value.manifestId !== manifest.id ||
        value.manifestDigest !== manifest.digest
      ) {
        throw new Error(
          'The space report belongs to a different review. Compile a fresh manifest.',
        );
      }
      setReport(value);
    } catch (error) {
      if (request.current === token) setError(errorMessage(error));
    } finally {
      if (request.current === token) setPending(false);
    }
  }
  const capacity = report?.capacity;
  return (
    <section aria-label="Copy space and fidelity" aria-busy={pending}>
      <button type="button" disabled={pending} onClick={() => void check()}>
        {pending ? 'Checking copy space…' : 'Check copy space and fidelity'}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {report && capacity && (
        <>
          <h4 ref={heading} tabIndex={-1}>
            Copy space estimate and fidelity
          </h4>
          <p>
            {capacity.status === 'insufficient_for_estimate'
              ? 'Observed space is below the copy estimate. Choose another destination or reduce the selection.'
              : 'Observed space meets this planning estimate.'}{' '}
            Space is not reserved; recheck before approval and copying.
          </p>
          {capacity.status === 'insufficient_for_estimate' && (
            <p role="alert">Insufficient observed space blocks copying.</p>
          )}
          <dl>
            {(
              [
                ['Selected file bytes', capacity.logicalBytes],
                ['Data rounded per file', capacity.roundedDataBytes],
                ['Metadata allowance', capacity.metadataAllowanceBytes],
                ['Journal allowance', capacity.journalAllowanceBytes],
                ['Headroom', capacity.headroomBytes],
                ['Estimated peak additional space', capacity.estimatedPeakAdditionalBytes],
                ['Observed available space', capacity.observedAvailableBytes],
                ['Observed total volume space', capacity.observedTotalBytes],
                ['Native allocation unit', capacity.allocationUnitBytes],
              ] satisfies [string, string][]
            ).map(([label, bytes]) => (
              <div key={label}>
                <dt>{label}</dt>
                <dd>{BigInt(bytes).toLocaleString()} bytes</dd>
              </div>
            ))}
          </dl>
          <p>
            {capacity.outputDirectories} output folders, including the pending output root. Staging
            and publication use the same file objects. Allowances are estimates, not a filesystem
            capacity guarantee.
          </p>
          <p>
            Originals stay in place. This report does not verify any copied content or independent
            backup. The current copy engine has these fidelity limits:
          </p>
          <div className="organization-table">
            <table>
              <caption>Copy fidelity scope</caption>
              <thead>
                <tr>
                  <th>Aspect</th>
                  <th>Current support</th>
                </tr>
              </thead>
              <tbody>
                {report.fidelity.map((item) => (
                  <tr key={item.aspect}>
                    <td>{aspects[item.aspect]}</td>
                    <td>{statuses[item.status]}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="help">
            Observed {report.observedAt}. This report does not approve copying.
          </p>
        </>
      )}
    </section>
  );
}
