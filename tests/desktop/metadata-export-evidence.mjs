import assert from 'node:assert/strict';

export function assertPreservedXmpExport(audit, review) {
  const files = review.files.map((expected) => {
    const matches = audit.exports.filter((file) => file.name === expected.name);
    assert.equal(matches.length, 1, `One audited output for ${expected.name}`);
    const [file] = matches;
    assert.equal(file.bytes, expected.bytes, `Reviewed size for ${expected.name}`);
    assert.equal(String(Buffer.byteLength(file.detail.text)), expected.bytes);
    assert.equal(file.digest, expected.digest, `Reviewed digest for ${expected.name}`);
    return file;
  });
  const preserved = files.filter(
    (file) => file.name.endsWith('.xmp') && file.detail.text.includes('Retain this unknown field'),
  );
  assert.equal(preserved.length, 2, `Reviewed export ${review.id} preserves both XMP packets`);
}
