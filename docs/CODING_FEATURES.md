# v0.4 coding and symbol behavior contract

This document separates reference evidence from SurveyCraft design. The reference column is an
independent summary of static TopoLT 17.2 help/menu evidence. No reference command was executed in
this audit, so exact runtime equivalence remains unknown. The SurveyCraft column is the public,
testable contract implemented from scratch in `surveycraft-coding`.

## Command mapping and options

| Reference action | Static evidence and exposed options | SurveyCraft v0.4 contract | Current boundary |
|---|---|---|---|
| `EDITCOD` | Select and edit the project code-mapping file. Entries associate code, description, priority and graphical behavior. | Open or validate one strict JSON/TOML code book. Code rules, layer references, matching precedence and graphics are one versioned model. | Native manager add/edit/delete, unsaved preview, drawing persistence and JSON export work on the tested custom host. SurveyCraft intentionally does not read the proprietary format. |
| `EDITLAY` | Select and edit the project layer-definition file used by point and drawing workflows. | Layers are embedded in the open code book with stable ID, name, RGB color and plot flag. | Schema validation and mapped CAD layer names work. A separate native layer-definition editor is not included in v0.4. |
| `JOINPCT` | Order points by point number; interpret start/end/close, arc and three-point-circle directives; optionally join without an explicit start; then apply the code-derived line style. | `generate_features` emits deterministic line, three-point arc and three-point circle plans. Options are plan scale, positive coordinate tolerance, and `includeUnmarked`. | Native line/arc/circle creation, stable provenance metadata, deterministic replacement and one-step Undo work on the tested custom host. |
| `SELCOD` | Match a chosen code, with leading/trailing wildcard forms, and select the point entity or the point plus its texts. | `select_points` accepts `ABC`, `ABC*`, `*ABC` or `*ABC*`, case-normalized by the code book, and returns stable point UUIDs. | Engine selection works. Mapping UUIDs to native point/label selections is pending. |
| `CHCOD` | Replace the first point code associated with selected linear-entity endpoints while preserving later codes. | `change_first_code` replaces only the first token for an explicit UUID set and preserves suffix tokens and separators. Empty, oversized, multi-token or directive replacements are rejected. | Engine mutation works. Discovering endpoint-linked points from CAD selection and transactionally rewriting labels is pending. |
| `IB` | Insert all mapped punctiform symbols or one chosen symbol; inspect multi-code values; avoid the same symbol twice at one point; size from plan and rule scale. | Feature planning emits one symbol plan per point and symbol ID, supports a following attribute token, and applies `planScale / 1000 * ruleScale`. | The original attributed six-symbol pack is inserted as editable native line/circle primitives plus native polygonal solid fills. Explicit or layer-inherited color and paper lineweight are applied. Native reusable block definitions and rendered suffix attributes remain future work. |
| `MSCBL` | Rescale all mapped symbols or a selected symbol, using plan/rule scale and a user multiplier. | `rescale_symbols` targets all, symbol ID or rule ID and requires a finite positive factor. | Engine mutation works; CAD selection/update is pending. |
| `ROTBL` | Rotate all mapped symbols or one symbol using a topographic bearing; reference zero is north and increases clockwise in world coordinates. | `rotate_symbols` accepts degrees or gon and converts bearing to a normalized CAD-world angle. | Engine mutation works; CAD selection/update is pending. |
| `DELBL` | Delete all mapped symbols or one selected symbol kind. | `delete_symbols` targets all, symbol ID or rule ID without affecting other planned symbols. | Engine mutation works; transactional CAD deletion is pending. |
| `CHL` | Choose a style for line/arc/circle/polyline geometry from the codes at its points; majority is used, and a two-code conflict uses code priority. | `choose_line_style` counts matched line rules, then resolves ties by optimization rank and stable rule ID. It returns layer, style, paper width and match counts. | Generated geometry uses the selected layer, applies layer color and paper lineweight, and stores its rule provenance. Native linetype mapping and restyling arbitrary existing geometry remain future work. |

## Synthetic expected results

The tests and fixtures are original SurveyCraft data. They establish these deterministic results:

| Case | Input | Expected result |
|---|---|---|
| Multi-code directive | `edg-sta` with case-insensitive rules | Primary `edge` rule and `start` directive. |
| Symbol suffix | `pin-17` with suffix attributes enabled | One `surveycraft.pin` plan whose attribute is `17`. |
| Wildcard selection | `TREE*` over `TREE-OAK` and `EDG-START` | Select only the tree point. |
| First-code change | Replace `TREE` with `PIN` in `TREE-OAK` | `PIN-OAK`; later token remains unchanged. |
| Ordered closed path | Point numbers 1–4 with start, arc-start, arc-end and close | One closed path: line, three-point arc, closing line. |
| Degenerate arc | Three collinear points carrying arc start/end | One line from arc start to arc end, not invalid arc geometry. |
| Circle | Three same-rule points carrying `ON_CIRCLE` | One three-point circle plan. |
| Symbol sizing | Tree rule scale 1.25 at 1:2000 | Initial symbol scale `2.5`. |
| Bearing | `100 gon` | CAD world rotation `0°`; `0 gon` becomes `90°`. |
| Style tie | Equal `edge` and `boundary` point counts, ranks 1 and 2 | Select `edge`, the lower-rank-number rule. |

JSON/TOML equivalence, strict schema validation and stable interpretations are exercised by
`crates/io/tests/code_book.rs`. Geometry, matching, mutation and symbol behavior are exercised by
the `surveycraft-coding` unit tests.

The custom-host adapter is exercised by transaction, replacement, metadata, editor-preview and
native curve tests in `surveycraft-cadcraft` and CADCraft's WASM runtime. The user workflow and
current limits are documented in [CODE_BOOK_WORKFLOW.md](CODE_BOOK_WORKFLOW.md).

## Unknowns requiring black-box comparison

- exact dialog defaults, selection prompts and persistence rules in the reference product;
- precise handling of nonnumeric mixed point numbers and interleaved code sequences;
- CAD entity metadata, layer side effects and duplicate detection across pre-existing blocks;
- arc/circle tolerance and failure messages;
- symbol anchor points, attribute rendering and host-specific block units;
- line-style behavior for geometry whose vertices are not linked to survey points.

These unknowns are not filled in by guesswork. SurveyCraft's documented behavior remains stable,
and later lawful black-box tests may add compatibility options without importing vendor code,
configuration or assets.
