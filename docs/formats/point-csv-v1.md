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

## Point Manager export

SurveyCraft v0.3 uses the same six canonical columns for deterministic manager export:
`number,easting,northing,elevation,code,description`. Records are ordered using numeric-aware point
number comparison, encoded as UTF-8 with CRLF line endings and quoted using RFC 4180 rules. Export
coordinates are stored CAD X/Easting and CAD Y/Northing values; no import-axis convention is
reapplied. The native manager limits returned text to 8 MiB.
