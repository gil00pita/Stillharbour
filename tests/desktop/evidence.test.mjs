import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { createEvidenceDirectory } from './evidence.mjs';

it('writes receipts with missing checkout parents and preserves earlier run evidence', async () => {
  const owned = await mkdtemp(join(tmpdir(), 'stillharbour-evidence-test-'));
  try {
    const parent = join(owned, '.context', 'windows-directory-selection');
    const first = await createEvidenceDirectory(parent);
    const receipt = join(first, 'windows-status.json');
    await writeFile(receipt, JSON.stringify({ state: 'complete' }));
    const second = await createEvidenceDirectory(parent);
    expect(second).not.toBe(first);
    expect(JSON.parse(await readFile(receipt, 'utf8'))).toEqual({ state: 'complete' });
    await writeFile(join(second, 'windows-source-verification.json'), '{}');
  } finally {
    // Only the exact temporary directory returned to this test is removed.
    await rm(owned, { recursive: true, force: true });
  }
});

it('reports evidence setup failure before polling starts', async () => {
  const owned = await mkdtemp(join(tmpdir(), 'stillharbour-evidence-test-'));
  try {
    const blocker = join(owned, 'not-a-directory');
    await writeFile(blocker, 'preserve');
    await expect(createEvidenceDirectory(join(blocker, 'evidence'))).rejects.toThrow();
    expect(await readFile(blocker, 'utf8')).toBe('preserve');
  } finally {
    await rm(owned, { recursive: true, force: true });
  }
});
