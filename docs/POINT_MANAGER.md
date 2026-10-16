# SurveyCraft Point Manager

SurveyCraft v0.3.0 adds a native CADCraft manager for the survey points already linked to the
current drawing. Open it from the orange **SurveyCraft** group in Drafting Tool Sets, choose
**SurveyCraft > Points > Point Manager**, or type `SCPOINTMANAGER` or `SCPM`.

The manager is rendered by CADCraft from bounded extension metadata. SurveyCraft supplies point
data and handles actions through its sandboxed WebAssembly command; it does not load a native UI
library or receive ambient file-system access.

## Columns and filtering

| Column | Editable | Meaning |
|---|---:|---|
| Point number | yes | Required, case-insensitively unique identifier |
| CAD X / Easting | yes | Native drawing X coordinate |
| CAD Y / Northing | yes | Native drawing Y coordinate |
| Elevation | yes | Native drawing Z and elevation-label value |
| Code | yes | Optional field code and code-label value |
| Description | yes | Optional project description; not drawn as a separate label |
| Quality | no | Current validation result |
| Link status | no | `Healthy`, `Missing` or `Modified` after comparing linked CAD entities |

The filter matches point number, code, description and status. The host virtualizes visible rows
and bounds one manager response to 2,000 points. Large file-level validation remains available
through SurveyCraft's core and CLI.

## Drawing and editing actions

- **Select in drawing** selects every existing CAD entity linked to the checked rows.
- **Locate** selects those entities and runs CADCraft's object zoom.
- **Save changes** validates and submits changed rows only. It updates the CAD point, number,
  elevation and optional code labels while retaining the point UUID.
- **Synchronize selected** explicitly repairs missing or modified linked entities from the stored
  point record. Newly created replacement entities receive fresh handles and the same point UUID.
- **Renumber** applies `prefix + number` in deterministic current-point-number order. Start must be
  non-negative and step must be positive.
- **Delete selected** requires a second confirmation click and removes the logical records plus all
  existing linked entities.

Point numbers are compared case-insensitively. Required fields, finite numeric values, field sizes
and duplicates are validated before drawing changes begin. A mutation is limited to 500 selected
points so the host operation budget remains bounded.

Each save, synchronize, renumber or delete action uses one guarded CADCraft transaction. On any
error it rolls back completely; on success one Undo reverses the whole action.

## CSV export

Use **Export all...** or **Export selected...**. CADCraft owns the save dialog and writes the text;
the WebAssembly extension only returns a suggested name and bounded UTF-8 content. Selected export
requires at least one checked row.

The deterministic output uses CRLF records and these columns:

```csv
number,easting,northing,elevation,code,description
```

Rows use numeric-aware point-number ordering, coordinates use stable finite decimal formatting and
CSV quoting follows RFC 4180. Export never applies the point-import axis swap: `easting` is stored
CAD X and `northing` is stored CAD Y. Empty selection at the engine/API layer means all points.

## Link status and recovery

`Healthy` means all expected entities, values, layers and SurveyCraft metadata match the stored
record. `Missing` means at least one linked handle no longer exists. `Modified` means a linked
entity exists but its kind, geometry, text, layer or metadata differs. Use synchronization only
when the SurveyCraft record should be authoritative; otherwise edit the Point Manager fields and
save. The separate `SCCHECKPOINTS` report also identifies orphaned tagged entities.

Saving and reopening DXF/DWG through a compatible CADCraft build preserves point UUIDs, links and
project data. SurveyCraft v0.3.0 requires the schema 2 extension host and manager UI currently
implemented by [CADCraft PR #1384](https://github.com/storytold/cadcraft/pull/1384).
