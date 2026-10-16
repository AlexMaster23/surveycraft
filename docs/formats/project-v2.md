# SurveyCraft project JSON v2

Project JSON v2 is the portable, CAD-independent SurveyCraft project model. It is UTF-8 JSON; the
canonical CLI output is pretty-printed with two-space indentation and a final newline.

## Root object

| Property | Type | Meaning |
|---|---|---|
| `schemaVersion` | integer | Must be `2`. |
| `id` | UUID | Stable project identity. |
| `name` | string | Non-empty display name, at most 256 UTF-8 bytes. |
| `crs` | object | Explicit coordinate-reference metadata. |
| `linearUnit` | string | `metre`, `millimetre`, `international-foot`, or `us-survey-foot`. |
| `numericPolicy` | object | Explicit calculation and display tolerances. |
| `styles` | object | Initial point and label style settings. |
| `points` | array | Survey points in deterministic import order. |

`styles.point.layer` names the point layer. `styles.labels` contains the number, elevation and
code layers, label height, X/Y offsets and elevation decimal count. All numbers must be finite;
label height is positive and the decimal count is between 0 and 12.

Every point has a stable UUID, number, finite 3D position, source row, optional code and
description, and optional quality flags. The drawing adapter stores CAD entity handles separately
from this domain model so project calculations remain independent of one CAD host.

## Migration

The reader accepts schema v1 and migrates it in memory to v2 by adding the documented default
styles. Unknown future versions are rejected. Writing always produces the current v2 form and
never rewrites the only source file implicitly.
