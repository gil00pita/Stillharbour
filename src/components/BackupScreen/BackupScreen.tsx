import { Box, Heading } from '@chakra-ui/react';
import type { ReactNode } from 'react';
import type { LibraryStatus } from '../../bindings';
import { PageHeader } from '@/components/PageHeader';
import { SnapshotWorkflow } from '@/components/SnapshotWorkflow';
import { useCatalogSwitchState } from '../../backup/catalogSwitchState';
import { CatalogSwitchStopped } from '@/components/CatalogSwitchStopped';

const ignoreActivity = () => {};

export function BackupScreen({
  status,
  catalogId = null,
  navigationControl,
  jobsControl,
}: {
  status: LibraryStatus;
  catalogId?: string | null;
  navigationControl?: ReactNode;
  jobsControl?: ReactNode;
}) {
  const stopped = useCatalogSwitchState();
  if (stopped) return <CatalogSwitchStopped state={stopped} />;
  return (
    <Box
      className="backup-page"
      css={{ display: 'flex', flexDirection: 'column', minHeight: '0', height: '100%' }}
    >
      <PageHeader>
        <Box
          as="header"
          className="toolbar"
          css={{
            flexShrink: '0',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '16px',
            minHeight: '64px',
            padding: '12px 24px',
            borderBottom: '1px solid {colors.border}',
            background: '{colors.bg.toolbar}',
            '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
          }}
        >
          {navigationControl}
          <Heading as="h1">Backup</Heading>
          <Box
            className="header-actions"
            css={{ display: 'flex', alignItems: 'center', gap: '8px' }}
          >
            {jobsControl}
          </Box>
        </Box>
      </PageHeader>
      <SnapshotWorkflow catalogId={catalogId} status={status} onActivityChange={ignoreActivity} />
    </Box>
  );
}
