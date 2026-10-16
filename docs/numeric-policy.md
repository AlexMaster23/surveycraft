# Numeric policy

Surveying results must not depend on undocumented global tolerances. Every SurveyCraft project
therefore records four values in its own linear unit:

| Field | Phase 0 default | Use |
|---|---:|---|
| `coordinateTolerance` | `1e-8` | Coordinate equality and normalization. |
| `distanceTolerance` | `1e-6` | Degenerate lengths and snapping decisions. |
| `areaTolerance` | `1e-6` | Degenerate rings and area comparisons. |
| `displayDecimals` | `3` | UI and report presentation only. |

Calculation tolerances must be finite and greater than zero. Display decimals must be between 0
and 12. Values are never rounded before a calculation; display rounding is presentation only.

Phase-specific algorithms may add named policies, but they must remain explicit, serialized, and
covered by boundary tests.
