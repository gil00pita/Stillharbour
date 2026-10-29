import { expect, it } from 'vitest';
import { GALLERY_DECODED_BYTES } from '../bindings';
import { fixtureAsset } from './__tests__/fixtures';
import { galleryPreviewKeys, galleryRenditions, pixelRatio } from './renditionPlan';
import type { PhotoRow } from './model';

function rows(count: number, width = 300, height = 300): PhotoRow[] {
  return Array.from({ length: count }, (_, index) => ({
    assets: [{ ...fixtureAsset(String(index)), width: 3000, height: 3000 }],
    widths: [width],
    height,
    imageHeight: height,
    headings: [],
  }));
}
it('uses rendered size and DPR, accounts for panorama crop, and caps gallery tiers', () => {
  expect(galleryRenditions(rows(1), 1, false).get('0')).toBe(512);
  expect(galleryRenditions(rows(1), 2, false).get('0')).toBe(1024);
  const panorama = rows(1);
  panorama[0]!.assets[0]!.width = 12000;
  expect(galleryRenditions(panorama, 1, false).get('0')).toBe(512);
  expect(galleryRenditions(panorama, 1, true).get('0')).toBe(1024);
  expect(galleryRenditions(rows(1, 3000, 3000), 3, false).get('0')).toBe(1024);
  expect([NaN, Infinity, 0, 9].map(pixelRatio)).toEqual([1, 1, 1, 3]);
});
it('admits at most240 images and bounds decoded-pixel estimates rather than allocating all high-DPI tiers', () => {
  const plan = galleryRenditions(rows(300), 3, false);
  expect(plan.size).toBe(240);
  const bytes = [...plan.values()].reduce((sum, edge) => sum + edge * edge * 4 + 32 * 32 * 4, 0);
  expect(bytes).toBeLessThanOrEqual(GALLERY_DECODED_BYTES);
  expect(new Set(plan.values())).toContain(256);
  expect(new Set(plan.values())).not.toContain(2048);
});

it('includes clear cache lookup keys while retaining bounded visible-first admission', () => {
  const assets = Array.from({ length: 300 }, (_, i) => fixtureAsset(String(i)));
  const keys = galleryPreviewKeys(assets, new Map([['0', 1024]]));
  expect(keys).toHaveLength(480);
  expect(keys.slice(0, 3).map((key) => [key.assetId, key.edge])).toEqual([
    ['0', 32],
    ['0', 1024],
    ['1', 32],
  ]);
});
