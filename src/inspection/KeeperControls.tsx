import { Button, Select } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import type { InspectionFinding, KeeperOrder, KeeperReview } from '../bindings';
import { errorMessage, libraryData } from '../ipc';

export function KeeperControls({
  jobId,
  groupId,
  rows,
  keep,
  onKeep,
}: {
  jobId: string;
  groupId: string;
  rows: InspectionFinding[];
  keep: Set<string>;
  onKeep: (ids: Set<string>) => void;
}) {
  const [order, setOrder] = useState<KeeperOrder>('manual');
  const [protectedFolders, setProtected] = useState<string[]>([]);
  const [preferredFolders, setPreferred] = useState<string[]>([]);
  const [review, setReview] = useState<KeeperReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  useEffect(
    () => () => {
      generation.current += 1;
    },
    [],
  );
  const folders = [
    ...new Map(
      rows.map((row) => [row.folderId, row.displayPath.slice(0, -row.name.length)]),
    ).entries(),
  ];
  async function apply() {
    const token = ++generation.current;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        {
          kind: 'inspection_keeper_review',
          input: {
            jobId,
            groupId,
            order,
            manualKeepers: [...keep],
            protectedFolders,
            preferredFolders,
          },
        },
        'inspection_keeper_review',
      );
      if (token === generation.current) setReview(result);
    } catch (failure) {
      if (token === generation.current) setError(errorMessage(failure));
    } finally {
      if (token === generation.current) setBusy(false);
    }
  }
  async function latest() {
    const token = ++generation.current;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        { kind: 'inspection_keeper_latest', job_id: jobId, group_id: groupId },
        'inspection_keeper_latest',
      );
      if (token === generation.current) {
        setReview(result);
        if (!result) setError('No keeper review has been retained for this group.');
      }
    } catch (failure) {
      if (token === generation.current) setError(errorMessage(failure));
    } finally {
      if (token === generation.current) setBusy(false);
    }
  }
  function toggle(ids: string[], id: string, checked: boolean) {
    return checked
      ? [...ids.filter((value) => value !== id), id]
      : ids.filter((value) => value !== id);
  }
  return (
    <fieldset disabled={busy}>
      <legend>Backend keeper review</legend>
      <label>
        Filesystem date policy{' '}
        <Select
          value={order}
          onChange={(event) => {
            setOrder(event.target.value as KeeperOrder);
            setReview(null);
          }}
        >
          <option value="manual">Manual keep choices</option>
          <option value="oldest_modified">Keep oldest modified</option>
          <option value="newest_modified">Keep newest modified</option>
          <option value="oldest_created">Keep oldest created/birth</option>
          <option value="newest_created">Keep newest created/birth</option>
        </Select>
      </label>
      <p>
        Explicit keep choices and protected folders take priority. Preferred folders follow the
        order selected. Unknown dates require review; date ties use stable IDs. Changes take effect
        in a newly retained review.
      </p>
      {folders.map(([id, path]) => (
        <div key={id}>
          <span>{path}</span>
          <label>
            <input
              type="checkbox"
              checked={protectedFolders.includes(id)}
              onChange={(event) => {
                setProtected(toggle(protectedFolders, id, event.target.checked));
                setReview(null);
              }}
            />{' '}
            Protect this folder
          </label>
          <label>
            <input
              type="checkbox"
              checked={preferredFolders.includes(id)}
              onChange={(event) => {
                setPreferred(toggle(preferredFolders, id, event.target.checked));
                setReview(null);
              }}
            />{' '}
            Prefer this folder{' '}
            {preferredFolders.includes(id) ? `(${preferredFolders.indexOf(id) + 1})` : ''}
          </label>
        </div>
      ))}
      <details>
        <summary>Manual keep choices on this member page</summary>
        {rows.map((row) => (
          <label key={row.id}>
            <input
              type="checkbox"
              checked={keep.has(row.id)}
              onChange={(event) => {
                const next = new Set(keep);
                if (event.target.checked) next.add(row.id);
                else next.delete(row.id);
                onKeep(next);
                setReview(null);
              }}
            />
            {row.displayPath}
          </label>
        ))}
      </details>
      <Button
        onClick={() => {
          onKeep(new Set());
          setReview(null);
        }}
      >
        Clear manual choices
      </Button>
      <Button onClick={() => void apply()}>
        {busy ? 'Reviewing…' : 'Retain backend keeper review'}
      </Button>
      <Button onClick={() => void latest()}>Load latest retained keeper review</Button>
      {error && <p role="alert">{error}</p>}
      {review && (
        <div role="status">
          <p>
            Retained review · {review.input.order.replaceAll('_', ' ')} ·{' '}
            {review.input.protectedFolders.length} protected folders ·{' '}
            {review.input.preferredFolders.length} preferred folders. These recorded choices may
            differ from the current form.
          </p>
          <Button
            onClick={() => {
              setOrder(review.input.order);
              setProtected(review.input.protectedFolders);
              setPreferred(review.input.preferredFolders);
              onKeep(new Set(review.input.manualKeepers));
            }}
          >
            Reuse this recorded policy in the form
          </Button>
          <p>{review.explanation}</p>
          <p>
            {review.keepers.length} suggested keepers; {review.blocks.length} cleanup blocks.
            Quarantine remains unavailable.
          </p>
          {review.suggestionBlock && <p>{review.suggestionBlock.replaceAll('_', ' ')}</p>}
          <ul>
            {review.keepers.map((id) => (
              <li key={id}>Keep: {rows.find((row) => row.id === id)?.displayPath ?? id}</li>
            ))}
          </ul>
          <ul>
            {review.blocks.slice(0, 64).map((block, index) => (
              <li key={`${block.assetId}:${index}`}>
                {rows.find((row) => row.id === block.assetId)?.name ?? block.assetId}:{' '}
                {block.reason.replaceAll('_', ' ')}
              </li>
            ))}
          </ul>
          {review.blocks.length > 64 && (
            <p>Showing the first 64 blocks; the complete review is retained in the catalog.</p>
          )}
        </div>
      )}
    </fieldset>
  );
}
