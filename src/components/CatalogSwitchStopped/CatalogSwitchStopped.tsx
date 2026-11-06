import type { CatalogSwitchState } from '../../backup/catalogSwitchState';

export function CatalogSwitchStopped({ state }: { state: CatalogSwitchState }) {
  return (
    <main className="catalog-switch-stopped" aria-label="Catalog switch session">
      <h1>
        {state.phase === 'preparing'
          ? 'Preparing the catalog switch'
          : state.phase === 'prepared'
            ? 'Catalog switch awaits restart'
            : 'Catalog preparation was not confirmed'}
      </h1>
      <p>Backup, library changes and further requests are unavailable in this session.</p>
      {state.phase === 'preparing' ? (
        <p role="status">
          Native preparation is in progress. No prepared switch or catalog activation has been
          confirmed.
        </p>
      ) : state.phase === 'prepared' ? (
        <div role="status">
          <p>{state.prepared.message}</p>
          <p>
            The current catalog is retained. The restored catalog is pending startup validation; it
            is not confirmed active.
          </p>
          <p>
            Transaction: <code>{state.prepared.transactionId}</code>
          </p>
        </div>
      ) : (
        <p role="alert">{state.message}</p>
      )}
      <p>
        Close and reopen Stillharbour to reconcile this catalog switch before continuing. Startup
        may reject an invalid or incomplete switch; it does not silently create an empty catalog.
      </p>
      <p>
        {state.sourceKind === 'retained_pre_restore' ? (
          <>
            Source: Local catalog saved before switch <code>{state.retainedTransactionId}</code>
          </>
        ) : (
          <>
            Source: Verified snapshot restore <code>{state.restoreJobId}</code>
          </>
        )}
      </p>
      <p>
        Catalog UUID: <code>{state.catalogId}</code>
      </p>
    </main>
  );
}
