import { describe, expect, it } from "vitest";

import dcmjs from "dcmjs";

import { readKeyObjects, readTid1500Groups, type ContentItem, type DocumentReference } from "@ocelli/core";

import {
  buildDocuments,
  froundPoints,
  groupToMark,
  markToGroup,
  notePartialReports,
  rememberReport,
  stableTracking,
  placeDocuments,
  readDocument,
  type Mark,
  type MarkImage,
  type MarkPoint,
} from "./mark-documents.mjs";

// F-215. Every Graphic Data value below is hand computed from PS3.3 2026d
// Table C.18.6-1, Graphic Data (0070,0022): (column,row) pairs with the
// origin at the top left hand corner of the top left pixel, 0\0, through
// Columns\Rows. Page image coordinates already have that origin, so no 0.5
// is added. Graphic Type is C.18.6.1.2: POLYLINE is closed when its first
// and last vertices are the same, and ELLIPSE gives the major axis
// endpoints then the minor axis endpoints. Graphic Data is FL, PS3.6. The
// group is PS3.16 2026d TID 1410.

const CT = "1.2.840.10008.5.1.4.1.1.2";
const ENHANCED_CT = "1.2.840.10008.5.1.4.1.1.2.1";

type Image = MarkImage;

function image(sopInstance: string, extra: Partial<Image> = {}): Image {
  return {
    key: "file:" + sopInstance + ":" + String(extra.frame ?? 0),
    sopClass: CT,
    sopInstance,
    series: "1.2.3.4.6",
    study: "1.2.3.4.5",
    frame: 0,
    frames: 1,
    columns: 128,
    rows: 96,
    patientName: "Synthetic^Phantom",
    patientId: "SYNTH-1",
    studyDate: "20240102",
    studyTime: "10:15:00",
    accession: "",
    ...extra,
  };
}

const ct = image("1.2.3.4.7");

type Point = MarkPoint;

function mark(type: string, points: Point[], extra: Partial<Mark> = {}): Mark {
  return { type, points, tracking: "2.25.900", ...extra };
}

function region(group: ContentItem | null): ContentItem & { valueType: "SCOORD" } {
  const found = group?.children.find((item) => item.valueType === "SCOORD");
  if (found === undefined || found.valueType !== "SCOORD") {
    throw new Error("expected an image region");
  }
  return found;
}

function flat(group: ContentItem | null): number[] {
  return region(group).points.flat();
}

function lengthReply(source: string, value: number, unit: string): unknown {
  return {
    shape: "length",
    calibration: { source, row_mm: 2, column_mm: 1 },
    length: { value, unit, text: value.toFixed(1) + " " + unit },
  };
}

describe("a mark as a TID 1410 group", () => {
  it("rect_is_a_closed_polyline", () => {
    // AC1. Corners (10,20) and (30,50): the four corners in order, then the
    // first again, column before row.
    const group = markToGroup(mark("rect", [{ x: 10, y: 20 }, { x: 30, y: 50 }]), ct);
    expect(region(group).graphic).toBe("POLYLINE");
    expect(flat(group)).toEqual([10, 20, 30, 20, 30, 50, 10, 50, 10, 20]);
  });

  it("ellipse_major_axis_first", () => {
    // AC2. Box (10,20) to (30,60) is 20 wide and 40 high, so the major axis
    // is vertical. cx = 20, cy = 40. Major (20,20) (20,60), minor (10,40) (30,40).
    const tall = markToGroup(mark("ellipse", [{ x: 10, y: 20 }, { x: 30, y: 60 }]), ct);
    expect(region(tall).graphic).toBe("ELLIPSE");
    expect(flat(tall)).toEqual([20, 20, 20, 60, 10, 40, 30, 40]);
    // Box (10,20) to (50,30) is 40 wide, so the major axis is horizontal.
    // cx = 30, cy = 25.
    const wide = markToGroup(mark("ellipse", [{ x: 10, y: 20 }, { x: 50, y: 30 }]), ct);
    expect(flat(wide)).toEqual([10, 25, 50, 25, 30, 20, 30, 30]);
    // Lead amendment 8. Corners dragged past each other use absolute widths.
    const crossed = markToGroup(mark("ellipse", [{ x: 30, y: 60 }, { x: 10, y: 20 }]), ct);
    expect(flat(crossed)).toEqual([20, 20, 20, 60, 10, 40, 30, 40]);
  });

  it("freeform_is_closed_and_length_is_open", () => {
    const contour = markToGroup(mark("contour", [{ x: 1, y: 2 }, { x: 5, y: 2 }, { x: 3, y: 6 }]), ct);
    expect(region(contour).graphic).toBe("POLYLINE");
    expect(flat(contour)).toEqual([1, 2, 5, 2, 3, 6, 1, 2]);
    const length = markToGroup(mark("length", [{ x: 0, y: 0 }, { x: 30, y: 40 }]), ct);
    expect(region(length).graphic).toBe("POLYLINE");
    expect(flat(length)).toEqual([0, 0, 30, 40]);
  });

  it("length_uses_row_then_column_spacing", () => {
    // AC3 under lead amendment 1. The NUM is the mark's own /measure reply.
    // The local process computes hypot(30 * 1, 40 * 2) = hypot(30, 80)
    // = 85.44003745317531 mm, with column spacing 1 and row spacing 2, and
    // tests/measure.rs holds that arithmetic. Twelve significant digits fit
    // the 16 characters of DS: 85.4400374532.
    const reply = lengthReply("pixel-spacing", Math.hypot(30, 80), "mm");
    const group = markToGroup(mark("length", [{ x: 0, y: 0 }, { x: 30, y: 40 }], { result: reply }), ct);
    const num = group?.children.find((item) => item.valueType === "NUM");
    expect(num).toMatchObject({
      valueType: "NUM",
      relationship: "CONTAINS",
      concept: { scheme: "SCT", value: "410668003", meaning: "Length" },
      number: 85.4400374532,
      unit: "mm",
    });
    // Imager Pixel Spacing corrected by the magnification factor is also mm.
    const imager = markToGroup(
      mark("length", [{ x: 0, y: 0 }, { x: 30, y: 40 }], { result: lengthReply("imager-spacing", 10, "mm") }),
      ct,
    );
    expect(imager?.children.some((item) => item.valueType === "NUM")).toBe(true);
  });

  it("detector_length_has_no_num", () => {
    // AC4. Imager Pixel Spacing alone is a length at the detector, not a
    // patient length, so the group holds the geometry and no NUM. Pixels
    // and a length still measuring get no NUM either.
    for (const result of [
      lengthReply("detector", 85.44, "mm at the detector"),
      lengthReply("uncalibrated", 50, "px"),
      // A detector source is never a patient length, whatever its unit says.
      lengthReply("detector", 85.44, "mm"),
      undefined,
    ]) {
      const group = markToGroup(mark("length", [{ x: 0, y: 0 }, { x: 30, y: 40 }], { result }), ct);
      expect(group?.children.some((item) => item.valueType === "NUM")).toBe(false);
      expect(flat(group)).toEqual([0, 0, 30, 40]);
    }
  });

  it("text_is_point_and_comment", () => {
    // AC7. One POINT and a (121106, DCM, "Comment") TEXT.
    const group = markToGroup(mark("text", [{ x: 5.5, y: 7.25 }], { text: "note" }), ct);
    expect(region(group).graphic).toBe("POINT");
    expect(flat(group)).toEqual([5.5, 7.25]);
    const comment = group?.children.find((item) => item.valueType === "TEXT" && item.concept.value === "121106");
    expect(comment).toMatchObject({ relationship: "CONTAINS", text: "note" });
    const back = groupToMark(group);
    expect(back).toMatchObject({ mark: { type: "text", text: "note", points: [{ x: 5.5, y: 7.25 }] } });
  });

  it("writes the tracking pair and the image it was drawn on", () => {
    const enhanced = image("1.2.3.4.9", { sopClass: ENHANCED_CT, frames: 4, frame: 2 });
    const group = markToGroup(mark("rect", [{ x: 1, y: 1 }, { x: 2, y: 2 }]), enhanced);
    expect(group?.concept.value).toBe("125007");
    expect(group?.children.slice(0, 2)).toMatchObject([
      { relationship: "HAS_OBS_CONTEXT", valueType: "TEXT", concept: { value: "112039" }, text: "ocelli-folder-viewer:rect" },
      { relationship: "HAS_OBS_CONTEXT", valueType: "UIDREF", concept: { value: "112040" }, uid: "2.25.900" },
    ]);
    expect(region(group).concept.value).toBe("111030");
    // A multi-frame object names its frame, counted from 1, PS3.3 C.18.4.
    expect(region(group).children).toMatchObject([
      { relationship: "SELECTED_FROM", valueType: "IMAGE", sopClass: ENHANCED_CT, sopInstance: "1.2.3.4.9", frame: 3 },
    ]);
    const single = markToGroup(mark("rect", [{ x: 1, y: 1 }, { x: 2, y: 2 }]), ct);
    expect("frame" in (region(single).children[0] ?? {})).toBe(false);
  });

  it("does not write a kind it does not save", () => {
    for (const type of ["angle", "cobb", "bidirectional", "arrow", "probe"]) {
      expect(markToGroup(mark(type, [{ x: 1, y: 1 }, { x: 2, y: 2 }]), ct)).toBeNull();
    }
  });
});

function report(marks: Mark[], target: Image = ct, extra: Record<string, unknown> = {}) {
  return buildDocuments({
    entries: [{ image: target, marks, key: false }],
    predecessors: {},
    contentDate: "20261108",
    contentTime: "093000",
    ...extra,
  });
}

function onlyBytes(built: ReturnType<typeof buildDocuments>, kind: string): Uint8Array {
  const found = built.documents.find((document) => document.kind === kind);
  if (found === undefined) {
    throw new Error("expected a " + kind + " document");
  }
  return found.bytes;
}

describe("documents", () => {
  it("zero_coordinate_survives", () => {
    // AC5. Column 0 is the left edge of the image, not a missing value.
    const built = report([mark("length", [{ x: 0, y: 0 }, { x: 0, y: 40 }])]);
    const read = readTid1500Groups(onlyBytes(built, "sr"));
    const back = groupToMark(read?.groups[0] ?? null);
    expect(back).toMatchObject({ mark: { type: "length", points: [{ x: 0, y: 0 }, { x: 0, y: 40 }] } });
  });

  it("graphic_data_is_fl", () => {
    // AC6. 100.1 has no float32, so it reads back as Math.fround(100.1).
    // After the page keeps that value a second save writes the same numbers.
    const drawn = mark("length", [{ x: 100.1, y: 3 }, { x: 4, y: 5 }]);
    // The group already holds the FL value, so the tree, the bytes and the
    // value read back agree and none of them is a mismatch.
    expect(region(markToGroup(drawn, ct)).points[0]?.[0]).toBe(Math.fround(100.1));
    const first = readTid1500Groups(onlyBytes(report([drawn]), "sr"));
    const point = region(first?.groups[0] ?? null).points[0];
    expect(point?.[0]).toBe(Math.fround(100.1));
    const kept = froundPoints(drawn);
    expect(kept.points[0]?.x).toBe(Math.fround(100.1));
    expect(froundPoints(kept)).toEqual(kept);
    const second = readTid1500Groups(onlyBytes(report([kept]), "sr"));
    expect(second?.groups).toEqual(first?.groups);
  });

  it("writes one SR and one Key Object Selection per study and copies the study", () => {
    const other = image("1.2.9.1", { study: "1.2.9", series: "1.2.9.2", key: "other" });
    const built = buildDocuments({
      entries: [
        { image: ct, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], key: true },
        { image: other, marks: [mark("text", [{ x: 1, y: 1 }], { text: "a", tracking: "2.25.901" })], key: false },
        { image: image("1.2.3.4.10", { key: "b" }), marks: [mark("angle", [{ x: 1, y: 1 }])], key: false },
      ],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    expect(built.failed).toBe("");
    expect(built.documents.map((document) => [document.kind, document.study])).toEqual([
      ["sr", "1.2.3.4.5"],
      ["ko", "1.2.3.4.5"],
      ["sr", "1.2.9"],
    ]);
    expect(built.saved).toEqual({ marks: 2, keyImages: 1 });
    expect(built.unsaved).toEqual({ angle: 1 });
    const keys = readKeyObjects(onlyBytes(built, "ko"));
    expect(keys?.references).toEqual([{ sopClass: CT, sopInstance: "1.2.3.4.7" }]);
    // Lead amendment 8: Study Time is F-212's shown text, written as TM.
    const sr = built.documents[0];
    expect(sr?.reference.sopInstance.startsWith("2.25.")).toBe(true);
    expect(sr?.name).toMatch(/^ocelli-marks-20261108-093000-[0-9]+\.dcm$/u);
  });

  it("does not save a mark on an image with no SOP Instance UID", () => {
    const built = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], image("", { key: "x" }));
    expect(built.documents).toEqual([]);
    expect(built.noUid).toBe(1);
  });

  it("names every earlier document of the study as a predecessor", () => {
    const earlier = { series: "2.25.71", sopClass: "1.2.840.10008.5.1.4.1.1.88.34", sopInstance: "2.25.72" };
    const built = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], ct, {
      predecessors: { "1.2.3.4.5": [earlier] },
    });
    expect(readTid1500Groups(onlyBytes(built, "sr"))?.predecessors).toEqual(["2.25.72"]);
  });
});

type Placed = ReturnType<typeof placeDocuments>;

function load(built: ReturnType<typeof buildDocuments>[], images: Image[], shown: string[] = []): Placed {
  const documents = built.flatMap((one) =>
    one.documents.map((document) => readDocument(document.name, document.bytes)),
  );
  return placeDocuments(documents, images, new Set(shown));
}

describe("placing loaded documents", () => {
  it("placement_by_sop_instance", () => {
    // AC12. A group lands on the image whose SOP Instance UID and frame it
    // names, wherever that image is in the folder.
    const enhanced = image("1.2.3.4.9", { sopClass: ENHANCED_CT, frames: 3, frame: 1, key: "enhanced:1" });
    const built = buildDocuments({
      entries: [
        { image: ct, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], key: false },
        { image: enhanced, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.902" })], key: true },
      ],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    const folder = [
      image("1.2.3.4.9", { sopClass: ENHANCED_CT, frames: 3, frame: 0, key: "enhanced:0" }),
      image("1.2.3.4.9", { sopClass: ENHANCED_CT, frames: 3, frame: 1, key: "enhanced:1" }),
      image("1.2.3.4.9", { sopClass: ENHANCED_CT, frames: 3, frame: 2, key: "enhanced:2" }),
      { ...ct, key: "moved" },
    ];
    const placed = load([built], folder);
    expect(placed.marks.map((one) => [one.key, one.mark.type, one.mark.tracking])).toEqual([
      ["moved", "rect", "2.25.900"],
      ["enhanced:1", "length", "2.25.902"],
    ]);
    expect(placed.marks[0]?.mark.points).toEqual([{ x: 1, y: 1 }, { x: 4, y: 4 }]);
    expect(placed.keys).toEqual(["enhanced:1"]);
    expect(placed.unplaced).toEqual({});
  });

  it("counts every reason a group is not placed", () => {
    const group = (graphic: string, points: number[][], frame?: number, sopInstance = "1.2.3.4.7"): ContentItem => ({
      relationship: null,
      valueType: "CONTAINER",
      concept: { scheme: "DCM", value: "125007", meaning: "Measurement Group" },
      children: [
        {
          relationship: "CONTAINS",
          valueType: "SCOORD",
          concept: { scheme: "DCM", value: "111030", meaning: "Image Region" },
          graphic: graphic as "POINT",
          points,
          children: [
            {
              relationship: "SELECTED_FROM",
              valueType: "IMAGE",
              concept: { scheme: "DCM", value: "121112", meaning: "Source of Measurement" },
              sopClass: CT,
              sopInstance,
              ...(frame === undefined ? {} : { frame }),
              children: [],
            },
          ],
        },
      ],
    });
    const square = [[1, 1], [5, 1], [5, 5], [1, 1]];
    const groups: ContentItem[] = [
      group("POLYLINE", square, undefined, "1.2.99"), // missing
      group("POLYLINE", square, undefined, "1.2.3.4.20"), // duplicate
      group("POLYLINE", square, undefined, "1.2.3.4.9"), // frameless on a multi-frame object
      group("POLYLINE", [[1, 1], [129, 1], [5, 5], [1, 1]]), // outside, 129 > Columns 128
      group("CIRCLE", [[5, 5], [8, 5]]), // graphic
      group("ELLIPSE", [[0, 0], [10, 10], [10, 0], [0, 10]]), // graphic, axes not parallel to the edges
      {
        relationship: null,
        valueType: "CONTAINER",
        concept: { scheme: "DCM", value: "125007", meaning: "Measurement Group" },
        children: [
          {
            relationship: "CONTAINS",
            valueType: "NUM",
            concept: { scheme: "SCT", value: "410668003", meaning: "Length" },
            number: 12.5,
            unit: "mm",
            children: [],
          },
        ],
      }, // geometry, the F-097 shape
      group("POLYLINE", [[1, 1], [128, 96], [0, 96], [1, 1]]), // placed, at Columns and Rows
    ];
    const documents = [
      {
        name: "a.dcm",
        kind: "sr" as const,
        study: "1.2.3.4.5",
        series: "2.25.1",
        sopClass: "1.2.840.10008.5.1.4.1.1.88.34",
        sopInstance: "2.25.2",
        predecessors: [],
        groups,
        unread: 1,
      },
    ];
    const folder = [
      ct,
      image("1.2.3.4.20", { key: "d1" }),
      image("1.2.3.4.20", { key: "d2" }),
      image("1.2.3.4.9", { frames: 2, frame: 0, key: "m0" }),
      image("1.2.3.4.9", { frames: 2, frame: 1, key: "m1" }),
    ];
    const placed = placeDocuments(documents, folder, new Set());
    expect(placed.unplaced).toEqual({
      missing: 1,
      duplicate: 1,
      frameless: 1,
      outside: 1,
      graphic: 2,
      geometry: 1,
      unread: 1,
    });
    expect(placed.marks).toHaveLength(1);
    expect(placed.marks[0]?.mark.type).toBe("contour");
  });

  it("predecessor_is_skipped", () => {
    // AC13 and lead amendment 3. Save, delete a mark, save again with the
    // first named as a predecessor, then load both: the deleted mark stays
    // gone and the kept one is placed once.
    const kept = mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.910" });
    const deleted = mark("rect", [{ x: 6, y: 6 }, { x: 9, y: 9 }], { tracking: "2.25.911" });
    const first = report([kept, deleted]);
    const firstSr = first.documents[0];
    if (firstSr === undefined) {
      throw new Error("expected the first save");
    }
    const second = report([kept], ct, { predecessors: { "1.2.3.4.5": [firstSr.reference] } });
    const placed = load([first, second], [ct]);
    expect(placed.marks.map((one) => one.mark.tracking)).toEqual(["2.25.910"]);
    expect(placed.replaced).toEqual([firstSr.name]);
    // Without the predecessor the same two files give the kept mark once,
    // by its Tracking Unique Identifier, and bring the deleted one back.
    const unlinked = load([first, report([kept])], [ct]);
    expect(unlinked.marks.map((one) => one.mark.tracking).sort()).toEqual(["2.25.910", "2.25.911"]);
    expect(unlinked.repeated).toBe(1);
    // A mark already shown is not added again.
    expect(load([second], [ct], ["2.25.910"]).marks).toEqual([]);
  });

  it("shows the newest Key Object Selection this package wrote", () => {
    // Lead amendment 4. C.17.6.2 has no Predecessor Documents Sequence, so
    // the newest by Content Date and Content Time wins.
    const two = image("1.2.3.4.11", { key: "two" });
    const older = buildDocuments({
      entries: [{ image: ct, marks: [], key: true }],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    const newer = buildDocuments({
      entries: [{ image: two, marks: [], key: true }],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "101500",
    });
    const placed = load([newer, older], [ct, two]);
    expect(placed.keys).toEqual(["two"]);
    expect(placed.keyObjects).toEqual([newer.documents[0]?.name]);
    expect(placed.replaced).toEqual([older.documents[0]?.name]);
  });

  it("draws a group another writer made when its graphic is in the table", () => {
    const back = groupToMark({
      relationship: null,
      valueType: "CONTAINER",
      concept: { scheme: "DCM", value: "125007", meaning: "Measurement Group" },
      children: [
        {
          relationship: "CONTAINS",
          valueType: "SCOORD",
          concept: { scheme: "DCM", value: "111030", meaning: "Image Region" },
          graphic: "ELLIPSE",
          points: [[10, 25], [50, 25], [30, 20], [30, 30]],
          children: [
            {
              relationship: "SELECTED_FROM",
              valueType: "IMAGE",
              concept: { scheme: "DCM", value: "121112", meaning: "Source of Measurement" },
              sopClass: CT,
              sopInstance: "1.2.3.4.7",
              children: [],
            },
          ],
        },
      ],
    });
    expect(back).toMatchObject({
      mark: { type: "ellipse", tracking: "", points: [{ x: 10, y: 20 }, { x: 50, y: 30 }] },
    });
  });

  it("reads a file that is not a document as nothing", () => {
    expect(readDocument("x.dcm", new Uint8Array([1, 2, 3]))).toBeNull();
  });

  it("names as predecessors only the reports whose content was read", () => {
    // Review pass 1, N6. Predecessor Documents Sequence lists documents whose
    // content the new one includes, PS3.3 C.17.2.
    const read = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])]);
    const documents = read.documents.map((document) => readDocument(document.name, document.bytes));
    const unreadable = {
      name: "foreign.dcm",
      kind: "sr" as const,
      study: "1.2.3.4.5",
      series: "2.25.81",
      sopClass: "1.2.840.10008.5.1.4.1.1.88.33",
      sopInstance: "2.25.82",
      predecessors: [],
      groups: [],
      unread: 3,
    };
    const placed = placeDocuments([...documents, unreadable], [ct], new Set());
    expect(placed.predecessors.map((one) => one.reference.sopInstance)).toEqual([read.documents[0]?.reference.sopInstance]);
    expect(placed.predecessors[0]?.study).toBe("1.2.3.4.5");
  });
});

function datasetOf(bytes: Uint8Array): Record<string, unknown> {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  return dcmjs.data.DicomMetaDictionary.naturalizeDataset(dcmjs.data.DicomMessage.readFile(copy).dict);
}

describe("review pass 1", () => {
  it("saves a study whose every mark was deleted", () => {
    // D2 and lead amendment 3. PS3.16 TID 1500 row 6 may be empty, so the
    // second save is an SR of no groups that names the first, and loading
    // both files places nothing.
    const first = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])]);
    const firstSr = first.documents[0];
    if (firstSr === undefined) {
      throw new Error("expected the first save");
    }
    const second = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [firstSr.reference] },
      contentDate: "20261108",
      contentTime: "100000",
    });
    expect(second.documents.map((document) => document.kind)).toEqual(["sr"]);
    expect(second.cleared).toBe(1);
    const placed = load([first, second], [ct]);
    expect(placed.marks).toEqual([]);
    expect(placed.replaced).toEqual([firstSr.name]);
  });

  it("counts the last key image of a study unflagged and writes no Key Object Selection", () => {
    // D2. TID 2010 rows 8 to 10 need at least one reference.
    const built = buildDocuments({
      entries: [{ image: ct, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], key: false }],
      studies: { "1.2.3.4.5": ct },
      keyStudies: ["1.2.3.4.5"],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    expect(built.documents.map((document) => document.kind)).toEqual(["sr"]);
    expect(built.unflagged).toBe(1);
  });

  it("copies the stored Study Time verbatim", () => {
    // D3. PS3.3 C.7.2.1 Study Time, a TM of PS3.5 6.2 with its fraction.
    const built = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], image("1.2.3.4.7", { studyTime: "101500.25" }));
    expect(datasetOf(onlyBytes(built, "sr")).StudyTime).toBe("101500.25");
    // A value that is not a TM is written empty, Type 2, rather than refused.
    const odd = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], image("1.2.3.4.7", { studyTime: "10:15:00" }));
    expect(datasetOf(onlyBytes(odd, "sr")).StudyTime).toBe("");
  });

  it("keeps a foreign Key Object Selection beside Ocelli's even when it is newer", () => {
    // S2. The newest rule is lead amendment 4's, for the Key Object
    // Selections this package wrote, because PS3.3 C.17.6.2 has no
    // Predecessor Documents Sequence. A foreign one is shown whatever its
    // Content Time, and it does not hide Ocelli's.
    const two = image("1.2.3.4.11", { key: "two" });
    const ours = buildDocuments({
      entries: [{ image: ct, marks: [], key: true }],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    const theirs = buildDocuments({
      entries: [{ image: two, marks: [], key: true }],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "120000",
    });
    const foreign = theirs.documents.map((document) => {
      const read = readDocument(document.name, document.bytes);
      return read === null || read.kind !== "ko" ? null : { ...read, manufacturer: "Other" };
    });
    const ocelli = ours.documents.map((document) => readDocument(document.name, document.bytes));
    const placed = placeDocuments([...ocelli, ...foreign], [ct, two], new Set());
    expect(placed.keys.sort()).toEqual([ct.key, "two"].sort());
    expect(placed.replaced).toEqual([]);
  });
});

describe("review pass 2", () => {
  it("counts an unflagged key image only for a study in this folder", () => {
    // D4. A Key Object Selection saved for a study of an earlier folder says
    // nothing about this one.
    const other = image("9.9.9.1", { study: "9.9.9", series: "9.9.9.2", key: "other" });
    const built = buildDocuments({
      entries: [{ image: other, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])], key: false }],
      studies: { "9.9.9": other },
      keyStudies: ["1.2.3.4.5"],
      predecessors: {},
      contentDate: "20261108",
      contentTime: "093000",
    });
    expect(built.unflagged).toBe(0);
  });

  it("does not replace a report whose marks this folder could not place", () => {
    // D5, by the predecessor rule alone. PS3.3 C.17.2: a predecessor is a
    // document whose content the new one includes. A report read in a folder
    // that lacks the image its mark is on was not included, so it is never
    // named, a save there writes nothing, and the whole study still shows
    // the mark.
    const elsewhere = image("1.2.3.4.30", { key: "elsewhere" });
    const saved = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.930" })], elsewhere);
    const documents = saved.documents.map((document) => readDocument(document.name, document.bytes));
    const partial = placeDocuments(documents, [ct], new Set());
    expect(partial.marks).toEqual([]);
    expect(partial.unplaced).toEqual({ missing: 1 });
    expect(partial.predecessors).toEqual([]);
    const save = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      contentDate: "20261108",
      contentTime: "100000",
    });
    expect(save.documents).toEqual([]);
    // Review pass 3, D3. Beside it, a report placed whole and then cleared:
    // the empty report names only that one, so the partial report is not
    // replaced, and the whole study shows its mark and not the deleted one.
    const deleted = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.931" })]).documents[0];
    if (deleted === undefined) {
      throw new Error("expected a report");
    }
    const mixed = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [deleted.reference] },
      contentDate: "20261108",
      contentTime: "100000",
    });
    expect(mixed.documents.map((document) => document.kind)).toEqual(["sr"]);
    const empty = mixed.documents[0];
    if (empty === undefined) {
      throw new Error("expected the empty report");
    }
    const emptyRead = readDocument(empty.name, empty.bytes);
    expect(emptyRead?.kind === "sr" ? emptyRead.predecessors : null).toEqual([deleted.reference.sopInstance]);
    const all = [...documents, readDocument(deleted.name, deleted.bytes), emptyRead];
    const back = placeDocuments(all, [ct, elsewhere], new Set());
    expect(back.marks.map((one) => one.mark.tracking)).toEqual(["2.25.930"]);
    const whole = placeDocuments(documents, [ct, elsewhere], new Set());
    expect(whole.marks.map((one) => one.mark.tracking)).toEqual(["2.25.930"]);
  });

  it("writes no second empty report when the only reports read were empty", () => {
    // A save with nothing to save after an empty report has nothing new to say.
    const first = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])]);
    const firstSr = first.documents[0];
    if (firstSr === undefined) {
      throw new Error("expected the first save");
    }
    const cleared = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [firstSr.reference] },
      contentDate: "20261108",
      contentTime: "100000",
    });
    const emptySr = cleared.documents[0];
    if (emptySr === undefined) {
      throw new Error("expected the empty report");
    }
    const read = placeDocuments([readDocument(emptySr.name, emptySr.bytes)], [ct], new Set());
    expect(read.predecessors).toEqual([{ study: "1.2.3.4.5", reference: emptySr.reference, empty: true }]);
    const again = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": read.predecessors.map((one) => ({ ...one.reference, empty: one.empty })) },
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(again.documents).toEqual([]);
  });
});

describe("review pass 4", () => {
  it("counts a report placed only in part as kept when a delete all is saved", () => {
    // D3. A report placed only in part is never a predecessor, PS3.3 C.17.2,
    // so a delete all does not replace it and its marks come back. The save
    // counts it, so the status line can say so.
    const elsewhere = image("1.2.3.4.30", { key: "elsewhere" });
    const part = buildDocuments({
      entries: [
        { image: elsewhere, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.940" })], key: false },
        { image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.941" })], key: false },
      ],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      contentDate: "20261108",
      contentTime: "090000",
    }).documents[0];
    const wholeReport = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.942" })]).documents[0];
    if (part === undefined || wholeReport === undefined) {
      throw new Error("expected two reports");
    }
    const documents = [readDocument(part.name, part.bytes), readDocument(wholeReport.name, wholeReport.bytes)];
    const placed = placeDocuments(documents, [ct], new Set());
    expect(placed.marks.map((one) => one.mark.tracking)).toEqual(["2.25.941", "2.25.942"]);
    expect(placed.partial).toEqual([{ study: "1.2.3.4.5", sopInstance: part.reference.sopInstance, reference: part.reference }]);
    expect(placed.predecessors.map((one) => one.reference.sopInstance)).toEqual([wholeReport.reference.sopInstance]);
    const cleared = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": placed.predecessors.map((one) => one.reference) },
      partialReports: { "1.2.3.4.5": [part.reference.sopInstance] },
      contentDate: "20261108",
      contentTime: "100000",
    });
    expect(cleared.documents.map((document) => document.kind)).toEqual(["sr"]);
    expect(cleared.kept).toBe(1);
    // Once the same report is placed whole it is a predecessor, so not kept.
    const later = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [wholeReport.reference, part.reference] },
      partialReports: { "1.2.3.4.5": [part.reference.sopInstance] },
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(later.kept).toBe(0);
    // A study not in this folder is not counted.
    const elsewhereStudy = buildDocuments({
      entries: [],
      studies: {},
      predecessors: {},
      partialReports: { "1.2.3.4.5": [part.reference.sopInstance] },
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(elsewhereStudy.kept).toBe(0);
  });
});

describe("review pass 5", () => {
  const elsewhere = image("1.2.3.4.30", { key: "elsewhere" });

  /** A report with a rectangle on an image not in the folder and a length on `ct`. */
  function partReport(tracking: string, time: string) {
    const built = buildDocuments({
      entries: [
        { image: elsewhere, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: tracking + "1" })], key: false },
        { image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: tracking + "2" })], key: false },
      ],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      contentDate: "20261108",
      contentTime: time,
    }).documents[0];
    if (built === undefined) {
      throw new Error("expected a report");
    }
    return built;
  }

  function asRecord(map: Map<string, Map<string, DocumentReference>>): Record<string, string[]> {
    return Object.fromEntries(Array.from(map, ([study, reports]) => [study, Array.from(reports.keys())]));
  }

  it("does not count a partial report a later document replaces", () => {
    // D1. Q names P in Predecessor Documents Sequence, PS3.3 C.17.2, so a
    // later read of P and Q shows Q and not P, and P's marks do not come
    // back. P is placed in part and Q, which holds P's length alone, is
    // placed whole. A save after reading both keeps nothing.
    const p = partReport("2.25.95", "090000");
    const q = buildDocuments({
      entries: [{ image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.952" })], key: false }],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [p.reference] },
      contentDate: "20261108",
      contentTime: "100000",
    }).documents[0];
    if (q === undefined) {
      throw new Error("expected Q");
    }
    // Review pass 6, N1. A second partial report of the study, which Q does
    // not name, stays recorded.
    const other = partReport("2.25.99", "091500");
    const session = new Map<string, Map<string, DocumentReference>>();
    const named = new Set<string>();
    const first = placeDocuments([p, other].map((report) => readDocument(report.name, report.bytes)), [ct], new Set(), named);
    notePartialReports(session, first, named);
    expect(asRecord(session)).toEqual({ "1.2.3.4.5": [p.reference.sopInstance, other.reference.sopInstance] });
    const second = placeDocuments([readDocument(q.name, q.bytes)], [ct], new Set(), named);
    expect(second.named).toEqual([p.reference.sopInstance]);
    notePartialReports(session, second, named);
    const save = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      partialReports: asRecord(session),
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(save.kept).toBe(1);
    expect(asRecord(session)).toEqual({ "1.2.3.4.5": [other.reference.sopInstance] });
  });

  /** P, a partial report, and Q, which holds P's length alone and names P. */
  function replacedPair() {
    const p = partReport("2.25.95", "090000");
    const q = buildDocuments({
      entries: [{ image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.952" })], key: false }],
      studies: { "1.2.3.4.5": ct },
      predecessors: { "1.2.3.4.5": [p.reference] },
      contentDate: "20261108",
      contentTime: "100000",
    }).documents[0];
    if (q === undefined) {
      throw new Error("expected Q");
    }
    return { p, q };
  }

  it("does not count a partial report read after the document that replaces it", () => {
    // Review pass 6, D1. Q, which names P, is read first, then P in a later
    // read. Q replaces P whatever order the page read them in, PS3.3 C.17.2,
    // so P is not placed and not kept.
    const { p, q } = replacedPair();
    const session = new Map<string, Map<string, DocumentReference>>();
    const named = new Set<string>();
    notePartialReports(session, placeDocuments([readDocument(q.name, q.bytes)], [ct], new Set(), named), named);
    const second = placeDocuments([readDocument(p.name, p.bytes)], [ct], new Set(), named);
    expect(second.replaced).toEqual([p.name]);
    notePartialReports(session, second, named);
    const save = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      partialReports: asRecord(session),
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(save.kept).toBe(0);
    expect(asRecord(session)).toEqual({});
  });

  it("does not place a report read after the document that replaces it, even whole", () => {
    // Review pass 6, D1. With every image of P in the folder, P read after Q
    // would place P's rectangle again. Q replaces P, so it does not.
    const { p, q } = replacedPair();
    const named = new Set<string>();
    const session = new Map<string, Map<string, DocumentReference>>();
    const first = placeDocuments([readDocument(q.name, q.bytes)], [ct, elsewhere], new Set(), named);
    notePartialReports(session, first, named);
    const shown = new Set(first.marks.map((one) => one.mark.tracking));
    const second = placeDocuments([readDocument(p.name, p.bytes)], [ct, elsewhere], shown, named);
    expect(second.marks).toEqual([]);
    expect(second.replaced).toEqual([p.name]);
  });

  it("counts each partial report of a study", () => {
    // N1. Two reports placed in part are two reports kept.
    const one = partReport("2.25.97", "090000");
    const two = partReport("2.25.98", "091000");
    const session = new Map<string, Map<string, DocumentReference>>();
    notePartialReports(session, placeDocuments([one, two].map((report) => readDocument(report.name, report.bytes)), [ct], new Set()));
    const save = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      partialReports: asRecord(session),
      contentDate: "20261108",
      contentTime: "110000",
    });
    expect(save.kept).toBe(2);
  });
});

describe("review pass 7", () => {
  const elsewhere = image("1.2.3.4.30", { key: "elsewhere" });

  /** P, a rectangle off `ct` and a length on it. Q, P's length alone, naming P. R, Q's length, naming Q. */
  function chain() {
    const p = buildDocuments({
      entries: [
        { image: elsewhere, marks: [mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "2.25.971" })], key: false },
        { image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.972" })], key: false },
      ],
      studies: { "1.2.3.4.5": ct },
      predecessors: {},
      contentDate: "20261108",
      contentTime: "090000",
    }).documents[0];
    if (p === undefined) {
      throw new Error("expected P");
    }
    const naming = (previous: typeof p, time: string) => {
      const next = buildDocuments({
        entries: [{ image: ct, marks: [mark("length", [{ x: 0, y: 0 }, { x: 9, y: 9 }], { tracking: "2.25.972" })], key: false }],
        studies: { "1.2.3.4.5": ct },
        predecessors: { "1.2.3.4.5": [previous.reference] },
        contentDate: "20261108",
        contentTime: time,
      }).documents[0];
      if (next === undefined) {
        throw new Error("expected a report");
      }
      return next;
    };
    const q = naming(p, "100000");
    return { p, q, r: naming(q, "110000") };
  }

  function read(document: { name: string; bytes: Uint8Array }) {
    return readDocument(document.name, document.bytes);
  }

  /** The page's record of the documents to name in the next save, by study, as `rememberDocument` keeps it. */
  function remember(record: Map<string, DocumentReference[]>, study: string, reference: DocumentReference) {
    const list = record.get(study) || [];
    if (!list.some((one) => one.sopInstance === reference.sopInstance)) list.push(reference);
    record.set(study, list);
  }

  it("names a report it skipped as replaced in the next save", () => {
    // D1. Q is read, then P with every image present. P is not placed,
    // because Q replaces it, PS3.3 C.17.2, so the next save names P as well
    // as Q, which Table C.17-2 allows for content included through Q. After
    // every mark is deleted and saved, P with the saved file alone shows
    // nothing, as the page showed.
    const { p, q } = chain();
    const named = new Set<string>();
    const record = new Map<string, DocumentReference[]>();
    const images = [ct, elsewhere];
    for (const document of [q, p]) {
      const placed = placeDocuments([read(document)], images, new Set(), named);
      notePartialReports(new Map(), placed, named);
      for (const one of placed.predecessors) remember(record, one.study, one.reference);
    }
    // Every mark shown was deleted by the user, which is Q's length.
    const save = buildDocuments({
      entries: [],
      studies: { "1.2.3.4.5": ct },
      predecessors: Object.fromEntries(record),
      contentDate: "20261108",
      contentTime: "120000",
    }).documents[0];
    if (save === undefined) {
      throw new Error("expected the save");
    }
    const saved = read(save);
    expect(saved?.kind === "sr" ? [...saved.predecessors].sort() : null).toEqual([p.reference.sopInstance, q.reference.sopInstance].sort());
    expect(placeDocuments([read(p), saved], images, new Set()).marks).toEqual([]);
  });

  it("names a partial report a later read replaced in the next save", () => {
    // D1, the other order. P is read in part, then Q, which names P. The
    // page drops P from its partial record and still names it next time.
    const { p, q } = chain();
    const named = new Set<string>();
    const partial = new Map<string, Map<string, DocumentReference>>();
    const record = new Map<string, DocumentReference[]>();
    for (const document of [p, q]) {
      const placed = placeDocuments([read(document)], [ct], new Set(), named);
      const removed = notePartialReports(partial, placed, named);
      for (const one of placed.predecessors) remember(record, one.study, one.reference);
      for (const one of removed) remember(record, one.study, one.reference);
    }
    expect(record.get("1.2.3.4.5")?.map((one) => one.sopInstance).sort()).toEqual([p.reference.sopInstance, q.reference.sopInstance].sort());
    expect(partial.size).toBe(0);
  });

  it("skips every report of a chain whatever order it is read in", () => {
    // N1. R names Q and Q names P. Read R, then Q, then P: Q is replaced by
    // R, and P by Q, whose names join the set although Q itself is skipped.
    const { p, q, r } = chain();
    const named = new Set<string>();
    const shown = new Set<string>();
    const results = [r, q, p].map((document) => {
      const placed = placeDocuments([read(document)], [ct, elsewhere], shown, named);
      notePartialReports(new Map(), placed, named);
      for (const one of placed.marks) shown.add(one.mark.tracking);
      return placed;
    });
    expect(results[0]?.marks.length).toBe(1);
    expect(results[1]?.replaced).toEqual([q.name]);
    expect(results[1]?.marks).toEqual([]);
    expect(results[2]?.replaced).toEqual([p.name]);
    expect(results[2]?.marks).toEqual([]);
    // The same chain read in one call: only R is placed, and Q's own name
    // for P counts although R replaces Q.
    const together = placeDocuments([p, q, r].map(read), [ct, elsewhere], new Set());
    expect(together.marks.map((one) => one.mark.tracking)).toEqual(["2.25.972"]);
    expect(together.replaced.sort()).toEqual([p.name, q.name].sort());
  });
});

describe("per-folder document state", () => {
  const study = "1.2.3.4.5";
  const reference = { series: "2.25.71", sopClass: "1.2.840.10008.5.1.4.1.1.88.34", sopInstance: "2.25.72" };

  it("writes nothing twice for an unchanged study", () => {
    // The operator's pass 10 decision retains the within-folder no-change rule.
    for (const time of ["110000", "120000"]) {
      const save = buildDocuments({
        entries: [], studies: { [study]: ct },
        predecessors: { [study]: [{ ...reference, empty: false }] },
        changed: [], contentDate: "20261108", contentTime: time,
      });
      expect(save.documents).toEqual([]);
      expect(save.unchanged).toBe(1);
    }
  });

  it("saves deletion of the last mark against the open folder's report", () => {
    // PS3.16 TID 1500 row 6 permits no measurement group. PS3.3 C.17.2
    // names the report replaced by that empty result.
    const save = buildDocuments({
      entries: [], studies: { [study]: ct },
      predecessors: { [study]: [{ ...reference, empty: false }] },
      changed: [study], contentDate: "20261108", contentTime: "110000",
    });
    expect(save.cleared).toBe(1);
    const document = save.documents[0];
    if (!document) throw new Error("expected an empty report");
    const read = readDocument(document.name, document.bytes);
    expect(read?.kind === "sr" ? read.groups : null).toEqual([]);
    expect(read?.kind === "sr" ? read.predecessors : null).toEqual([reference.sopInstance]);
  });

  it("keeps one reference when a report is read twice", () => {
    const record = new Map<string, (DocumentReference & { empty?: boolean })[]>();
    rememberReport(record, study, { ...reference, empty: true });
    rememberReport(record, study, { ...reference, empty: false });
    expect(record.get(study)).toEqual([{ ...reference, empty: false }]);
  });

  it("counts a report hidden by an earlier read in this folder as a change", () => {
    const original = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }])]).documents[0];
    if (!original) throw new Error("expected a report");
    const empty = buildDocuments({
      entries: [], studies: { [study]: ct }, predecessors: { [study]: [original.reference] },
      contentDate: "20261108", contentTime: "110000",
    }).documents[0];
    if (!empty) throw new Error("expected an empty report");
    const named = new Set<string>();
    const first = placeDocuments([readDocument(empty.name, empty.bytes)], [ct], new Set(), named);
    notePartialReports(new Map(), first, named);
    const later = placeDocuments([readDocument(original.name, original.bytes)], [ct], new Set(), named);
    expect(later.hidden).toEqual([study]);
    expect(later.marks).toEqual([]);
    const together = placeDocuments([readDocument(original.name, original.bytes), readDocument(empty.name, empty.bytes)], [ct], new Set());
    expect(together.hidden).toEqual([]);
  });

  it("keeps untracked group identities within a folder and clears them for the next", () => {
    // PS3.16 TID 1410 row 3 is optional. The operator scopes our assigned
    // identifier to a folder, keyed by report UID and group index.
    const document = report([mark("rect", [{ x: 1, y: 1 }, { x: 4, y: 4 }], { tracking: "" })]).documents[0];
    if (!document) throw new Error("expected a report");
    const stable = new Map<string, string>();
    let next = 800;
    const fresh = () => "2.25." + String(++next);
    const read = () => {
      const placed = placeDocuments([readDocument(document.name, document.bytes)], [ct], new Set());
      stableTracking(stable, placed, fresh);
      return placed.marks[0]?.mark.tracking;
    };
    expect(read()).toBe("2.25.801");
    expect(read()).toBe("2.25.801");
    stable.clear();
    expect(read()).toBe("2.25.802");
  });
});
