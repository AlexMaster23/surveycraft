# Import and insert survey points

SurveyCraft's first CAD workflow turns a reviewed coordinate table into native CADCraft drawing
entities. It is available from the orange-marked **SurveyCraft** group at the top of the Drafting
Tool Sets palette, from **SurveyCraft > Points > Import and insert points...**, and from the
`SCPOINTS` or `SCPIMPORT` command aliases. Every entry point starts the same command and native
table.

## Table columns

| Column | Required | Meaning |
|---|---:|---|
| Point number | yes | Unique text identifier; matching is case-insensitive |
| Input X | yes | X value exactly as named by the source file or survey convention |
| Input Y | yes | Y value exactly as named by the source file or survey convention |
| Elevation (Z) | yes | Drawing Z coordinate and elevation-label value |
| Code / description | no | Short field code or point description |

Choose the source's coordinate convention explicitly above the table:

- **CAD / GIS** keeps the axes: input X becomes CAD X and input Y becomes CAD Y. Use this for
  ordinary Cartesian X/Y and for Easting/Northing data.
- **Survey / TopoLT** swaps the named survey axes: input X becomes CAD Y and input Y becomes CAD X.
  Use this for Romanian Stereo 70 / TopoLT-style X/Y files.

The default is CAD / GIS because it is the international Cartesian convention. The current choice
is submitted with the rows, so previewed values cannot be silently interpreted by another mode.

Coordinates must be finite numbers. Decimal dots are accepted in every file; decimal commas are
also accepted when the file uses semicolons or tabs as its field delimiter. A maximum of 2,000
points is accepted per insertion.

## CSV and text input

Use **Open CSV / text** in the dialog to select a UTF-8 `.csv`, `.coo`, `.txt` or `.xyz` file.
Comma, semicolon and tab delimiters are detected automatically. Headers can use the labels above or
common aliases such as `number`, `id`, `nr`, `x`, `easting`, `e`, `y`, `northing`, `n`, `elevation`,
`z`, `h`, `cota`, `code`, `description` and `cod`. Header matching is case-insensitive. Romanian
`numar`/`număr` and `cota`/`cotă` are supported. `Easting` maps to input X and `Northing` maps to
input Y; literal `X` and `Y` headers remain literal and are interpreted by the selected convention.

Example:

```csv
number,Y,X,elevation,code
L1,595286.485,337275.053,0.000,
L2,595309.544,337337.121,0.000,
```

Select **Survey / TopoLT** for that example. Files without a recognized header use the dialog's
declared order: point number, input X, input Y, elevation, code. Review both the values and the
coordinate convention before insertion.

## Drawing output

Each accepted row creates one native CAD point and text labels on predictable layers:

| Layer | Entity |
|---|---|
| `SC-POINTS` | Point at the convention-mapped `(CAD X, CAD Y, Elevation)` |
| `SC-POINT-NUMBERS` | Point-number text |
| `SC-POINT-ELEVATIONS` | Elevation text rounded to three decimals |
| `SC-POINT-CODES` | Optional code/description text |

SurveyCraft creates missing layers through the CADCraft host. All entities from one submission are
staged together. If any host operation fails, nothing is added; after success, one Undo removes the
complete insertion.

## Persistent identity and link checks

Every inserted point receives a stable UUID. SurveyCraft stores the logical project, initial point
and label styles, and the exact CAD entity handles inside the drawing. Each point and label is also
tagged with its project UUID, point UUID and role. Saving and reopening a supported drawing keeps
these relationships intact.

Run **SurveyCraft > Points > Check point links**, press its Tool Sets button, or type
`SCCHECKPOINTS` to compare the stored project with the current CAD entities. The report distinguishes
missing entities, changed geometry/text/layers, damaged metadata and unlinked SurveyCraft entities.
It does not silently repair or overwrite external edits.

## Current MVP limits

- Input text must be UTF-8.
- Point and label styles are persisted but do not yet have a settings editor.
- Code-book rules and interactive point/label style controls remain planned.
- Use the v0.3 [Point Manager](POINT_MANAGER.md) for editing, renumbering, repair, deletion and
  export after insertion.

These limits are explicit so drawings created by the MVP remain understandable and reversible.
