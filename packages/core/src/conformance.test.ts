/**
 * Reader for the F-225 report.
 *
 * The fixtures below are written from the plan. They are not copies of
 * `to_json` output. Sections on the wire are the word "wire" so a reader
 * that trusts the wire fails.
 */
import { describe, expect, it } from "vitest";

import {
  CONFORMANCE_RULES,
  CONFORMANCE_SCOPE,
  SOP_CLASS_NOTES,
  readConformanceReport,
} from "./index.js";

const CT_SOP = "1.2.840.10008.5.1.4.1.1.2";

const CT_MODULES = [
  "Patient",
  "General Study",
  "General Series",
  "General Image",
  "Image Plane",
  "Image Pixel",
  "SOP Common",
];

/** Decision 4, in the order the checker writes. */
const NOT_CHECKED = [
  "Laterality",
  "ContentDate",
  "ContentTime",
  "AnatomicalOrientationType",
  "PixelAspectRatio",
  "PixelPaddingRangeLimit",
  "PixelDataProviderURL",
  "PatientAlternativeCalendar",
  "PatientSpeciesDescription",
  "PatientSpeciesCodeSequence",
  "PatientBreedDescription",
  "PatientBreedCodeSequence",
  "BreedRegistrationSequence",
  "ResponsiblePerson",
  "ResponsiblePersonRole",
  "ResponsibleOrganization",
  "DeidentificationMethod",
  "DeidentificationMethodCodeSequence",
  "EncryptedAttributesSequence",
  "HL7StructuredDocumentReferenceSequence",
  "QueryRetrieveView",
  "ConversionSourceAttributesSequence",
];

function omit(body: Record<string, unknown>, key: string): Record<string, unknown> {
  const copy: Record<string, unknown> = {};
  for (const [name, value] of Object.entries(body)) {
    if (name !== key) {
      copy[name] = value;
    }
  }
  return copy;
}

function sectionOf(id: string): string {
  const found = CONFORMANCE_RULES.find((rule) => rule.id === id);
  return found === undefined ? "" : found.section;
}

function ctReport(findings: unknown[], elements = 30): Record<string, unknown> {
  return {
    scope: CONFORMANCE_SCOPE,
    edition: "2026d",
    rules: [{ id: "NOT-A-RULE", section: "wire" }],
    modules: {
      evaluated: true,
      sopClass: CT_SOP,
      sopClassNote: null,
      iod: "CT Image",
      table: "PS3.3 A.3.3",
      modules: CT_MODULES,
      notChecked: NOT_CHECKED,
    },
    counts: { elements, notCheckedUn: 0 },
    findings,
    note: "ignored",
  };
}

function finding(rule: string, extra: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    tag: "(0010,1010)",
    keyword: "PatientAge",
    path: [],
    rule,
    section: "wire",
    value: 1,
    measure: null,
    ...extra,
  };
}

describe("readConformanceReport", () => {
  it("requires the exact scope sentence and checks it before the rule", () => {
    expect(CONFORMANCE_SCOPE).toBe(
      "This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.",
    );
    expect(sectionOf("AS-FORMAT")).toBe("PS3.5 6.2 Table 6.2-1");
    expect(sectionOf("TYPE1-EMPTY")).toBe("PS3.5 7.4.1");
    expect(sectionOf("UI-FORMAT")).toBe("PS3.5 9.1");
    expect(sectionOf("MULTIPLICITY")).toBe("PS3.5 6.4");
    expect(sectionOf("CHARSET-UNDECLARED")).toBe("PS3.5 6.1.2.2");

    const text = JSON.stringify(ctReport([finding("NOT-A-RULE")]));
    const changed = text.replace(CONFORMANCE_SCOPE, `${CONFORMANCE_SCOPE.slice(0, -1)}?`);
    expect(readConformanceReport(changed)).toEqual({ ok: false, reason: "scope" });
    expect(readConformanceReport(text)).toEqual({ ok: false, reason: "rule" });
  });

  it("accepts the valid CT report and ignores the wire catalogue", () => {
    const read = readConformanceReport(JSON.stringify(ctReport([])));
    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.report.edition).toBe("2026d");
    expect(read.report.findings).toEqual([]);
    expect(read.report.counts).toEqual({ elements: 30, notCheckedUn: 0 });
    expect(read.report.modules).toEqual({
      evaluated: true,
      sopClass: CT_SOP,
      sopClassNote: null,
      iod: "CT Image",
      table: "PS3.3 A.3.3",
      modules: CT_MODULES,
      notChecked: NOT_CHECKED,
    });
    expect("rules" in read.report).toBe(false);
  });

  it("accepts the reproducer and fills section from the catalogue", () => {
    // CT base is 30 elements. The reproducer blanks SOP Instance UID and
    // Modality and adds Patient Age and Institution Name, so the count is 32.
    const text = JSON.stringify(
      ctReport(
        [
          finding("TYPE1-EMPTY", {
            tag: "(0008,0018)",
            keyword: "SOPInstanceUID",
            value: null,
            measure: null,
          }),
          finding("TYPE1-EMPTY", {
            tag: "(0008,0060)",
            keyword: "Modality",
            value: null,
            measure: null,
          }),
          finding("VALUE-LENGTH", {
            tag: "(0008,0080)",
            keyword: "InstitutionName",
            value: 1,
            measure: { unit: "characters", measured: 170, limit: 64 },
          }),
          finding("AS-FORMAT", {
            tag: "(0010,1010)",
            keyword: "PatientAge",
            value: 1,
            measure: null,
          }),
        ],
        32,
      ),
    );
    expect(text.includes("040")).toBe(false);
    expect(text.includes("SECRET")).toBe(false);
    const read = readConformanceReport(text);
    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.report.counts.elements).toBe(32);
    expect(read.report.findings.map((item) => item.rule)).toEqual([
      "TYPE1-EMPTY",
      "TYPE1-EMPTY",
      "VALUE-LENGTH",
      "AS-FORMAT",
    ]);
    expect(read.report.findings[0]).toMatchObject({
      tag: "(0008,0018)",
      value: null,
      measure: null,
      section: "PS3.5 7.4.1",
    });
    expect(read.report.findings[2]).toMatchObject({
      tag: "(0008,0080)",
      keyword: "InstitutionName",
      value: 1,
      section: sectionOf("VALUE-LENGTH"),
      measure: { unit: "characters", measured: 170, limit: 64 },
    });
    expect(read.report.findings[3]).toMatchObject({
      rule: "AS-FORMAT",
      section: sectionOf("AS-FORMAT"),
      measure: null,
    });
    expect(read.report.findings.every((item) => item.section !== "wire")).toBe(true);
  });

  it("refuses invalid JSON, a missing findings array, and a bad shape", () => {
    expect(readConformanceReport("{")).toEqual({ ok: false, reason: "json" });
    expect(readConformanceReport("")).toEqual({ ok: false, reason: "json" });
    expect(readConformanceReport("[]")).toEqual({ ok: false, reason: "shape" });
    expect(readConformanceReport("null")).toEqual({ ok: false, reason: "shape" });
    expect(readConformanceReport(JSON.stringify(omit(ctReport([]), "findings")))).toEqual({
      ok: false,
      reason: "shape",
    });
    expect(readConformanceReport(JSON.stringify(omit(ctReport([]), "scope")))).toEqual({
      ok: false,
      reason: "shape",
    });
    expect(readConformanceReport(JSON.stringify(ctReport([finding("TYPE1-EMPTY", { rule: 1 })])))).toEqual({
      ok: false,
      reason: "shape",
    });
  });

  it("accepts the other measure forms, a nested path, and an unevaluated SOP", () => {
    const read = readConformanceReport(
      JSON.stringify(
        ctReport([
          finding("VALUE-LENGTH", {
            tag: "(0008,0008)",
            keyword: "ImageType",
            measure: { unit: "bytes", measured: 17, limit: 16 },
          }),
          finding("MULTIPLICITY", {
            tag: "(0028,0030)",
            keyword: "PixelSpacing",
            value: null,
            measure: { unit: "values", measured: 1, minimum: 2, maximum: 2 },
          }),
          finding("MULTIPLICITY", {
            tag: "(0008,0005)",
            keyword: "SpecificCharacterSet",
            value: null,
            measure: { unit: "values", measured: 3, minimum: 1, maximum: null },
          }),
          finding("REPERTOIRE", {
            tag: "(0008,0060)",
            keyword: "Modality",
            path: [{ sequence: "(0054,0410)", item: 2 }],
            measure: { unit: "character", index: 0, class: "outside-repertoire" },
          }),
        ]),
      ),
    );
    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.report.findings[3]?.path).toEqual([{ sequence: "(0054,0410)", item: 2 }]);
    expect(read.report.findings[3]?.section).toBe("PS3.5 6.2 Table 6.2-1");

    const unevaluated = ctReport([]);
    unevaluated.modules = {
      evaluated: false,
      sopClass: "1.2.840.10008.5.1.4.1.1.7",
      sopClassNote: null,
    };
    const second = readConformanceReport(JSON.stringify(unevaluated));
    expect(second.ok).toBe(true);
    if (second.ok) {
      expect(second.report.modules).toEqual({
        evaluated: false,
        sopClass: "1.2.840.10008.5.1.4.1.1.7",
        sopClassNote: null,
      });
    }
    unevaluated.modules = { evaluated: false, sopClass: null, sopClassNote: null };
    unevaluated.edition = "1993";
    const third = readConformanceReport(JSON.stringify(unevaluated));
    expect(third.ok).toBe(true);
    if (third.ok) {
      expect(third.report.edition).toBe("1993");
      expect(third.report.modules).toEqual({
        evaluated: false,
        sopClass: null,
        sopClassNote: null,
      });
    }
    unevaluated.modules = {
      evaluated: false,
      sopClass: null,
      sopClassNote: "not a valid UID",
    };
    const noted = readConformanceReport(JSON.stringify(unevaluated));
    expect(noted.ok).toBe(true);
    if (noted.ok) {
      expect(noted.report.modules).toEqual({
        evaluated: false,
        sopClass: null,
        sopClassNote: "not a valid UID",
      });
    }
    unevaluated.modules = {
      evaluated: false,
      sopClass: "SECRET^NAME",
      sopClassNote: null,
    };
    expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
    unevaluated.modules = {
      evaluated: false,
      sopClass: null,
      sopClassNote: "SECRET^NAME",
    };
    expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
    unevaluated.modules = {
      evaluated: false,
      sopClass: "1.2.840.10008.5.1.4.1.1.7",
      sopClassNote: "not a valid UID",
    };
    expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
      ok: false,
      reason: "shape",
    });

    const badClass = ctReport([
      finding("REPERTOIRE", {
        measure: { unit: "character", index: 0, class: "nope" },
      }),
    ]);
    expect(readConformanceReport(JSON.stringify(badClass))).toEqual({
      ok: false,
      reason: "shape",
    });
  });

  it("accepts only the closed sop class notes", () => {
    // PS3.5 9.1 and PS3.6. The note set is the generated closed set. A private
    // UID and a single component of digits are grammatical UIDs and are not
    // repeated, so the reader refuses them in sopClass.
    expect(SOP_CLASS_NOTES).toEqual(["not a valid UID", "not a DICOM UID"]);
    const unevaluated = ctReport([]);
    unevaluated.modules = {
      evaluated: false,
      sopClass: null,
      sopClassNote: "not a DICOM UID",
    };
    const read = readConformanceReport(JSON.stringify(unevaluated));
    expect(read.ok).toBe(true);
    if (read.ok) {
      expect(read.report.modules).toEqual({
        evaluated: false,
        sopClass: null,
        sopClassNote: "not a DICOM UID",
      });
    }
    for (const sopClass of ["1.2.3.4", "19620203"]) {
      unevaluated.modules = { evaluated: false, sopClass, sopClassNote: null };
      expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
        ok: false,
        reason: "shape",
      });
    }
    unevaluated.modules = {
      evaluated: false,
      sopClass: "1.2.3.4",
      sopClassNote: "not a DICOM UID",
    };
    expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
    unevaluated.modules = {
      evaluated: false,
      sopClass: null,
      sopClassNote: "private UID",
    };
    expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
    const evaluated = ctReport([]);
    const modules = evaluated.modules as Record<string, unknown>;
    modules.sopClassNote = "not a DICOM UID";
    expect(readConformanceReport(JSON.stringify(evaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
    delete modules.sopClassNote;
    expect(readConformanceReport(JSON.stringify(evaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
  });

  it("refuses a sop class outside the DICOM root components", () => {
    // PS3.5 9.1 and PS3.6. Components must begin with 1, 2, 840, 10008 and
    // one more component must follow. 1.2.840.100081.2 shares the characters
    // and its next component is 100081. 1.2.840.10008 is the root alone.
    // An evaluated sopClass of 1.2.3.4 is a private UID.
    const unevaluated = ctReport([]);
    for (const sopClass of ["1.2.840.100081.2", "1.2.840.10008"]) {
      unevaluated.modules = { evaluated: false, sopClass, sopClassNote: null };
      expect(readConformanceReport(JSON.stringify(unevaluated))).toEqual({
        ok: false,
        reason: "shape",
      });
    }
    const child = ctReport([]);
    child.modules = {
      evaluated: false,
      sopClass: "1.2.840.10008.1",
      sopClassNote: null,
    };
    expect(readConformanceReport(JSON.stringify(child)).ok).toBe(true);
    const evaluated = ctReport([]);
    const modules = evaluated.modules as Record<string, unknown>;
    modules.sopClass = "1.2.3.4";
    expect(readConformanceReport(JSON.stringify(evaluated))).toEqual({
      ok: false,
      reason: "shape",
    });
  });
});
