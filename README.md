# SurveyCraft

<p align="center">
  <img src="assets/brand/surveycraft-lockup.svg" alt="SurveyCraft — open surveying tools for CADCraft" width="720">
</p>

SurveyCraft is an open-source surveying, terrain and cadastral extension for CADCraft, distributed
from its own repository as an installable extension package.

**Developer preview: current SurveyCraft releases require a custom CADCraft build. They do not
work with official CADCraft v0.7.0.** The required extension host is proposed upstream and has not
been accepted or released. The CAD-independent v0.4 engine is in development, while its drawing
adapter remains gated on the host proposal. See
[host compatibility and the upstream plan](docs/HOST_COMPATIBILITY.md) before installing.

This repository contains the **Phase 0 engineering foundation** and the working Phase 1 point
import, persistent point identity and Point Manager workflow. The implementation plan is in
[BLUEPRINT.md](BLUEPRINT.md), the product parity ledgers are in
[docs/FEATURE_MATRIX.md](docs/FEATURE_MATRIX.md) and
[docs/TOPO_LT_COVERAGE.md](docs/TOPO_LT_COVERAGE.md), the versioned release plan is in
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

The surveying engine is written as pure, testable Rust crates. Its CADCraft adapter compiles to
WebAssembly and currently runs in the proposed schema-2 host. The host architecture remains subject
to upstream review; SurveyCraft will adapt its integration to the accepted API.

The generic CADCraft extension-system work is tracked separately in
[storytold/cadcraft#1384](https://github.com/storytold/cadcraft/pull/1384), which preserves the full
integration as a draft reference. The [upstream plan](docs/HOST_COMPATIBILITY.md#upstream-review-plan)
starts with the smaller [package/AutoLISP foundation in PR #1446](https://github.com/storytold/cadcraft/pull/1446).
SurveyCraft remains in this standalone project.

## Build the extension

For development, SurveyCraft v0.3.0 was verified against custom CADCraft host commit
[`6f51a62`](https://github.com/AlexMaster23/cadcraft/commit/6f51a623fba868838ec26f27eb8933c537fe8850)
from PR #1384. The smaller foundation branch alone cannot run it. Build the guest and create the
installable package with:

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
cargo run -p surveycraft-cli -- codebook validate tests/fixtures/codebooks/synthetic-field.json
cargo run -p surveycraft-cli -- codebook preview tests/fixtures/codebooks/synthetic-field.toml edg-sta
cargo run -p surveycraft-cli -- symbolpack check-codebook tests/fixtures/codebooks/synthetic-field.json assets/symbols/surveycraft-basic.symbol-pack.json
```

Format contracts are documented in [docs/formats/point-csv-v1.md](docs/formats/point-csv-v1.md),
[docs/formats/project-v2.md](docs/formats/project-v2.md), and
[docs/formats/code-book-v1.md](docs/formats/code-book-v1.md). The audited coding-command behavior
and remaining host work are in [docs/CODING_FEATURES.md](docs/CODING_FEATURES.md). The original
vector library contract is [docs/formats/symbol-pack-v1.md](docs/formats/symbol-pack-v1.md). See also
[docs/numeric-policy.md](docs/numeric-policy.md). The host decision is recorded in
[docs/adr/0001-portable-wasm-host.md](docs/adr/0001-portable-wasm-host.md).

## Releases

Every completed roadmap version receives a tagged GitHub Release with the installable
`surveycraft.cadcraft-extension` package and a SHA-256 checksum. Pre-1.0 versions are published as
prereleases. See the [latest releases](https://github.com/AlexMaster23/surveycraft/releases) and
the [version exit criteria](ROADMAP.md#version-exit-criteria).
