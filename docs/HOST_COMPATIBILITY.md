# Host compatibility and upstream plan

## Current installation status

SurveyCraft v0.1.0 through v0.4.1 are developer prereleases. They require a custom CADCraft host;
official CADCraft v0.7.0 cannot install and run them. No minimum official supported version can be
named until the required host capabilities are merged and released.

The tested pairing for v0.4.1 is:

| Component | Revision |
|---|---|
| SurveyCraft extension | [v0.4.1](https://github.com/AlexMaster23/surveycraft/releases/tag/v0.4.1) |
| Custom CADCraft host | [d5684bbcc192c22eec11749976d7c512e3c849a1](https://github.com/AlexMaster23/cadcraft/commit/d5684bbcc192c22eec11749976d7c512e3c849a1) |
| Full host integration reference | [Draft PR #1384](https://github.com/storytold/cadcraft/pull/1384) |

The manifest's `>=0.7.0, <1.0.0` range describes the development host's package version. It is not
a claim that stock CADCraft v0.7.0 has the required runtime or UI. Install against the specific
tested host revision until an official compatible release is documented here.

The older v0.4.0 package remains paired with CADCraft commit
[`f4b7a97`](https://github.com/AlexMaster23/cadcraft/commit/f4b7a9775eacb0d76448e4e25565fd4a410f00bb).
It does not use the newer `set-entity-appearance` and `create-solid-fill` host calls.

## What belongs in each repository

SurveyCraft owns survey-point identities and records, parsing/export, validation, survey commands
and surveying algorithms. CADCraft owns extension loading, generic drawing access, transactions,
persistence, command discovery and native UI rendering. The current integration relies on the
proposed WASM runtime, point/text/line/arc/circle/solid-fill operations, bounded entity appearance,
namespaced metadata, import tables, Tool Sets and record managers with add/preview controls. Those host additions require upstream
agreement; the current UI API may change during review.

## Upstream review plan

The full integration is preserved in #1384 as a draft reference. The smaller
[PR #1446: installable AutoLISP extension packages](https://github.com/storytold/cadcraft/pull/1446)
starts from upstream `9b2308b` and contains the original package/store and AutoLISP foundation.
It is independently useful, but cannot run SurveyCraft's WASM package by itself.

The proposed review stages follow the existing implementation's dependencies:

| Stage | Scope | Status |
|---|---|---|
| 1 | Package format, per-user store, existing AutoLISP runtime integration, desktop/CLI management | [PR #1446](https://github.com/storytold/cadcraft/pull/1446) open for review |
| 2 | Schema-2 WASM execution and guarded drawing transactions | Deferred pending foundation feedback |
| 3 | Point/text operations, native import tables, selectors and Tool Sets | Deferred |
| 4 | Namespaced persistent drawing/entity data, query/update/delete | Deferred |
| 5 | Native record managers, preview controls and host-owned text export | Deferred |
| 6 | Guarded native arc/circle/solid-fill creation and entity appearance for coded geometry | Deferred |

The later stages will be adjusted to the accepted host design and tested against upstream before
submission. Their code is available in the integration reference; no promise of upstream acceptance
or a release date is implied.

## Conditions for continuing

The complete published v0.4.1 workflow is implemented and verified against the custom host above,
including native code-book editing, preview, transactional feature/symbol generation, audited
input/identity/geometry fixes and exact symbol appearance. Development can
continue there while the project responds to foundation review and agrees on a supported runtime,
drawing API and UI extension approach. Installation on stock CADCraft will only be advertised after
all capabilities required by the relevant SurveyCraft version ship in an official release and an
installation/save/reopen/undo test passes against that release.

At that point, update the manifest's minimum CADCraft version, compatibility documentation and
release notes to the tested official version. If upstream declines the required extension approach,
reassess the adapter with the project owner before committing to long-term fork maintenance.
