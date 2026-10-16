# Import and insert survey points

SurveyCraft's first CAD workflow turns a reviewed coordinate table into native CADCraft drawing
entities. It is available from **SurveyCraft > Points > Import and insert points...** and from the
`SCPOINTS` or `SCPIMPORT` command aliases.

## Table columns

| Column | Required | Meaning |
|---|---:|---|
| Point number | yes | Unique text identifier; matching is case-insensitive |
| Northing (Y) | yes | Drawing Y coordinate |
| Easting (X) | yes | Drawing X coordinate |
| Elevation (Z) | yes | Drawing Z coordinate and elevation-label value |
| Code / description | no | Short field code or point description |

Coordinates must be finite numbers. Decimal dots are accepted in every file; decimal commas are
also accepted when the file uses semicolons or tabs as its field delimiter. A maximum of 2,000
points is accepted per insertion.

## CSV and text input

Use **Open CSV / text** in the dialog to select a UTF-8 `.csv`, `.coo`, `.txt` or `.xyz` file.
Comma, semicolon and tab delimiters are detected automatically. Headers can use the labels above or
common aliases such as `number`, `id`, `nr`, `northing`, `n`, `y`, `easting`, `e`, `x`, `elevation`,
`z`, `h`, `cota`, `code`, `description` and `cod`. Header matching is case-insensitive. Romanian
`numar`/`număr` and `cota`/`cotă` are supported.

Example:

```csv
number,northing,easting,elevation,code
101,300000.500,500000.250,100.125,STN
102,300010.000,500015.500,100.310,EDGE
```

Files without a recognized header use the dialog's declared order: point number, Northing,
Easting, elevation, code. Review that mapping in the table before insertion.

## Drawing output

Each accepted row creates one native CAD point and text labels on predictable layers:

| Layer | Entity |
|---|---|
| `SC-POINTS` | Point at `(Easting, Northing, Elevation)` |
| `SC-POINT-NUMBERS` | Point-number text |
| `SC-POINT-ELEVATIONS` | Elevation text rounded to three decimals |
| `SC-POINT-CODES` | Optional code/description text |

SurveyCraft creates missing layers through the CADCraft host. All entities from one submission are
staged together. If any host operation fails, nothing is added; after success, one Undo removes the
complete insertion.

## Current MVP limits

- Input text must be UTF-8.
- Labels use a fixed default height and fixed SurveyCraft layers.
- Points do not yet carry persistent SurveyCraft identity metadata.
- Editing, renumbering, point styles and code-book rules belong to the upcoming Point Manager.

These limits are explicit so drawings created by the MVP remain understandable and reversible.
