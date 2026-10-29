import { describe, expect, it } from 'vitest';
import golden from '../tests/fixtures/catalog-page-v1.json?raw';
import type { AssetPage, PackedAssetPage } from './bindings';
import { unpackPage } from './pageTransport';

const fixture: { page: AssetPage; packed: PackedAssetPage } = JSON.parse(golden);
describe('native page wire format', () => {
  it('decodes the Rust-verified v1 fixture without changing table/viewer data', () => {
    expect(unpackPage(fixture.packed)).toEqual(fixture.page);
    expect(unpackPage(fixture.packed).assets[0]?.bytes).toBe('9007199254740993');
  });
  it('rejects unknown formats and unbounded pages', () => {
    expect(() => unpackPage({ ...fixture.packed, format: 2 })).toThrow('Unsupported catalog page');
    expect(() =>
      unpackPage({ ...fixture.packed, rows: Array(241).fill(fixture.packed.rows[0]) }),
    ).toThrow('Unsupported catalog page');
    const malformed = structuredClone(fixture.packed);
    malformed.rows[0]!.pop();
    expect(() => unpackPage(malformed)).toThrow('Unsupported catalog page');
  });
  it('retains explicit nulls and empty pages', () => {
    const row = structuredClone(fixture.packed.rows[0]!);
    row[6] = null;
    row[17] = null;
    expect(unpackPage({ ...fixture.packed, rows: [row] }).assets[0]).toMatchObject({
      detectedFormat: null,
      width: null,
    });
    expect(unpackPage({ ...fixture.packed, rows: [], nextCursor: null, total: 0 }).assets).toEqual(
      [],
    );
  });
});
