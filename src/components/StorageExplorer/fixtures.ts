// Generated test data only. Never imported by the production application.
import type { InspectionFinding, LibraryRoot, StorageSummary } from '../../bindings';
import { fixturePreferences } from '../../library/__tests__/fixtures';

export const storageRoot: LibraryRoot = {
  id: 'root',
  label: 'Generated library',
  displayPath: '/generated',
  mode: 'referenced',
  isBase: false,
  availability: 'online',
  filter: fixturePreferences().scanFilter,
  lastScan: null,
  fileCount: 3,
};
export function storageFile(id: string, patch: Partial<InspectionFinding> = {}): InspectionFinding {
  return {
    id,
    assetId: null,
    rootId: 'root',
    folderId: 'folder',
    name: `${id}.txt`,
    displayPath: `/generated/Projects/${id}.txt`,
    bytes: '1000000',
    reportedAllocatedBytes: null,
    objectId: id,
    links: '1',
    isMedia: false,
    inventory: 'present',
    content: 'not_checked',
    checksum: 'not_checked',
    comparisonName: null,
    filesystemModified: null,
    filesystemCreated: null,
    readonly: false,
    baselineId: null,
    decoderPolicy: null,
    pixelSignature: null,
    pixelDigest: null,
    width: null,
    height: null,
    format: null,
    coverage: 'Generated observations',
    error: null,
    ...patch,
  };
}
export const storageFiles = [
  storageFile('video', {
    name: 'Holiday.mov',
    displayPath: '/generated/Trips/Holiday.mov',
    bytes: '9600000000',
    isMedia: true,
    assetId: 'asset',
    filesystemModified: '2020-01-01T00:00:00Z',
  }),
  storageFile('notes'),
  storageFile('empty', { bytes: '0', readonly: true }),
];
export const storageSummary: StorageSummary = {
  total: {
    id: 'scope',
    parentId: null,
    rootId: null,
    name: 'Selected scope',
    files: 3,
    directFiles: 0,
    directLogicalBytes: '0',
    logicalBytes: '9601000000',
    uniqueObjectLogicalBytes: null,
    reportedAllocatedBytes: null,
  },
  potentialDuplicateLogicalBytes: null,
  reclaimablePhysicalBytes: null,
  quarantineFreedBytes: '0',
  accountingIncomplete: false,
};
