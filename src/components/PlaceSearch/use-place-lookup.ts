import { useEffect, useState } from 'react';
import type { PlaceSearchReply } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

export function usePlaceLookup(open: boolean, text: string, selectedId: string | null | undefined) {
  const [attempt, setAttempt] = useState(0);
  const key = JSON.stringify([open ? text : '', selectedId ?? null]);
  const [response, setResponse] = useState<{ key: string; data: PlaceSearchReply } | null>(null);
  const [failure, setFailure] = useState<{ key: string; message: string } | null>(null);
  useEffect(() => {
    if (!open && !selectedId) return;
    let active = true;
    const timer = setTimeout(() => {
      void libraryData(
        { kind: 'places', text: open ? text : '', selected_id: selectedId ?? null },
        'places',
      )
        .then((data) => {
          if (active) {
            setResponse({ key, data });
            setFailure(null);
          }
        })
        .catch((error: unknown) => {
          if (active) setFailure({ key, message: errorMessage(error) });
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [open, text, selectedId, key, attempt]);
  const candidate = response?.data.selected;
  return {
    current: response?.key === key ? response.data : null,
    error: failure?.key === key ? failure.message : null,
    selected: candidate?.id === selectedId ? candidate : null,
    retry: () => setAttempt((value) => value + 1),
  };
}
