# SurveyCraft WebAssembly extension

This portable guest contains SurveyCraft's CAD-facing commands. Its point-import command receives
the native CADCraft table's validated rows, creates point markers plus number/elevation/code labels,
stores stable point/entity links and commits the entire insertion as one undo step. The point-link
inspection route checks persisted entities after save/reopen. The Point Manager route lists linked
points and performs bounded edit, synchronize/repair, renumber, delete and CSV-export actions. Each
mutation commits as one undo step; read-only list/export actions commit an empty host transaction as
required by the runtime. The hidden reference-line route remains as a small contract diagnostic.

Build and package the guest with:

```text
cargo build --release --target wasm32-unknown-unknown -p surveycraft-extension
cargo run -p xtask
```

The second command componentizes the core WASM module and writes an installable bundle to
`target/extension-package/surveycraft.cadcraft-extension`. The manifest is `extension.json`; it
requires CADCraft's schema 2 WebAssembly host plus declarative table and manager UI support. Adapter
tests verify commit, rollback, migration, reconciliation and Point Manager behavior without
modifying a drawing.
