# SurveyCraft WebAssembly extension

This portable guest contains SurveyCraft's CAD-facing commands. Its point-import command receives
the native CADCraft table's validated rows, creates point markers plus number/elevation/code labels,
stores stable point/entity links and commits the entire insertion as one undo step. The point-link
inspection route checks persisted entities after save/reopen. The Point Manager route lists linked
points and performs bounded edit, synchronize/repair, renumber, delete and CSV-export actions. Each
mutation commits as one undo step; read-only list/export actions commit an empty host transaction as
required by the runtime. The hidden reference-line route remains as a small contract diagnostic.
The Code Book route lists, previews, adds, edits, deletes, persists and exports bounded rule sets.
The coded-feature route evaluates persistent points and writes native lines, arcs, circles and
original symbol primitives in one replace-all transaction with stable provenance metadata.

Build and package the guest with:

```text
cargo build --release --target wasm32-unknown-unknown -p surveycraft-extension
cargo run -p xtask
```

The second command componentizes the core WASM module and writes an installable bundle to
`target/extension-package/surveycraft.cadcraft-extension`. The manifest is `extension.json`; it
requires CADCraft's schema 2 WebAssembly host plus declarative table and manager UI support. The
bundle also carries the validated, original `SurveyCraft Basic Symbols` vector pack under
`assets/symbols/` and the country-neutral starter code book under `assets/codebooks/`; the packager
checks that every starter rule resolves before writing the archive. The v0.4 CAD adapter turns
those plans into native editable entities on the tested custom host. Adapter tests verify commit,
rollback, replacement, provenance, migration, reconciliation, Point Manager and Code Book behavior
without modifying a real drawing.
