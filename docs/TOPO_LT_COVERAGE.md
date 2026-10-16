# TopoLT workflow coverage ledger

Status: **command-level baseline established; v0.4 coding slice implemented; remaining option-level
and runtime parity audit incomplete**

This ledger accounts for the exposed TopoLT 17.2 action surface identified in the existing static
analysis: 109 menu commands, 30 additional integration entry points, and one Load action. Its 140
rows are scope commitments, not claims that SurveyCraft already matches TopoLT.

SurveyCraft is a clean-room implementation. Command names are factual interoperability identifiers;
the behavior requirements below are independently normalized. Proprietary source, pseudocode,
artwork, symbols, templates, configuration data, help prose and sample drawings are not build inputs
and must not be copied into SurveyCraft.

## Evidence boundary

- **S1:** shipped menu/help intent corroborated against the AutoLISP command wrapper.
- **S2 observed/inferred/unknown:** non-menu AutoLISP entry-point analysis, with uncertainty retained.
- Reference artifact: TopoLT 17.2 static report. No command was dynamically executed for this
  baseline, so every parity result remains unverified until a synthetic black-box case is run.
- The source report found 648 AutoLISP functions and 52 native bridge primitives. Private helpers
  are implementation detail; this ledger tracks exposed workflows and will add any newly observed
  user-visible behavior discovered during option/runtime auditing.
- The ten-command v0.4 evidence, option contract, original synthetic results and retained unknowns
  are expanded in [CODING_FEATURES.md](CODING_FEATURES.md).

## Status language

- **Partial:** SurveyCraft has related functionality and tests, but the full reference workflow and
  its options have not been verified.
- **Planned:** assigned to a release or delivery pack but not implemented.
- **Needs native audit:** CADCraft may already provide the workflow; equivalence must be documented
  and tested before it can satisfy this row.
- **Pending behavior audit:** the static evidence does not establish enough detail for an
  implementation contract.
- A target ending in **+** means that release delivered a subset and follow-up work remains.

## Host dependency codes

| Code | Required host capability |
|---|---|
| H1 | Extension packaging/loading and command discovery |
| H2 | Drawing entities, selection, transactions and persistent metadata |
| H3 | Raster entity, clipping and image-placement APIs |
| H4 | 3D/surface geometry, display and selection APIs |
| H5 | Views, layouts, plotting and host-owned output |
| H6 | User-mediated files or explicitly consented network access |
| H7 | Explicitly consented serial/device access |
| H8 | Audit of an existing native CADCraft workflow |

## Coverage summary

| Baseline | Count |
|---|---:|
| Menu commands | 109 |
| Non-menu integration commands | 30 |
| Load action | 1 |
| Total tracked actions | **140** |
| Dynamically parity-verified actions | **0** |

## Load and configuration

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-001 | `(load "topolt.lsp")` | Loads TopoLT, selects the host-specific native module, initializes resources, and defines the commands below. | S1 | CADCraft | Upstream host | H1 | Planned | Synthetic parity case TBD |
| TLT-002 | `TOPOCFG` | Opens program configuration. Settings cover drawing scale/format, points, layers, areas, images, 3D model/contours, plans and exported data; drawing-resident settings can trigger rescaling/redrawing. | S1 | SurveyCraft | v1.0 | H2 | Partial | SC tests exist; parity case TBD |
| TLT-003 | `EDITCOD` | Opens the allocated `.cod` file in the companion editor. Codes map survey records to descriptions and graphical behavior. | S1 | SurveyCraft | v0.4 | H2+H6 | Implemented (SC contract) | Native rule manager, preview, drawing persistence and JSON export; proprietary-file parity intentionally excluded |
| TLT-004 | `EDITLAY` | Opens the allocated `.lay` file in the companion editor. | S1 | SurveyCraft | v0.4 | H2+H6 | Partial | Embedded layer schema and mapped native layer names work; separate layer editor and reference parity TBD |
| TLT-005 | `PUR` | Purges unused drawing resources through the host CAD purge workflow. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |

## Point-set management

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-006 | `RENPCT` | Renumbers TopoLT points in the drawing according to the selected order/options. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-007 | `FIXHPCT` | Makes point elevations agree with the elevations of the point entities. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-008 | `HPCT` | Translates selected points vertically by an elevation change. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-009 | `AUTOPCT` | Batch operation that inserts graphical points, rounds/fixes points to decimals, breaks entities at points, and fixes nearby entities to points according to options. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-010 | `OPTPCT` | Repositions point text to reduce text-to-text and text-to-entity overlap. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-011 | `JOINPCT` | Connects points according to point-code rules. | S1 | SurveyCraft | v0.4 | H2 | Implemented (SC contract) | Transactional native line/arc/circle generation, replacement and provenance tests; reference parity TBD |
| TLT-012 | `SELCOD` | Builds a selection set of points matching selected codes. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC wildcard-selection tests; native selection parity TBD |
| TLT-013 | `CHCOD` | Changes codes for points on/associated with selected entities. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC first-code replacement tests; entity-endpoint parity TBD |
| TLT-014 | `XPCT` | Turns off the layers allocated to point components. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-015 | `OPCT` | Restores point-layer visibility to its previously recorded state. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-016 | `ACTPCT` | Rebuilds/redraws topographic point graphics using the current TopoLT point settings. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-017 | `DELPCT` | Deletes all recognized TopoLT points from the current drawing. This is destructive. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-018 | `OPTP` | Opens the point working-mode configuration. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |

## Individual point operations

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-019 | `MAXDENPCT` | Searches the drawing and reports/uses the largest numeric point name. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-020 | `IPCT` | Interactively inserts one graphical TopoLT point. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-021 | `MIPCT` | Repeats interactive graphical point insertion. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-022 | `EPCT` | Deletes one selected TopoLT point and its associated graphical components. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-023 | `ASCPCT` | Moves/hides a point on the configured layer for overlapped texts. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-024 | `MTT` | Manually relocates a selected point label. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-025 | `ZDD` | Finds/zooms to a point by name. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-026 | `REDD` | Changes a point's name, elevation, or code. | S1 | SurveyCraft | v0.3+ | H2 | Partial | SC tests exist; parity case TBD |
| TLT-027 | `FIXPCT` | Rounds/fixes a point to configured decimals and drags or breaks nearby entities at the point. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-028 | `IP` | Inserts a point whose elevation is linearly interpolated from selected geometry/points. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-029 | `IPC` | Copies points while calculating their elevations by interpolation. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |
| TLT-030 | `IPLAN` | Inserts points with elevations interpolated on a plane defined by three 3D points. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |

## Coordinate files and calculations

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-031 | `RAPPCT` | Imports a coordinate file and creates configured point entities/labels. Supports text/CSV and instrument-oriented formats, ordering, duplicate handling, code interpretation, and text optimization options. | S1 | SurveyCraft | v0.1+ | H2+H6 | Partial | SC tests exist; parity case TBD |
| TLT-032 | `CRAD` | Calculates sightshot/polar point coordinates from radial measurements, using the companion program, then can draw/import the results. | S1 | SurveyCraft | v0.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-033 | `REFCOO` | Rewrites/updates a coordinate file from points currently in the drawing. | S1 | SurveyCraft | v0.3+ | H2+H6 | Partial | SC tests exist; parity case TBD |
| TLT-034 | `TABCOO` | Draws a coordinate table from selected/recognized drawing points. | S1 | SurveyCraft | v0.5 | H2 | Planned | Synthetic parity case TBD |

## Total-station communication

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-035 | `RXCOO` | Receives coordinates from the configured total station and passes them into the point-import workflow. | S1 | Instrument pack | v1.4 | H7 | Planned | Synthetic parity case TBD |
| TLT-036 | `TXCOO` | Exports selected drawing points through the configured total-station connection. | S1 | Instrument pack | v1.4 | H7 | Planned | Synthetic parity case TBD |
| TLT-037 | `RECCOM` | Starts `TopoLT.exe` in `REC` mode to capture raw measurements over the configured serial/COM connection. | S1 | Instrument pack | v1.4 | H7 | Planned | Synthetic parity case TBD |
| TLT-038 | `OINS` | Chooses the instrument definition and transfer parameters such as format/port settings. | S1 | Instrument pack | v1.4 | H7 | Planned | Synthetic parity case TBD |

## Code-driven symbols

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-039 | `IB` | Inserts punctiform/block symbols based on point codes and the active `.cod` mapping. | S1 | SurveyCraft | v0.4 | H2 | Implemented (SC contract) | Original symbols create native primitives with unit placement and dedup; reusable-block/reference parity TBD |
| TLT-040 | `MSCBL` | Rescales symbols by name or for all mapped codes. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC selector/rescale tests; CAD parity TBD |
| TLT-041 | `ROTBL` | Rotates symbols by name or for all mapped codes. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC degrees/gon bearing tests; CAD parity TBD |
| TLT-042 | `DELBL` | Deletes symbols by name or for all mapped codes. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC selector/delete tests; CAD parity TBD |
| TLT-043 | `CHL` | Applies/changes line types for linear entities according to point-code rules. | S1 | SurveyCraft | v0.4 | H2 | Partial | SC majority/priority tests; CAD parity TBD |

## Areas, parcels, and building surveys

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-044 | `ARR` | Calculates parcel area, creates an area-calculation table, and can export `arr.txt` in the ARIA program format. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-045 | `ARR2` | Draws an area table for selected boundary geometry. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-046 | `DETAS` | Splits a polyline or circle to detach a requested area. Documented methods include parallel/perpendicular directions, proportional division, a forced point/opening, side translation, and equidistant offset. | S1 | SurveyCraft | v0.7 | H2 | Planned | Synthetic parity case TBD |
| TLT-047 | `MDETAS` | Repeats the area-detachment process to produce multiple parcels. | S1 | SurveyCraft | v0.7 | H2 | Planned | Synthetic parity case TBD |
| TLT-048 | `RECALCAREA` | Recomputes areas after boundary changes and updates linked text labels. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-049 | `REFLINK` | Rebuilds the link between one parcel polyline and its descriptive texts. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-050 | `MREFLINK` | Rebuilds parcel-to-text relationships in bulk. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-051 | `SENSPOLY` | Sets a polyline's starting vertex and vertex order/direction. | S1 | SurveyCraft | v0.7 | H2 | Planned | Synthetic parity case TBD |
| TLT-052 | `TABPARC` | Displays TopoLT-created land plots in a table. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-053 | `TABREL` | Draws a room inventory table for a building survey. | S1 | SurveyCraft | v0.7 | H2 | Planned | Synthetic parity case TBD |
| TLT-054 | `HASPL` | Hatches selected closed polylines using chosen settings. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |
| TLT-055 | `OSUP` | Opens area/parcel working-mode configuration. | S1 | SurveyCraft | v0.6 | H2 | Planned | Synthetic parity case TBD |

## Raster imagery and georeferencing

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-056 | `TIMG` | Correlates/transforms raster images against drawing control points. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-057 | `CHIMG` | Converts image format and/or colors through the companion image workflow. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-058 | `JOINIMG` | Mosaics multiple selected raster images. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-059 | `TRIMIMG` | Crops an image by its ImageClip boundary or by a selected polyline, circle, or ellipse. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-060 | `RESIMG` | Changes an image's pixel dimensions. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-061 | `SAVEGEOREF` | Writes image placement and ImageClip information to georeferencing/world files. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-062 | `IMGGEOREF` | Restores image placement and clipping from georeferencing files. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-063 | `DRAWGEOREF` | Batch-inserts georeferenced images from a folder. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-064 | `WEBIMG` | Downloads online map imagery for the chosen drawing extent/settings and inserts it. This performs network and file I/O. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-065 | `IMGOFF` | Turns image display off. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-066 | `IMGON` | Turns image display on. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-067 | `UIMG` | Restores the image state saved before a TopoLT image transformation. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |
| TLT-068 | `OIMG` | Opens raster/image working-mode configuration. | S1 | SurveyCraft | v1.2 | H3+H6 | Planned | Synthetic parity case TBD |

## 3D model, contours, and volume

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-069 | `M3D` | Builds a triangulated surface from elevations, with boundary/breakline and gradient-color options. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-070 | `TRIMM3D` | Clips/removes surface triangles using selected limits. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-071 | `JOINM3D` | Joins two triangulated models into one compatible surface. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-072 | `CHM3D` | Alters surface shape using lines that force slope changes/breaklines. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-073 | `VM3D` | Opens real-time viewing for a model, selected 3D entities, or `.3ds` files. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-074 | `CNN` | Derives and draws contour lines from a 3D model. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |
| TLT-075 | `PROJM3D` | Vertically projects 2D polylines, lines, circles, arcs, or ellipses onto the surface. | S1 | SurveyCraft | v0.9 | H4 | Planned | Synthetic parity case TBD |
| TLT-076 | `VOL` | Calculates cut/fill or volume using 3D model geometry and selected limits/reference. | S1 | SurveyCraft | v0.9 | H4 | Planned | Synthetic parity case TBD |
| TLT-077 | `OM3D` | Opens surface and contour working-mode configuration. | S1 | SurveyCraft | v0.8 | H4 | Planned | Synthetic parity case TBD |

## Plan sheets and plotting

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-078 | `CJ` | Draws a coordinate grid/graticule using current scale and labeling settings. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |
| TLT-079 | `FORMPLNS` | Creates numbered named views for plan-sheet frames. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |
| TLT-080 | `NRPLAN` | Loads/associates a plan number with the active layout. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |
| TLT-081 | `PRNPLAN` | Batch-plots loaded plan layouts to PDF, TIFF, or BMP. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |
| TLT-082 | `DRPLN` | Draws plan-sheet frames in one or more standard/custom formats. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |
| TLT-083 | `OPLN` | Selects printer/plot configuration and plan-sheet format. | S1 | SurveyCraft | v1.3 | H5 | Planned | Synthetic parity case TBD |

## Coordinate and entity transformations

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-084 | `TRCOO` | Uses a selected TransLT model to transform drawing coordinates interpreted as E/N/H or longitude/latitude/height; supports epoch input for time-dependent models. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |
| TLT-085 | `MR` | Aligns entities using two common points, applying translation and rotation. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |
| TLT-086 | `MRS` | Aligns entities using two common points with translation, rotation, and uniform scale. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |
| TLT-087 | `RED0` | Sets selected entities onto the Z=0 plane. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |
| TLT-088 | `LIFTUP` | Assigns elevations to lines, polylines, blocks, circles, texts, and points from nearby TopoLT point elevations. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |
| TLT-089 | `OTRN` | Opens the coordinate/entity transformation settings. | S1 | SurveyCraft | v1.1 | H2 | Planned | Synthetic parity case TBD |

## General CAD utilities and exchange

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-090 | `SUMTXT` | Parses numeric selected text values and reports their sum. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-091 | `BOXTXT` | Draws borders/boxes around selected text entities. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-092 | `ALTXT` | Aligns selected texts along a chosen direction. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-093 | `REPLTXT` | Finds/replaces content in selected `TEXT` entities. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-094 | `DIMPL` | Adds dimensions along a selected polyline, line, or arc. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-095 | `MDIMPL` | Applies the polyline-dimensioning workflow in bulk. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-096 | `INSVX` | Inserts a vertex into a line or polyline. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-097 | `DELVX` | Deletes selected vertices from one or more polylines. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-098 | `MBR` | Breaks one or more polylines at a selected point. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-099 | `MJOIN` | Converts/joins selected compatible entities into polylines. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-100 | `JOINPOLY` | Joins two selected polylines. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-101 | `DELLAY` | Deletes every entity on a selected layer. This is destructive. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-102 | `ALLBLACK` | Changes drawing entity colors to black for monochrome-style output. | S1 | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-103 | `GEDRAW` | Transforms and sends selected polygons, paths, and points to Google Earth using a TransLT model. | S1 | SurveyCraft | v1.1 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-104 | `DRAWKML` | Imports/draws entities from a Google Earth KML file through a TransLT model. | S1 | SurveyCraft | v1.1 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-105 | `DXFEXP` | Exports selected entities to DXF, optionally keeping only layers enabled in the TopoLT DXF configuration. | S1 | SurveyCraft | v1.1 | H2+H6 | Planned | Synthetic parity case TBD |

## Language, licensing, help, and updates

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-106 | `CHLNG` | Selects one of the installed UI/help languages and reloads localized resources/menu state. | S1 | SurveyCraft | v1.0 | H2 | Planned | Synthetic parity case TBD |
| TLT-107 | `TLTCHKUPD` | Calls the companion/native update workflow; this can contact the vendor. | S1 | SurveyCraft | v1.0 | H6 | Planned | Synthetic parity case TBD |
| TLT-108 | `TLTSETSER` | Opens the registration/license-key workflow. | S1 | Open-source replacement | v1.0 | H2 | Replacement planned | Synthetic parity case TBD |
| TLT-109 | `TLTHELP` | Opens the selected compiled HTML help file with `hh.exe`. | S1 | SurveyCraft | v1.0 | H2 | Planned | Synthetic parity case TBD |
| TLT-110 | `TLTABOUT` | Opens product/version/license information. | S1 | SurveyCraft | v1.0 | H2 | Planned | Synthetic parity case TBD |

## Additional non-menu `C:` entry points

| ID | Reference action | Required outcome | Evidence | Delivery | Target | Host | Implementation | Verification |
|---|---|---|---|---|---|---|---|---|
| TLT-111 | `+DELENTIMOB` | Reads parcel handles from an integration control file, deletes the corresponding polyline entities, records deleted identifiers, and reports completion. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-112 | `+DRAWENTS` | Loads a generated LISP payload and asks the native layer to remove/finish it. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-113 | `+DRAWTAB` | Runs the shared integration exchange routine used to draw/save a returned table. | S2 unknown | Romania pack | v1.5 | H2+H6 | Pending behavior audit | Synthetic parity case TBD |
| TLT-114 | `+GETAREAPOLY` | Prompts for a 2D polyline, calculates its area, and writes the value/status to an integration control file. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-115 | `+INSPLN` | Inserts/draws a plan frame and related marks/entities from current settings, then returns the generated selection. | S2 inferred | SurveyCraft | v1.3 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-116 | `+LOADCFG` | Loads integration configuration from a control file and signals `CFG_LOADED`. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-117 | `+MARKLINE` | Interactively marks two points with temporary line/point graphics. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-118 | `+MARKPNT` | Interactively marks one point with temporary graphics. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-119 | `+SAVEALLIMOB` | Starts the save/draw-all-properties integration routine and requires an otherwise empty drawing. | S2 inferred | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-120 | `+SAVECFG` | Saves integration configuration and signals `CFG_SAVED`. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-121 | `+SAVEIMOB` | Runs the shared integration exchange routine for a property (`imobil`). | S2 unknown | Romania pack | v1.5 | H2+H6 | Pending behavior audit | Synthetic parity case TBD |
| TLT-122 | `+SAVESECT` | Runs the shared integration exchange routine for a cadastral sector. | S2 unknown | Romania pack | v1.5 | H2+H6 | Pending behavior audit | Synthetic parity case TBD |
| TLT-123 | `+SELENTPOLY` | Selects a 2D polyline, optionally normalizes it, and writes its handle and vertex geometry to the control file. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-124 | `+SELPLNS` | Associates selected plan-frame polylines with text numbers inside them and returns those pairs. | S2 observed | SurveyCraft | v1.3 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-125 | `+SELPOINT` | Prompts for a point (optionally showing a supplied point first) and returns its coordinates. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-126 | `+SELVECIMOB` | Selects neighboring-property text around a parcel and returns the selected text data. | S2 inferred | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-127 | `+SELZONA` | Prompts for two opposite corners and returns a rectangular zone to the integration caller. | S2 observed | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-128 | `+SETNEWCFG` | Initializes a new TopoLT configuration in the drawing/integration context. | S2 inferred | CADCraft/SC adapter | v1.0/host | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-129 | `CONLINK` | Attaches selected construction entity handles to a parcel's stored relationship data. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-130 | `COTAPROF` | Computes a new level/elevation text from a base elevation, scale factor, and measured drawing distance. | S2 observed | SurveyCraft | v0.9 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-131 | `GENCM` | Sends a selected cadastral-map (`CM`) scope to the companion generator. | S2 inferred | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-132 | `GENCP` | Main cadastral-plan/property generator. Prompts for operation types including first registration, subdivision, merger, data update, technical reception, boundary correction, title registration, and cadastral-number allocation. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-133 | `GENSC` | Sends a selected cadastral-sector (`SC`) scope to the companion generator. | S2 inferred | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-134 | `GENUAT` | Sends a selected administrative-territorial-unit (`UAT`) scope to the companion generator. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-135 | `GETHANDENT` | Prompts for an entity and prints its AutoCAD handle. | S2 observed | CADCraft or SC | Host audit | H8 | Needs native audit | Synthetic parity case TBD |
| TLT-136 | `IDREFLINK` | Reassigns/repairs parcel land IDs from text found inside selected cadastral polylines. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-137 | `INFOIMOB` | Finds a property polygon around a picked interior point, gathers linked entities, and opens the companion information view. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |
| TLT-138 | `OCAD` | Invokes the companion executable's `GDC` cadastral workflow with drawing scale/host parameters. | S2 unknown | Romania pack | v1.5 | H2+H6 | Pending behavior audit | Synthetic parity case TBD |
| TLT-139 | `RENFILES` | Starts `TopoLT.exe /RF`, apparently a file-renaming workflow. | S2 inferred | Romania pack | v1.5 | H2+H6 | Pending behavior audit | Synthetic parity case TBD |
| TLT-140 | `SUBLINK` | Stores selected subdivided subparcel handles on a parent parcel. | S2 observed | Romania pack | v1.5 | H2+H6 | Planned | Synthetic parity case TBD |

## Completion gate

Full workflow coverage cannot be claimed until every row has:

1. an option-level behavior specification with units, axis convention, rounding and tolerance;
2. an implemented SurveyCraft, optional-pack or verified native-CADCraft delivery path;
3. a small synthetic fixture with independently calculated expected results;
4. a public command/integration test including failure, Undo and save/reopen behavior where relevant;
5. a black-box result comparison for the reference version, or a documented reason that the
   open-source product intentionally replaces a non-functional concern such as license activation;
6. no unresolved **Planned**, **Partial**, **Needs native audit**, **Pending behavior audit** or
   parity-TBD state.

## Next audit slice

The next coding pass should add native linetype application, reusable symbol blocks and rendered
suffix attributes after the accepted CADCraft host API can express them. Native color, paper
lineweight and polygonal solid fills are implemented on the paired custom host. Black-box reference
comparisons remain required before any parity claim.
