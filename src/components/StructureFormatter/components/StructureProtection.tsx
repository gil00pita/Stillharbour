import { Box, Heading, Text } from '@chakra-ui/react';
import { useEffect, useState } from 'react';
import { Button } from '@/components/Button';
import type {
  OrganizationProtectionBlocker,
  OrganizationProtectionStatus,
} from '../../../bindings';
import { library } from '../../../ipc';

const messages: Record<OrganizationProtectionBlocker, string> = {
  backup_active: 'Wait for the active backup, restore or recovery verification to finish.',
  sync_active: 'Wait for the active sync or folder comparison to finish.',
  backup_recovery_required: 'Review unfinished backup jobs and retained partial outputs in Backup.',
  sync_recovery_required: 'Finish or resolve paused and interrupted jobs in Sync.',
  backup_scheduled: 'Turn off enabled backup schedules in Backup before restructuring.',
};

export function StructureProtection() {
  const [status, setStatus] = useState<OrganizationProtectionStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let closed = false;
    let pending = false;
    async function refresh(force = false) {
      if (pending || (!force && document.visibilityState === 'hidden')) return;
      pending = true;
      try {
        const response = await library({ kind: 'organization_protection_status' });
        if (response.kind !== 'organization_protection_status') throw new Error('Unexpected reply');
        if (!closed) {
          setStatus(response.data);
          setFailed(false);
        }
      } catch {
        if (!closed) {
          setStatus(null);
          setFailed(true);
        }
      } finally {
        pending = false;
      }
    }
    void refresh(true);
    const interval = window.setInterval(() => void refresh(), 2000);
    const onFocus = () => void refresh(true);
    window.addEventListener('focus', onFocus);
    document.addEventListener('visibilitychange', onFocus);
    return () => {
      closed = true;
      window.clearInterval(interval);
      window.removeEventListener('focus', onFocus);
      document.removeEventListener('visibilitychange', onFocus);
    };
  }, [attempt]);
  return (
    <Box as="section" aria-labelledby="structure-protection-title" mt="5" p="4" bg="bg.subtle">
      <Heading as="h3" fontSize="sm" id="structure-protection-title" mb="2">
        Backup &amp; sync check
      </Heading>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
        id="structure-protection-help"
      >
        Restructuring requires backup and sync to be idle, with no enabled future schedules or
        unresolved jobs. You can keep configuring rules and previewing the library.
      </Text>
      <Box aria-live="polite" aria-atomic="true" mt="2">
        {failed ? (
          <Text>Could not verify backup and sync. Restructuring stays blocked. Check again.</Text>
        ) : status ? (
          status.blockers.length ? (
            <Box as="ul" ps="5">
              {status.blockers.map((blocker) => (
                <Box as="li" key={blocker}>
                  {messages[blocker]}
                </Box>
              ))}
            </Box>
          ) : (
            <Text>
              No active backup or sync, unresolved jobs or enabled backup schedules found.
            </Text>
          )
        ) : (
          <Text>Checking backup, sync and schedules…</Text>
        )}
      </Box>
      <Box mt="3">
        <Button onClick={() => setAttempt((value) => value + 1)}>Check again</Button>
      </Box>
    </Box>
  );
}
