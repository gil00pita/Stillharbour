# ADR-0017 — Page-scoped photo viewing and bounded cache transport retry

Date: 2026-10-30. Status: accepted for the owner's M1 viewer request.

Replace the single-photo viewer's whole-window showModal presentation with an open nonmodal HTML dialog inside the Photos/Tags or Folders page. The covered underlay is inert; the sidebar stays usable. Focus the X, preserve Escape/Info behavior and filtered navigation, and restore the original gallery position on close. Native Compare retains its existing separately invoked presentation.

Reuse the source thumbnail and existing progressive preview pipeline. A fitted frame uses measured thumbnail-to-viewer translation/scale on entry and the visible current tile on exit. Fall back locally when no valid tile exists; Reduced Motion/hidden windows skip spatial animation. Animate only transform/opacity with bounded cleanup and keep the effect clipped to the page. A loading-only glare and textual status end on actual image readiness or terminal error; hidden/reduced-motion states remain explicit. No animation framework is introduced.

A confirmed frontend image-error latch motivates a shared one-attempt owned-cache transport retry. Exhaustion keeps existing smaller/error states; the gallery offers explicit native revalidation through its current bounded queue. Clear old ready results before explicit retry so an already-evicted image is not immediately redisplayed. Do not confuse this with original integrity or repair. Native cache-error responses are non-storable; successful immutable revision/policy responses are unchanged.

No catalog/schema/identity/dependency migration, new IPC command, permission, original mutation or automatic cache deletion is included. [Viewer requirements and evidence](../VIEWER_PRESENTATION.md) supersede ADR-0003's whole-window single-photo modality while preserving its read-only queue, navigation and focus guarantees.
