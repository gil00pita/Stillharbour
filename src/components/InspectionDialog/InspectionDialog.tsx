import { useEffect, useState, type ReactNode } from 'react';
import { Box, useRecipe, Heading, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { inspectionDialogRecipe } from './InspectionDialog.recipe';

export function InspectionDialog({
  title,
  children,
  onClose,
  footer,
  busy = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  busy?: boolean;
}) {
  const [element, setElement] = useState<HTMLDialogElement | null>(null);
  const styles = useRecipe({ recipe: inspectionDialogRecipe })();
  useEffect(() => {
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, [element]);
  function close() {
    if (busy) return;
    element?.close();
    onClose();
  }
  return (
    <Box asChild css={styles}>
      <dialog
        ref={setElement}
        className="inspection-dialog"
        aria-label={title}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !event.defaultPrevented) {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
        onCancel={(event) => {
          event.preventDefault();
          close();
        }}
      >
        <chakra.header>
          <Heading as="h2" unstyled>
            {title}
          </Heading>
          <Button disabled={busy} onClick={close}>
            Close
          </Button>
        </chakra.header>
        <Box
          className="inspection-content"
          css={{
            overflow: 'auto',
            minHeight: '0',
            padding: '24px',
            '& > *': { maxWidth: '1440px' },
            '& h2:not(:where(.duplicate-studio *, .storage-explorer *))': {
              fontSize: '18px',
              margin: '24px 0 12px',
            },
            '& h3:not(:where(.duplicate-studio *, .storage-explorer *))': { fontSize: '16px' },
            '& p:not(:where(.duplicate-studio *, .storage-explorer *))': {
              color: '{colors.fg.muted}',
            },
            '&\n  label:not(:where(.duplicate-studio *, .storage-explorer *)):not(.inspection-choice-card):not(\n    .inspection-choice-label\n  )':
              {
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'stretch',
                gap: '8px',
                margin: '12px 0',
                fontSize: '13px',
              },
            "&\n  label:not(:where(.duplicate-studio *, .storage-explorer *)):has(> input[type='checkbox'])":
              { flexDirection: 'row', alignItems: 'center' },
            "&\n  input:not([type='checkbox']):not([type='radio']):not(\n    :where(.duplicate-studio *, .storage-explorer *)\n  )":
              {
                padding: '7px 10px',
                maxWidth: '100%',
                color: 'inherit',
                background: '{colors.bg}',
                border: '1px solid {colors.border}',
                borderRadius: 'var(--control-radius)',
              },
            '& select:not(:where(.duplicate-studio *, .storage-explorer *))': {
              padding: '7px 10px',
              maxWidth: '100%',
              color: 'inherit',
              background: '{colors.bg}',
              border: '1px solid {colors.border}',
              borderRadius: 'var(--control-radius)',
            },
            '@media (max-width: 800px)': { padding: '16px' },
            '& :is(button, input, summary):focus-visible': {
              outline: '2px solid {colors.brand.solid}',
              outlineOffset: '3px',
            },
            "& input[type='checkbox']": { accentColor: '{colors.brand.solid}' },
            '& ::selection': { background: '{colors.brand.solid}', color: '{colors.bg}' },
            '@media (max-width: 760px)': { '&:has(.duplicate-controls)': { padding: '20px 16px' } },
          }}
        >
          {children}
        </Box>
        {footer && <chakra.footer className="inspection-dialog-footer">{footer}</chakra.footer>}
      </dialog>
    </Box>
  );
}
