# SurveyCraft feature matrix

This is the product-level parity ledger derived from the documented TopoLT command surface. It is
not an implementation claim and does not authorize copying vendor code or assets.

Legend: **P0** foundation/MVP, **P1** first practical release, **P2** advanced, **P3** optional or
jurisdiction-specific.

| Module | Observed behavior to cover | SurveyCraft direction | Priority |
|---|---|---|---:|
| Project/configuration | Drawing-specific scales, layers, point/model/image/plan settings | Versioned `SurveyProject` stored as extension metadata, with explicit migrations | P0 |
| Point import | Read coordinate/text and field formats, order/deduplicate, create points | Preview-first mapping, diagnostics, deterministic import, source-record trace | P0 |
| Point management | Renumber, update, delete, transform labels/codes/elevations | Point Manager panel plus batch commands, all undoable | P0 |
| Point quality | Detect close/duplicate points, optimize labels, snap geometry to survey points | Non-destructive diagnostics with selectable fixes | P1 |
| COGO | Polar/sightshot calculations and coordinate reports | Inverse, traverse, intersections and observation reduction in pure Rust | P0/P1 |
| Instrument transfer | Receive/send coordinates and record measurements | Separate opt-in device capability and open instrument profiles | P3 |
| Code book | Map field codes to layers, symbols, line types and scales | Open JSON/TOML code book; regex/prefix rules; live preview | P0 |
| Feature lines | Join points by code and create coded linework | Stable feature identities referencing point UUIDs | P0 |
| Symbols | Insert topographic blocks based on code | Original open symbol library in a separate attributed pack | P1 |
| Areas | Area/perimeter labels and calculation tables | Robust planar rings, explicit units/rounding and linked labels | P0 |
| Parcel division | Split a parcel by parallel/perpendicular, proportion, point/opening, offset or side translation | Exact target-area solver with preview and residual report | P1 |
| Parcel links | Recalculate labels/tables after boundary edits | Typed metadata links and stale-link diagnostics | P0 |
| Building survey tables | Room/space inventories | Generic room schema and configurable table templates | P2 |
| Raster placement | Read/write world/georeference files and restore image positions | Standard affine transform plus CRS metadata | P2 |
| Raster operations | Crop, resize, convert and mosaic planning | Safe image pipeline; preserve source and write new output | P2 |
| Online imagery | Download map imagery for an extent | Separate provider plugins, explicit network consent, cache provenance | P3 |
| Terrain/TIN | Build triangular surface with boundaries and breaklines | Constrained Delaunay surface with deterministic topology | P1 |
| Surface editing | Clip, join, reshape and inspect terrain models | Transactional surface operations with validation report | P2 |
| Contours | Generate and style contour lines | Major/minor intervals, labels and optional explicit smoothing | P1 |
| Projection/profile | Project 2D geometry to surface and inspect profiles | Surface sampling, profile/cross-section model and CAD output | P2 |
| Volumes | Compare surfaces or surface/reference and report cut/fill | Signed prism integration with balance and tolerance report | P1 |
| CRS transforms | Transform E/N/H or geographic coordinates, including epoch-aware workflows | CRS metadata, axis-order warning, operation audit and public test vectors | P2 |
| Entity alignment | Move/rotate/scale from control points; flatten/lift geometry | Reuse CADCraft transforms with survey-specific reports | P1 |
| Graticules | Draw coordinate grids and labels | Declarative grid style linked to CRS and sheet scale | P2 |
| Sheets | Frames, named views/layouts, plan numbering and batch output | CADCraft layouts plus SurveyCraft sheet definitions | P2 |
| Exchange | DXF/KML and geographic display | Public-format DXF/KML/GeoJSON exporters; no external-app dependency | P2 |
| Drafting utilities | Text alignment/replacement/sum, polyline editing/dimension helpers | Include only survey workflows not already native to CADCraft | P2 |
| Language/help/update | Localized interface and documentation | English first, localization-ready message IDs, normal release updater | P1 |
| Cadastral integration | Romanian property/construction/subparcel workflows and generators | Separate Romania pack based only on public ANCPI schemas | P3 |

## Current implementation status

- **Release train:** [ROADMAP.md](../ROADMAP.md) maps these modules to versioned GitHub milestones
  from the released v0.1.0 point-import preview through v1.0.0 and the post-1.0 phases.
- **Implemented locally:** deterministic point CSV/project parsing, CLI validation, native manual
  and CSV coordinate table, required/numeric/duplicate validation, CAD point and label insertion,
  fixed output layers, transaction rollback and one-step undo.
- **Next:** persistent point identity/metadata, style controls, editing existing drawing points,
  full Point Manager and code-book-driven symbols/feature lines.
- **Host dependency:** installation currently requires the unreleased generic schema 2 capability
  from CADCraft PR #1384; SurveyCraft itself remains a separate repository and package.

## Product decisions

- CADCraft already owns generic CAD utilities; SurveyCraft will not duplicate them merely for
  command-count parity.
- Network imagery, device communication and country-specific cadastral generation remain optional
  modules so the core extension stays portable and easy to audit.
- Native `.3ds` rendering is not a core surveying requirement; terrain visualization should use
  CADCraft's renderer and open exchange formats.
- Proprietary symbol/template/data libraries are replaced with original open packs or user-supplied
  content.
