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

### Lead rulings after review pass 1

- **D1.** `readTid1500Groups` decodes text as the file's Specific Character Set says. Under ISO_IR 192 every string is re-decoded as UTF-8, and a multi-valued TextValue is joined with a backslash, so UT text comes back whole. A round-trip test writes and reads "Läsion", "肺" and "a\b" and requires them unchanged, red on today's reader.
- **D2.** Deleting every mark of a study is saved. The page writes an SR whose Imaging Measurements container is empty and which names its predecessors, so the old marks do not come back on load. TID 1500 allows the empty container. A Key Object Selection cannot hold zero references, so the page and README say that unflagging the last key image of a study is not saved, and why. The amendment 3 test gains the delete-all case.
- **D3.** The inspect reply carries the stored Study Time value, and the documents copy it verbatim. The display text is not the source.
- **D4.** `part10` writes File Meta Information Version (0002,0001) and Implementation Class UID (0002,0012), PS3.10 Table 7.1-1. F-196's writer gains them too, through the same helper.
- **D5.** README says which SR classes are read as documents, the four `DOCUMENT_CLASSES`, and that other SR, such as an X-Ray Radiation Dose SR, still shows as a series. `DOCUMENT_CLASSES` is not widened.
- **D6.** The comment states the 1e-3 relative tolerance.
- **S1.** A browser step drags a freeform grip past the image edge and requires it clamped, red with the clamp removed.
- **S2.** A test where a foreign Key Object Selection is newer than Ocelli's requires Ocelli's to win, red under M16.
- **S3.** Both Key Object writers write IMAGE only for an image storage class, and COMPOSITE otherwise, per TID 2010. A test covers a non-image reference.
- **S4.** Every UID the writers make is a `2.25.` value below 2^128, from the same generator the page uses. A test draws many and checks each, red under dcmjs `uid()`.
- **Nitpicks taken.** N2, the unit's code meaning is "millimeter" as CID 7460 gives it. N4, the README line is rewrapped. N5, the user docs say a key image reference must be in the evidence. N6, only an SR whose content was read is named as a predecessor. N1 and N3 stay as they are.
- **AC19 and dciodvfy.** Carried to sprint close with the operator's click-through. dciodvfy is not installed on this host.

### Lead rulings after review pass 2

- **Pass 1's S2 restated.** A newer foreign Key Object Selection does not hide Ocelli's, and both are shown, which is what the code does and what amendment 4 asks. Only Ocelli's own are ranked.
- **D1.** The reader takes the character set from the file's own (0008,0005) bytes, not from dcmjs's rewritten value. It turns each nested string back into its bytes, inverting the windows-1252 or Latin-1 decode the host used, and decodes them with that character set's label, PS3.5 6.1 and Annex D of PS3.3 C.12.1.1.2's defined terms. A group whose character set is unsupported, or uses code extensions, is counted unread rather than guessed. Tests: a vitest case reading an ISO_IR 100 file, one reading an ISO_IR 192 file, and a browser step that round-trips "肺" through save and Load marks in Chromium. Each is red on today's reader. `public-api.md` says what is now true.
- **D2.** The test writes once and copies that one result.
- **D3.** A study that held a key image in the session and is in the folder is counted before the early return, so flag, save, unflag, save shows why the unflag is not saved.
- **D4.** Only studies in the current folder are counted.
- **D5.** A report is named as a predecessor only when every group it holds was placed. An empty report is written only for a study whose read reports were all fully placed. A save in a partial folder never supersedes marks it could not place. A test covers the partial-folder path, red on today's code.
- **D6.** The image class list follows one stated rule: every PS3.4 Table B.5-1 class whose IOD includes the Image Pixel Module, plus the retired image classes of PS3.6 Annex A. Each entry is checked against its IOD in PS3.3. That adds the three classes the review names and any others the rule reaches, including the retired Hardcopy classes. Height Map Segmentation is checked. The LLD states the rule, not a claim of completeness.
- **S1.** A test requires the empty report to omit the Current Requested Procedure Evidence Sequence, red under X4.
- **S2.** A waveform storage class is written WAVEFORM, per TID 2010 row 9, with a test.
- **Nitpicks taken.** The README line is rewrapped to 79 columns. A Save with nothing to save, and only empty reports read, writes nothing. The `key-object.md` header is bumped. `secondary-capture.ts` goes through the same file meta helper.

### Lead rulings after review pass 3

- **D3, the lead's own ruling corrected.** The second sentence of the pass 2 D5 ruling is withdrawn. Its rule that an empty report is written only for a study whose read reports were all fully placed is gone. The predecessor rule alone satisfies D5: a report is named as a predecessor only when every group it holds was placed. Pass 3's page probe shows that rule keeps a partial report's marks and saves a delete-all. The `partialStudies` gate is dropped. The test at `mark-documents.test.ts:624` states the predecessor rule. A test covers the review's path, a study once placed in part and then whole, delete all, save, reload, and requires the deleted mark to stay gone, red on today's code.
- **D1.** The parse is retried with `ignoreErrors` whenever the reader decodes the character set itself, so ISO_IR 203 is read. A vitest case requires `€` back from an ISO_IR 203 file. `public-api.md` and `use.html` name the sets that are read.
- **D2.** When the raw walk cannot run and dcmjs's dataset has no Specific Character Set, the default repertoire is used. The walk also reads Explicit VR Big Endian. A regression test reads an ASCII report from a big endian file and from a deflated one with no groups unread.
- **S1.** One vitest case for each of the C1 refusal, the multi-valued set refusal and the implicit VR walk, red under X1, X2 and X3.
- **Nitpicks taken.** N1, the README line is 79 columns. N2, `ISO_IR 6` is read as the default repertoire, which other tools write. N3, the ISO_IR 166 comment gives 88 characters, and bytes TIS 620 leaves undefined are refused. N4, the byte-to-string step works in chunks, with a test at 200000 bytes.

### Lead rulings after review pass 4

- **D1.** Under ISO_IR 166, A0 is NO-BREAK SPACE, per the ISO-IR 166 registration and ISO 8859-11, and is read as U+00A0. Only DB to DE and FC to FF are refused. The comment says so, and the test loops the refusal over all eight bytes and expects A0 read.
- **D2.** The parse is always retried with `ignoreErrors` when the first parse throws, so a deflated report in a set the walk cannot confirm is counted unread rather than skipped as no document. A regression test reads a deflated ISO_IR 203 file. `use.html` says a deflated file's text is decoded only when it carries no Specific Character Set.
- **D3.** README and the status line say that a report placed only in part is kept, so the marks it holds come back after a delete-all and save.
- **S1.** A vitest case reads a deflated ISO_IR 100 file of ASCII text and requires it counted unread, red under O2.
- **N1** is covered by D1's loop. **N2** stays at module level.

### Lead rulings after review pass 5

- **D1.** `placeDocuments` returns the predecessor UIDs it found, and the page removes them from `sessionPartialReports`, so a partial report a later document replaces is not counted kept. README says such a report "is not replaced by a save from this page". The regression test places P in part, then Q naming P in a second call, and expects `kept` 0, red on today's code.
- **D2.** The comment says a truncated file returns nothing, and a file damaged inside its content can read as fewer groups with or without the retry.
- **Nitpicks taken.** N1, a test expects `kept` 2 for two partial reports in one study. N2, `public-api.md` names the C1 refusal and states the ISO_IR 166 rule once. N3, README names every cause of a report placed in part: an image not in the folder, an unread group, and a mark outside its image.

### Lead rulings after review pass 6

- **D1, and the replaced report placed again.** The page keeps one session-wide set of every SOP Instance UID any read has named as replaced. `placeDocuments` never places a report in that set, whichever order the reads come in, and `notePartialReports` never adds or keeps one. This also closes pass 6's concern outside the range, where a page that read Q and later read P whole placed P's marks again. Tests: Q read, then P read in part, expects `kept` 0. Q read, then P read whole, expects P's marks not placed. Both are red on today's code.
- **D2.** The reason clause is deleted, leaving "A truncated file still returns nothing."
- **D3.** README says a report is placed only in part when one of its marks is not placed or a group of it cannot be read, and names the folder lacking an image as the common case. It does not claim a complete list.
- **N1 taken.** The D1 test holds a second partial report of the same study and expects it to stay recorded, red under pass 6's study-drop mutant.

### Lead rulings after review pass 7

- **D1.** An SR that `placeDocuments` skips as replaced is returned in `predecessors` with `empty: false`, so the page remembers it and the next save names it. PS3.3 Table C.17-2 allows a predecessor whose content is wholly or partially included. Each `partial` row also carries its `reference`, so a report the page removes from `sessionPartialReports` is still remembered. Regression test: read Q, then read P whole, delete every mark, save, then read P and the saved file alone and expect 0 marks. It is red on today's code. A page step in `marks.mjs` covers the two-folder path the review used.
- **N1 taken.** A committed test reads R, Q and P in three calls, R naming Q and Q naming P, and expects neither P nor Q placed.
- **N2 taken.** The `notePartialReports` comment drops the case that cannot happen.
- **Not changed.** A foreign TID 1500 SR that names an Ocelli report hides it on load, which is the operator's skip-on-load answer to open question 3 applied across reads.

### Lead rulings after review pass 8

Pass 8 is clean in its range. Its model of every read order, 401 260 cases, has no failure at the tip. These rulings close what it found beside the range.

- **A mark nobody deleted is never superseded.** Pass 8 found that opening folder A with Q, then folder B with no document, then saving writes an SR with no marks naming Q, so Q's length is lost though nobody deleted it. The rule is that a save names a report as a predecessor only when every mark that report holds is either in this save or was deleted by the user in this page load. The page records deletions, and the delete-all path names a report only under the same rule. Test: the review's folder A with Q, then folder B with no document, then save, then reopen with Q, expects Q's mark back. Red on today's code.
- **N3, no change and no file.** A Save writes no document for a study when no mark of it was added, moved or deleted since its documents were read and since its last save. Test: a folder holding O and the empty SR that replaces it, Save twice, expects no file. Red on today's code.
- **N1.** A page step covers the forward order through `page.html`'s loop: open `ct` with P alone, Load marks with Q, delete the length, save, then reopen `ct` with P and the new file, expecting 0 marks and no "kept" sentence. Red under pass 8's PA.
- **N2.** A vitest case reads E, then P whole, saves with no entries, and expects one document naming P. Red under O1.
- Pass 8's model, extended with these two rules, is re-run by the next reviewer.

### Lead confirmation of the implementer's two readings of the pass 8 rulings

- A report the page hid because a later document replaces it is already settled by that document, so a save names it whatever the page shows. Every other remembered report is named only when each of its marks is in the save or was deleted by the user in this page load. An SR this page saved is remembered with the marks it wrote.
- For N3, a report hidden by a name an earlier read gave counts as a change to the study, because what the folder shows differs from its own files. A report hidden by a document in the same folder does not count as a change.

### Lead rulings after review pass 9

Pass 9's model, 280 420 cases, holds the two pass 8 invariants at zero failures and finds one new class: a deleted mark that returns with nothing saying so.

- **D1, said, not silently lost.** A save does not carry marks it cannot show into a new SR. Instead `buildDocuments` counts every remembered report that is not replaced, not settled, and holds a mark the user deleted, and the status line names them: "N earlier reports still hold a mark you deleted, so that deletion is not saved. Open the folder holding them and save there." "Replaces the earlier one" is said only when no such report is left. A vitest case and a page step cover the review's trigger, folder A with T, delete one mark, folder B with no document, draw, save, red on today's code.
- **D2.** A mark read from an SR with no Tracking Unique Identifier gets an identifier that is stable for the page load, keyed by the report's SOP Instance UID and the group's index, so reading the same report twice yields the same identifier and the report can be settled. A vitest case and a page step, folder FU twice, delete, save, reopen, red on today's code.
- **D3.** README says the no-change rule as the code measures it: a study with no mark added, moved, deleted or edited, and no key image flagged or unflagged, since the page loaded or since its last save, writes nothing. The pass 8 ruling's wording is corrected to the same.
- **Nitpicks taken.** N1, a page step pressing Save twice expects one file and the no-change sentence, red under PG and PD. N3, a vitest case reaches the deletion half of `settled`, red under O1. N2 and N4 are recorded and not changed.

### Operator decision after review pass 10, 2026-11-08: state resets per folder

Passes 9 and 10 found the same class in changing shapes, a deleted mark lost or returned with nothing said, and every case needed two or more folders opened in one page session. The operator chose to reset per folder. Opening a folder clears every remembered document, deletion, replacement, partial report, stable identifier and changed study. A save reasons only about the open folder's own files and any SR read into it by Load marks since the folder was opened. Nothing carries from one folder to the next. The rulings after passes 5 to 9 that exist only for state across folders are withdrawn where this makes them unreachable, and those within one folder stand. README says that a folder's marks are saved from that folder, and that opening another folder starts afresh. Pass 10's D1, D2 and D3 are judged under this rule: each is fixed if it still occurs within one folder, and its test is replaced by a one-folder case or removed if it needed two folders. The page steps that needed two folders are rewritten as one-folder steps where their behaviour still applies, or replaced by a step showing that a second folder carries nothing from the first.

### Per-folder implementation clarification, 2026-11-08

The cross-folder settlement and held-deletion records are removed. Within one folder, reports hidden by an earlier Load marks read still count as a change. Identifiers assigned to groups without Tracking Unique Identifiers remain stable within that folder and are cleared on the next folder open. Reading the same untracked group again does not add another visible mark.

### Lead rulings after review pass 11, 2026-11-08

- **D1.** Load the document module before taking the synchronous save snapshot, and discard a pending save if the folder generation changed during that load. Same-folder edits made during the load are included. Permanent browser cases cover both paths through the Save marks button and inspect the downloaded groups.
- **D2.** A repeated untracked group is counted as already shown, not newly placed. The browser check requires one visible mark and the corresponding status counts.
- **D3.** The within-folder predecessor rule applies to marks already displayed. A loaded note records its source report. Reading a later SR removes notes sourced from its predecessors before deduplication, preserving unrelated drawings. A successful save associates its saved marks with the new report, so a later read cannot discard that report's content merely by naming an older source. This defect predates the reset and is taken here because the retained read-order policy requires it.
