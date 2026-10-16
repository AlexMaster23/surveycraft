# Code Book and coded-feature workflow

SurveyCraft v0.4 turns codes stored on persistent survey points into editable native CAD geometry.
It is a developer prerelease that requires the exact custom CADCraft host listed in
[HOST_COMPATIBILITY.md](HOST_COMPATIBILITY.md); stock CADCraft v0.7.0 cannot run this package.

## Quick start

1. Import coded points with **SurveyCraft > Points > Import and insert points...** (`SCPOINTS`).
2. Open **SurveyCraft > Coding > Code Book...** (`SCCODEBOOK` or `SCCB`).
3. Add or edit rules, then choose **Preview changes**. Preview evaluates the unsaved table against
   points in the active drawing without persisting it.
4. Resolve every validation error and choose **Save changes**. The validated code book is stored as
   namespaced drawing metadata.
5. Choose **SurveyCraft > Coding > Generate coded features** (`SCGENERATE` or `SCGEN`).

The first Code Book opening uses the country-neutral starter book. Its line codes are `EDG` and
`BND`; symbol codes are `TREE`, `PIN`, `STN`, `POLE`, `MH` and `BM`. Its mapped CAD layers are
`SC-FEATURES`, `SC-BOUNDARIES` and `SC-SYMBOLS`.

## Rule columns

- **Rule ID** is the stable unique rule identity.
- **Pattern** and **Match mode** define an `exact`, `prefix` or `regex` match.
- **Rank** is a positive precedence number; lower numbers win deterministic ties.
- **Layer ID** must refer to a layer in the book.
- **Graphic kind** is `none`, `line` or `symbol`.
- **Style / symbol** holds a line-style name or one of the bundled symbol IDs.
- **Scale / width** is the symbol scale or the line's paper width in millimetres.
- **Suffix attribute** accepts `yes` or `no` for symbol rules. For example, `PIN-17` may carry
  `17` in its plan metadata when suffix attributes are enabled.
- **Preview** is read-only and reports the number of drawing points matched by that rule.

Use **Add record** for a blank rule, **Delete** for selected rules, and **Export** for a
deterministic JSON copy. A saved book is always validated as a complete model, including its layer
and symbol references. The manager accepts at most 512 rules.

## Directives and output

Point codes use `-` as the starter book's token separator. `START`/`STA`, `END`, `CLOSE`,
`ARC_START`, `ARC_END` and `ON_CIRCLE` control ordered geometry. Rule matching is
case-insensitive in the starter book. The complete portable schema is documented in
[formats/code-book-v1.md](formats/code-book-v1.md).

Generation creates native CAD `Line`, `Arc` and `Circle` entities. Symbols from the original,
attributed SurveyCraft Basic Symbols pack are decomposed into editable native lines and circles,
with closed filled primitives represented by native polygonal `HATCH` entities. Explicit or
layer-inherited colors and paper lineweights are preserved. Symbol size follows
`plan scale / 1000 × rule scale`; the native command currently
uses a 1:1000 plan scale. The programmatic command accepts, for example,
`{"planScaleDenominator":500}`.

Every generated primitive stores its stable feature ID, matched rule ID, source point UUIDs and
role as extension metadata. Running generation again removes the previous SurveyCraft-coded output
and writes the new result in the same transaction. The complete regeneration is therefore one Undo
step; point markers and labels are not replaced.

## Diagnostics and v0.4 limits

Unknown codes, conflicting matches and degenerate geometry are returned as structured diagnostics.
The command reports at most 200 diagnostic entries and refuses a plan requiring more than 4,000
native entities. Errors before commit roll back the transaction, preserving the previous output.

The adapter applies native geometry, mapped layers, colors, paper lineweights and solid fills.
Direct CAD linetype mapping, reusable block definitions, rendered suffix attributes, a separate
layer editor and operations on arbitrary non-SurveyCraft geometry are later work. These limitations
are explicit; they are not claimed as reference-product parity.
