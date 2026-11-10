# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

React renders inside a native Tauri desktop window for Windows and macOS. This
describes the renderer technology, not a browser-only distribution. There is no
mobile application.

## Stack

Tauri 2, a Rust core, strict React/TypeScript, Vite, Chakra UI v3 and local SQLite.
See [Architecture](docs/ARCHITECTURE.md) for ownership and build boundaries.

## Users and purpose

People organizing local photo folders need to browse, inspect and organize their
collections while keeping originals in ordinary folders. Preservation takes
precedence over convenience, speed and space savings.

## Implemented scope

The [build capability reference](docs/CODEX_SPEC.md) and
[user guide](docs/USER_GUIDE.md) describe browsing, catalog annotations/albums,
reviewed new metadata exports, maps, read-only utilities and structure previews.
All claims remain subject to [format limits](docs/FORMAT_SUPPORT.md) and
[release status](docs/RELEASE_STATUS.md).

Default builds do not execute original-file mutations, snapshots, restore or
schedules. Experimental compile flags and generated fixtures grant no authority
over a real library. No account, telemetry, provider import or image editor is
enabled. A preview or catalog export is not a backup of original photographs.

## Brand and interface

Use Stillharbour, the unchanged owner aperture, locally bundled Inter and neutral
light/dark Chakra surfaces. First use follows the system; a saved manual choice
takes precedence. Photos and the original mark carry color. See
[DESIGN.md](DESIGN.md) for implemented presentation and
[product identity](docs/PRODUCT_IDENTITY.md) for persisted-name rules.

## Previous plans

The [changelog snapshot](docs/changelog/2026-11-09/README.md) preserves earlier
product/design notes, milestone plans and dated evidence. Those are historical
context; they neither expand this scope nor waive unresolved safety/release gates.
