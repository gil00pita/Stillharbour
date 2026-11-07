import { Box, Heading, Text, chakra } from '@chakra-ui/react';
import type { CatalogSwitchState } from '../../backup/catalogSwitchState';

export function CatalogSwitchStopped({ state }: { state: CatalogSwitchState }) {
  return (
    <chakra.main
      className="catalog-switch-stopped"
      css={{
        maxWidth: '880px',
        minHeight: '100vh',
        marginInline: 'auto',
        padding: '48px 32px',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        gap: '20px',
        overflowWrap: 'anywhere',
      }}
      aria-label="Catalog switch session"
    >
      <Heading as="h1" unstyled>
        {state.phase === 'preparing'
          ? 'Preparing the catalog switch'
          : state.phase === 'prepared'
            ? 'Catalog switch awaits restart'
            : 'Catalog preparation was not confirmed'}
      </Heading>
      <Text>Backup, library changes and further requests are unavailable in this session.</Text>
      {state.phase === 'preparing' ? (
        <Text role="status">
          Native preparation is in progress. No prepared switch or catalog activation has been
          confirmed.
        </Text>
      ) : state.phase === 'prepared' ? (
        <Box role="status">
          <Text>{state.prepared.message}</Text>
          <Text>
            The current catalog is retained. The restored catalog is pending startup validation; it
            is not confirmed active.
          </Text>
          <Text>
            Transaction: <code>{state.prepared.transactionId}</code>
          </Text>
        </Box>
      ) : (
        <Text role="alert">{state.message}</Text>
      )}
      <Text>
        Close and reopen Stillharbour to reconcile this catalog switch before continuing. Startup
        may reject an invalid or incomplete switch; it does not silently create an empty catalog.
      </Text>
      <Text>
        {state.sourceKind === 'retained_pre_restore' ? (
          <>
            Source: Local catalog saved before switch <code>{state.retainedTransactionId}</code>
          </>
        ) : (
          <>
            Source: Verified snapshot restore <code>{state.restoreJobId}</code>
          </>
        )}
      </Text>
      <Text>
        Catalog UUID: <code>{state.catalogId}</code>
      </Text>
    </chakra.main>
  );
}
