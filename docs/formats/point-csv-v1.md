# Point CSV v1

Point CSV v1 is SurveyCraft's first deterministic interchange format. It is deliberately small so
that field exports can be validated before any drawing is modified.

## Encoding and structure

- UTF-8 text with one header row and comma-separated records.
- Header names are trimmed and compared case-insensitively.
- Required columns: `number`, `easting`, `northing`, `elevation`.
- Optional columns: `code`, `description`.
- Decimal separator is `.`; thousands separators are not accepted.
- Coordinates must be finite IEEE-754 decimal values. `NaN` and infinity are rejected.
- Blank rows and malformed records produce diagnostics rather than CAD entities.

Coordinates use the project unit and CRS metadata supplied by the caller. The importer never
guesses a unit or transforms coordinates.

## Safety limits

| Limit | Value |
|---|---:|
| Input size | 64 MiB |
| Data rows | 1,000,000 |
| Individual field | 64 KiB |

Rows that exceed a field limit are skipped. Files that exceed the input or row limit are rejected.
Duplicate point numbers are retained for inspection, marked with `duplicate-number`, and reported
as validation errors.
