import { useSyncExternalStore } from 'react';
import type { CatalogRestorePrepared } from '../bindings';

type Context = { catalogId: string; restoreJobId: string; reviewDigest: string };
export type CatalogSwitchState = Context & { ticket: number } & (
    | { phase: 'preparing' }
    | { phase: 'prepared'; prepared: CatalogRestorePrepared }
    | { phase: 'unconfirmed'; message: string }
  );

// Process-local display state. Native admission and sealed startup records remain authoritative.
export function createCatalogSwitchStore() {
  let value: CatalogSwitchState | null = null;
  let nextTicket = 0;
  const listeners = new Set<() => void>();
  function publish(next: CatalogSwitchState) {
    value = next;
    for (const listener of listeners) listener();
  }
  return {
    getSnapshot: () => value,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    begin(context: Context) {
      if (value)
        throw new Error('This catalog session requires closing and reopening before more work.');
      const ticket = ++nextTicket;
      publish({ ...context, ticket, phase: 'preparing' });
      return ticket;
    },
    prepared(ticket: number, prepared: CatalogRestorePrepared) {
      if (!value || value.ticket !== ticket || value.phase !== 'preparing') return;
      if (
        prepared.catalogId !== value.catalogId ||
        !prepared.transactionId ||
        !prepared.currentCatalogRetained ||
        !prepared.restartRequired
      )
        throw new Error('Catalog preparation did not confirm preservation and a pending restart.');
      publish({ ...value, phase: 'prepared', prepared });
    },
    unconfirmed(ticket: number, message: string) {
      if (!value || value.ticket !== ticket || value.phase !== 'preparing') return;
      publish({ ...value, phase: 'unconfirmed', message });
    },
  };
}
export const catalogSwitchStore = createCatalogSwitchStore();
export function useCatalogSwitchState() {
  return useSyncExternalStore(catalogSwitchStore.subscribe, catalogSwitchStore.getSnapshot);
}
