import { useState } from 'react';
import { Field } from '@chakra-ui/react';
import type { DuplicateGroup } from '../../../bindings';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { keeperRules, type KeeperRule, type CleanupPolicy } from '../model';
import { ReviewDialog } from './ReviewDialog';

export function CleanupRules({
  groups,
  policy,
  onPolicy,
  onClose,
}: {
  groups: DuplicateGroup[];
  policy: CleanupPolicy;
  onPolicy: (policy: CleanupPolicy) => void;
  onClose: () => void;
}) {
  const { rule, threshold, protections } = policy;
  const [preview, setPreview] = useState(false);
  return (
    <ReviewDialog title="Cleanup rules" onClose={onClose}>
      <p>Preview a keeper policy. These session choices never approve file removal.</p>
      <Field.Root>
        <Field.Label>Retain this version</Field.Label>
        <NativeSelect
          value={rule}
          onChange={(event) => {
            onPolicy({ ...policy, rule: event.target.value as KeeperRule });
            setPreview(false);
          }}
        >
          {keeperRules.map((item) => (
            <option key={item.value} value={item.value} disabled={Boolean(item.unavailable)}>
              {item.label}
              {item.unavailable ? ' · unavailable' : ''}
            </option>
          ))}
        </NativeSelect>
        <Field.HelperText>
          Resolution, RAW and size suggestions are available inside each complete member page. Ties
          keep multiple.
        </Field.HelperText>
      </Field.Root>
      <fieldset>
        <legend>Always protect</legend>
        {['Favourites', 'RAW files', 'Video masters', 'Shared albums', 'Edited derivatives'].map(
          (item) => (
            <label key={item}>
              <input
                type="checkbox"
                checked={protections.includes(item)}
                onChange={(event) => {
                  onPolicy({
                    ...policy,
                    protections: event.target.checked
                      ? [...protections, item]
                      : protections.filter((value) => value !== item),
                  });
                  setPreview(false);
                }}
              />
              {item}
            </label>
          ),
        )}
      </fieldset>
      <p>
        These protections are requirements for a future cleanup plan. This build cannot verify
        shared membership, edit lineage or video masters, so removal stays blocked even if a
        protection is unchecked.
      </p>
      <Field.Root>
        <Field.Label>Minimum visual confidence · {threshold}%</Field.Label>
        <Input
          type="range"
          aria-label="Minimum visual confidence"
          min="85"
          max="100"
          value={threshold}
          onChange={(event) => {
            onPolicy({ ...policy, threshold: Number(event.target.value) });
            setPreview(false);
          }}
        />
        <Field.HelperText>
          No visual scores are measured by this inspection engine. An exact byte match is a separate
          type of evidence.
        </Field.HelperText>
      </Field.Root>
      <div className="duplicate-rule-preview" role="status">
        <strong>{groups.length} groups in scope</strong>
        <p>
          0 scored groups meet {threshold}% · 0 files eligible for automatic removal · recoverable
          storage unknown.
        </p>
      </div>
      <details>
        <summary>Unavailable keeper signals</summary>
        {keeperRules
          .filter((item) => item.unavailable)
          .map((item) => (
            <p key={item.value}>
              {item.label}: {item.unavailable}
            </p>
          ))}
      </details>
      <Button onClick={() => setPreview(true)}>Preview cleanup</Button>
      {preview && (
        <section className="duplicate-cleanup-confirmation" aria-label="Cleanup confirmation">
          <h3>Keep your originals in place</h3>
          <p>
            No files can move to Recently Deleted. Fresh byte verification, complete companion
            protection and exact-version independent recovery are required. No disk space will be
            freed.
          </p>
          <p>
            Proposed keeper preference: {keeperRules.find((item) => item.value === rule)?.label}.{' '}
            {protections.length} protection requirements selected. Changing a policy requires a new
            backend plan.
          </p>
          <Button disabled>Move to Recently Deleted unavailable</Button>
        </section>
      )}
    </ReviewDialog>
  );
}
