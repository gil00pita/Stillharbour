import { useEffect, useRef, useState } from 'react';
import type { AnnotationPatch, AssetDetail, AssetSummary, LibraryRoot } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { displayDate, fileSize } from './model';

export function Inspector({
  id,
  revision,
  onClose,
  onAnnotate,
  onError,
}: {
  id: string;
  revision: string;
  onClose: () => void;
  onAnnotate: (patch: AnnotationPatch, id?: string) => Promise<void>;
  onError: (error: unknown) => void;
}) {
  const [detail, setDetail] = useState<AssetDetail | null>(null);
  const [tags, setTags] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  useEffect(() => {
    let active = true;
    libraryData({ kind: 'detail', asset_id: id }, 'detail')
      .then((value) => {
        if (active) {
          setDetail(value);
          setLoadError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setLoadError(errorMessage(error));
          onError(error);
        }
      });
    return () => {
      active = false;
    };
  }, [id, revision, onError, attempt]);
  async function save(patch: AnnotationPatch) {
    setBusy(true);
    setSaveError(null);
    try {
      await onAnnotate(patch, id);
    } catch (error) {
      setSaveError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  const asset = detail?.asset;
  return (
    <aside className="inspector" aria-label="Photo details">
      <div className="section-heading">
        <h2 ref={heading} tabIndex={-1}>
          Photo details
        </h2>
        <button onClick={onClose}>Close</button>
      </div>
      {asset && detail ? (
        <>
          <h3>
            <bdi>{asset.name}</bdi>
          </h3>
          <p className="path">
            <bdi>{asset.relativePath}</bdi>
          </p>
          <AssetFields asset={asset} root={detail.root} />
          {(asset.metadataError || asset.previewError) && (
            <p className="help">{asset.metadataError ?? asset.previewError}</p>
          )}
          {saveError && <p role="alert">{saveError}</p>}
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void save({
                favorite: null,
                rating: null,
                tags: (tags ?? asset.tags.join(', '))
                  .split(',')
                  .map((s) => s.trim())
                  .filter(Boolean),
              });
            }}
          >
            <h3>Catalog notes</h3>
            <p className="help">Saved here. Originals and sidecars stay unchanged.</p>
            <label className="check-row">
              <input
                type="checkbox"
                disabled={busy}
                checked={asset.favorite}
                onChange={(event) =>
                  void save({ favorite: event.target.checked, rating: null, tags: null })
                }
              />{' '}
              Favorite
            </label>
            <label>
              Rating
              <select
                disabled={busy}
                value={asset.rating}
                onChange={(event) =>
                  void save({ favorite: null, rating: Number(event.target.value), tags: null })
                }
              >
                {[0, 1, 2, 3, 4, 5].map((value) => (
                  <option key={value} value={value}>
                    {value === 0 ? 'Unrated' : `${value} / 5`}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Tags, separated by commas
              <input
                value={tags ?? asset.tags.join(', ')}
                maxLength={2048}
                onChange={(event) => setTags(event.target.value)}
              />
            </label>
            <button disabled={busy} type="submit">
              Save tags
            </button>
          </form>
          {detail.companions.length > 0 && (
            <details>
              <summary>Possible companions ({detail.companions.length})</summary>
              <ul>
                {detail.companions.map((path) => (
                  <li key={path}>
                    <bdi>{path}</bdi>
                  </li>
                ))}
              </ul>
              <p className="help">
                Matching names are shown for reference; files are not automatically grouped or
                changed.
              </p>
            </details>
          )}
          <details>
            <summary>Raw metadata</summary>
            <pre>{detail.rawMetadata ?? 'Metadata unavailable.'}</pre>
          </details>
        </>
      ) : loadError ? (
        <div>
          <p>Photo details couldn’t be loaded.</p>
          <p className="help">{loadError}</p>
          <button
            onClick={() => {
              setLoadError(null);
              setAttempt((value) => value + 1);
            }}
          >
            Retry details
          </button>
        </div>
      ) : (
        <p>Loading details…</p>
      )}
    </aside>
  );
}

function AssetFields({ asset, root }: { asset: AssetSummary; root: LibraryRoot }) {
  return (
    <dl>
      <dt>Library</dt>
      <dd>
        {root.label} · {root.availability}
      </dd>
      <dt>Date shown</dt>
      <dd>{displayDate(asset.date)}</dd>
      <dt>Date source</dt>
      <dd>{asset.dateSource.replaceAll('_', ' ')}</dd>
      <dt>Original date</dt>
      <dd>{asset.rawDate ?? 'Not recorded'}</dd>
      <dt>Time zone</dt>
      <dd>{asset.offset ?? 'Unknown · capture time kept as recorded'}</dd>
      <dt>Dimensions</dt>
      <dd>{asset.width && asset.height ? `${asset.width} × ${asset.height}` : 'Unavailable'}</dd>
      <dt>Format / size</dt>
      <dd>
        {asset.detectedFormat ?? asset.extension.toUpperCase()} · {fileSize(asset.bytes)}
      </dd>
      <dt>Camera</dt>
      <dd>{asset.camera ?? 'Not recorded'}</dd>
      <dt>Added to catalog</dt>
      <dd>{displayDate(asset.addedAt)}</dd>
      <dt>File modified</dt>
      <dd>{asset.modifiedAt ? displayDate(asset.modifiedAt) : 'Unknown'}</dd>
      <dt>Availability</dt>
      <dd>{asset.presence}</dd>
      <dt>Metadata / preview</dt>
      <dd>
        {asset.metadataState} / {asset.previewState}
      </dd>
    </dl>
  );
}
