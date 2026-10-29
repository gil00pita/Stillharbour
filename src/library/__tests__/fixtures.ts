import type { AssetSummary } from '../../bindings';

export function fixtureAsset(id: string, date = '2026-03-01T12:00:00'): AssetSummary {
  return {
    id,
    rootId: 'generated-root',
    name: `Generated ${id}.jpg`,
    relativePath: `${id}.jpg`,
    folder: '',
    extension: 'jpg',
    detectedFormat: 'JPEG',
    bytes: '1500',
    date,
    dateSource: 'exif_original',
    rawDate: date,
    offset: null,
    modifiedAt: null,
    bornAt: null,
    addedAt: '2026-10-27',
    revision: 'fixture-revision',
    presence: 'present',
    width: 600,
    height: 400,
    camera: null,
    metadataState: 'ready',
    metadataError: null,
    previewState: 'pending',
    previewError: null,
    favorite: false,
    rating: 0,
    tags: [],
  };
}

export function fixturePreferences(): import('../../bindings').AppPreferences {
  return {
    version: 1,
    defaultRootId: null,
    rootMode: 'managed',
    scanFilter: {
      version: 1,
      includeGlobs: [],
      excludeGlobs: [],
      excludedFolders: [],
      extensions: ['jpg'],
      includeHiddenFiles: false,
      includeHiddenFolders: false,
      minBytes: null,
      maxBytes: null,
      fromDate: null,
      throughDate: null,
    },
    galleryView: 'gallery',
    thumbnailSize: 'comfortable',
    gridFit: 'contain',
    sort: 'capture',
    navigationCollapsed: false,
  };
}
