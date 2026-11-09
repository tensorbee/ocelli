/**
 * Typed reader for the F-225 conformance report.
 *
 * The rule identifiers, their sections and the scope sentence come from the
 * generated catalogue. This module does not check DICOM values. It does not
 * view wasm memory. A section carried on the wire is ignored.
 */

import {
  CONFORMANCE_RULES,
  CONFORMANCE_SCOPE,
  SOP_CLASS_NOTES,
} from "./conformance.generated.js";

export { CONFORMANCE_RULES, CONFORMANCE_SCOPE, SOP_CLASS_NOTES };

/** One `sopClassNote` string. Null is the other member of the closed set. */
export type ConformanceSopClassNote = (typeof SOP_CLASS_NOTES)[number];

/**
 * PS3.6. The DICOM root is the components 1, 2, 840, 10008.
 * The trailing dot is the component boundary, not another component.
 */
const DICOM_UID_ROOT = "1.2.840.10008.";

const RULE_SECTION = new Map<string, string>(
  CONFORMANCE_RULES.map((rule) => [rule.id, rule.section]),
);

/** A length, a count, or one character position. Never a value. */
export type ConformanceMeasure =
  | { readonly unit: "bytes"; readonly measured: number; readonly limit: number }
  | { readonly unit: "characters"; readonly measured: number; readonly limit: number }
  | {
      readonly unit: "values";
      readonly measured: number;
      readonly minimum: number;
      readonly maximum: number | null;
    }
  | {
      readonly unit: "character";
      readonly index: number;
      readonly class: "control" | "backslash" | "outside-repertoire" | "non-default";
    };

/** One finding. `section` is the catalogue text, not the wire text. */
export interface ConformanceFinding {
  readonly tag: string;
  readonly keyword: string | null;
  readonly path: readonly { readonly sequence: string; readonly item: number }[];
  readonly rule: string;
  readonly section: string;
  readonly value: number | null;
  readonly measure: ConformanceMeasure | null;
}

/** Module scope. `notChecked` is present only when the IOD was evaluated. */
export type ConformanceModules =
  | {
      readonly evaluated: true;
      readonly sopClass: string;
      /** Always null. An evaluated IOD repeats a DICOM-root UID. */
      readonly sopClassNote: null;
      readonly iod: string;
      readonly table: string;
      readonly modules: readonly string[];
      readonly notChecked: readonly string[];
    }
  | {
      readonly evaluated: false;
      readonly sopClass: string | null;
      /**
       * Closed set from `SOP_CLASS_NOTES`, or null when the UID is absent
       * or repeated. PS3.5 6.2 and 9.1, and the PS3.6 DICOM root.
       */
      readonly sopClassNote: ConformanceSopClassNote | null;
    };

/** The report the reader returns. The wire `rules` array is not part of it. */
export interface ConformanceReport {
  readonly scope: string;
  readonly edition: string;
  readonly findings: readonly ConformanceFinding[];
  readonly modules: ConformanceModules;
  readonly counts: { readonly elements: number; readonly notCheckedUn: number };
}

export type ConformanceRefusal = "json" | "scope" | "rule" | "shape";

export type ReadConformanceResult =
  | { ok: true; report: ConformanceReport }
  | { ok: false; reason: ConformanceRefusal };

/** Largest integer the Rust report stores in a count. */
const U32_MAX = 4_294_967_295;

const TAG = /^\([0-9A-F]{4},[0-9A-F]{4}\)$/;

/**
 * Read one report.
 *
 * Invalid JSON is `json`. A scope string that is not the catalogue sentence
 * is `scope`, and that check runs before findings, so a wrong sentence wins
 * over an unknown rule. An unknown rule id is `rule`. Any other mismatch is
 * `shape`. `edition` must be a string and is not compared with `2026d`.
 */
export function readConformanceReport(text: string): ReadConformanceResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return { ok: false, reason: "json" };
  }
  if (!isRecord(parsed) || typeof parsed.scope !== "string") {
    return { ok: false, reason: "shape" };
  }
  if (parsed.scope !== CONFORMANCE_SCOPE) {
    return { ok: false, reason: "scope" };
  }
  if (typeof parsed.edition !== "string") {
    return { ok: false, reason: "shape" };
  }
  const modules = readModules(parsed.modules);
  if (modules === "shape") {
    return { ok: false, reason: "shape" };
  }
  const counts = readCounts(parsed.counts);
  if (counts === "shape") {
    return { ok: false, reason: "shape" };
  }
  if (!Array.isArray(parsed.findings)) {
    return { ok: false, reason: "shape" };
  }
  const findings: ConformanceFinding[] = [];
  for (const item of parsed.findings) {
    const finding = readFinding(item);
    if (finding === "shape" || finding === "rule") {
      return { ok: false, reason: finding };
    }
    findings.push(finding);
  }
  return {
    ok: true,
    report: {
      scope: parsed.scope,
      edition: parsed.edition,
      findings,
      modules,
      counts,
    },
  };
}

function readModules(value: unknown): ConformanceModules | "shape" {
  if (!isRecord(value) || typeof value.evaluated !== "boolean") {
    return "shape";
  }
  if (value.evaluated) {
    const modules = readStrings(value.modules);
    const notChecked = readStrings(value.notChecked);
    if (
      !("sopClassNote" in value) ||
      value.sopClassNote !== null ||
      typeof value.sopClass !== "string" ||
      !isDicomRootUid(value.sopClass) ||
      typeof value.iod !== "string" ||
      typeof value.table !== "string" ||
      modules === "shape" ||
      notChecked === "shape"
    ) {
      return "shape";
    }
    return {
      evaluated: true,
      sopClass: value.sopClass,
      sopClassNote: null,
      iod: value.iod,
      table: value.table,
      modules,
      notChecked,
    };
  }
  if (!("sopClassNote" in value)) {
    return "shape";
  }
  const note = value.sopClassNote;
  if (!(note === null || isSopClassNote(note))) {
    return "shape";
  }
  if (!(value.sopClass === null || typeof value.sopClass === "string")) {
    return "shape";
  }
  if (note !== null && value.sopClass !== null) {
    return "shape";
  }
  if (typeof value.sopClass === "string" && !isDicomRootUid(value.sopClass)) {
    return "shape";
  }
  return {
    evaluated: false,
    sopClass: value.sopClass,
    sopClassNote: note,
  };
}

function isSopClassNote(value: unknown): value is ConformanceSopClassNote {
  return SOP_CLASS_NOTES.some((note) => note === value);
}

/**
 * PS3.5 6.2 and 9.1, and PS3.6. True when the components begin with
 * 1, 2, 840, 10008 and at least one further component is present.
 */
function isDicomRootUid(text: string): boolean {
  return isUid(text) && componentsBeginWithDicomRoot(text);
}

/**
 * `DICOM_UID_ROOT` ends with the boundary dot. Without that dot the constant
 * is only the character prefix 1.2.840.10008, which also matches
 * 1.2.840.100081.2 and the bare root. The reader tests refuse that prefix.
 */
function componentsBeginWithDicomRoot(text: string): boolean {
  if (!DICOM_UID_ROOT.endsWith(".")) {
    return text.startsWith(DICOM_UID_ROOT);
  }
  const root = DICOM_UID_ROOT.slice(0, -1).split(".");
  if (root.some((part) => part.length === 0)) {
    return false;
  }
  const components = text.split(".");
  if (components.length <= root.length) {
    return false;
  }
  const extra = components[root.length];
  if (extra === undefined || extra.length === 0) {
    return false;
  }
  return root.every((part, index) => components[index] === part);
}

/** PS3.5 6.2 and 9.1. At most 64 bytes, digits and dots, no leading zero. */
function isUid(text: string): boolean {
  if (text.length === 0 || text.length > 64) {
    return false;
  }
  if (text.includes(" ") || text.includes("\0")) {
    return false;
  }
  if (text.startsWith(".") || text.endsWith(".")) {
    return false;
  }
  return text.split(".").every((part) => {
    if (part.length === 0) {
      return false;
    }
    if (part.length > 1 && part.startsWith("0")) {
      return false;
    }
    return [...part].every((ch) => ch >= "0" && ch <= "9");
  });
}

function readCounts(
  value: unknown,
): { elements: number; notCheckedUn: number } | "shape" {
  if (!isRecord(value) || !isU32(value.elements) || !isU32(value.notCheckedUn)) {
    return "shape";
  }
  return { elements: value.elements, notCheckedUn: value.notCheckedUn };
}

function readFinding(value: unknown): ConformanceFinding | "shape" | "rule" {
  if (!isRecord(value) || !isTag(value.tag)) {
    return "shape";
  }
  if (!(value.keyword === null || typeof value.keyword === "string")) {
    return "shape";
  }
  const path = readPath(value.path);
  if (path === "shape") {
    return "shape";
  }
  if (typeof value.rule !== "string") {
    return "shape";
  }
  const section = RULE_SECTION.get(value.rule);
  if (section === undefined) {
    return "rule";
  }
  if (!(value.value === null || isPositiveU32(value.value))) {
    return "shape";
  }
  const measure = readMeasure(value.measure);
  if (measure === "shape") {
    return "shape";
  }
  return {
    tag: value.tag,
    keyword: value.keyword,
    path,
    rule: value.rule,
    section,
    value: value.value,
    measure,
  };
}

function readPath(
  value: unknown,
): readonly { sequence: string; item: number }[] | "shape" {
  if (!Array.isArray(value)) {
    return "shape";
  }
  const steps: { sequence: string; item: number }[] = [];
  for (const step of value) {
    if (!isRecord(step) || !isTag(step.sequence) || !isPositiveU32(step.item)) {
      return "shape";
    }
    steps.push({ sequence: step.sequence, item: step.item });
  }
  return steps;
}

function readMeasure(value: unknown): ConformanceMeasure | null | "shape" {
  if (value === null) {
    return null;
  }
  if (!isRecord(value)) {
    return "shape";
  }
  if (value.unit === "bytes" || value.unit === "characters") {
    if (!isU32(value.measured) || !isU32(value.limit)) {
      return "shape";
    }
    return { unit: value.unit, measured: value.measured, limit: value.limit };
  }
  if (value.unit === "values") {
    if (
      !isU32(value.measured) ||
      !isU32(value.minimum) ||
      !(value.maximum === null || isU32(value.maximum))
    ) {
      return "shape";
    }
    return {
      unit: "values",
      measured: value.measured,
      minimum: value.minimum,
      maximum: value.maximum,
    };
  }
  if (value.unit === "character") {
    if (!isU32(value.index) || !isCharacterClass(value.class)) {
      return "shape";
    }
    return { unit: "character", index: value.index, class: value.class };
  }
  return "shape";
}

function readStrings(value: unknown): readonly string[] | "shape" {
  if (!Array.isArray(value)) {
    return "shape";
  }
  const out: string[] = [];
  for (const item of value) {
    if (typeof item !== "string") {
      return "shape";
    }
    out.push(item);
  }
  return out;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isU32(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= U32_MAX;
}

function isPositiveU32(value: unknown): value is number {
  return isU32(value) && value >= 1;
}

function isTag(value: unknown): value is string {
  return typeof value === "string" && TAG.test(value);
}

function isCharacterClass(
  value: unknown,
): value is "control" | "backslash" | "outside-repertoire" | "non-default" {
  return (
    value === "control" ||
    value === "backslash" ||
    value === "outside-repertoire" ||
    value === "non-default"
  );
}
