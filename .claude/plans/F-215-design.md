# F-215, Folder viewer marks saved as TID 1500 and key images as a Key Object Selection

**Status**: approved
**Epic ref**: E38.25
**Sprint**: S46
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/11-decision-log.md`:

| **\#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D10 | Keep dcmjs for SR generation | Reimplement TID 1500 in Rust | Standards conformance for near-zero cost |
| D13 | The annotation type is a DICOM SR content tree | An internal model with SR adapters | Every measurement is interoperable by construction |

`docs/hld/07-concurrency-and-typescript.md` section 10 keeps "The SVG annotation drawing layer" and "the dcmjs bridge for TID 1500 structured reports" in TypeScript.

`docs/hld/B-parity-surface.md` lists `Adapters | 4 | SR TID 1500, SEG, RTSTRUCT, parametric map`. It has no per-tool row.

The operator's note on the story, from `docs/sprints/allocation.json`: "This deliberately retires the sentences that say marks are not saved. Nothing is written back into the opened folder."

What exists in `packages/core`, read for this plan:

- `sr.ts`, F-094. `ContentItem` has value types `CONTAINER`, `TEXT`, `NUM`, `CODE`, `UIDREF`, `IMAGE`, `SCOORD` and `SCOORD3D`. `GraphicType` is `"POINT" | "POLYLINE" | "CIRCLE"`. `graphicCount` requires 1 point for POINT, at least 2 for POLYLINE, and 2 for CIRCLE. `IMAGE` carries `sopClass` and `sopInstance` and no frame number.
- `tid1500.ts`, F-097. `writeTid1500(items, source)` refuses any root that does not hold exactly one `NUM`, and writes each NUM as a TID 1501 group with one `ReferencedSOPSequence`. SOP class `1.2.840.10008.5.1.4.1.1.88.34`. `readTid1500` reads NUMs only, under the container whose concept is `126010`.
- `tid2010.ts`, F-196. `writeKeyObject` writes one `COMPOSITE` reference and refuses a study, series or instance UID that does not start `2.25.`. `readKeyObject` refuses a document with other than two content items. `docs/lld/key-object.md` records that it omits Current Requested Procedure Evidence Sequence, Content Date, Content Time, Instance Number, Series Number, Patient Name and Patient ID, and that "A conformant receiver may refuse the files."

There is no Rust SR writer. `git grep` over `crates/` for TID 1500, the SR SOP classes, Key Object and SCOORD finds nothing.

The image coordinate rule. DICOM PS3.3 C.18.6.1.1, Graphic Data, for SCOORD content items, transcribed from the standard text without a copy at hand, which is a transcription from memory. The flag stays, and AC20 makes the reviewer check it against the current edition at dicom.nema.org before the mark mapping is implemented:

```text
Graphic Data (0070,0022) ... column and row values relative to the image,
with sub-pixel resolution, such that the origin at the Top Left Hand Corner
(TLHC) of the TLHC pixel is 0.0\0.0, the Bottom Right Hand Corner (BRHC) of
the TLHC pixel is 1.0\1.0, and the BRHC of the BRHC pixel is Columns\Rows.
```

PS3.3 C.18.6.1.2, Graphic Type, same caveat and the same AC20 check:

```text
POINT      a single pixel denoted by a single (column,row) pair
MULTIPOINT multiple pixels each denoted by a (column,row) pair
POLYLINE   a connected series of line segments with vertices denoted by
           (column,row) pairs. If the first and last vertices are the same
           it is a closed polygon.
CIRCLE     a circle defined by two (column,row) pairs. The first point is
           the central pixel. The second point is a pixel on the perimeter.
ELLIPSE    an ellipse defined by four pixel (column,row) pairs, the first two
           points specifying the endpoints of the major axis and the second
           two points specifying the endpoints of the minor axis
```

Graphic Data is VR FL, 32-bit float, PS3.6.

## What the specification does not cover

**How the page reaches `@ocelli/core`.** The page is inline script with no bundler and is served by the Rust process. Three options:

- A. Serve the built `packages/core/dist` and `dcmjs`'s own ES build from the local process, and map the bare names with an import map. The writer is the published one. Recommended.
- B. A Rust SR writer. None exists, and D10 rejects writing one.
- C. Copy a bundled `@ocelli/core` into the example. A second tracked copy of the writer, which drifts.

This plan takes A, and the choice is Open question 1.

The operator chose A on 2026-11-05, with one change: the served names come from an allowlist of file names rather than a name pattern. Under the sprint build order F-213 lands before this story and writes the `/lib/` routes, the allowlist and the import map, because its region tool imports `windowRegion` the same way. That route set, as F-213 specifies it:

| Path | Source | Refusal |
|---|---|---|
| `/lib/ocelli-core/<name>.js` | `<CARGO_MANIFEST_DIR>/../../packages/core/dist/<name>.js` | `<name>` must be in `LIBRARY_FILES`, the non-test modules of `packages/core/src/index.ts`'s relative import closure. Anything else is 404. A listed file that is missing is 404 with `X-Ocelli-Error: library` |
| `/lib/dcmjs.js` | `<CARGO_MANIFEST_DIR>/../../node_modules/dcmjs/build/dcmjs.es.js` | Missing is 404 with `X-Ocelli-Error: library` |

This story adds one route to `serve.rs`:

| Path | Source | Refusal |
|---|---|---|
| `/marks.mjs` | `include_str!("marks.mjs")`, tracked | none |

Both library roots are read at request time, so the crate builds and its tests run without `npm run build`. The served type is `text/javascript; charset=utf-8`. No other path under `/lib/` is served, and the client never supplies a directory. The page carries, from F-213:

```html
<script type="importmap">{"imports":{"@ocelli/core":"/lib/ocelli-core/index.js","dcmjs":"/lib/dcmjs.js"}}</script>
```

and loads `/marks.mjs` with a dynamic `import()` only when Save or a document load needs it. If that import fails, the marks stay and the page says "Saving needs the library build. Run npm run build in the repository, then start the viewer again." It does not fall back to anything.

Checked for this plan: `dcmjs.es.js` has no `import` statement, and the non-test modules in `packages/core/dist` import only relative paths and `dcmjs`.

**The content tree for each mark.** PS3.16 TID 1500 nests TID 1501 and TID 1410 groups. The HLD does not map a page mark onto them. This plan uses one Measurement Group per mark:

```text
CONTAINER (125007, DCM, "Measurement Group")
  HAS OBS CONTEXT  TEXT    (112039, DCM, "Tracking Identifier")         "ocelli-folder-viewer:<kind>"
  HAS OBS CONTEXT  UIDREF  (112040, DCM, "Tracking Unique Identifier")  2.25 UID, kept across saves
  CONTAINS         SCOORD  (111030, DCM, "Image Region")                graphic and points below
    SELECTED FROM  IMAGE   referenced SOP class, instance, and frame number on a multi-frame object
  CONTAINS         NUM     (410668003, SCT, "Length")  unit mm          length marks only, see below
  CONTAINS         TEXT    (121106, DCM, "Comment")                     text marks only
```

| Page kind | Stored page points | Graphic Type | Graphic Data, column then row |
|---|---|---|---|
| `contour` | n points, n at least 3 | POLYLINE, closed | the n points, then the first point again |
| `rect` | corners `(x0,y0)` `(x1,y1)` | POLYLINE, closed | `x0,y0, x1,y0, x1,y1, x0,y1, x0,y0` |
| `ellipse` | box corners `(x0,y0)` `(x1,y1)` | ELLIPSE | with `cx=(x0+x1)/2`, `cy=(y0+y1)/2`. When `x1-x0 >= y1-y0` the major axis is horizontal, `x0,cy, x1,cy, cx,y0, cx,y1`. Otherwise `cx,y0, cx,y1, x0,cy, x1,cy` |
| `length` | two points | POLYLINE, open | the two points |
| `text` | one point and a string | POINT | the point |

Page image coordinates already have the C.18.6.1.1 origin. `drawGrey` sets the canvas to `grey.width` by `grey.height`, which are Columns and Rows, and sets the SVG `viewBox` to `0 0 Columns Rows`. `clientToImage` returns `x` across columns and `y` down rows in that box. The pointer cross reads `floor(x) + 1` as the column counted from 1, which makes the centre of column 1 at `x = 0.5`. So **no 0.5 offset is added or removed** in either direction, and `x` is written first because Graphic Data is column then row. Rotation and flip are display state applied after these coordinates, so they are not written and do not change a saved point.

The Length NUM is written only when the image's spacing source is `PixelSpacing`. Its value is F-207's formula, `hypot(dCol * columnSpacing, dRow * rowSpacing)`, evaluated with the spacing of the image the mark is on, read from that image's `/inspect` fields, not from `shownFacts`. It is written as `Number(mm.toPrecision(12))` so the DS text fits 16 characters. Under `ImagerPixelSpacing` the page labels the length "at the detector", and a NUM in mm there would tell another system a patient length that has not been corrected for magnification. So the group holds the geometry and no NUM. Pixel lengths also get no NUM.

Graphic Data is FL. A point is written as its `Math.fround` value, and the page stores the `Math.fround` value back into the mark when it saves, so a second save writes the same numbers.

Colour and text size are display choices and are not written. A loaded mark takes the first ink and size M. The marks section says so.

**Document attributes.** The new writers fill what `docs/lld/key-object.md` lists as missing, because these files will sit beside real images. Patient Name, Patient ID, Study Instance UID, Study Date and Accession Number are copied from the referenced images' `/inspect` fields. Other Type 2 patient and study attributes are written empty. Series Instance UID and SOP Instance UID are new `2.25` values. Series Number and Instance Number are 1. Content Date and Content Time are the save time. Completion Flag is `COMPLETE` and Verification Flag is `UNVERIFIED` on the SR. Current Requested Procedure Evidence Sequence lists every referenced instance by study and series. The PS3.3 module sections for these, C.17.2 SR Document General and C.17.6 Key Object Document, are not transcribed in `docs/`. Before the document writers are implemented, the reviewer transcribes their attribute lists into this plan from the current edition at dicom.nema.org and checks the Type of every attribute named above against them, which is AC20. Open question 4.

**One document per study.** A folder can hold more than one study. Save writes one TID 1500 document for each study that has a mark and one Key Object Selection for each study that has a key image. Most folders give two files.

**Supersession.** Nothing is written into the folder, so an older saved file stays there. A new document lists every document it was loaded from in Predecessor Documents Sequence `(0040,A360)`. On load, a document that another loaded document names as a predecessor is skipped, and a mark whose Tracking Unique Identifier is already shown is not added twice. Open question 3.

**Recognising a document.** `inspect_json` gains `sopClassUid` `(0008,0016)` and `studyUid` `(0020,000D)`. A file whose SOP class is Comprehensive 3D SR `1.2.840.10008.5.1.4.1.1.88.34`, Comprehensive SR `...88.33`, Enhanced SR `...88.22` or Key Object Selection Document `...88.59` is a document. It is taken out of `groupSeries`, so it no longer shows as a series that fails to preview.

**Placing a loaded mark.** The page keys marks by `noteKey(image)`, which is the file path, size, mtime and frame. That key cannot survive a save. The join is the referenced SOP Instance UID and frame. After the series are grouped, the page builds one map from `sopInstanceUid + "\0" + frame` to `noteKey(image)`. A referenced frame number `n` maps to page frame `n - 1`. A reference with no frame number maps to frame 0 of a single-frame object. A loaded item is not placed, and is counted by reason, when:

- its SOP Instance UID is not in the folder
- two images in the folder share that SOP Instance UID
- it references a multi-frame object without a frame number, for a mark. A key image reference of that shape marks every listed frame of the object
- a point lies outside `0..Columns` by `0..Rows` of that image
- its graphic is one this table does not produce, such as CIRCLE, MULTIPOINT or an ELLIPSE whose axes are not parallel to the image edges
- it is a NUM-only group of the F-097 shape, which has no geometry to draw

A group without the `ocelli-folder-viewer:` tracking prefix is still drawn when its graphic is in the table. A closed POLYLINE becomes a freeform mark and an open two-point POLYLINE becomes a length mark.

## Problem and reproducer

`cargo run -p ocelli-folder-viewer`, open a folder, draw a rectangle, mark a key image, reload. Everything is gone. The page says "These are drawings in this browser. They are not DICOM SR. They are not written to the file. They are gone when you reload." and the key image sentence says "It is not saved and it does not write a Key Object Selection." An SR file in the opened folder is listed as a series that fails to preview.

## Scope, API and compatibility

In `packages/core`, additive, existing exports unchanged:

```ts
// sr.ts
export type GraphicType = "POINT" | "POLYLINE" | "CIRCLE" | "ELLIPSE"
// IMAGE gains an optional frame, 1-based, as Referenced Frame Number
(ItemBase & { valueType: "IMAGE"; sopClass: string; sopInstance: string; frame?: number })

// tid1500.ts
export type ReportSource = {
  study: string
  patientName: string
  patientId: string
  studyDate: string
  accession: string
  evidence: readonly { series: string; sopClass: string; sopInstance: string }[]
  predecessors: readonly { series: string; sopClass: string; sopInstance: string }[]
}
export function writeTid1500Groups(groups: readonly ContentItem[], source: ReportSource): Uint8Array | null
export function readTid1500Groups(bytes: Uint8Array | ArrayBuffer): TidGroups | null
export type TidGroups = { sopInstance: string; predecessors: readonly string[]; groups: readonly ContentItem[] }

// tid2010.ts
export function writeKeyObjects(references: readonly KeyReference[], source: ReportSource): Uint8Array | null
export function readKeyObjects(bytes: Uint8Array | ArrayBuffer): KeyObjects | null
export type KeyReference = { sopClass: string; sopInstance: string; frame?: number }
export type KeyObjects = { sopInstance: string; predecessors: readonly string[]; references: readonly KeyReference[] }
```

`ELLIPSE` takes exactly 4 points in `graphicCount`. A frame that is not a positive integer is refused by `contentTree`. `writeTid1500Groups` refuses a root that is not a `CONTAINER` with concept `125007`, any node `contentTree` refuses, and a count at or over `MEASUREMENT_ITEM_LIMIT`. It serialises every value type generically, writing the relationship with a space where the type has an underscore. `writeKeyObjects` accepts any well-formed study UID, not only `2.25`, because the study is the user's. An empty reference list returns nothing. `writeTid1500`, `readTid1500`, `writeKeyObject` and `readKeyObject` keep their F-097 and F-196 behaviour and tests.

In the example:

- `examples/folder-viewer/src/marks.mjs`, new, no DOM. `marksToGroups(marks, image)`, `groupsToMarks(groups, image)`, `buildDocuments(state)` and `placeDocuments(documents, images)`, each a pure function over plain objects, importing from `@ocelli/core`.
- `page.html`: a Save marks button and a Load marks file input in the MARKS section, documents split out in `readFolder`, the import map, and the sentences below.
- `serve.rs`: the three routes. `preview.rs`: the two `inspect_json` fields.

The sentences that change:

| Where | Today | After |
|---|---|---|
| `page.html` MARKS note | These are drawings in this browser. They are not DICOM SR. They are not written to the file. They are gone when you reload. | Marks stay in this browser until you save them. Save marks downloads a DICOM SR TID 1500 file, and a Key Object Selection file for key images. Nothing is written into the opened folder. Put the downloaded files in the folder and open it again to see the marks. Colour and text size are not saved. |
| `SENTENCE.contour`, `rect`, `ellipse`, `text`, `length` | ends "It is not saved." or "it is not saved." | ends "Save marks writes it as TID 1500." |
| `SENTENCE["key-image"]` | Marks this image in the series list for this session. It is not saved and it does not write a Key Object Selection. | Marks this image in the series list. Save marks writes the key images as a Key Object Selection. |
| `more.html` line 455 | The key image flag on the image page is only a mark in this session. It does not write a Key Object Selection and it is not saved. | Save marks on the image page writes the key images through these functions. |

Out of scope: writing into the folder, which a `webkitdirectory` input cannot do anyway. Mark kinds other than the five F-207 kinds. F-211 lands first under the sprint build order, so its kinds, angle, Cobb, bidirectional, arrow and probe, are counted and named as not saved in the Save status line, for example "2 angle marks were not saved." Cornerstone3D's adapters hydrate tools from TID 300 shaped groups, where a NUM is `INFERRED FROM` its SCOORD. This plan's Image Region groups will not hydrate as cornerstone tools, which is Open question 5. GSPS, SEG and Secondary Capture are out of scope.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Save marks | `buildDocuments` then `writeTid1500Groups` and `writeKeyObjects` | One document of each kind per study. Spacing per image | One download per document, a status line with counts |
| Folder open | `readFolder`, `/inspect` `sopClassUid` | Document classes leave `groupSeries` | Series list without document rows |
| Folder open with documents | `readTid1500Groups`, `readKeyObjects`, `placeDocuments` | Join on SOP Instance UID and frame. Predecessors skipped | Marks and key flags on the images, a status line naming what was not placed |
| Load marks input | same as above | Applied to the open study, synthetic included | same, merged by Tracking UID |
| Length label | unchanged `lengthText` | Recomputed from geometry and spacing | unchanged |
| Library routes | `serve.rs` `library_reply`, written by F-213 | `LIBRARY_FILES` allowlist, fixed roots | JS or 404 with `X-Ocelli-Error: library` |
| Folder open | F-216's `POST /file` for each file, whose body is the `/inspect` JSON | Each file posted once | `sopClassUid` and `studyUid` for document recognition |
| `writeTid1500`, `writeKeyObject` and the React example | unchanged | F-201 and F-196 callers | unchanged |

Units: Graphic Data in image pixels, NUM in mm. The page owns the mark to tree mapping. `@ocelli/core` owns bytes. Rounding: `Math.fround` for points and 12 significant digits for the NUM, both in `marks.mjs`.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Marks on two images of one study, one key image | Save marks | Two downloads. Status "Saved 2 marks and 1 key image in 2 files." | A writer returning null: no download at all, status names the failure, marks unchanged |
| Marks, library build absent | Save marks | Nothing downloaded. The library sentence | Marks unchanged |
| A mark on an image with no SOP Instance UID | Save marks | The others are saved. Status names how many were not | That mark stays in the page |
| Folder with images and a saved SR and KO | Open | Marks and key flags appear on their images | Unplaced items counted by reason. Images still open |
| Folder with document A and its successor B | Open | Only B is applied | A is named as replaced in the status |
| Marks loaded, user deletes one, saves | Save marks | New documents name the loaded ones as predecessors | The deleted mark does not come back when both files are in the folder |
| Folder reopened while documents are being read | Open | The older read is dropped by `currentGeneration` | No mark from the old folder lands on the new one |
| Same marks, Save twice | Save marks | Two sets of files with new SOP Instance UIDs and the same Tracking UIDs | Loading both places each mark once |

Invariant across consumers: a mark is placed only on the image whose SOP Instance UID and frame it references. The file path never enters a document.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Document bytes | `buildDocuments` | SOP Instance UID per document | Every document built before the first download starts | One null result cancels every download in that save |
| Download | `URL.createObjectURL` on a `Blob`, a clicked `<a download>` | The object URL | The click | The URL is revoked after the click. A browser that blocks the second download leaves the status saying how many were offered |
| Marks map `notes` and `keyImages` | `placeDocuments` result applied by the page | `currentGeneration` at the start of the folder read | All items of one load are applied together | A stale generation applies nothing. A refused document adds nothing and does not clear marks already shown |
| Tracking UID | assigned when a mark is created, or read on load | The UID | Written in each save | Kept when a mark moves or is recoloured. A new mark gets a new one |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | A rect `(10,20)` to `(30,50)` writes closed POLYLINE `10,20,30,20,30,50,10,50,10,20`. C.18.6.1.2 | `marks.test.ts` `rect_is_a_closed_polyline` | Row before column, an added 0.5, or four points without closure | implementation |
| AC2 | An ellipse box `(10,20)` to `(30,60)` writes ELLIPSE `20,20,20,60,10,40,30,40`, major axis vertical because height 40 exceeds width 20 | `ellipse_major_axis_first` | Writing the two box corners, or the minor axis first | implementation |
| AC3 | A length `(0,0)` to `(30,40)` with row spacing 2 and column spacing 1 under `PixelSpacing` writes NUM `85.4400374532` mm, `hypot(30 * 1, 40 * 2)` | `length_uses_row_then_column_spacing` | Swapped spacing, which gives `hypot(60, 40)` = 72.11, or `shownFacts` from another image | implementation |
| AC4 | The same length under `ImagerPixelSpacing` writes no NUM | `detector_length_has_no_num` | A detector length written as a patient length | implementation |
| AC5 | A point at column 0 survives a round trip as 0 | `zero_coordinate_survives` | A `value || fallback` that drops 0, which is dcmjs's own `flattenPoints` shape | implementation |
| AC6 | `100.1` reads back as `Math.fround(100.1)` and a second save writes the same bytes for Graphic Data | `graphic_data_is_fl` | Comparing float64 and calling the loss a mismatch, or drift on every save | implementation |
| AC7 | Text at `(5.5,7.25)` "note" writes POINT and a `121106` TEXT, and reads back as a text mark | `text_is_point_and_comment` | Text dropped, or stored as a code meaning | implementation |
| AC8 | Every value type round-trips through `writeTid1500Groups` and `readTid1500Groups`, IMAGE with frame 3 included | `packages/core/src/tid1500-groups.test.ts` | Frame number dropped, or `HAS_OBS_CONTEXT` written with an underscore | implementation |
| AC9 | `writeTid1500Groups` refuses a root that is not a `125007` container, and an ELLIPSE with 2 points | same | Accepting a NUM root, or an ellipse missing its minor axis | implementation |
| AC10 | `writeKeyObjects` with two references, one with frame 2, reads back both and lists both in the evidence sequence | `tid2010.test.ts` extended | Only the first reference, or the F-196 `2.25` study check on a real study | implementation |
| AC11 | `readTid1500` and `readKeyObject` still pass their F-097 and F-196 tests | existing tests | Changing the old writers | implementation |
| AC12 | `placeDocuments` places a group on the image whose SOP Instance UID and frame match, and counts each refusal reason in the list above | `marks.test.ts` `placement_by_sop_instance` | Matching by file name or by position in the series | implementation |
| AC13 | A document named as a predecessor is skipped, and a repeated Tracking UID is placed once | `predecessor_is_skipped` | A deleted mark coming back, or every mark doubled | implementation |
| AC14 | `inspect_json` reports `sopClassUid` and `studyUid` | `inspect_reports_sop_class_and_study` in `tests/preview.rs` | A document recognised by file name | implementation |
| AC15 | After this story's package changes, F-213's `library_routes_serve_only_listed_modules` and `library_files_match_the_index_closure` pass unchanged, so every module `index.ts` now imports is listed and nothing else is served | Those F-213 tests, rerun | A new module imported by `index.ts` and missing from `LIBRARY_FILES`, which breaks Save only in the browser | implementation |
| AC16 | `/marks.mjs` is served as JavaScript and the page holds the import map | `the_page_offers_a_folder_picker` extended | A bare specifier the browser cannot resolve | implementation |
| AC17 | The retired sentences are gone and the new ones are present | Test changes below | One "It is not saved" left on a tool | implementation |
| AC18 | A live save, then a reopen of the folder with the downloads in it, shows each mark and key image on its image, on Chrome | Click-through against a running viewer with `npm run build` done | Anything the unit tests cannot see, such as a blocked download or an import map error | external, sprint close, operator |
| AC19 | A saved SR opens in a second DICOM reader and shows its regions on the right images | Optional, operator's choice of reader | A file only Ocelli reads | external, open |
| AC20 | Before the mark mapping and the document writers are implemented, the reviewer checks the transcriptions in this plan against the DICOM standard text at dicom.nema.org, current edition, and transcribes the two missing attribute lists: PS3.3 C.18.6.1.1 (Graphic Data origin), C.18.6.1.2 (Graphic Type), C.17.2 (SR Document General Module), C.17.6 (Key Object Document Module), the PS3.6 entry for Graphic Data (0070,0022) and its VR FL, and PS3.16 TID 1500, TID 1501 and TID 1410 with the codes 125007, 112039, 112040, 111030, 410668003 and 121106. Any correction lands in this plan before the code | Review note naming the edition read and each section | A writer built on attribute Types nobody checked | reviewer, before implementation |

The exact changes to `examples/folder-viewer/tests/preview.rs`:

- `the_page_offers_a_folder_picker`: the line `if !page.contains("not DICOM SR") || !page.contains("gone when you reload")` is replaced. The test now requires `"TID 1500"`, `"Key Object Selection"`, `">Save marks<"`, `"Nothing is written into the opened folder"` and `"type=\"importmap\""`. It refuses `"not DICOM SR"`, `"gone when you reload"`, `"is not saved"` and `"does not write a Key Object Selection"`. The error text becomes "the page does not say how marks are saved".
- `more_and_use_are_static_pages`: `/more` must no longer contain `"It does not write a Key Object Selection and it is not saved"`.
- `inspect_json_reports_the_series_fields`: unchanged needles keep passing. AC14 is a new test.
- AC15 and the `/marks.mjs` route are new tests.

## Approach

Extend `sr.ts` with `ELLIPSE` and the IMAGE frame. Add the generic serialiser and reader to `tid1500.ts` beside the existing pair, sharing `part10` style helpers already used by `tid2010.ts`. Add the multi-reference pair to `tid2010.ts`. Export all four from `index.ts`. Write `marks.mjs` against those exports. In the page, split documents out in `groupSeries`, call `placeDocuments` after the series exist, and add Save marks and Load marks. In `serve.rs`, add `library_reply` and the routes.

Rejected: dcmjs `TID300` `Ellipse` and `Polyline`, which attach the SCOORD to the first measurement only and so write no geometry for a drawing with no number. Rejected: putting the mark mapping in `@ocelli/core`, where it would be example-specific API. Rejected: keying documents by file path.

## Dependencies, write ownership and resources

Declared: F-201, F-196 and F-207, all done. Source-level: F-094 `sr.ts` and F-097 `tid1500.ts`.

Writes `packages/core/src/sr.ts`, `tid1500.ts`, `tid2010.ts`, `index.ts` and their tests, a new `packages/core/src/tid1500-groups.test.ts`, `examples/folder-viewer/src/marks.mjs` and `marks.test.ts`, `page.html`, `more.html`, `serve.rs`, `preview.rs`, `lib.rs`, `tests/preview.rs`, `CHANGELOG.md`, `docs/lld/public-api.md` and `docs/lld/key-object.md`.

Serial with F-214 and every other S46 folder viewer story, which share `page.html` and `preview.rs`.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, F-216, F-210, F-211, F-208, F-209, F-212, F-213, F-214, then this story last. F-119 is independent. This story therefore builds `readFolder`'s document split on F-216's `POST /file`, reuses F-213's `/lib/` routes and import map, and names F-211's kinds as not saved. No GPU. The packages gate needs `npm run build` before vitest, as the React example's tests already do.

## Verification and evidence reuse

`npx vitest run packages/core/src/tid1500-groups.test.ts packages/core/src/tid2010.test.ts packages/core/src/sr.test.ts examples/folder-viewer/src/marks.test.ts` after `npm run build`. `cargo test -p ocelli-folder-viewer --test preview`. `bin/ocelli.sh gate packages` and `bin/ocelli.sh gate types`. Mutation: add 0.5 to x in `marksToGroups` and see AC1 go red. Swap row and column spacing and see AC3 go red. Remove the predecessor skip and see AC13 go red. A package or page change after the sprint record invalidates it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. Documents carry coordinates and UIDs, not pixels.
- Render-loop allocation: none. Save and load run on a button or a folder open.
- unsafe: none
- Tier A (WebGPU): n/a. Writing and reading documents does not draw.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. The grey image path is unchanged.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Graphic Data for rect, ellipse, freeform, length and text against PS3.3 C.18.6.1.1 and C.18.6.1.2, hand computed above | `examples/folder-viewer/src/marks.test.ts` |
| fixture | Length NUM from row then column spacing, `85.4400374532` mm | same |
| unit | Generic TID 1500 group round trip, refusals, ELLIPSE count | `packages/core/src/tid1500-groups.test.ts`, `sr.test.ts` |
| unit | Multi-reference Key Object round trip and evidence | `packages/core/src/tid2010.test.ts` |
| unit | Placement by SOP Instance UID, each refusal reason, predecessors | `marks.test.ts` |
| contract | Routes, inspect fields, sentences, import map | `examples/folder-viewer/tests/preview.rs` |

## Parity surface covered

The `SR TID 1500` adapter row is unchanged in status. This story writes planar regions into it from the folder viewer. It does not claim cornerstone3D hydration of those regions.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` for the four new exports. `docs/lld/key-object.md` for the multi-reference writer and the attributes it now fills.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 8's closing sentences, "Marks last until you reload the page. They are drawings on that image. They are not written back into the file.", become: Save marks downloads a TID 1500 file and a Key Object Selection file per study, nothing is written into the opened folder, putting the files in the folder and opening it again shows the marks, Load marks reads them into the open study, colour and text size are not saved, and Save needs `npm run build` first. Step 5's Key image names the Key Object Selection. The paragraph on skipped files says SR and Key Object files are read as documents and not listed as series.
- `examples/folder-viewer/src/more.html`, `#documents` (already in the write set): the closing sentence as in the Scope table, and the `Save, load, undo` and `Key Object Selection` code blocks gain `writeTid1500Groups`, `readTid1500Groups`, `writeKeyObjects` and `readKeyObjects`.
- `examples/folder-viewer/src/use.html`, chapter 14 "Annotations as SR": the value type list gains `ELLIPSE` as a graphic type and the IMAGE item's optional `frame`, and one paragraph names the four new exports. Chapter 4: F-213's sentence on `npm run build` gains Save marks.

`README.md` and `use.html` join the write set for these edits.

## Open questions

1. **Route to `@ocelli/core`.** Affects AC15, AC16 and AC18. Options A, B and C above. Recommendation A, serving the built package and dcmjs from the local process. Owner: operator. Deferral blocks the page work.
2. **Order against F-211.** Affects scope. If F-211's angle, Cobb, bidirectional, arrow, probe and region marks exist first, this story either writes them too, which adds weeks, or names them as not saved. Recommendation: land F-215 before F-211 and let F-211 add its kinds to `marks.mjs`. Owner: operator. Deferral keeps the five kinds.
3. **Supersession.** Affects AC13. Options: Predecessor Documents Sequence with a skip on load, which this plan takes, or Tracking UID deduplication alone, under which a deleted mark comes back from an older file. Owner: operator. Deferral keeps the predecessor rule.
4. **Patient and study attributes copied into the files.** Affects AC10, AC18 and AC19. Copying Patient Name and Patient ID puts identifying data in the download. That file is the user's and is never in git, and without it another reader cannot file the document under its patient. The C.17.2 and C.17.6 attribute lists are to be transcribed into this plan before approval. Owner: operator. Deferral writes them empty, Type 2 allows that, and AC19 then cannot pass.
5. **Cornerstone3D hydration.** Affects AC19 and the parity row. The Image Region shape is TID 1410 style. Cornerstone3D's adapters read TID 300 style groups. Options: keep TID 1410 groups, or write the length as a TID 300 NUM `INFERRED FROM` its SCOORD so cornerstone3D 5.8.2 hydrates it. Owner: operator. Deferral keeps TID 1410 and claims no hydration.
6. **Load marks input.** Affects AC12 and AC18. The synthetic study is not a folder, so without a file input its marks cannot be loaded back. Recommendation: keep the input, which this plan includes. Owner: operator. Deferral removes it and leaves the synthetic study save-only.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

1. Answered by the operator, shared with F-213 Q4: A. The Rust process serves the built `packages/core/dist` and dcmjs's ES module under `/lib/` from an allowlist of file names, with an import map in the page. F-213 writes the routes. If the build is absent, Save says so and keeps the marks.
2. Answered by the operator's build order: F-211 lands before this story, which is the other branch of the plan's options. This story keeps the five F-207 kinds and names F-211's kinds as not saved, as the plan's deferral line and its out-of-scope rule state. Saving the F-211 kinds is a later story.
3. Answered: recommended, Predecessor Documents Sequence with a skip on load.
4. Answered: recommended, Patient Name, Patient ID, Study Instance UID, Study Date and Accession Number are copied from the referenced images, as Document attributes says. The C.17.2 and C.17.6 attribute lists are transcribed and checked by the reviewer before implementation of the writers, which is AC20, rather than before approval.
5. Answered: the plan's stated choice, keep TID 1410 style Image Region groups and claim no cornerstone3D hydration.
6. Answered: recommended, keep the Load marks file input.

The transcriptions written from memory keep their flag, and AC20 names every section the reviewer checks. No question is left open.

## Lead amendments before implementation, against F-209's tree

Made by the sprint lead on 2026-11-07 from read-only groundwork against `work/f-209-xai` at 3c206f67, under the operator's delegation of the quality call. Where these differ from the text above, these win. F-215 starts after F-214 lands and re-checks every path and line named here.

1. **A saved length is the mark's own `/measure` reply.** The page does no arithmetic on a measurement. The SR NUM is written only when the reply's calibration source is Pixel Spacing or Imager Pixel Spacing, in the units the reply states. Otherwise the mark is saved without a NUM and the page says so.
2. **Kinds that are not saved stay pinned.** The "It is not saved." sentences pinned in `tests/preview.rs` change only for the kinds this story saves. Angle, Cobb, bidirectional, arrow and probe keep theirs.
3. **Supersession, SR.** Each save names in Predecessor Documents Sequence every document this viewer loaded or saved earlier in the session for that study, so a mark deleted between saves does not come back. A test saves, deletes a mark, saves again, loads both files and shows the mark absent.
4. **Supersession, Key Object Selection.** PS3.3 C.17.6.2 has no Predecessor Documents Sequence. On load, among Key Object Selections this viewer wrote for a study, the newest by Content Date and Content Time, then Instance Number, is shown, and the page names the one shown. A test loads two and shows the newer.
5. **Key images are IMAGE items.** Per TID 2010 row 8 and PS3.3 C.18.4, a key image with a frame is an IMAGE content item with Referenced Frame Number. If F-196's writer in `@ocelli/core` writes COMPOSITE for an image, F-215 corrects it with a test, because F-215 is the first consumer of that output.
6. **Coordinates stay inside the image.** Moving a mark or a freeform handle clamps every point to the range PS3.3 Table C.18.6-1 allows, so every saved mark loads back. A test drags a mark past an edge, saves, loads and finds it.
7. **The mandatory template content is written.** TID 1500 row 3 Observation Context, and the Type 2 attributes Referenced Performed Procedure Step Sequence and Manufacturer on both documents and Performed Procedure Code Sequence on the SR, are present, empty where unknown. The AC20 codes are as transcribed in the groundwork.
8. **Citations and names.** The coordinate rule is PS3.3 Table C.18.6-1. New `inspect_json` UID reads use the non-failing form. Study Time comes from F-212's field. Ellipse axes use absolute widths. The page module is named so that its stem does not collide with `packages/core/src/marks.ts`. Line numbers in the docs are measured at implementation.
