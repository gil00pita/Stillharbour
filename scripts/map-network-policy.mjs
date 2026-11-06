import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const provider = JSON.parse(
  readFileSync(new URL('../assets/maps/satellite-provider.json', import.meta.url)),
);

export function assertMapNetworkPolicy(security) {
  const prefix =
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: thumbnail: http://thumbnail.localhost original: http://original.localhost ";
  const suffix = "; object-src 'none'; base-uri 'none'; frame-src 'none'; form-action 'none'";
  const common = `${prefix}${provider.tileBase}; connect-src ipc: http://ipc.localhost ${provider.tileBase}`;
  assert.equal(
    security.csp,
    common + suffix,
    'Only the reviewed NASA tile path may be added to production CSP',
  );
  assert.equal(
    security.devCsp,
    common + ' http://127.0.0.1:* ws://127.0.0.1:*' + suffix,
    'Development CSP must retain the same NASA restriction',
  );
}
