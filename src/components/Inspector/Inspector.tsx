import { Field, Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type { AnnotationPatch, AssetDetail, AssetSummary, LibraryRoot } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { displayDate, fileSize } from '../../library/model';
import { AssetNotesPanel } from '@/components/AssetNotesPanel';
import { KeywordImport } from '@/components/KeywordImport';

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
    <chakra.aside className="inspector" aria-label="Photo details">
      <Box className="section-heading">
        <Heading as="h2" unstyled ref={heading} tabIndex={-1}>
          Photo details
        </Heading>
        <Button onClick={onClose}>Close</Button>
      </Box>
      {asset && detail ? (
        <>
          <Heading as="h3" unstyled>
            <bdi>{asset.name}</bdi>
          </Heading>
          <Text className="path">
            <bdi>{asset.relativePath}</bdi>
          </Text>
          <AssetFields asset={asset} root={detail.root} />
          {(asset.metadataError || asset.previewError) && (
            <Text className="help">{asset.metadataError ?? asset.previewError}</Text>
          )}
          {saveError && <Text role="alert">{saveError}</Text>}
          <chakra.form
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
            <Heading as="h3" unstyled>
              Catalog notes
            </Heading>
            <Text className="help">Saved here. Originals and sidecars stay unchanged.</Text>
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
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Rating</span>
                </Field.Label>
                <NativeSelect
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
                </NativeSelect>
              </label>
            </Field.Root>
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Tags, separated by commas</span>
                </Field.Label>
                <Input
                  value={tags ?? asset.tags.join(', ')}
                  maxLength={2048}
                  onChange={(event) => setTags(event.target.value)}
                />
              </label>
            </Field.Root>
            <Button disabled={busy} type="submit">
              Save tags
            </Button>
          </chakra.form>
          <AssetNotesPanel assetId={asset.id} />
          <KeywordImport assetId={asset.id} onImported={() => setAttempt((value) => value + 1)} />
          {detail.companions.length > 0 && (
            <details>
              <summary>Possible companions ({detail.companions.length})</summary>
              <List.Root unstyled>
                {detail.companions.map((path) => (
                  <List.Item unstyled key={path}>
                    <bdi>{path}</bdi>
                  </List.Item>
                ))}
              </List.Root>
              <Text className="help">
                Matching names are shown for reference; files are not automatically grouped or
                changed.
              </Text>
            </details>
          )}
          <details>
            <summary>Raw metadata</summary>
            <pre>{detail.rawMetadata ?? 'Metadata unavailable.'}</pre>
          </details>
        </>
      ) : loadError ? (
        <Box>
          <Text>Photo details couldn’t be loaded.</Text>
          <Text className="help">{loadError}</Text>
          <Button
            onClick={() => {
              setLoadError(null);
              setAttempt((value) => value + 1);
            }}
          >
            Retry details
          </Button>
        </Box>
      ) : (
        <Text>Loading details…</Text>
      )}
    </chakra.aside>
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
