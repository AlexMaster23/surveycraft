# Changelog

All notable SurveyCraft changes are recorded here. Versions follow Semantic Versioning while the
project is pre-1.0; compatibility and migration notes are included whenever persisted formats or
CADCraft host requirements change.

## Unreleased

- Next target: v0.3.0 Point Manager and round-trip editing.

## 0.2.0 — 2026-10-16

- Added project JSON schema v2 with explicit v1 migration and persisted point/label styles.
- Added stable drawing-project and point UUIDs with exact CAD point/label entity links.
- Added namespaced drawing and entity metadata through the generic CADCraft extension host.
- Added save/reopen reconciliation for missing, externally edited and orphaned entities.
- Added the `surveycraft.points.check-links` command, `SCCHECKPOINTS` alias and Tool Sets button.
- Added duplicate-number protection across multiple imports into the same drawing.

## 0.1.0 — 2026-10-16

- Added the standalone Rust workspace, clean-room rules and open-source licensing.
- Added deterministic point CSV/project parsing, CLI validation and golden fixtures.
- Added the native manual/CSV point-import table.
- Added CAD/GIS and Survey/TopoLT coordinate-convention selection.
- Added transactional CAD point, number, elevation and optional code insertion.
- Added the native SurveyCraft Tool Sets entry and standalone extension package.
