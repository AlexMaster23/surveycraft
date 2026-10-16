# SurveyCraft feature matrix

This is the product-level parity ledger derived from the documented TopoLT command surface. It is
not an implementation claim and does not authorize copying vendor code or assets.

## Scope and completion rule

Complete TopoLT user-visible workflow coverage is the long-term product requirement. Nothing is
silently removed because it is advanced, country-specific or requires a future CADCraft API.
Optional modules are optional for users to install, not optional to account for in this ledger.
Version milestones are delivery slices; v1.0 alone does not establish full parity.

The [command-level coverage ledger](TOPO_LT_COVERAGE.md) now accounts for the 109 menu commands,
30 additional integration entry points and Load action identified by the static report. It is not
yet an exhaustive option/runtime inventory. Before claiming full coverage, audit every dialog
option, accepted format and workflow variant. Each detailed record must contain:

- a stable tracking ID and a factual, independently worded behavior description;
- evidence provenance and reference version, or an explicit unverified/unknown marker;
- a SurveyCraft command, separately delivered pack or verified native CADCraft equivalent;
- target release or an explicitly tracked unscheduled item, plus any host dependency;
- implementation status and a separate parity-verification status;
- synthetic input, expected result, units/axes/rounding/tolerance and regression-test reference;
- known differences, unsupported cases and the next verification or implementation step.

Full coverage is not complete while any required record is unimplemented, blocked, unverified or
unaccounted for. Changes to scope require an explicit product decision; deferral is not removal.

Legend: **P0** foundation/MVP, **P1** first practical release, **P2** advanced, **P3** optional or
jurisdiction-specific.

Implementation status is deliberately separate from verified TopoLT equivalence. **Partial**
means a subset works in the custom host; **Planned** means not delivered by SurveyCraft;
**Unverified mapping** means a native CADCraft equivalent still needs an explicit workflow check;
**Pending audit** means the precise reference behavior has not yet been established.
No module below is currently certified as complete TopoLT parity.

| Module | Behavior to cover | SurveyCraft direction | Priority | Implementation status |
|---|---|---|---:|---|
| Project/configuration | Drawing-specific scales, layers, point/model/image/plan settings | Versioned `SurveyProject` stored as extension metadata, with explicit migrations | P0 | Partial |
| Point import | Read coordinate/text and field formats, order/deduplicate, create points | Preview-first mapping, diagnostics, deterministic import, source-record trace | P0 | Partial |
| Point management | Renumber, update, delete, transform labels/codes/elevations | **Implemented in v0.3.0:** native Point Manager, selection/location, synchronized edits, repair, batch renumber/delete and CSV export | P0 | Partial |
| Point quality | Detect close/duplicate points, optimize labels, snap geometry to survey points | Non-destructive diagnostics with selectable fixes | P1 | Partial |
| COGO | Polar/sightshot calculations and coordinate reports | Inverse, traverse, intersections and observation reduction in pure Rust | P0/P1 | Planned |
| Instrument transfer | Receive/send coordinates and record measurements | Separate opt-in device capability and open instrument profiles | P3 | Planned |
| Code book | Map field codes to layers, symbols, line types and scales | Open JSON/TOML code book; exact/prefix/regex rules; structured diagnostics | P0 | Partial — engine and CLI implemented; editor/preview pending |
| Feature lines | Join points by code and create coded linework | Stable feature identities referencing point UUIDs | P0 | Partial — deterministic path/arc/circle plans tested; CAD adapter pending |
| Symbols | Insert topographic blocks based on code | Original open symbol library in a separate attributed pack | P1 | Partial — scale/rotate/delete plans tested; original pack and CAD adapter pending |
| Areas | Area/perimeter labels and calculation tables | Robust planar rings, explicit units/rounding and linked labels | P0 | Planned |
| Parcel division | Split a parcel by parallel/perpendicular, proportion, point/opening, offset or side translation | Exact target-area solver with preview and residual report | P1 | Planned |
| Parcel links | Recalculate labels/tables after boundary edits | Typed metadata links and stale-link diagnostics | P0 | Planned |
| Building survey tables | Room/space inventories | Generic room schema and configurable table templates | P2 | Planned |
| Raster placement | Read/write world/georeference files and restore image positions | Standard affine transform plus CRS metadata | P2 | Planned |
| Raster operations | Crop, resize, convert and mosaic planning | Safe image pipeline; preserve source and write new output | P2 | Planned |
| Online imagery | Download map imagery for an extent | Separate provider plugins, explicit network consent, cache provenance | P3 | Planned |
| Terrain/TIN | Build triangular surface with boundaries and breaklines | Constrained Delaunay surface with deterministic topology | P1 | Planned |
| Surface editing | Clip, join, reshape and inspect terrain models | Transactional surface operations with validation report | P2 | Planned |
| 3D visualization/exchange | Terrain viewing and 3D exchange requirements, including `.3ds`; exact behavior needs audit | Native CADCraft rendering and public-specification format adapters where required | P2 | Pending audit |
| Contours | Generate and style contour lines | Major/minor intervals, labels and optional explicit smoothing | P1 | Planned |
| Projection/profile | Project 2D geometry to surface and inspect profiles | Surface sampling, profile/cross-section model and CAD output | P2 | Planned |
| Volumes | Compare surfaces or surface/reference and report cut/fill | Signed prism integration with balance and tolerance report | P1 | Planned |
| CRS transforms | Transform E/N/H or geographic coordinates, including epoch-aware workflows | CRS metadata, axis-order warning, operation audit and public test vectors | P2 | Planned |
| Entity alignment | Move/rotate/scale from control points; flatten/lift geometry | Reuse CADCraft transforms with survey-specific reports | P1 | Unverified mapping |
| Graticules | Draw coordinate grids and labels | Declarative grid style linked to CRS and sheet scale | P2 | Planned |
| Sheets | Frames, named views/layouts, plan numbering and batch output | CADCraft layouts plus SurveyCraft sheet definitions | P2 | Planned |
| Exchange | DXF/KML and geographic display | Public-format DXF/KML/GeoJSON exporters; no external-app dependency | P2 | Unverified mapping |
| Drafting utilities | Text alignment/replacement/sum, polyline editing/dimension helpers | Include only survey workflows not already native to CADCraft | P2 | Unverified mapping |
| Language/help/update | Localized interface and documentation | English first, localization-ready message IDs, normal release updater | P1 | Partial |
| Cadastral integration | Romanian property/construction/subparcel workflows and generators | Separate Romania pack based only on public ANCPI schemas | P3 | Planned |

## Current implementation status

- **Release train:** [ROADMAP.md](../ROADMAP.md) maps these modules to versioned GitHub milestones
  from the released v0.1.0 point-import preview through v1.0.0 and the post-1.0 phases.
- **Released through v0.3.0:** deterministic point CSV/project parsing, schema-v1-to-v2 migration,
  native manual and CSV coordinate table, required/numeric/duplicate validation, persistent UUID
  and CAD entity links, stored point/label styles, link reconciliation, Point Manager filtering,
  selection/location, editing, repair, renumbering, deletion and deterministic CSV export. Every
  mutation rolls back atomically and becomes one Undo step.
- **Current priority:** finish the CAD-independent v0.4.0 engine while upstream reviews the
  CADCraft extension host. Native layers, symbols, feature entities and UI remain gated; see
  [HOST_COMPATIBILITY.md](HOST_COMPATIBILITY.md).
- **Host dependency:** installation currently requires the unreleased generic schema 2 capability
  from CADCraft PR #1384; SurveyCraft itself remains a separate repository and package.

## Product decisions

- CADCraft already owns generic CAD utilities; SurveyCraft will not duplicate them merely for
  command-count parity. Each required workflow still needs a documented, tested native equivalent
  or an identified SurveyCraft gap.
- Network imagery, device communication and country-specific cadastral generation remain optional
  modules so the core extension stays portable and easy to audit. Their workflow coverage remains
  in scope, with public standards, provider terms and hardware availability tracked as dependencies.
- Terrain visualization should use CADCraft's renderer. Any user-visible 3D exchange requirement,
  including `.3ds`, must be investigated and tracked explicitly; it is not silently excluded merely
  because it is outside the core surveying engine. Use public format specifications for implementation.
- Proprietary symbol/template/data libraries are replaced with original open packs or user-supplied
  content.
