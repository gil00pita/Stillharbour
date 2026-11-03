import { createHash } from 'node:crypto';
import { expect, it } from 'vitest';
import { assertPreservedXmpExport } from './metadata-export-evidence.mjs';

function fixture(id) {
  const exports = [
    { name: `${id}-original-0.xmp`, detail: { text: 'Retain this unknown field · 港' } },
    { name: `${id}.xmp`, detail: { text: 'Merged: Retain this unknown field · 港' } },
    { name: `${id}-manifest.json`, detail: { text: '{}' } },
  ].map((file) => ({
    ...file,
    bytes: String(Buffer.byteLength(file.detail.text)),
    digest: createHash('sha256').update(file.detail.text).digest('hex'),
  }));
  return {
    exports,
    review: {
      id,
      files: exports.map((file) => ({
        name: file.name,
        bytes: file.bytes,
        digest: file.digest,
      })),
    },
  };
}

it('checks the reviewed pair when the same sidecar was exported twice', () => {
  const first = fixture('ui-selected-zero');
  const retained = fixture('explicit-zero');
  const audit = { exports: [...first.exports, ...retained.exports].reverse() };
  expect(() => assertPreservedXmpExport(audit, retained.review)).not.toThrow();
});

it('does not substitute another export for missing reviewed packets', () => {
  const first = fixture('ui-selected-zero');
  const retained = fixture('explicit-zero');
  const audit = { exports: [...first.exports, ...retained.exports.slice(2)] };
  expect(() => assertPreservedXmpExport(audit, retained.review)).toThrow();
});

it('rejects duplicate evidence for a reviewed file', () => {
  const retained = fixture('explicit-zero');
  expect(() =>
    assertPreservedXmpExport(
      { exports: [...retained.exports, retained.exports[2]] },
      retained.review,
    ),
  ).toThrow();
});

it.each([0, 1])('rejects lost unknown content in reviewed packet %s', (index) => {
  const retained = fixture('explicit-zero');
  const file = retained.exports[index];
  file.detail.text = file.detail.text.replace('Retain', 'Remove');
  file.digest = createHash('sha256').update(file.detail.text).digest('hex');
  retained.review.files[index].digest = file.digest;
  expect(() => assertPreservedXmpExport(retained, retained.review)).toThrow();
});

it('rejects changed output even if unknown content remains', () => {
  const retained = fixture('explicit-zero');
  retained.exports[1].detail.text = retained.exports[1].detail.text.replace('Merged', 'Edited');
  retained.exports[1].bytes = String(Buffer.byteLength(retained.exports[1].detail.text));
  retained.exports[1].digest = createHash('sha256')
    .update(retained.exports[1].detail.text)
    .digest('hex');
  expect(() => assertPreservedXmpExport(retained, retained.review)).toThrow();
});

it('rejects an absent reviewed manifest even when both packets remain', () => {
  const retained = fixture('explicit-zero');
  retained.exports.pop();
  expect(() => assertPreservedXmpExport(retained, retained.review)).toThrow();
});
