import { IconButton, Portal, Stack, Toast, Toaster, createToaster } from '@chakra-ui/react';
import { useEffect, useState } from 'react';
import { Button } from '../ui/controls';

type Props = {
  error: string | null;
  onDismiss: () => void;
  onReconnect?: (() => void) | undefined;
};

/** Chakra's toaster composition, scoped to this application session. */
export function ErrorToast({ error, onDismiss, onReconnect }: Props) {
  const [toaster] = useState(() =>
    createToaster({ placement: 'bottom', max: 1, duration: Infinity, offsets: '16px' }),
  );
  useEffect(() => {
    if (!error) return;
    const id = toaster.create({
      id: 'application-error',
      type: 'error',
      title: 'Request failed',
      description: error,
      duration: Infinity,
      removeDelay: 0,
      closable: true,
      action: onReconnect
        ? {
            label: 'Reconnect',
            onClick: () => {
              toaster.remove('application-error');
              onReconnect();
            },
          }
        : undefined,
    });
    return () => toaster.remove(id);
  }, [error, onReconnect, toaster]);
  function dismiss() {
    toaster.remove('application-error');
    onDismiss();
  }

  return (
    <Portal>
      <Toaster toaster={toaster} aria-live="off">
        {(toast) => (
          <Toast.Root
            role="alert"
            aria-live="assertive"
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                event.preventDefault();
                event.stopPropagation();
                dismiss();
              }
            }}
          >
            <Toast.Indicator />
            <Stack gap="2" minWidth="0" align="start">
              <Toast.Title>{toast.title}</Toast.Title>
              <Toast.Description tabIndex={0}>{toast.description}</Toast.Description>
              {toast.action && (
                <Toast.ActionTrigger asChild>
                  <Button>{toast.action.label}</Button>
                </Toast.ActionTrigger>
              )}
            </Stack>
            <Toast.CloseTrigger asChild>
              <IconButton aria-label="Dismiss error" onClick={dismiss} variant="outline">
                <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="m6 6 12 12M6 18 18 6" stroke="currentColor" strokeWidth="1.5" />
                </svg>
              </IconButton>
            </Toast.CloseTrigger>
          </Toast.Root>
        )}
      </Toaster>
    </Portal>
  );
}
