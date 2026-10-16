# SurveyCraft code book v1

The SurveyCraft code book is an original, open JSON/TOML format for turning point codes into
deterministic feature and symbol plans. It is not compatible with, derived from, or a copy of any
vendor configuration file. JSON and TOML carry the same data model; unknown properties are
rejected so spelling mistakes cannot silently change a drawing result.

The readable synthetic examples are
[`synthetic-field.json`](../../tests/fixtures/codebooks/synthetic-field.json) and
[`synthetic-field.toml`](../../tests/fixtures/codebooks/synthetic-field.toml).

## Root properties

| Property | Type | Contract |
|---|---|---|
| `schemaVersion` | integer | Must be `1`. |
| `name` | string | Non-empty display name, at most 256 UTF-8 bytes. |
| `caseSensitive` | boolean | Defaults to `false`; affects codes, directives and regular expressions. |
| `tokenSeparators` | character array | One to eight unique non-alphanumeric separators; defaults to `-`. |
| `directives` | object | Optional aliases for path and circle control tokens. |
| `layers` | array | Optional named drawing-layer definitions. |
| `rules` | array | Code matching and graphical-result rules. |

Inputs are limited to 8 MiB, 1,024 layers and 4,096 rules. A point code is limited to 4 KiB and
64 tokens. Feature planning accepts at most 1,000,000 points. Identifiers contain only ASCII
letters, digits, `.`, `_` or `-`. Colors use `#RRGGBB`; every scale and paper width must be finite
and positive.

## Layers and rules

A layer has an `id`, CAD-facing `name`, optional RGB `color`, and `plot` flag. A rule contains:

- a stable `id`, description, optional layer ID and `optimizationRank` from 1 (most important) to
  9 (least important);
- a `pattern` and `matchMode` of `exact`, `prefix` or `regex`;
- a `graphic` with `kind` equal to `none`, `line` or `symbol`.

Line graphics define `style` and `paperWidthMm`. Symbol graphics define the original symbol-pack
identifier, a positive `scale`, and `attributeFromSuffix`. When the latter is true, the next
non-directive token becomes the symbol attribute instead of another code.

If several rules match one token, SurveyCraft uses exact before prefix before regex, then lower
optimization rank, then lexical rule ID. A structured `ambiguous-code` warning records the other
matches. A non-directive token with no match produces `unknown-code`.

## Join directives

The default aliases are `START`, `END`, `CLOSE`, `ARC_START`, `ARC_END` and `ON_CIRCLE`; a code book
may replace or extend each alias list. Aliases are reserved and cannot also be used as replacement
codes.

Feature planning sorts points by numeric point number where possible, with a stable lexical
fallback. It then applies these rules:

- `START` begins a path; another `START` finishes the current path and begins another one.
- `END` finishes an open path. `CLOSE` finishes it and adds the last-to-first segment.
- `ARC_START`, one following point, and a later `ARC_END` produce a three-point arc. Additional
  intermediate points are ignored for the arc definition. A missing end falls back to ordinary
  line segments; a repeated or collinear three-point arc becomes a line.
- Every three same-rule points carrying `ON_CIRCLE` produce one three-point circle. Incomplete or
  degenerate groups produce diagnostics instead of invalid geometry.
- `--join-unmarked` additionally joins consecutive same-rule points that were not consumed by an
  explicit path. It is off by default.

The result is a CAD-independent `FeaturePlan`: stable path/circle/symbol IDs, source point UUIDs,
layer/style data and diagnostics. CAD insertion remains an adapter responsibility and must occur
in one host transaction.

## Symbols and line styles

Symbol placement suppresses the same symbol ID appearing more than once on one point. The initial
model-space scale is:

```text
plan-scale denominator / 1000 * rule scale
```

For example, a rule scale of `1.25` at 1:2000 produces `2.5`. Rescale applies a positive user
factor. Rotation input is a topographic bearing—north is zero and values increase clockwise—and is
converted to the CAD world angle. Degrees and gon are supported. Delete, rescale and rotate may
target all symbols, one symbol ID or one code-rule ID.

When applying a line style to existing geometry, SurveyCraft counts the line rule at each coded
point. The majority wins; ties use the lower optimization rank and then the stable rule ID.

## CLI validation and preview

The CLI provides a host-independent way to check code books and results:

```text
cargo run -p surveycraft-cli -- codebook validate tests/fixtures/codebooks/synthetic-field.json
cargo run -p surveycraft-cli -- codebook preview tests/fixtures/codebooks/synthetic-field.toml edg-sta
cargo run -p surveycraft-cli -- codebook generate book.json project.json --output plan.json --plan-scale 1000
```

`generate` validates the current project schema, uses its coordinate tolerance, and writes a
pretty-printed JSON plan. Add `--join-unmarked` only when implicit consecutive joining is intended.
