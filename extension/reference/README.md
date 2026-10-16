# SurveyCraft WebAssembly extension

This portable guest contains SurveyCraft's CAD-facing commands. Its point-import command receives
the native CADCraft table's validated rows, creates point markers plus number/elevation/code labels,
and commits the entire insertion as one undo step. The hidden reference-line route remains as a
small contract diagnostic.

Build and package the guest with:

```text
cargo build --release --target wasm32-unknown-unknown -p surveycraft-extension
cargo run -p xtask
```

The second command componentizes the core WASM module and writes an installable bundle to
`target/extension-package/surveycraft.cadcraft-extension`. The manifest is `extension.json`; it
requires CADCraft's schema 2 WebAssembly host and declarative table-dialog support. Adapter tests
verify commit and rollback behavior without modifying a drawing.
