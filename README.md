# SurveyCraft

<p align="center">
  <img src="assets/brand/surveycraft-lockup.svg" alt="SurveyCraft — open surveying tools for CADCraft" width="720">
</p>

SurveyCraft is an open-source surveying, terrain and cadastral extension for CADCraft. It is an
ordinary installable extension with its own repository and release package, not a fork of CADCraft
and not source code bundled into CADCraft's extension-system pull request.

This repository contains the **Phase 0 engineering foundation** and the working Phase 1 point
import, persistent point identity and Point Manager workflow. The implementation plan is in
[BLUEPRINT.md](BLUEPRINT.md), the product parity ledger is in
[docs/FEATURE_MATRIX.md](docs/FEATURE_MATRIX.md), the versioned release plan is in
[ROADMAP.md](ROADMAP.md), and the interface specification is in
[docs/UI_BLUEPRINT.md](docs/UI_BLUEPRINT.md). Released changes are recorded in
[CHANGELOG.md](CHANGELOG.md).

The point-import command opens a native CADCraft table, accepts manually entered or imported
coordinates, lets the user choose CAD/GIS or Survey/TopoLT X/Y interpretation, validates the rows,
and inserts CAD points plus number, elevation and optional code labels. Every logical point and
its CAD entities receive persistent links, stored styles and reconciliation diagnostics. The whole
operation is transactional and is removed by one Undo. The native Point Manager then filters,
selects, locates, edits, synchronizes, renumbers, deletes and exports those linked points without
losing their stable identities. Each write action is one undoable transaction. The schema 2 host is
implemented and proven locally on CADCraft's `feature/extension-system` working branch; it is not
part of a public CADCraft release yet.

## Product promise

- Import, validate, edit and export survey points.
- Turn field codes into CAD layers, symbols and feature lines.
- Calculate and document parcels and building-survey areas.
- Build terrain surfaces, contours, profiles and cut/fill volumes.
- Transform coordinate systems with explicit metadata and audit trails.
- Georeference raster imagery and prepare plan sheets.
- Produce repeatable results through a tested, scriptable command surface.

## Project boundaries

SurveyCraft will be a clean-room implementation based on documented behavior, public geodesy and
geometry references, and synthetic test cases. No TopoLT source code, binary code, icons, symbols,
templates, textures, fonts, databases, or other proprietary assets will be copied into this
project.

## Runtime direction

The surveying engine will be written as pure, testable Rust crates. The production CADCraft
adapter will compile to WebAssembly once the CADCraft extension host supports a portable WASM
runtime. A thin AutoLISP/bootstrap layer may remain for command ergonomics, but terrain and
coordinate mathematics will not be implemented as an opaque LISP monolith.

The generic CADCraft extension-system work is tracked separately in
[storytold/cadcraft#1384](https://github.com/storytold/cadcraft/pull/1384). That pull request contains
the host capability only. SurveyCraft remains in this standalone project and is mentioned there
only as an external integration proof.

## Build the extension

Until the generic schema 2 host is released by CADCraft, use a CADCraft build from PR #1384. Build
the guest and create the installable package with:

```text
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p surveycraft-extension
cargo run -p xtask
```

This writes `target/extension-package/surveycraft.cadcraft-extension`. In CADCraft, open
**Tools > Extensions...** and install that file. The orange-marked **SurveyCraft** group appears at
the top of the Drafting Tool Sets palette; press **Import and insert points...** there, use
**SurveyCraft > Points > Import and insert points...**, or type `SCPOINTS`. See
[docs/POINT_IMPORT.md](docs/POINT_IMPORT.md) for the exact input contract and drawing output.
Use **Check point links** or `SCCHECKPOINTS` to diagnose CAD entities deleted or edited outside
SurveyCraft. Press **Point Manager** or type `SCPOINTMANAGER`/`SCPM` to manage existing points;
[docs/POINT_MANAGER.md](docs/POINT_MANAGER.md) documents the complete workflow.

## Core and CLI

With a current Rust toolchain:

```text
cargo run -p surveycraft-cli -- validate tests/fixtures/points/basic.csv
cargo run -p surveycraft-cli -- import tests/fixtures/points/basic.csv --output survey.json
cargo run -p surveycraft-cli -- inspect survey.json
```

Format contracts are documented in [docs/formats/point-csv-v1.md](docs/formats/point-csv-v1.md),
[docs/formats/project-v2.md](docs/formats/project-v2.md), and
[docs/numeric-policy.md](docs/numeric-policy.md). The host decision is recorded in
[docs/adr/0001-portable-wasm-host.md](docs/adr/0001-portable-wasm-host.md).

## Releases

Every completed roadmap version receives a tagged GitHub Release with the installable
`surveycraft.cadcraft-extension` package and a SHA-256 checksum. Pre-1.0 versions are published as
prereleases. See the [latest releases](https://github.com/AlexMaster23/surveycraft/releases) and
the [version exit criteria](ROADMAP.md#version-exit-criteria).
