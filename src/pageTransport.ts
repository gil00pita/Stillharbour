import type { AssetPage, PackedAssetPage } from './bindings';

/** Decode the versioned native page once, keeping existing UI/selection models intact. */
export function unpackPage(page: PackedAssetPage): AssetPage {
  if (page.format !== 1 || page.rows.length > 240 || page.rows.some((row) => row.length !== 27))
    throw new Error('Unsupported catalog page. Reopen Stillharbour.');
  if (
    page.startIndex !== undefined &&
    (!Number.isInteger(page.startIndex) || page.startIndex < 0 || page.startIndex > 0xffffffff)
  )
    throw new Error('Unsupported catalog page position. Reopen Stillharbour.');
  return {
    ...(page.startIndex === undefined ? {} : { startIndex: page.startIndex }),
    nextCursor: page.nextCursor,
    total: page.total,
    revision: page.revision,
    assets: page.rows.map((row) => {
      const [
        id,
        rootId,
        name,
        relativePath,
        folder,
        extension,
        detectedFormat,
        bytes,
        date,
        dateSource,
        rawDate,
        offset,
        modifiedAt,
        bornAt,
        addedAt,
        revision,
        presence,
        width,
        height,
        camera,
        metadataState,
        metadataError,
        previewState,
        previewError,
        favorite,
        rating,
        tags,
      ] = row;
      return {
        id,
        rootId,
        name,
        relativePath,
        folder,
        extension,
        detectedFormat,
        bytes,
        date,
        dateSource,
        rawDate,
        offset,
        modifiedAt,
        bornAt,
        addedAt,
        revision,
        presence,
        width,
        height,
        camera,
        metadataState,
        metadataError,
        previewState,
        previewError,
        favorite,
        rating,
        tags,
      };
    }),
  };
}
