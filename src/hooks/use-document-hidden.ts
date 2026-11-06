import { useSyncExternalStore } from 'react';
function subscribe(changed: () => void) {
  document.addEventListener('visibilitychange', changed);
  return () => document.removeEventListener('visibilitychange', changed);
}
export function useDocumentHidden() {
  return useSyncExternalStore(
    subscribe,
    () => document.visibilityState === 'hidden',
    () => true,
  );
}
