import { mkdir, mkdtemp } from 'node:fs/promises';
import { join } from 'node:path';

export async function createEvidenceDirectory(parent) {
  // A fresh checkout has no ignored evidence folders. Prepare them before
  // polling so setup errors surface directly instead of becoming timeouts.
  await mkdir(parent, { recursive: true });
  return mkdtemp(join(parent, 'native-'));
}
