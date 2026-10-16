# ADR 0001: Portable WebAssembly CAD host boundary

- Status: Implemented locally; upstream review pending
- Date: 2026-10-16

## Context

SurveyCraft needs substantial geometry, terrain and coordinate-system code while remaining an
ordinary CADCraft extension. CADCraft extension schema 1 can install assets, expose menus and load
AutoLISP, but it does not yet host portable compiled extension logic.

## Decision

SurveyCraft's domain crates remain independent of CADCraft and compile to
`wasm32-unknown-unknown`. CADCraft extension schema 2 hosts a WebAssembly component through a
small versioned WIT interface. CAD access is capability-based and all mutations require an
explicit transaction.

The current contract in `crates/cadcraft/wit/surveycraft-host.wit` intentionally exposes only:

1. begin transaction;
2. create line;
3. create point;
4. create text;
5. commit transaction;
6. rollback transaction.

The Rust reference adapter proves the failure behavior against a mock host today. A tiny guest in
`extension/reference` compiles to a WebAssembly component and calls the same transaction surface.
Its matching `extension.json` declares a host-rendered coordinate table and is installable in the
local CADCraft schema 2 implementation. CADCraft reads a user-selected text file and passes only
its bounded contents to the dialog; the WebAssembly guest receives only validated submitted rows
and never receives ambient file-system access. A normal CADCraft release will support it only
after the generic host change is reviewed and accepted upstream.

## Consequences

- The calculation engine is reusable in tests, the CLI, CADCraft and other future hosts.
- CAD writes are auditable and one-step undoable.
- The host can deny undeclared permissions.
- The WIT surface will grow only when a real SurveyCraft command requires a capability.
- SurveyCraft remains a separate package and repository; no SurveyCraft implementation is shipped
  in the CADCraft host pull request.
- The contract is proven in the local CADCraft branch but is not a public CADCraft capability
  until the host change is reviewed, merged and released upstream.
