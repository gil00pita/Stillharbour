# ADR-0001: M0 desktop foundation

Date: 2026-10-27  
Status: adopted for M0; release capabilities remain gated

The repository contained the owner's specification package and logo, with no application source. Implement its M0 foundation without changing the roadmap or persisted user identities.

Use a modular monolith: one `stillharbour-core` crate, one thin Tauri binary, and a React renderer. Core modules separate domain rules, the catalog actor, filesystem primitives, job admission, media processes and helper arguments. Fixture copying is behind an explicit `fixture-probes` feature and has no IPC route. Extract additional crates only when a demonstrated boundary needs them.

Use Tauri 2.12.5, Rust 1.99.1, React 19.3.1, Vite 8.3.5, TypeScript 6.0.3 and npm 11.16.0. TypeScript 7.0.2 was evaluated but is outside typescript-eslint 8.72.1's supported peer range. Exact direct pins and both lockfiles make the selected compatible set reproducible. npm is already available and avoids introducing another package manager.

Generate request, response and structured-error TypeScript declarations from Rust with ts-rs. A single allowlisted command accepts an internally tagged request with no path arguments. Empty struct variants enforce unknown-field rejection; unit variants were found to ignore additional fields. Only the local main window has the command permission. Native paths and selected folder identity remain backend-owned; display strings cannot become execution inputs.

rusqlite bundles SQLite 3.54.1. The runtime refuses versions older than 3.51.3, the upstream WAL-reset fix baseline. A bounded database actor owns a lifetime OS lock, transactionally initialises schema 1, rejects future/foreign databases, and uses SQLite's Online Backup API for an independently openable snapshot. This is a foundation, not N2's arbitrary catalog import/migration system. New migrations beyond schema 1 require retained pre-migration snapshots and separate review.

On this Mac, `renamex_np(RENAME_EXCL)` proves atomic same-volume no-replace publication; stable handle metadata proves inode/device identity and hard-link detection. Other platforms fail closed for these primitives. Ancestor race-proof containment, Windows handle-based identity/publication, full metadata fidelity, independent recovery and all real-library mutations remain blocked. A same-device fixture copy cannot satisfy the preservation recovery contract.

Use one short-lived decoder process at a time, with input, dimension, allocation, output and elapsed-time limits. Enable only JPEG/PNG. Apply EXIF orientation to derived pixels, avoid upscaling small fixtures, and reject ICC-tagged inputs until a tested color transform exists. This is process isolation, not an OS sandbox or a hard whole-process RSS guarantee. No decoder work runs on the UI/async executor thread.

Use optional WebdriverIO Rust plugins and a dedicated debug config for native tests. Global Tauri APIs and additional test permissions exist only in that config. A compile-time guard refuses release builds with `desktop-test`; the normal feature graph and frontend mode exclude instrumentation. No renderer filesystem, shell, SQL or generic fetch permission is provided.

Set macOS 13.0 as a **provisional deployment target**, not a verified support minimum. Local evidence is macOS 27.0.1 on arm64. Windows 11 x64, the oldest supported macOS, Intel, clean-machine helpers and signed bundles need their own native gates. `local.stillharbour.foundation` is a development identifier; distribution needs an owner-controlled namespace before first release, with no later silent identity migration.

A minimal empty shell is sufficient for M0. Gallery virtualization needs indexed queries in M1; the map/provider/privacy spike belongs to M5. No map or virtualization dependency is added just to render an empty window. LibRaw/HEIF and libvips remain gated in the helper plan rather than increasing native distribution cost without evidence.

Primary references checked: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), [custom permissions](https://v2.tauri.app/security/permissions/), [WebdriverIO plugin setup](https://webdriver.io/docs/desktop-testing/tauri/plugin-setup), [rusqlite backup API](https://docs.rs/rusqlite/0.41.0/rusqlite/backup/index.html), [SQLite WAL fix](https://sqlite.org/wal.html#walresetbug), [image decoder API](https://docs.rs/image/0.25.10/image/struct.ImageReader.html).
