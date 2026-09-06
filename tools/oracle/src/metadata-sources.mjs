// Raw-DICOM provenance for the metadata fields the comparator judges.
//
// This module deliberately has no Node imports. The browser reads the source
// while it still has dicom-parser's data set, and the Node-side fixture tests
// exercise the same function.

function itemDataSet(dataSet, sequenceTag) {
  return dataSet?.elements?.[sequenceTag]?.items?.[0]?.dataSet ?? null;
}

function functionalGroupHas(dataSet, groupTag, macroTag, attributeTag) {
  if (macroTag === null) {
    return false;
  }
  const group = itemDataSet(dataSet, groupTag);
  const macro = itemDataSet(group, macroTag);
  return macro?.elements?.[attributeTag] !== undefined;
}

function sourceFor(dataSet, macroTag, attributeTag) {
  // PS3.3 C.7.6.16.2.2.1: Per-frame Functional Groups override Shared
  // Functional Groups. Top-level is the source only when neither sequence
  // supplies the attribute.
  if (functionalGroupHas(dataSet, "x52009230", macroTag, attributeTag)) {
    return "per-frame";
  }
  if (functionalGroupHas(dataSet, "x52009229", macroTag, attributeTag)) {
    return "shared-functional-group";
  }
  return dataSet?.elements?.[attributeTag] === undefined ? null : "top-level";
}

/** Provenance of every resolved field covered by metadata-truth.json. */
export function metadataSourcesFor(dataSet) {
  const source = (macro, tag) => sourceFor(dataSet, macro, tag);
  return {
    cornerstoneMetadata: {
      modalityLutModule: {
        rescaleSlope: source("x00289145", "x00281053"),
        rescaleIntercept: source("x00289145", "x00281052"),
      },
      voiLutModule: {
        windowCenter: source("x00289132", "x00281050"),
        windowWidth: source("x00289132", "x00281051"),
        voiLUTFunction: source("x00289132", "x00281056"),
      },
      imagePlaneModule: {
        pixelSpacing: source("x00289110", "x00280030"),
        sliceThickness: source("x00289110", "x00180050"),
      },
      imagePixelModule: {
        bitsStored: source(null, "x00280101"),
        highBit: source(null, "x00280102"),
        pixelRepresentation: source(null, "x00280103"),
      },
    },
  };
}
