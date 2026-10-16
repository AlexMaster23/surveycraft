# Changelog

All notable SurveyCraft changes are recorded here. Versions follow Semantic Versioning while the
project is pre-1.0; compatibility and migration notes are included whenever persisted formats or
CADCraft host requirements change.

## Unreleased

- Next target: v0.4.0 coding and feature generation.
- Added a bounded CAD-independent code-book engine with exact, prefix and regular-expression
  matching, deterministic precedence and structured diagnostics.
- Added point-code selection/replacement, ordered line/arc/circle planning, line-style decisions,
  and symbol plan insertion, scaling, topographic rotation and deletion.
- Added strict equivalent JSON/TOML code-book readers, synthetic fixtures, CLI
  `codebook validate`, `preview` and `generate` commands, and the v0.4 behavior contract.

## 0.3.0 — 2026-10-16

- Added the native, virtualized Point Manager with filtering and point/link health columns.
- Added drawing selection and location for managed point entities.
- Added round-trip editing of number, CAD coordinates, elevation, code and description while
  preserving stable point UUIDs.
- Added deterministic batch renumbering and duplicate-number prevention.
- Added explicit synchronization that repairs missing or externally modified point/label entities.
- Added linked deletion with two-step UI confirmation.
- Added deterministic CSV export for all or selected points.
- Added generic host-rendered extension data managers and host-owned text-file export to CADCraft
  PR #1384.
- Verified package installation, save/reopen persistence, repair, editing, renumbering, export and
  deletion against a real CADCraft drawing.

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
