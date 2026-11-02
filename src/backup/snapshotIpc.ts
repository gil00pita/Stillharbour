import type { SnapshotReply, SnapshotRequest } from '../bindings';
import { libraryData } from '../ipc';

type ReplyData<K extends SnapshotReply['kind']> = Extract<SnapshotReply, { kind: K }>['data'];
export async function snapshotRequest<K extends SnapshotReply['kind']>(
  request: SnapshotRequest,
  kind: K,
): Promise<ReplyData<K>> {
  const reply = await libraryData({ kind: 'backup_snapshots', request }, 'backup_snapshots');
  if (reply.kind !== kind) throw new Error('Unexpected snapshot response. Refresh the profile.');
  return reply.data as ReplyData<K>;
}
