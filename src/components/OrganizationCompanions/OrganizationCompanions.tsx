import { Box, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type { OrganizationCompanionReview } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

export function OrganizationCompanions({
  assetId,
  previewRevision,
  revision,
}: {
  assetId: string;
  previewRevision: string;
  revision: string;
}) {
  const request = useRef(0);
  const [review, setReview] = useState<OrganizationCompanionReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );

  async function load() {
    const id = ++request.current;
    setBusy(true);
    setError(null);
    setReview(null);
    try {
      const value = await libraryData(
        { kind: 'organization_companions', asset_id: assetId },
        'organization_companions',
      );
      if (request.current === id) setReview(value);
    } catch (error) {
      if (request.current === id) setError(errorMessage(error));
    } finally {
      if (request.current === id) setBusy(false);
    }
  }

  const stale =
    review !== null &&
    (review.catalogRevision !== revision || review.catalogRevision !== previewRevision);
  return (
    <Box className="organization-companions">
      <Button type="button" disabled={busy} onClick={() => void load()}>
        {busy
          ? 'Loading companions…'
          : review || error
            ? 'Refresh companion review'
            : 'Review indexed companions'}
      </Button>
      {error && (
        <Text role="alert" className="error">
          {error}
        </Text>
      )}
      {review && (
        <chakra.section aria-label="Indexed companion candidates">
          {stale && (
            <Text role="alert">
              The catalog changed. Build a fresh preview and review companions again.
            </Text>
          )}
          <Text className="help">
            Name matches need review; they are not confirmed bundles. Unindexed files may be absent.
            These files have no proposed destination and are not included in selected byte totals.
          </Text>
          {review.truncated && (
            <Text role="status">
              Showing {review.candidates.length} candidates; more indexed matches remain.
            </Text>
          )}
          {review.candidates.length === 0 ? (
            <Text>No indexed name matches. This does not prove there are no companions.</Text>
          ) : (
            <List.Root unstyled>
              {review.candidates.map((candidate) => (
                <List.Item unstyled key={candidate.assetId}>
                  <bdi>{candidate.sourcePath}</bdi>
                  <Text>
                    {candidate.filenameSidecar
                      ? 'Filename-sidecar candidate'
                      : 'Same-stem candidate'}
                    {' · '}
                    {candidate.isMedia ? 'Media' : 'Non-photo file'}
                    {' · '}
                    {BigInt(candidate.bytes).toLocaleString()} bytes
                  </Text>
                  <Text>
                    Catalog state: {candidate.presence} · Library: {candidate.rootAvailability}
                  </Text>
                </List.Item>
              ))}
            </List.Root>
          )}
        </chakra.section>
      )}
    </Box>
  );
}
