import { Box, Heading } from '@chakra-ui/react';
import type { ReactNode } from 'react';
import type { LibraryStatus } from '../../bindings';
import { PageHeader } from '@/components/PageHeader';
import { SnapshotWorkflow } from '@/components/SnapshotWorkflow';
import { useCatalogSwitchState } from '../../backup/catalogSwitchState';
import { CatalogSwitchStopped } from '@/components/CatalogSwitchStopped';
import '../../backup/backup.css';

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
    <Box className="backup-page">
      <PageHeader>
        <Box as="header" className="toolbar">
          {navigationControl}
          <Heading as="h1">Backup</Heading>
          <Box className="header-actions">{jobsControl}</Box>
        </Box>
      </PageHeader>
      <SnapshotWorkflow catalogId={catalogId} status={status} onActivityChange={ignoreActivity} />
    </Box>
  );
}
