// Local diagnostic builds only. Export numeric observations, never request data or URLs.
type Metric =
  | 'ipc_ms'
  | 'ipc_request_bytes'
  | 'ipc_response_bytes'
  | 'layout_ms'
  | 'react_render_ms'
  | 'raf_interval_ms'
  | 'image_load_ms'
  | 'image_load_to_raf_ms'
  | 'mounted_items'
  | 'resident_summaries';
type Event = { metric: Metric; value: number; tMs: number };
const capacity = 4096;
const allowed = new Set<Metric>([
  'ipc_ms',
  'ipc_request_bytes',
  'ipc_response_bytes',
  'layout_ms',
  'react_render_ms',
  'raf_interval_ms',
  'image_load_ms',
  'image_load_to_raf_ms',
  'mounted_items',
  'resident_summaries',
]);
let active = false;
let startedAt = 0;
let events: Event[] = [];
let dropped = 0;
let stopObservers: (() => void) | undefined;
export function diagnosticClock(): number | undefined {
  return import.meta.env.MODE === 'desktop-test' && active ? performance.now() : undefined;
}
export function diagnosticValue(metric: Metric, value: number) {
  if (
    import.meta.env.MODE !== 'desktop-test' ||
    !active ||
    !allowed.has(metric) ||
    !Number.isFinite(value) ||
    value < 0
  )
    return;
  if (events.length < capacity) events.push({ metric, value, tMs: performance.now() - startedAt });
  else dropped++;
}
export function diagnosticSince(metric: Metric, start: number | undefined) {
  if (start !== undefined) diagnosticValue(metric, performance.now() - start);
}
export function diagnosticPayload(
  metric: 'ipc_request_bytes' | 'ipc_response_bytes',
  value: unknown,
) {
  if (diagnosticClock() === undefined) return;
  // Count a transient UTF-8 serialization; never retain its contents.
  diagnosticValue(metric, new TextEncoder().encode(JSON.stringify(value)).byteLength);
}
export function installDiagnostics() {
  if (import.meta.env.MODE !== 'desktop-test') return;
  Object.assign(window, {
    __stillharbourPerf: {
      start() {
        stopObservers?.();
        active = true;
        startedAt = performance.now();
        events = [];
        dropped = 0;
        let frame = 0,
          previous = performance.now();
        const seen = new WeakMap<HTMLImageElement, number>();
        function track(node: Node) {
          if (!(node instanceof Element)) return;
          for (const image of node.matches('img') ? [node] : node.querySelectorAll('img'))
            if (image instanceof HTMLImageElement && !seen.has(image))
              seen.set(image, performance.now());
        }
        function tick(now: number) {
          diagnosticValue('raf_interval_ms', now - previous);
          previous = now;
          frame = requestAnimationFrame(tick);
        }
        function loaded(event: globalThis.Event) {
          if (!(event.target instanceof HTMLImageElement)) return;
          const started = seen.get(event.target);
          if (started !== undefined) diagnosticSince('image_load_ms', started);
          const load = performance.now();
          requestAnimationFrame(() =>
            requestAnimationFrame(() => diagnosticSince('image_load_to_raf_ms', load)),
          );
        }
        const observer = new MutationObserver((changes) => {
          for (const change of changes) {
            for (const node of change.addedNodes) track(node);
            if (change.type === 'attributes' && change.target instanceof HTMLImageElement)
              seen.set(change.target, performance.now());
          }
          diagnosticValue('mounted_items', document.querySelectorAll('[data-asset-id]').length);
          const count = Number(
            document.querySelector<HTMLElement>('.collection-main')?.dataset.feedCount ?? 0,
          );
          diagnosticValue('resident_summaries', count);
        });
        observer.observe(document.body, {
          childList: true,
          subtree: true,
          attributes: true,
          attributeFilter: ['src'],
        });
        track(document.body);
        frame = requestAnimationFrame(tick);
        document.addEventListener('load', loaded, true);
        stopObservers = () => {
          cancelAnimationFrame(frame);
          observer.disconnect();
          document.removeEventListener('load', loaded, true);
        };
      },
      stop() {
        active = false;
        stopObservers?.();
        stopObservers = undefined;
      },
      read() {
        return { events: events.slice(), dropped, capacity };
      },
      drain() {
        const segment = { events, dropped, capacity };
        events = [];
        dropped = 0;
        return segment;
      },
    },
  });
}
