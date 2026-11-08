// The folder viewer's marks as DICOM documents, F-215.
//
// One TID 1410 Measurement Group per mark, PS3.16 TID 1500 row 7, and one
// Key Object Selection per study for the key images, PS3.16 TID 2010. The
// page owns the mark to tree mapping and `@ocelli/core` owns the bytes.
// Every function here is pure and takes plain objects, so the tests run it
// without a page.
//
// Coordinates. PS3.3 Table C.18.6-1 puts Graphic Data in (column,row) pairs
// whose origin is the top left hand corner of the top left pixel, 0\0,
// through Columns\Rows. The page's image coordinates already have that
// origin, `x` across columns and `y` down rows, so a point is written as it
// is, `x` first, with no half pixel added or taken away. Rotation and flip
// are display state and are not written. Graphic Data is FL, PS3.6, so each
// value is its `Math.fround`.
//
// The local rules are in `.claude/plans/F-215-design.md`.

import { readKeyObjects, readTid1500Groups, writeKeyObjects, writeTid1500Groups } from "@ocelli/core";

/** The Tracking Identifier text of every group this page writes, then the kind. */
export const TRACKING_PREFIX = "ocelli-folder-viewer:";

/** The mark kinds a save writes. The others are counted and named as not saved. */
export const SAVED_KINDS = Object.freeze(["contour", "rect", "ellipse", "length", "text"]);

/** The SOP classes the page reads as documents rather than as images. */
export const DOCUMENT_CLASSES = Object.freeze([
  "1.2.840.10008.5.1.4.1.1.88.22",
  "1.2.840.10008.5.1.4.1.1.88.33",
  "1.2.840.10008.5.1.4.1.1.88.34",
  "1.2.840.10008.5.1.4.1.1.88.59",
]);

const GROUP = { scheme: "DCM", value: "125007", meaning: "Measurement Group" };
const TRACKING = { scheme: "DCM", value: "112039", meaning: "Tracking Identifier" };
const TRACKING_UID = { scheme: "DCM", value: "112040", meaning: "Tracking Unique Identifier" };
const IMAGE_REGION = { scheme: "DCM", value: "111030", meaning: "Image Region" };
const SOURCE = { scheme: "DCM", value: "121112", meaning: "Source of Measurement" };
const LENGTH = { scheme: "SCT", value: "410668003", meaning: "Length" };
const COMMENT = { scheme: "DCM", value: "121106", meaning: "Comment" };

/**
 * The calibration sources of the local process's `/measure` reply whose
 * length is a patient length in mm. `detector` is a length at the detector,
 * not corrected for magnification, and `uncalibrated` is pixels.
 */
const PATIENT_MM = new Set(["pixel-spacing", "imager-spacing"]);

function fround(value) {
  return Math.fround(value);
}

/** The mark with every point at its FL value, which a second save writes again unchanged. */
export function froundPoints(mark) {
  return { ...mark, points: mark.points.map((point) => ({ x: fround(point.x), y: fround(point.y) })) };
}

function pair(point) {
  return [fround(point.x), fround(point.y)];
}

/** The SCOORD of one mark, as the Graphic Type and its points. */
function graphicOf(mark) {
  const points = mark.points;
  if (mark.type === "contour" && points.length >= 3) {
    const first = points[0];
    return { graphic: "POLYLINE", points: [...points.map(pair), pair(first)] };
  }
  if ((mark.type === "rect" || mark.type === "ellipse") && points.length === 2) {
    const x0 = Math.min(points[0].x, points[1].x);
    const x1 = Math.max(points[0].x, points[1].x);
    const y0 = Math.min(points[0].y, points[1].y);
    const y1 = Math.max(points[0].y, points[1].y);
    if (mark.type === "rect") {
      const corners = [{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }, { x: x0, y: y0 }];
      return { graphic: "POLYLINE", points: corners.map(pair) };
    }
    // PS3.3 C.18.6.1.2: the major axis endpoints first. A box at least as
    // wide as it is high has its major axis across the columns.
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    const axes = x1 - x0 >= y1 - y0
      ? [{ x: x0, y: cy }, { x: x1, y: cy }, { x: cx, y: y0 }, { x: cx, y: y1 }]
      : [{ x: cx, y: y0 }, { x: cx, y: y1 }, { x: x0, y: cy }, { x: x1, y: cy }];
    return { graphic: "ELLIPSE", points: axes.map(pair) };
  }
  if (mark.type === "length" && points.length === 2) {
    return { graphic: "POLYLINE", points: points.map(pair) };
  }
  if (mark.type === "text" && points.length === 1) {
    return { graphic: "POINT", points: [pair(points[0])] };
  }
  return null;
}

/** The IMAGE a mark was drawn on. A multi-frame object names its frame, counted from 1, PS3.3 C.18.4. */
function imageItem(image, relationship) {
  const item = {
    relationship,
    valueType: "IMAGE",
    concept: SOURCE,
    sopClass: image.sopClass,
    sopInstance: image.sopInstance,
    children: [],
  };
  return image.frames > 1 ? { ...item, frame: image.frame + 1 } : item;
}

/** The mark's length NUM from its own `/measure` reply, or null when the reply is not a patient length. */
function lengthNumber(mark) {
  const reply = mark.result;
  if (mark.type !== "length" || !reply || !reply.calibration || !reply.length) return null;
  if (!PATIENT_MM.has(reply.calibration.source) || reply.length.unit !== "mm") return null;
  const value = Number(reply.length.value);
  if (!Number.isFinite(value)) return null;
  // DS holds at most 16 characters, PS3.5 6.2. Twelve significant digits fit.
  return Number(value.toPrecision(12));
}

/** One TID 1410 Measurement Group for one mark, or null for a kind this page does not save. */
export function markToGroup(mark, image) {
  if (!SAVED_KINDS.includes(mark.type)) return null;
  const region = graphicOf(mark);
  if (region === null) return null;
  const children = [
    { relationship: "HAS_OBS_CONTEXT", valueType: "TEXT", concept: TRACKING, text: TRACKING_PREFIX + mark.type, children: [] },
  ];
  if (mark.tracking) {
    children.push({ relationship: "HAS_OBS_CONTEXT", valueType: "UIDREF", concept: TRACKING_UID, uid: mark.tracking, children: [] });
  }
  children.push({
    relationship: "CONTAINS",
    valueType: "SCOORD",
    concept: IMAGE_REGION,
    graphic: region.graphic,
    points: region.points,
    children: [imageItem(image, "SELECTED_FROM")],
  });
  const length = lengthNumber(mark);
  if (length !== null) {
    // CID 7460 gives the meaning of (mm, UCUM).
    children.push({ relationship: "CONTAINS", valueType: "NUM", concept: LENGTH, number: length, unit: "mm", unitMeaning: "millimeter", children: [] });
  }
  if (mark.type === "text") {
    children.push({ relationship: "CONTAINS", valueType: "TEXT", concept: COMMENT, text: String(mark.text || ""), children: [] });
  }
  return { relationship: null, valueType: "CONTAINER", concept: GROUP, children };
}

function same(a, b) {
  return a[0] === b[0] && a[1] === b[1];
}

/**
 * Whether `value` is within a relative 1e-3 of `want`, at least 1e-3 pixel,
 * for an ellipse centre another writer may have rounded differently. This is
 * much wider than one float32 step.
 */
function near(value, want) {
  return Math.abs(value - want) <= 1e-3 * Math.max(1, Math.abs(want));
}

/** The page mark for one SCOORD, or a reason it is not one. */
function markOf(graphic, points, kind) {
  const xy = (point) => ({ x: point[0], y: point[1] });
  if (graphic === "POINT" && points.length === 1) return { type: "text", points: [xy(points[0])] };
  if (graphic === "POLYLINE" && points.length === 2 && !same(points[0], points[1])) {
    return { type: "length", points: points.map(xy) };
  }
  if (graphic === "POLYLINE" && points.length >= 4 && same(points[0], points[points.length - 1])) {
    const open = points.slice(0, -1);
    const [a, b, c, d] = open;
    const square = open.length === 4 && a[1] === b[1] && b[0] === c[0] && c[1] === d[1] && d[0] === a[0];
    if (kind === "rect" && square) {
      return { type: "rect", points: [{ x: Math.min(a[0], c[0]), y: Math.min(a[1], c[1]) }, { x: Math.max(a[0], c[0]), y: Math.max(a[1], c[1]) }] };
    }
    return { type: "contour", points: open.map(xy) };
  }
  if (graphic === "ELLIPSE" && points.length === 4) {
    const [p0, p1, p2, p3] = points;
    const across = p0[1] === p1[1] && p2[0] === p3[0];
    const down = p0[0] === p1[0] && p2[1] === p3[1];
    if (across || down) {
      const xs = across ? [p0[0], p1[0]] : [p2[0], p3[0]];
      const ys = across ? [p2[1], p3[1]] : [p0[1], p1[1]];
      const cx = across ? p2[0] : p0[0];
      const cy = across ? p0[1] : p2[1];
      if (near(cx, (xs[0] + xs[1]) / 2) && near(cy, (ys[0] + ys[1]) / 2)) {
        return { type: "ellipse", points: [{ x: Math.min(...xs), y: Math.min(...ys) }, { x: Math.max(...xs), y: Math.max(...ys) }] };
      }
    }
  }
  return null;
}

/**
 * One group as a page mark and the image it names, or `{ reason }`.
 * `geometry` is a group with no image region, such as the NUM-only F-097
 * shape, and `graphic` is a region this page does not draw.
 */
export function groupToMark(group) {
  if (!group || group.valueType !== "CONTAINER") return { reason: "geometry" };
  let kind = "";
  let tracking = "";
  let text = "";
  let region = null;
  for (const item of group.children) {
    if (item.valueType === "TEXT" && item.concept.value === TRACKING.value && item.text.startsWith(TRACKING_PREFIX)) {
      kind = item.text.slice(TRACKING_PREFIX.length);
    } else if (item.valueType === "UIDREF" && item.concept.value === TRACKING_UID.value) {
      tracking = item.uid;
    } else if (item.valueType === "TEXT" && item.concept.value === COMMENT.value) {
      text = item.text;
    } else if (item.valueType === "SCOORD" && region === null) {
      region = item;
    }
  }
  const image = region ? region.children.find((item) => item.valueType === "IMAGE" && item.relationship === "SELECTED_FROM") : undefined;
  if (!region || !image) return { reason: "geometry" };
  const mark = markOf(region.graphic, region.points, kind);
  if (!mark || (mark.type === "text" && text === "")) return { reason: "graphic" };
  if (mark.type === "text") mark.text = text;
  mark.tracking = tracking;
  const reference = image.frame === undefined ? { sopInstance: image.sopInstance } : { sopInstance: image.sopInstance, frame: image.frame };
  return { mark, reference };
}

/**
 * The stored Study Time copied verbatim, PS3.3 C.7.2.1, when it is a TM of
 * PS3.5 6.2. Anything else is written empty, which Type 2 allows.
 */
function studyTime(stored) {
  const text = String(stored || "");
  return /^[0-9]{2}(?:[0-9]{2}(?:[0-9]{2}(?:\.[0-9]{1,6})?)?)?$/u.test(text) ? text : "";
}

function studyDate(stored) {
  const text = String(stored || "");
  return /^[0-9]{8}$/u.test(text) ? text : "";
}

function referenceOf(image) {
  return { series: image.series, sopClass: image.sopClass, sopInstance: image.sopInstance };
}

function sourceOf(first, evidence, predecessors, state) {
  return {
    study: first.study,
    patientName: first.patientName || "",
    patientId: first.patientId || "",
    studyDate: studyDate(first.studyDate),
    studyTime: studyTime(first.studyTime),
    accession: first.accession || "",
    contentDate: state.contentDate,
    contentTime: state.contentTime,
    evidence,
    predecessors,
  };
}

function count(table, key) {
  table[key] = (table[key] || 0) + 1;
}

/**
 * Every document one save writes, built before any download starts.
 *
 * `state.entries` is one `{ image, marks, key }` per image with a mark or a
 * key flag. `state.predecessors` maps a study to the documents this page
 * loaded or saved for it earlier, lead amendment 3. One TID 1500 document
 * is written for each study with a saved mark and one Key Object Selection
 * for each study with a key image. A writer that returns nothing fails the
 * whole save, so `documents` is then empty and `failed` names the kind.
 */
export function buildDocuments(state) {
  const studies = new Map();
  const unsaved = {};
  let noUid = 0;
  let withoutNumber = 0;
  const saved = { marks: 0, keyImages: 0 };
  for (const entry of state.entries) {
    const image = entry.image;
    const savable = entry.marks.filter((mark) => SAVED_KINDS.includes(mark.type));
    for (const mark of entry.marks) {
      if (!SAVED_KINDS.includes(mark.type)) count(unsaved, mark.type);
    }
    if (!image.sopInstance || !image.study || !image.series || !image.sopClass) {
      noUid += savable.length + (entry.key ? 1 : 0);
      continue;
    }
    let study = studies.get(image.study);
    if (!study) {
      study = { first: image, groups: [], groupEvidence: [], keys: [], keyEvidence: [] };
      studies.set(image.study, study);
    }
    for (const mark of savable) {
      const group = markToGroup(mark, image);
      if (group === null) continue;
      if (mark.type === "length" && lengthNumber(mark) === null) withoutNumber += 1;
      study.groups.push(group);
      study.groupEvidence.push(referenceOf(image));
      saved.marks += 1;
    }
    if (entry.key) {
      study.keys.push(image.frames > 1
        ? { sopClass: image.sopClass, sopInstance: image.sopInstance, frame: image.frame + 1 }
        : { sopClass: image.sopClass, sopInstance: image.sopInstance });
      study.keyEvidence.push(referenceOf(image));
      saved.keyImages += 1;
    }
  }
  // Review pass 1, D2. A study this page saved or read a report for earlier
  // and whose marks are now all gone still gets a report, of no groups, so
  // the earlier one is replaced. `state.studies` gives an image of each study
  // in the folder to copy the study attributes from.
  // The operator's decision after review pass 10: the page's state lasts
  // for one open folder, so every report it remembers was read into this
  // folder or saved from it, and each of its marks is either shown, and so
  // in this save, or was deleted here. Every one of them is named.
  const earlier = state.predecessors || {};
  // Review pass 8, N3. With `state.changed`, a study no mark of which was
  // added, moved, deleted or edited, and no key image changed, since the
  // folder was opened or the study last saved gets no document. Without it
  // every study counts as changed.
  const changed = state.changed ? new Set(state.changed) : null;
  const unchangedStudy = (uid) => changed !== null && !changed.has(uid);
  // A study with a remembered report is considered, so an unchanged one is
  // counted and the status says why.
  for (const [uid, list] of Object.entries(earlier)) {
    const first = state.studies && state.studies[uid];
    if (!list.length || studies.has(uid) || !first) continue;
    studies.set(uid, { first, groups: [], groupEvidence: [], keys: [], keyEvidence: [] });
  }
  // A Key Object Selection needs at least one reference, PS3.16 TID 2010
  // rows 8 to 10, so a study whose last key image was unflagged is counted
  // and nothing is written for it.
  // Review pass 4, D3. Reports of a study in this folder that were read but
  // not placed whole, and have not been placed whole since. No save replaces
  // them, so their marks come back, and the status line says so.
  let kept = 0;
  for (const [uid, reports] of Object.entries(state.partialReports || {})) {
    if (!state.studies || !state.studies[uid] || unchangedStudy(uid)) continue;
    const named = new Set((earlier[uid] || []).map((one) => one.sopInstance));
    kept += new Set(reports.filter((sopInstance) => !named.has(sopInstance))).size;
  }
  let unflagged = 0;
  for (const uid of state.keyStudies || []) {
    // Review pass 2, D4. Only a study in this folder.
    if (!state.studies || !state.studies[uid] || unchangedStudy(uid)) continue;
    const study = studies.get(uid);
    if (!study || !study.keys.length) unflagged += 1;
  }
  const documents = [];
  const stamp = state.contentDate + "-" + String(state.contentTime).replace(/[^0-9]/gu, "");
  let index = 0;
  let cleared = 0;
  let unchanged = 0;
  const failed = (kind) => ({ documents: [], saved, unsaved, noUid, withoutNumber, cleared: 0, unflagged, kept, unchanged, failed: kind });
  for (const [uid, study] of studies) {
    if (unchangedStudy(uid)) {
      unchanged += 1;
      continue;
    }
    const predecessors = earlier[uid] || [];
    // No second empty report when the only earlier reports were empty. A
    // report this page could not place whole is never a predecessor, so an
    // empty report never replaces it, review pass 3, D3.
    const replaceable = !predecessors.every((one) => one.empty);
    if (study.groups.length || (predecessors.length && replaceable)) {
      if (!study.groups.length) cleared += 1;
      const bytes = writeTid1500Groups(study.groups, sourceOf(study.first, study.groupEvidence, predecessors, state));
      const read = bytes === null ? null : readTid1500Groups(bytes);
      if (read === null) return failed("sr");
      index += 1;
      documents.push({
        kind: "sr",
        empty: !study.groups.length,
        study: uid,
        bytes,
        name: "ocelli-marks-" + stamp + "-" + index + ".dcm",
        reference: { series: read.series, sopClass: read.sopClass, sopInstance: read.sopInstance },
      });
    }
    if (study.keys.length) {
      const bytes = writeKeyObjects(study.keys, sourceOf(study.first, study.keyEvidence, [], state));
      const read = bytes === null ? null : readKeyObjects(bytes);
      if (read === null) return failed("ko");
      index += 1;
      documents.push({
        kind: "ko",
        study: uid,
        bytes,
        name: "ocelli-key-images-" + stamp + "-" + index + ".dcm",
        reference: { series: read.series, sopClass: DOCUMENT_CLASSES[3], sopInstance: read.sopInstance },
      });
    }
  }
  return { documents, saved, unsaved, noUid, withoutNumber, cleared, unflagged, kept, unchanged, failed: "" };
}

/** One file's bytes as a TID 1500 or Key Object document, or null. */
export function readDocument(name, bytes) {
  const report = readTid1500Groups(bytes);
  if (report !== null) return { name, kind: "sr", ...report };
  const keys = readKeyObjects(bytes);
  if (keys !== null) return { name, kind: "ko", ...keys };
  return null;
}

function stampOf(document) {
  return [document.contentDate || "", Number(document.contentTime) || 0, document.instanceNumber || 0];
}

function newer(a, b) {
  const left = stampOf(a);
  const right = stampOf(b);
  if (left[0] !== right[0]) return left[0] > right[0];
  if (left[1] !== right[1]) return left[1] > right[1];
  return left[2] > right[2];
}

/**
 * Place loaded documents on the folder's images by SOP Instance UID and frame.
 *
 * `images` is one `{ key, sopInstance, frame, frames, columns, rows }` per
 * page image, `frame` counted from 0. `shown` is the Tracking Unique
 * Identifiers of marks already on the page. A TID 1500 document another
 * loaded document names as a predecessor is skipped, and so is one in
 * `replacedBefore`, the SOP Instance UIDs earlier reads into the open folder
 * named as replaced, the folder itself or Load marks, so the order of reads
 * does not matter, PS3.3 C.17.2 and review pass 6, D1. Among the Key Object
 * Selections this package wrote for a study only the newest by Content Date,
 * Content Time and Instance Number is shown, lead amendment 4. Each item not
 * placed is counted by reason in `unplaced`.
 */
export function placeDocuments(documents, images, shown, replacedBefore = new Set()) {
  const byInstance = new Map();
  for (const image of images) {
    if (!image.sopInstance) continue;
    const list = byInstance.get(image.sopInstance) || [];
    list.push(image);
    byInstance.set(image.sopInstance, list);
  }
  const unplaced = {};
  /** The page images one reference names, or a reason. */
  function target(reference, whole) {
    const list = byInstance.get(reference.sopInstance) || [];
    if (!list.length) return { reason: "missing" };
    const frames = new Set(list.map((image) => image.frame));
    if (frames.size !== list.length) return { reason: "duplicate" };
    const multi = list.some((image) => image.frames > 1);
    if (reference.frame === undefined) {
      if (!multi) return { images: list.filter((image) => image.frame === 0) };
      return whole ? { images: list } : { reason: "frameless" };
    }
    const found = list.filter((image) => image.frame === reference.frame - 1);
    return found.length ? { images: found } : { reason: "missing" };
  }
  const reports = [];
  const seenDocuments = new Set();
  for (const document of documents) {
    if (!document || seenDocuments.has(document.sopInstance)) continue;
    seenDocuments.add(document.sopInstance);
    reports.push(document);
  }
  const named = new Set(replacedBefore);
  for (const document of reports) {
    if (document.kind === "sr") for (const uid of document.predecessors) named.add(uid);
  }
  const replaced = [];
  const newest = new Map();
  for (const document of reports) {
    if (document.kind !== "ko" || document.manufacturer !== "Ocelli") continue;
    const best = newest.get(document.study);
    if (!best || newer(document, best)) newest.set(document.study, document);
  }
  const marks = [];
  const keys = [];
  const keyObjects = [];
  const tracked = new Set(shown);
  let repeated = 0;
  const wholly = [];
  const skipped = [];
  const hidden = new Set();
  // Review pass 5, D1. Every report a document of this read names as
  // replaced, whether or not that report was among these documents.
  const namedHere = new Set();
  for (const document of reports) {
    if (document.kind === "sr") for (const uid of document.predecessors) namedHere.add(uid);
  }
  // Review pass 4, D3. The reports not placed whole, which a later save
  // keeps, so the page can say their marks come back.
  const partial = [];
  for (const document of reports) {
    if (document.kind === "sr") {
      if (named.has(document.sopInstance)) {
        replaced.push(document.name);
        skipped.push(document);
        if (!namedHere.has(document.sopInstance)) hidden.add(document.study);
        continue;
      }
      if (document.unread) unplaced.unread = (unplaced.unread || 0) + document.unread;
      // Review pass 2, D5. Whether every group of this report is on the page
      // now, placed or already shown, so a later save includes it.
      let whole = !document.unread;
      for (const [group, item] of document.groups.entries()) {
        const found = groupToMark(item);
        if (found.reason) {
          count(unplaced, found.reason);
          whole = false;
          continue;
        }
        const placed = target(found.reference, false);
        if (placed.reason) {
          count(unplaced, placed.reason);
          whole = false;
          continue;
        }
        const image = placed.images[0];
        const inside = found.mark.points.every((point) => point.x >= 0 && point.y >= 0 && point.x <= image.columns && point.y <= image.rows);
        if (!inside) {
          count(unplaced, "outside");
          whole = false;
          continue;
        }
        if (found.mark.tracking && tracked.has(found.mark.tracking)) {
          repeated += 1;
          continue;
        }
        if (found.mark.tracking) tracked.add(found.mark.tracking);
        marks.push({ key: image.key, mark: found.mark, report: document.sopInstance, group });
      }
      if (whole) wholly.push(document);
      else partial.push({ study: document.study, sopInstance: document.sopInstance, reference: referenceOfDocument(document) });
    } else {
      if (document.manufacturer === "Ocelli" && newest.get(document.study) !== document) {
        replaced.push(document.name);
        continue;
      }
      keyObjects.push(document.name);
      for (const reference of document.references) {
        const placed = target(reference, true);
        if (placed.reason) {
          count(unplaced, placed.reason);
          continue;
        }
        for (const image of placed.images) if (!keys.includes(image.key)) keys.push(image.key);
      }
    }
  }
  // Review pass 1, N6, and review pass 2, D5. A later save includes a
  // report's content, and so names it as a predecessor, PS3.3 C.17.2, only
  // when every group of it is on the page. A report with a group unread or
  // not placed is not named, so no later report, empty or not, replaces it.
  // `empty` marks a report of no groups.
  const predecessors = wholly.map((document) => ({
    study: document.study,
    reference: referenceOfDocument(document),
    empty: document.groups.length === 0,
  }));
  // Review pass 7, D1. A report skipped as replaced is named too, so the next
  // save replaces what the page hid. Its content is included through the
  // document that names it, which PS3.3 Table C.17-2 allows.
  for (const document of skipped) {
    predecessors.push({ study: document.study, reference: referenceOfDocument(document), empty: false });
  }
  return {
    marks,
    keys,
    keyObjects,
    replaced,
    unplaced,
    repeated,
    predecessors,
    partial,
    named: Array.from(namedHere),
    hidden: Array.from(hidden),
  };
}

/**
 * Record one report the next save names, in the page's `record`, a Map of
 * study to entries, for the open folder. A report recorded twice keeps one
 * entry, which is empty only when both say so.
 */
export function rememberReport(record, study, entry) {
  const list = record.get(study) || [];
  const known = list.find((one) => one.sopInstance === entry.sopInstance);
  if (!known) list.push({ ...entry });
  else known.empty = Boolean(known.empty && entry.empty);
  record.set(study, list);
  return record;
}

/** Give an untracked group the same identifier on each read into this folder. */
export function stableTracking(stable, placed, fresh) {
  for (const one of placed.marks) {
    if (one.mark.tracking) continue;
    const key = one.report + "\0" + one.group;
    const tracking = stable.get(key) || fresh();
    stable.set(key, tracking);
    one.mark.tracking = tracking;
  }
}

function referenceOfDocument(document) {
  return { series: document.series, sopClass: document.sopClass, sopInstance: document.sopInstance };
}

/**
 * The page's record of reports placed only in part, by study and then by
 * SOP Instance UID, after one read. `replaced` is the page's one set of every
 * SOP Instance UID any read into the open folder has named as replaced,
 * PS3.3 C.17.2, and this
 * read's names join it. This read's partial reports are added, and every
 * report earlier reads kept that is now in the set is removed, because its
 * marks no longer come back once the document naming it is in the folder,
 * review pass 6, D1. A report this read placed in part is never in the set,
 * because `placeDocuments` skipped every report in it. The removed reports
 * are returned with their references, so the page still names them in the
 * next save, review pass 7, D1.
 */
export function notePartialReports(partialReports, placed, replaced = new Set()) {
  for (const uid of placed.named) replaced.add(uid);
  for (const { study, sopInstance, reference } of placed.partial) {
    const reports = partialReports.get(study) || new Map();
    reports.set(sopInstance, reference);
    partialReports.set(study, reports);
  }
  const removed = [];
  for (const [study, reports] of partialReports) {
    for (const [sopInstance, reference] of reports) {
      if (!replaced.has(sopInstance)) continue;
      reports.delete(sopInstance);
      removed.push({ study, reference });
    }
    if (!reports.size) partialReports.delete(study);
  }
  return removed;
}
