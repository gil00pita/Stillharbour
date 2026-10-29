# P2 — Bounded gallery and rendition follow-through

The owner's 2026-10-29 “start next stage” request continues the performance track after P1 and the partial progressive-preview slice. Complete this bounded P2 stage, report actual results and stop before P3.

Reuse the existing row virtualizer, three-page/720-summary feed, stable-ID/pixel anchor, text-only mode and catalog-only date seeks. Add density/geometry-aware gallery tiers up to1024px, an explicitly opened viewer tier up to2048px, progressive retained fallbacks and a bounded versioned binary worker response. Keep original access read-only, protected cache publication, all existing worker input/dimension/time/concurrency/RSS gates and the256MiB indexed cache bound. No dependency or schema migration, automatic cache purge, larger decoder admission or original-file operation.

Verify tier/admission selection, malformed/truncated/oversized worker frames, color/orientation/no-upscale behavior, per-tier errors, legacy positive/offline reuse and maintenance barriers. Measure generated image worker cold/warm results with independent manifests; distinguish synthetic catalog/functional WebView checks from full-media/release presentation and memory evidence. Preserve existing results and record unavailable Windows/clean-machine/full-process/native-input gates honestly in docs/PERFORMANCE_P2.md.

Local implementation and diagnostic checks are recorded in [PERFORMANCE_P2.md](../docs/PERFORMANCE_P2.md), including retained failures and the corrected version2 synthetic fixtures. The local repository/native checks pass; full platform acceptance remains blocked by the explicitly listed Windows/release-presentation/full-memory environments. Preserve these distinctions at the gate. P3 is not started.
