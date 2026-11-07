import { Box, Dialog, Heading, Portal, Text } from '@chakra-ui/react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/Button';
import type { LibraryOrganizationPreview } from '../../../bindings';
import { errorMessage } from '../../../ipc';
import { structureDialogStyle } from '../StructureFormatter.recipe';
import { focusRing } from '../../../theme/focus-ring';

export function StructureSimulationDialog({
  preview,
  stale,
  onValidate,
  onComplete,
  onClose,
}: {
  preview: LibraryOrganizationPreview;
  stale: boolean;
  onValidate: () => Promise<LibraryOrganizationPreview>;
  onComplete: (preview: LibraryOrganizationPreview) => void;
  onClose: () => void;
}) {
  const cancel = useRef<HTMLButtonElement>(null);
  const generation = useRef(0);
  const [validationRequest, setValidationRequest] = useState<number | null>(null);
  const busy = validationRequest !== null;
  const [error, setError] = useState<string | null>(null);
  useEffect(
    () => () => {
      generation.current += 1;
    },
    [],
  );
  async function confirm() {
    const request = ++generation.current;
    setValidationRequest(request);
    setError(null);
    try {
      const current = await onValidate();
      if (request !== generation.current) return;
      if (
        !current.complete ||
        !current.summary ||
        current.previewId !== preview.previewId ||
        current.catalogRevision !== preview.catalogRevision
      )
        throw new Error('The preview changed. Return to editing and build it again.');
      onComplete(current);
    } catch (cause) {
      if (request === generation.current) setError(errorMessage(cause));
    } finally {
      if (request === generation.current) {
        setValidationRequest(null);
      }
    }
  }
  function close() {
    generation.current += 1;
    onClose();
  }
  return (
    <Dialog.Root
      open
      onOpenChange={({ open }) => {
        if (!open) close();
      }}
      initialFocusEl={() => cancel.current}
    >
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content css={structureDialogStyle}>
            <Dialog.Header>
              <Dialog.Title>Simulate this plan?</Dialog.Title>
              <Dialog.Description>
                This demonstrates completion using the reviewed proposals. No photos, folders or
                filenames will be changed.
              </Dialog.Description>
            </Dialog.Header>
            <Dialog.Body>
              <Text>
                {preview.summary?.moves.toLocaleString() ?? '0'} proposed moves ·{' '}
                {preview.summary?.renames.toLocaleString() ?? '0'} proposed renames. Skipped and
                blocked items remain untouched.
              </Text>
              <Text color="fg.muted" mt="3">
                Simulation is a rehearsal. Actual restructuring remains disabled.
              </Text>
              {stale && (
                <Text role="alert" mt="3">
                  The catalog changed. Return to editing and rebuild the preview.
                </Text>
              )}
              {error && (
                <Text role="alert" mt="3">
                  {error}
                </Text>
              )}
            </Dialog.Body>
            <Dialog.Footer>
              <Button type="button" ref={cancel} onClick={close}>
                Keep reviewing
              </Button>
              <Button
                type="button"
                className="primary"
                css={{
                  marginTop: '28px',
                  border: '0',
                  background: '{colors.brand.solid}',
                  color: '{colors.brand.contrast}',
                  fontWeight: '600',
                  fontSize: '13px',
                  lineHeight: '20px',
                  borderRadius: 'var(--control-radius)',
                  padding: '12px 24px',
                  minHeight: '44px',
                  transition: 'background-color 140ms var(--ui-ease)',
                  '&:hover:enabled': { background: '{colors.brand.hover}' },
                }}
                disabled={busy || stale}
                onClick={() => void confirm()}
              >
                {busy ? 'Rechecking preview…' : 'Run simulation'}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}

export function StructureSimulationComplete({
  preview,
  onEdit,
  onReview,
}: {
  preview: LibraryOrganizationPreview;
  onEdit: () => void;
  onReview: () => void;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
  }, []);
  return (
    <Box className="structure-completion">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.5" />
        <path d="m7 12 3 3 7-7" stroke="currentColor" strokeWidth="1.5" />
      </svg>
      <Heading as="h2" fontSize="2xl" ref={heading} tabIndex={-1} _focusVisible={focusRing}>
        Simulation complete
      </Heading>
      <Text mt="3" mb="6">
        No files were moved or renamed. Your real library is unchanged.
      </Text>
      {preview.summary && (
        <Text as="section" aria-label="Simulated changes">
          {preview.summary.moves.toLocaleString()} simulated moves ·{' '}
          {preview.summary.renames.toLocaleString()} simulated renames ·{' '}
          {preview.summary.folders.toLocaleString()} destination folders
        </Text>
      )}
      <Text color="fg.muted" my="5">
        These are simulated results from the preview. Skipped and blocked items were excluded; no
        filesystem work or backup operation was performed.
      </Text>
      <Box display="flex" gap="3" flexWrap="wrap">
        <Button
          type="button"
          className="primary"
          css={{
            marginTop: '28px',
            border: '0',
            background: '{colors.brand.solid}',
            color: '{colors.brand.contrast}',
            fontWeight: '600',
            fontSize: '13px',
            lineHeight: '20px',
            borderRadius: 'var(--control-radius)',
            padding: '12px 24px',
            minHeight: '44px',
            transition: 'background-color 140ms var(--ui-ease)',
            '&:hover:enabled': { background: '{colors.brand.hover}' },
          }}
          onClick={onEdit}
        >
          Back to editing
        </Button>
        <Button type="button" onClick={onReview}>
          Review preview
        </Button>
        <Button type="button" disabled aria-describedby="structure-complete-help">
          Run restructure
        </Button>
      </Box>
      <Text
        id="structure-complete-help"
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
        mt="4"
      >
        Actual restructuring remains disabled pending reviewed execution and proven recovery.
      </Text>
    </Box>
  );
}
