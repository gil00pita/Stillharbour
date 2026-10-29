import { once } from 'node:events';

// Register before the child can exit, including before any asynchronous setup.
export function samplerStop(child) {
  const closed = once(child, 'close');
  // Retain a spawn error for teardown without an unhandled rejection in the route.
  closed.catch(() => {});
  return async () => {
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
    await closed;
  };
}
