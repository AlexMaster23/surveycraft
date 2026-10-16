# SurveyCraft contributor rules

SurveyCraft is a clean-room, open-source surveying extension for CADCraft.

## Non-negotiable rules

- Product scope is complete coverage of TopoLT's user-visible workflows, tracked in
  `docs/FEATURE_MATRIX.md`. Optional packs are optional to install, not silently excluded from
  coverage. Record unknowns, host dependencies and behavioral differences; do not claim complete
  parity from a partial milestone or an unaudited command count.
- Do not copy or translate TopoLT code, decompiler output, icons, drawings, symbols, templates,
  textures, databases, fonts, help text, configuration files or other proprietary assets.
- Use public standards, public mathematical references and independently written synthetic tests.
- Handwritten production Rust forbids `unsafe`, panics, `unwrap`, `expect`, `todo`, `unimplemented`
  and `unreachable`. A tightly scoped `allow(unsafe_code)` is permitted only around generated
  component-ABI bindings whose generator requires unsafe implementation details.
- Treat files, coordinates, counts and geometry as hostile input. Bound allocations and reject
  non-finite numbers.
- All CAD mutations are transactional and undoable. A failed command must leave the drawing
  unchanged.
- The domain engine must not depend on CADCraft, a GUI toolkit or the filesystem.
- User-visible actions are commands and must work through the same stable command ID in every host
  surface.
- Do not commit proprietary comparison files. Golden fixtures are small, synthetic and readable.

## Quality gate

Before every commit run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --target wasm32-unknown-unknown -p surveycraft-geometry -p surveycraft-core -p surveycraft-io -p surveycraft-cadcraft -p surveycraft-ui-model
cargo build --release --target wasm32-unknown-unknown -p surveycraft-extension
cargo run -p xtask
```
