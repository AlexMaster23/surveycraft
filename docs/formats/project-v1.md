# SurveyCraft project JSON v1

The Phase 0 project file is UTF-8 JSON. The canonical CLI output is pretty-printed with two-space
indentation and a final newline. Property order in external files is not significant.

## Root object

| Property | Type | Meaning |
|---|---|---|
| `schemaVersion` | integer | Must be `1`. |
| `id` | UUID | Stable project identity. |
| `name` | string | Non-empty display name, at most 256 UTF-8 bytes. |
| `crs` | object | Explicit coordinate-reference metadata. |
| `linearUnit` | string | `metre`, `millimetre`, `international-foot`, or `us-survey-foot`. |
| `numericPolicy` | object | Explicit calculation and display tolerances. |
| `points` | array | Survey points in deterministic import order. |

Every point has a UUID, number, validated finite 3D position, source row, optional code and
description, and optional quality flags. The CSV importer derives the point UUID from the project
UUID, source row, and point number using UUID v5. Re-importing the same data therefore produces the
same identities and byte-for-byte canonical JSON.

Readers must validate the model after deserialization. Unknown future schema versions are rejected
instead of being interpreted approximately.
