# SurveyCraft release roadmap

SurveyCraft ships in small, usable versions. A version is reached only when every item in its exit
criteria is implemented, documented, tested and available in an installable
`.cadcraft-extension` package. Dates are intentionally not promised before the required CADCraft
host capabilities exist.

The product phases and clean-room engineering constraints remain defined in
[BLUEPRINT.md](BLUEPRINT.md). GitHub milestones mirror the versions below.

## Release train

| Version | Product phase | Status | Usable outcome |
|---|---|---|---|
| [v0.1.0](https://github.com/AlexMaster23/surveycraft/releases/tag/v0.1.0) | Phase 1 preview | Released | Import manual/CSV points with an explicit coordinate convention and insert native point/label entities in one undo step. |
| [v0.2.0](https://github.com/AlexMaster23/surveycraft/releases/tag/v0.2.0) | Phase 1 | Released | Persist stable survey-point identities and CAD entity links across save/reopen. |
| [v0.3.0](https://github.com/AlexMaster23/surveycraft/releases/tag/v0.3.0) | Phase 1 | Released | Manage, edit, renumber, delete, select, locate, repair and export existing survey points. |
| v0.4.0 | Phase 1 | Next | Apply open code-book rules to layers, labels, feature lines and original symbols. |
| v0.5.0 | Phase 1 beta | Planned | Diagnose point quality and provide the first practical COGO tool set. |
| v0.6.0 | Phase 2 | Planned | Discover and validate parcels; create linked area/perimeter labels and tables. |
| v0.7.0 | Phase 2 | Planned | Divide parcels to exact target areas and export parcel/room reports. |
| v0.8.0 | Phase 3 | Planned | Build and inspect bounded TIN surfaces and generate styled contours. |
| v0.9.0 | Phase 3 | Planned | Create profiles/cross-sections and audited cut/fill volume reports. |
| v1.0.0 | Stable | Planned | Deliver the supported points-to-parcels-to-terrain workflow on a normal stable CADCraft release. |
| v1.1.0 | Phase 4 | Planned | Add explicit CRS metadata, audited transforms and KML/GeoJSON exchange. |
| v1.2.0 | Phase 4 | Planned | Place and manage georeferenced rasters with standard affine/world-file metadata. |
| v1.3.0 | Phase 5 | Planned | Add grids, frames, named sheet views and repeatable batch output. |
| v1.4.0 | Phase 5 | Planned | Add opt-in instrument transfer through open, independently testable profiles. |
| v1.5.0 | Phase 5 | Planned | Publish the jurisdiction-pack interface while keeping the core country-neutral. |

## Version exit criteria

### v0.1.0 — point import preview (released)

- manual and CSV/COO/TXT/XYZ coordinate tables;
- required, numeric and duplicate-number validation;
- CAD/GIS and Survey/TopoLT axis conventions;
- native CAD point, number, elevation and optional code entities;
- one transaction and one Undo step;
- standalone package, documentation and synthetic regression tests.

### v0.2.0 — persistent point model (released)

- versioned `SurveyProject` metadata with explicit migrations;
- stable UUID for every survey point;
- links from one logical point to its CAD point and label entities;
- save/reopen and reconciliation tests, including missing or externally edited entities;
- initial point-style and label-style settings;
- generic CADCraft host support for bounded extension metadata and entity query/update/delete.

Exit: importing, saving and reopening preserves point identity and relationships without relying on
layer names or geometry guesses.

### v0.3.0 — Point Manager and round-trip editing (released)

- native Point Manager with virtualized rows, filtering and quality/status columns;
- select and zoom to drawing points;
- edit coordinates, elevation, number, code and description;
- batch renumber and delete, with duplicate prevention;
- synchronized geometry/label updates in one undoable transaction;
- deterministic export of all or selected points to CSV.

Exit: users can round-trip a point job without reimporting or losing identity.

### v0.4.0 — coding and feature generation

- documented JSON/TOML code-book format with schema validation;
- prefix/regex rules mapping codes to layers, styles and geometry kinds;
- stable feature lines referencing point UUIDs;
- live preview and structured diagnostics for unknown or conflicting codes;
- an original, attributed symbol-pack interface with no proprietary assets.

Exit: a coded field file generates predictable, editable CAD linework.

### v0.5.0 — point quality and COGO beta

- duplicate and near-point diagnostics with selectable, non-destructive fixes;
- point-label visibility, placement and style controls;
- inverse distance/direction, polar/traverse point, line/circle intersections;
- coordinate listing and stakeout report;
- Phase 1 end-to-end workflow and migration tests.

Exit: a surveyor can import, repair, code, edit, calculate and export a complete point job.

### v0.6.0 — parcels and linked areas

- robust closed-boundary discovery with gap/self-intersection diagnostics;
- explicit area/perimeter units and rounding;
- linked labels and tables that report stale boundaries and recalculate after edits;
- synthetic parcel golden tests and one-step undo.

### v0.7.0 — parcel division and reports

- target-area splits by parallel/perpendicular direction, proportion, point/opening and offset;
- preview, residual and tolerance report before commit;
- start-vertex/direction normalization;
- parcel and room tables plus CSV/PDF-ready report data.

### v0.8.0 — terrain and contours

- deterministic constrained Delaunay TIN with outer boundaries, holes and breaklines;
- surface inspection, exclusions, clipping and bounded editing;
- major/minor contours with labels and explicit optional smoothing;
- analytical and public-mesh golden tests.

### v0.9.0 — profiles and volumes

- elevation queries and 2D-to-surface projection;
- longitudinal profiles and cross-sections;
- surface/surface and surface/reference-plane cut/fill;
- signed volume, balance, tolerance and audit tables.

### v1.0.0 — first stable release

- v0.1.0 through v0.9.0 workflows work on the latest stable CADCraft without a custom build;
- supported project/package migrations and recovery diagnostics;
- complete command, automation and accessibility coverage for the stable feature set;
- reproducible package, user documentation and upgrade/uninstall instructions;
- no known data-loss or crash-class defects in supported workflows.

Post-1.0 milestones add CRS/geographic exchange, georeferenced rasters, sheet production,
instrument profiles and the country-neutral jurisdiction-pack interface described in the table.

## Release policy

Every reached version receives:

1. a completed GitHub milestone and updated roadmap/status documentation;
2. a SemVer version update in the Rust workspace and extension manifest;
3. a clean full quality gate and end-to-end installation test;
4. a version tag named `vMAJOR.MINOR.PATCH`;
5. a GitHub Release containing release notes, `surveycraft.cadcraft-extension` and its SHA-256 file;
6. upgrade and compatibility notes, including the minimum CADCraft version.

Tags matching `v*.*.*` run the repository's release workflow. Versions before `v1.0.0` are marked
as prereleases. Patch versions contain compatible fixes; minor versions complete one roadmap
milestone; major versions may migrate stored project data or raise host requirements.
