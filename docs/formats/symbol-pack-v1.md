# SurveyCraft symbol pack v1

Symbol pack v1 is an original, portable JSON format for small surveying symbols. Packs contain
vector instructions and attribution, not DWG blocks or copied vendor resources. Unknown fields are
rejected and every numeric value is bounded before a pack can enter the catalog.

The bundled example and initial open library is
[`SurveyCraft Basic Symbols`](../../assets/symbols/surveycraft-basic.symbol-pack.json). Its six
symbols—tree, survey pin, survey station, utility pole, manhole and benchmark—were drawn for this
project and are licensed under `MIT OR Apache-2.0` with the rest of SurveyCraft.

## Pack object

| Property | Contract |
|---|---|
| `schemaVersion` | Must be `1`. |
| `id` | Stable ASCII pack identifier. |
| `name`, `version` | Bounded, printable identity fields. |
| `license.spdx` | SPDX license expression displayed to users. |
| `license.notice` | Required origin/attribution notice. |
| `authors` | One to 32 bounded author names. |
| `homepage` | Optional bounded project URL. |
| `symbols` | One to 512 unique vector definitions. |

A catalog may combine multiple packs, but pack IDs and symbol IDs must remain globally unique.
Code-book validation reports every referenced symbol that is not present in the catalog.

## Symbol definitions

Each definition has a stable `id`, display `name`, insertion `anchor`, and one to 256 primitives.
Supported primitives are:

- `line`: two distinct points, paper line width and optional RGB color;
- `circle`: center, positive radius, paper line width, optional fill and color;
- `polyline`: at least two points, or three when closed, plus line width, fill and color.

An omitted color inherits the target CAD layer/entity color. Colors use `#RRGGBB`. A primitive may
contain at most 1,024 points; local coordinates are finite and limited to ±10,000. Filled geometry
is declarative—the CAD adapter decides which supported native fill representation to use while
preserving the visible outline.

## Units and placement

One local vector unit is one model metre at the 1:1000 reference scale, equivalent to one paper
millimetre. The code-book engine produces this scale:

```text
plan scale denominator / 1000 * code-rule scale
```

Placement then divides by the project's metres-per-drawing-unit conversion, rotates around the
symbol anchor using the normalized CAD-world angle, and translates to the survey point. Thus the
same definition has the correct model size in metre, millimetre, international-foot and
US-survey-foot projects. `strokeWidthMm` remains a paper line-weight value rather than a model-space
distance.

Placement first produces CAD-independent line, circle and polyline geometry. The current CADCraft
adapter then creates editable native line/circle primitives, maps closed filled primitives to
polygonal `HATCH` entities, applies explicit or layer-inherited color and paper lineweight, assigns
stable metadata, and commits the complete generation as one Undo step. Reusable native block
definitions remain future work.

## Validation tools

```text
cargo run -p surveycraft-cli -- symbolpack validate assets/symbols/surveycraft-basic.symbol-pack.json
cargo run -p surveycraft-cli -- symbolpack inspect assets/symbols/surveycraft-basic.symbol-pack.json surveycraft.tree
cargo run -p surveycraft-cli -- symbolpack check-codebook tests/fixtures/codebooks/synthetic-field.json assets/symbols/surveycraft-basic.symbol-pack.json
```

The extension packager validates the bundled symbol pack and includes it at
`assets/symbols/surveycraft-basic.symbol-pack.json` inside the `.cadcraft-extension` archive. It
also validates that the bundled country-neutral starter code book resolves every referenced
symbol before packaging.
