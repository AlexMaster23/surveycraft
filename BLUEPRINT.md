# SurveyCraft product and engineering blueprint

Status: **Phase 0 complete; Phase 1 persistent point model implemented; CADCraft host pending upstream review**
Brand direction: **orange SurveyCraft family mark**
Target host: **CADCraft desktop first; portable core and WASM-compatible extension runtime**

## 1. Goal

Build an open-source surveying extension that gives a normal CADCraft user a coherent workflow
from field observations to finished plans:

```text
field files / CSV / manual points
              |
              v
     validate + normalize points
              |
      +-------+---------+
      |                 |
      v                 v
 feature coding     coordinate transforms
      |                 |
      +-------+---------+
              v
       CAD geometry + metadata
              |
     +--------+---------+----------+
     |                  |          |
     v                  v          v
 parcels/reports   terrain/TIN   raster context
     |                  |          |
     +--------+---------+----------+
              v
       sheets / exports / audit
```

The long-term target is complete coverage of TopoLT's user-visible workflows, not a clone of its
internals or visual design. SurveyCraft should produce equivalent mathematically correct results
for equivalent inputs, with explicit units, axis conventions, rounding and numerical tolerances,
while using its own data model, commands, interface, documentation and assets. Differences and
unknowns must be recorded rather than silently accepted as parity.

Every identified command, dialog option, supported format and workflow variant must be accounted
for in the [feature ledger](docs/FEATURE_MATRIX.md). A workflow may be delivered by SurveyCraft,
an optional pack or a verified native CADCraft equivalent; it must not disappear from scope because
it is difficult, country-specific or dependent on host support. The versioned roadmap stages this
work. The first stable v1.0 milestone is not a claim that full TopoLT coverage has been achieved.

## 2. Evidence baseline and clean-room rules

The existing static-analysis report identifies 109 documented menu commands plus additional
integration entry points. This is a provisional discovery baseline, not proof of an exhaustive
behavior inventory or of verified parity. Command options, dialogs, formats and end-to-end variants
still need a command-level audit. The recorded feature groups are the baseline for the parity ledger:

- point-set management and individual point operations;
- coordinate-file calculations and total-station communication;
- code-driven symbols, layers and linework;
- areas, parcels and building surveys;
- raster imagery and georeferencing;
- triangulated terrain, contours and volumes;
- plan sheets and batch output;
- coordinate and entity transformations;
- general drafting utilities and geographic exchange.

The public repository records clean-room behavior categories in
[docs/FEATURE_MATRIX.md](docs/FEATURE_MATRIX.md) and the exposed-action baseline in
[docs/TOPO_LT_COVERAGE.md](docs/TOPO_LT_COVERAGE.md). Research notes and any proprietary comparison
environment stay outside this repository; they are not build inputs or redistributable assets.

Rules for implementation:

1. Do not copy or translate proprietary source, decompiled pseudocode, data files, images, symbols,
   templates, help text, UI artwork, or binary resources.
2. Use the analysis only to identify externally visible behavior and workflow boundaries.
3. Derive mathematics from public standards, textbooks, EPSG guidance, file-format specifications,
   and independently written test vectors.
4. Build synthetic fixtures from known coordinates and simple geometry. Keep any comparison output
   from proprietary software outside the repository.
5. Label every parity claim as implemented, verified, partially verified, or unknown.

## 3. Architecture

SurveyCraft is one repository with a pure domain engine and thin adapters:

```text
surveycraft-core        geometry, points, parcels, TIN, contours, volumes
       ^
       |
surveycraft-io          CSV/text/field formats, reports, project interchange
       ^
       |
surveycraft-cadcraft    commands, drawing transactions, selection and previews
       ^
       |
surveycraft-package     manifest, translations, icons and WASM extension bundle

surveycraft-cli         headless validation, conversion and golden-test runner
```

Proposed repository layout:

```text
surveycraft/
  Cargo.toml
  crates/
    core/               # no CAD or UI dependencies
    geometry/           # robust predicates and survey-specific geometry
    io/                 # field files, CSV and project/report formats
    cadcraft/            # portable CADCraft host adapter
    ui-model/            # serializable panels/forms, no renderer dependency
  apps/
    surveycraft-cli/
  extension/
    extension.json
    main.lsp             # optional small command/bootstrap layer
  assets/brand/
  docs/
  tests/golden/
```

### Current host dependency

The custom CADCraft integration in draft PR #1384 implements package installation, command
registration, permissions, menus, Tool Sets and CLI management. Its schema-2 WebAssembly path
currently provides the capabilities used by SurveyCraft:

- schema-2 `wasm-component` entry point;
- a versioned host API for guarded drawing transactions and entity queries/updates;
- namespaced persistent drawing/entity metadata;
- native data-entry tables and record managers, including drawing selection/location;
- host-owned file selection and text export, without ambient guest file/network/process access;
- bounded fuel, memory and input, with rollback on guest failure.

The native host, standalone SurveyCraft package, guarded drawing transaction, native coordinate
table and one-step undo work end to end. The generic host exposes line, point and text creation;
SurveyCraft supplies only command metadata and sandboxed guest logic. Browser execution, richer
host services and upstream review remain. Once the host capability ships in a normal CADCraft
release, users install one `surveycraft.cadcraft-extension` package without a custom CADCraft
build. The staged [upstream plan](docs/HOST_COMPATIBILITY.md) begins with package/AutoLISP support
alone; that first stage does not yet run SurveyCraft.

## 4. Domain model

The first stable model should be explicit, serializable and unit-aware:

- `SurveyProject`: CRS, linear/angular units, precision, code book, point sets and audit metadata.
- `SurveyPoint`: stable UUID, user point number, easting, northing, elevation, code, description,
  source record and quality flags.
- `Observation`: station, backsight, target, horizontal/vertical angle, slope distance,
  instrument/reflector heights and timestamp.
- `FeatureCodeRule`: input code pattern to layer, geometry kind, symbol, line style and attributes.
- `Feature`: ordered point references plus CAD entity linkage.
- `Parcel`: outer ring, holes, identifier, attributes, labels and linked CAD handles.
- `TinSurface`: vertices, triangles, breaklines, outer boundary, exclusions and CRS.
- `RasterReference`: image identifier, affine/world transform, CRS, crop and source metadata.
- `SheetDefinition`: scale, paper, margins, frame, grid, north arrow and output profile.

Coordinates stay as `f64` internally, but every public API carries units and CRS metadata. Point
numbers are strings, not integers, because real field data commonly contains prefixes and suffixes.

## 5. Command and UI surface

Every user-visible action is a command and has the same ID across menus, command search, scripts,
automation and the CLI where applicable. Proposed menu groups:

```text
SurveyCraft
  Project & Settings
  Points
  Coding & Features
  Parcels & Areas
  Terrain & Contours
  Volumes & Profiles
  Coordinates & COGO
  Raster & Georeference
  Sheets & Export
  Quality & Diagnostics
```

Command IDs use a stable `sc.` namespace (`sc.points.import`, `sc.terrain.build`,
`sc.parcel.divide`, `sc.volume.compare`). Friendly aliases such as `SCIMPORT` remain optional and
must never shadow CADCraft commands.

The UI is specified in [docs/UI_BLUEPRINT.md](docs/UI_BLUEPRINT.md). CADCraft owns the window,
theme, focus, docking and accessibility. SurveyCraft supplies command metadata and declarative view
models, so the extension looks native and does not bundle a second GUI toolkit.

## 6. Delivery roadmap

The phases below describe product scope. [ROADMAP.md](ROADMAP.md) divides them into independently
installable versions with explicit exit criteria and GitHub milestones.

### Phase 0 — foundations

- agree on this blueprint and brand;
- create the Rust workspace, CI, licensing and contribution rules;
- propose the CADCraft WASM host API using one tiny reference extension;
- define project JSON, point CSV and numeric tolerance policies;
- build golden-test harnesses and a synthetic survey fixture.

Exit: a deterministic core/CLI can read points and the reference WASM extension can create one CAD
entity through a guarded transaction.

### Phase 1 — points and field-to-drawing MVP (in progress)

- [MVP implemented] CSV/text import with header mapping, preview and diagnostics;
- [MVP implemented] manual coordinate rows, duplicate detection and one-step drawing insertion;
- [MVP implemented] CAD point, number, elevation and optional code layers;
- [Implemented] persistent point UUIDs, exact CAD entity links, stored styles and stale-link diagnostics;
- manual point creation/editing, renumbering, duplicate detection and elevation edits;
- code book with layer/symbol/feature-line rules;
- point labels, styles and CAD metadata links;
- coordinate listing/export and a point manager panel;
- basic COGO: inverse, traverse/polar point, intersections and stakeout report.

Exit: a surveyor can import a field file, see/repair problems, generate coded drawing objects and
round-trip the project without losing identity.

### Phase 2 — parcels, areas and reports

- robust closed-boundary discovery and validation;
- area/perimeter labels and tables;
- linked labels that recalculate after edits;
- parcel split methods with live preview and exact target-area solver;
- direction/start-vertex normalization, room/parcel tables and report export.

Exit: synthetic parcel jobs match independently computed areas and all edits are undoable.

### Phase 3 — terrain, contours, profiles and volumes

- constrained Delaunay TIN with boundaries, holes and breaklines;
- triangle inspection/editing and surface joining/clipping;
- indexed/major/minor contours with smoothing as a separate explicit operation;
- 2D-to-surface projection, profiles/cross-sections and elevation queries;
- cut/fill between surface/surface and surface/reference plane with audit tables.

Exit: known analytical surfaces and published test meshes stay within declared tolerances.

### Phase 4 — CRS, raster and geographic exchange

- project CRS with EPSG/WKT2 or PROJJSON metadata;
- audited transforms with axis order, units, epoch and grid requirements made visible;
- world-file reading/writing and affine image placement;
- crop, mosaic planning and batch georeferenced insertion;
- KML/GeoJSON/DXF exchange using public specifications.

Exit: published control-point transformations and raster footprints pass repeatable golden tests.

### Phase 5 — sheets, instruments and jurisdiction packs

- graticules, frames, numbered views and batch PDF output;
- serial/device communication behind explicit opt-in capabilities;
- an open format for instrument profiles;
- optional country/jurisdiction packs maintained separately from the engine;
- Romanian cadastral workflows only from public schemas and user-owned inputs.

Exit: core SurveyCraft remains jurisdiction-neutral; specialized packs are independently versioned.

## 7. Correctness and safety

- All drawing mutations occur inside one CADCraft transaction and support undo.
- Long tasks expose progress and cancellation and never block the UI thread.
- Imported counts, strings, meshes and recursion are bounded before allocation.
- Invalid geometry returns structured diagnostics; production code never panics.
- Tolerance policy distinguishes exact topology predicates from display rounding.
- Every destructive command previews its scope and names the affected entity count.
- Project and extension formats are versioned and migrations never overwrite the only copy.
- File/network/device access is denied unless declared by the package and initiated by the user.

## 8. Verification strategy

Each feature needs four levels of proof:

1. **Unit tests** for formulas and parsing.
2. **Property tests** for invariants such as area conservation, transform round trips and stable
   triangulation.
3. **Golden tests** with small human-readable inputs and expected JSON/CSV/DXF summaries.
4. **CAD integration tests** that execute the public command, inspect resulting entities and verify
   undo/redo.

Parity comparisons use fresh synthetic inputs and compare numeric/result summaries, never copied
vendor drawings or assets. Tolerances and unknowns are recorded next to the test.

## 9. Definition of the first stable release

SurveyCraft 1.0 is ready when a user on the latest stable CADCraft can:

- install a signed/reproducible `.cadcraft-extension` bundle through Tools > Extensions;
- import and validate points, apply codes, create feature lines and edit point data;
- calculate/link parcel areas and generate an area table;
- build a bounded TIN, create contours and compute a basic cut/fill result;
- save/reopen the DWG with SurveyCraft relationships intact;
- use the same commands from menus, command line and automation;
- uninstall the extension without modifying the CADCraft installation.

## 10. Immediate implementation sequence after approval

1. [Done] Scaffold the Rust workspace and CI.
2. [Done] Implement units, point/project types and deterministic CSV import.
3. [Done] Add the CLI and golden-test fixture.
4. [Done locally] Implement the CADCraft WASM host contract and standalone extension package.
5. [Done locally] Implement native table/CSV point insertion, CAD display layers and one-step undo.
6. [Done locally] Add persistent point identity/metadata, migrations and initial point styles.
7. [Done locally] Build the v0.3 Point Manager with filtering, selection/location, linked editing,
   repair, renumbering, deletion and deterministic CSV export.
8. [Current] Obtain upstream review of the staged CADCraft host proposal; see
   [HOST_COMPATIBILITY.md](docs/HOST_COMPATIBILITY.md).
9. [Paused pending host direction] Build the v0.4 open code-book format and code-driven feature
   generation.
