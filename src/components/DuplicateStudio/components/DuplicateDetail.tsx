import { useEffect, useRef, useState } from 'react';
import { Field, Box, Heading, Text, chakra } from '@chakra-ui/react';
import type { AssetSummary, DuplicateGroup, FindingPage, LibraryStatus } from '../../../bindings';
import { errorMessage, libraryData } from '../../../ipc';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { DuplicateComparison } from '@/components/DuplicateComparison';
import { KeeperControls } from '@/components/KeeperControls';
import { displayBytes } from '../../../inspection/format';
import {
  duplicateBytes,
  evidenceLabels,
  keeperRules,
  suggestKeepers,
  type KeeperRule,
} from '../model';

export function DuplicateDetail({
  jobId,
  group,
  status,
  rule,
  onRule,
}: {
  jobId: string;
  group: DuplicateGroup;
  status: LibraryStatus;
  rule: KeeperRule;
  onRule: (rule: KeeperRule) => void;
}) {
  const [page, setPage] = useState<FindingPage | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [keep, setKeep] = useState(new Set<string>());
  const moreInFlight = useRef(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [left, setLeft] = useState('');
  const [right, setRight] = useState('');
  const [detail, setDetail] = useState<{ key: string; assets: AssetSummary[] } | null>(null);
  const [comparisonFailure, setComparisonFailure] = useState<{
    key: string;
    message: string;
  } | null>(null);
  useEffect(() => {
    let active = true;
    void libraryData(
      { kind: 'inspection_group_members', job_id: jobId, group_id: group.id, after: null },
      'inspection_findings',
    )
      .then((value) => {
        if (!active) return;
        setPage(value);
        setError(null);
        const media = value.rows.filter((row) => row.isMedia && row.assetId);
        setLeft(media[0]?.id ?? '');
        setRight(media[1]?.id ?? '');
      })
      .catch((failure: unknown) => {
        if (active) setError(errorMessage(failure));
      });
    return () => {
      active = false;
    };
  }, [jobId, group.id, attempt]);
  const pairKey = JSON.stringify([left, right, status.revision, attempt]);
  const pair = [left, right].map((id) => page?.rows.find((row) => row.id === id));
  const leftAsset = pair[0]?.assetId;
  const rightAsset = pair[1]?.assetId;
  useEffect(() => {
    if (!leftAsset || !rightAsset || leftAsset === rightAsset) return;
    let active = true;
    void Promise.all(
      [leftAsset, rightAsset].map((assetId) =>
        libraryData({ kind: 'detail', asset_id: assetId }, 'detail'),
      ),
    )
      .then((values) => {
        if (active) {
          setDetail({ key: pairKey, assets: values.map((value) => value.asset) });
          setComparisonFailure(null);
        }
      })
      .catch((failure: unknown) => {
        if (active) setComparisonFailure({ key: pairKey, message: errorMessage(failure) });
      });
    return () => {
      active = false;
    };
  }, [leftAsset, rightAsset, pairKey]);
  async function more() {
    if (!page?.next || moreInFlight.current) return;
    moreInFlight.current = true;
    setLoadingMore(true);
    try {
      const value = await libraryData(
        { kind: 'inspection_group_members', job_id: jobId, group_id: group.id, after: page.next },
        'inspection_findings',
      );
      setPage((previous) => ({ ...value, rows: [...(previous?.rows ?? []), ...value.rows] }));
      setError(null);
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      moreInFlight.current = false;
      setLoadingMore(false);
    }
  }
  const bytes = page ? duplicateBytes(group, page.rows) : null;
  const assets = detail?.key === pairKey ? detail.assets : [];
  const media = page?.rows.filter((row) => row.isMedia && row.assetId) ?? [];
  return (
    <>
      <chakra.header className="duplicate-detail-head">
        <Box>
          <Heading as="h3" unstyled>
            {evidenceLabels[group.evidence]}
          </Heading>
          <Text>
            {group.memberCount} files ·{' '}
            {group.evidence === 'exact_bytes' ? Math.max(0, group.memberCount - 1) : 'Unconfirmed'}{' '}
            duplicate candidate
            {group.evidence === 'exact_bytes' && group.memberCount === 2 ? '' : 's'}
          </Text>
        </Box>
        <Text as="span" className="duplicate-evidence-badge">
          {group.evidence === 'exact_bytes' ? 'Byte-verified at inspection' : 'Manual review'}
        </Text>
      </chakra.header>
      <dl className="duplicate-detail-stats">
        <Box>
          <dt>Logical duplicate bytes</dt>
          <dd>{bytes === null ? 'Unknown / not proven' : displayBytes(bytes)}</dd>
        </Box>
        <Box>
          <dt>Visual confidence</dt>
          <dd>Not measured</dd>
        </Box>
        <Box>
          <dt>Recoverable storage</dt>
          <dd>Not verified</dd>
        </Box>
      </dl>
      {error && (
        <Box role="alert">
          <Text>{error}</Text>
          <Button onClick={() => setAttempt((value) => value + 1)}>Retry group</Button>
        </Box>
      )}
      {!page && !error && <Text role="status">Loading group members…</Text>}
      {page && (
        <>
          {media.length >= 2 ? (
            <>
              <Box className="duplicate-pair-controls">
                {['First file', 'Second file'].map((label, index) => (
                  <Field.Root key={label}>
                    <Field.Label>{label}</Field.Label>
                    <NativeSelect
                      value={index === 0 ? left : right}
                      onChange={(event) =>
                        index === 0 ? setLeft(event.target.value) : setRight(event.target.value)
                      }
                    >
                      {media.map((row) => (
                        <option
                          key={row.id}
                          value={row.id}
                          disabled={row.id === (index === 0 ? right : left)}
                        >
                          {row.name}
                        </option>
                      ))}
                    </NativeSelect>
                  </Field.Root>
                ))}
              </Box>
              {assets.length === 2 ? (
                <DuplicateComparison key={pairKey} assets={assets} />
              ) : comparisonFailure?.key === pairKey ? (
                <Box role="alert">
                  {comparisonFailure.message}
                  <Button onClick={() => setAttempt((value) => value + 1)}>Retry comparison</Button>
                </Box>
              ) : (
                <Text role="status">Loading comparison details…</Text>
              )}
            </>
          ) : (
            <Text>
              Two indexed media files are needed for visual comparison. Non-media files and linked
              paths remain reviewable below.
            </Text>
          )}
          <chakra.section className="duplicate-signals" aria-label="Matching signals">
            <Heading as="h4" unstyled>
              Why these files were grouped
            </Heading>
            <Text>{group.explanation}</Text>
            <dl>
              <Box>
                <dt>File bytes</dt>
                <dd>
                  {group.evidence === 'exact_bytes'
                    ? 'Complete equality recorded at inspection; fresh verification required before removal.'
                    : 'Complete equality is not established by this evidence.'}
                </dd>
              </Box>
              <Box>
                <dt>Capture-time proximity</dt>
                <dd>
                  {assets.length === 2 &&
                  assets.every((asset) => asset.dateSource.startsWith('exif') && asset.rawDate)
                    ? `${assets[0]!.rawDate} / ${assets[1]!.rawDate} · offsets may be unknown`
                    : 'Capture-time provenance not established for this pair.'}
                </dd>
              </Box>
              <Box>
                <dt>Framing and derivative exports</dt>
                <dd>
                  {assets.length === 2 && assets.every((asset) => asset.width && asset.height)
                    ? `${assets[0]!.width} × ${assets[0]!.height} / ${assets[1]!.width} × ${assets[1]!.height}. Different dimensions do not establish export lineage.`
                    : 'Framing and export lineage not measured.'}
                </dd>
              </Box>
              <Box>
                <dt>Changed annotations</dt>
                <dd>
                  {assets.length === 2
                    ? assets[0]!.favorite !== assets[1]!.favorite ||
                      assets[0]!.rating !== assets[1]!.rating ||
                      JSON.stringify([...assets[0]!.tags].sort()) !==
                        JSON.stringify([...assets[1]!.tags].sort())
                      ? 'Catalog favourites, ratings or tags differ. Keep both until reviewed.'
                      : 'Catalog favourites, ratings and tags match. Embedded annotations have not been compared.'
                    : 'Catalog annotations not loaded.'}
                </dd>
              </Box>
              <Box>
                <dt>Audio fingerprints</dt>
                <dd>Not measured · matching video frames never prove matching audio.</dd>
              </Box>
            </dl>
          </chakra.section>
          <chakra.section className="duplicate-member-review" aria-label="Choose keeper files">
            <Heading as="h4" unstyled>
              Choose what stays
            </Heading>
            <Box className="duplicate-rule-actions">
              <Field.Root>
                <Field.Label>Keeper suggestion</Field.Label>
                <NativeSelect
                  value={rule}
                  onChange={(event) => onRule(event.target.value as KeeperRule)}
                >
                  {keeperRules.map((item) => (
                    <option
                      key={item.value}
                      value={item.value}
                      disabled={Boolean(item.unavailable)}
                    >
                      {item.label}
                      {item.unavailable ? ' · unavailable' : ''}
                    </option>
                  ))}
                </NativeSelect>
              </Field.Root>
              <Button
                disabled={Boolean(page.next) || group.membersTruncated}
                onClick={() => setKeep(suggestKeepers(page.rows, rule))}
              >
                Suggest keepers
              </Button>
              <Button onClick={() => setKeep(new Set(page.rows.map((row) => row.id)))}>
                Keep all loaded files
              </Button>
            </Box>
            <Text>
              Suggestions keep ties and unknown resolution. They never remove files or override
              protections. Load every member before applying a group-wide rule.
            </Text>
            <Box className="duplicate-member-list">
              {page.rows.map((row) => (
                <label key={row.id}>
                  <input
                    type="checkbox"
                    checked={keep.has(row.id)}
                    onChange={(event) =>
                      setKeep((previous) => {
                        const value = new Set(previous);
                        if (event.target.checked) value.add(row.id);
                        else value.delete(row.id);
                        return value;
                      })
                    }
                  />
                  <Text as="span">
                    <strong>
                      <bdi>{row.name}</bdi>
                    </strong>
                    <small>
                      <bdi>{row.displayPath}</bdi>
                    </small>
                    <small>
                      {displayBytes(row.bytes)} ·{' '}
                      {row.width && row.height
                        ? `${row.width} × ${row.height}`
                        : 'Dimensions unknown'}{' '}
                      ·{' '}
                      {status.roots.find((root) => root.id === row.rootId)?.availability ??
                        'Availability unknown'}
                    </small>
                  </Text>
                  <Text as="span">{keep.has(row.id) ? 'Keep' : 'Undecided'}</Text>
                </label>
              ))}
            </Box>
            {page.next && (
              <Button disabled={loadingMore} onClick={() => void more()}>
                {loadingMore ? 'Loading group members…' : 'Load more group members'}
              </Button>
            )}
            <details className="duplicate-backend-review">
              <summary>Retain keeper choices and folder protections</summary>
              <KeeperControls
                jobId={jobId}
                groupId={group.id}
                rows={page.rows}
                keep={keep}
                onKeep={setKeep}
              />
            </details>
          </chakra.section>
        </>
      )}
    </>
  );
}
