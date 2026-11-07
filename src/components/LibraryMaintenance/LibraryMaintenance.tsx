import { Box, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { MaintenancePreview, MaintenanceResult, MaintenanceTarget } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

export function LibraryMaintenance({
  target,
  onClose,
  onComplete,
}: {
  target: MaintenanceTarget;
  onClose: () => void;
  onComplete: (result: MaintenanceResult) => Promise<void>;
}) {
  const [preview, setPreview] = useState<MaintenancePreview | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [completed, setCompleted] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    let active = true;
    heading.current?.focus();
    libraryData({ kind: 'prepare_maintenance', target }, 'maintenance_preview')
      .then((value) => {
        if (active) {
          setPreview(value);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      })
      .finally(() => {
        if (active) {
          setBusy(false);
        }
      });
    return () => {
      active = false;
    };
  }, [target, attempt]);
  async function confirm(event: FormEvent) {
    event.preventDefault();
    if (!preview || busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        { kind: 'apply_maintenance', plan_id: preview.id },
        'maintenance_done',
      );
      setCompleted(true);
      setPreview(null);
      await onComplete(result);
    } catch (error) {
      setError(errorMessage(error));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  }
  const removing = target.kind === 'library';
  return (
    <chakra.section
      className="maintenance-review"
      css={{
        margin: '22px 0',
        padding: '20px',
        border: '1px solid {colors.border}',
        '& p': { maxWidth: '72ch', lineHeight: '1.6' },
        '& .settings-library-actions': { marginTop: '18px' },
      }}
      aria-labelledby="maintenance-review-title"
    >
      <chakra.form
        aria-label="Review library maintenance"
        onSubmit={(event) => void confirm(event)}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !busy) {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <Heading as="h3" unstyled id="maintenance-review-title" tabIndex={-1} ref={heading}>
          {completed ? 'Completed' : removing ? 'Remove this library?' : 'Clear preview cache?'}
        </Heading>
        {completed ? (
          <Text role="status">
            {removing
              ? 'Library removed. Original photos and sidecars are unchanged.'
              : 'Indexed preview cache cleared. Libraries and catalog notes are unchanged.'}
          </Text>
        ) : (
          <>
            {removing && (
              <Text role={busy ? 'status' : undefined}>
                {busy
                  ? 'Stopping this library’s jobs and preparing removal…'
                  : preview
                    ? 'This library’s active jobs are stopped before review. Canceling removal does not restart them.'
                    : 'Review could not be completed. Check the error before trying again.'}
              </Text>
            )}
            <Text>
              {removing
                ? 'This forgets the selected folder, its indexed records, catalog favorites, ratings, tags and scan history, and clears its indexed cached previews. You can add the folder again, but its catalog notes will not return. Library change history, including previous paths, remains in this catalog.'
                : 'Clear indexed cached previews for every library. Originals, library settings and catalog notes stay unchanged. Previews regenerate when accessible photos are viewed; offline photos need their originals reconnected.'}
            </Text>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Original photos and sidecars stay unchanged. Only indexed previews are cleared;
              unexpected files are retained.
            </Text>
            {preview?.rootDisplayPath && (
              <Text
                className="path"
                css={{
                  fontSize: '13px',
                  lineHeight: '1.6',
                  overflowWrap: 'anywhere',
                  whiteSpace: 'pre-wrap',
                  margin: '8px 0 0',
                  color: '{colors.fg.muted}',
                }}
              >
                <bdi>{preview.rootDisplayPath}</bdi>
              </Text>
            )}
            {preview && (
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                <strong>
                  <bdi>{preview.rootLabel ?? 'All libraries'}</bdi>
                </strong>{' '}
                · {removing ? `${preview.assetCount.toLocaleString()} catalog records · ` : ''}
                {preview.cachedPreviews.toLocaleString()} cached previews ·{' '}
                {BigInt(preview.cacheBytes).toLocaleString()} bytes
              </Text>
            )}
          </>
        )}
        {error && (
          <Text
            role="alert"
            className="error"
            css={{
              border: '1px solid {colors.border.control}',
              borderRadius: 'var(--control-radius)',
              background: '{colors.danger.bg}',
              color: '{colors.danger.fg}',
              padding: '16px 20px',
              margin: '24px 0 0',
              fontSize: '13px',
              lineHeight: '1.6',
              '& p': { margin: '0' },
              '& button': {
                marginTop: '12px',
                border: '1px solid currentColor',
                background: 'transparent',
                color: 'inherit',
                borderRadius: '4px',
                padding: '7px 12px',
              },
            }}
          >
            {completed
              ? 'The action completed, but refreshing Settings failed. Reopen Settings to refresh.'
              : error}
          </Text>
        )}
        <Box
          className="settings-library-actions"
          css={{ display: 'flex', flexWrap: 'wrap', alignItems: 'start', gap: '8px' }}
        >
          <Button type="button" disabled={busy} onClick={onClose}>
            {completed ? 'Done' : 'Cancel'}
          </Button>
          {!completed && !preview && !busy && (
            <Button
              type="button"
              onClick={() => {
                setBusy(true);
                setError(null);
                setAttempt((value) => value + 1);
              }}
            >
              Review again
            </Button>
          )}
          {!completed && (
            <Button
              type="submit"
              className={removing ? 'destructive' : ''}
              css={{
                '&.destructive': { 'button&:not(.primary)': { color: '{colors.fg.secondary}' } },
              }}
              disabled={busy || !preview}
            >
              {busy
                ? removing
                  ? 'Stopping jobs…'
                  : 'Please wait…'
                : removing
                  ? 'Remove library and its previews'
                  : 'Clear all cached previews'}
            </Button>
          )}
        </Box>
      </chakra.form>
    </chakra.section>
  );
}
