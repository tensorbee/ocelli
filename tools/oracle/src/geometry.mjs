// The harness's own reading of a series' geometry, from the files.
//
// PS3.3 C.7.6.2.1.1 is the normative source for every line below. It gives
// Image Orientation (Patient) (0020,0037) as the direction cosines of the
// first ROW and the first COLUMN of the image, and Pixel Spacing (0028,0030)
// as `[between rows, between columns]`. Image Position (Patient) (0020,0032)
// is the position of the centre of the first transmitted voxel.
//
// **This module never reads a cornerstone3D module.** It is the second opinion
// the sidecar already carries for pixel metadata, applied to geometry, and it
// exists for the same reason: a harness that transcribed the reference's own
// answer could not show the reference getting it wrong, which is exactly the
// case on a non-uniform series. See docs/lld/oracle.md.
//
// Two derivations in here are the ones a reviewer should check against the
// standard rather than against the comment above them, because both have a
// plausible wrong form:
//
//  1. **Through-plane spacing is the difference of PROJECTED
//     `ImagePositionPatient` values**, projected onto the cross product of the
//     two direction cosines. Never `SpacingBetweenSlices` (0018,0088), which is
//     Type 3 and frequently wrong, and never `SliceThickness` (0018,0050),
//     which is Type 2 and describes the slab rather than the step.
//     `scripts/corpus_synth.py`'s `case_series` omits (0018,0088) deliberately
//     and gives both series a (0018,0050) of 2.5, so a reader that took either
//     tag would answer 2.5 for a series whose gaps are 2.5, 3.75 and 1.25.
//  2. **`PixelSpacing[0]` multiplies the COLUMN direction cosine.** Advancing
//     one row moves along the column direction, which is the second triplet of
//     (0020,0037). The corpus's deliberately non-square `[0.5, 0.25]` is what
//     makes the transposition visible, and `tests/geometry_test.mjs` asserts
//     it by hand.
//
// HLD section 19 declares `spacing: [f64; 3]` in millimetres and `origin` as
// "IPP of the first slice", and says NOTHING about how the through-plane
// element is derived. That gap is real and is recorded in docs/lld/oracle.md
// so the story that builds the volume can raise it with evidence. This module
// cites PS3.3 directly instead.
//
// Every number here is `World` in HLD section 16's sense: DICOM patient
// coordinates, LPS, millimetres.

/** HLD 25.1: "Geometry: world coordinates within 1e-6 mm". */
export const GEOMETRY_TOLERANCE_MM = 1e-6;

/** The cross product, in the order `row x col`. */
export function cross(a, b) {
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
}

/** The dot product of two three-vectors. */
export function dot(a, b) {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

/** A three-vector scaled by a number. */
export function scale(v, k) {
  return [v[0] * k, v[1] * k, v[2] * k];
}

/** The Euclidean length of a three-vector. */
export function norm(v) {
  return Math.sqrt(dot(v, v));
}

/** Whether two numbers agree within HLD 25.1's geometry tolerance. */
export function within(a, b, toleranceMm = GEOMETRY_TOLERANCE_MM) {
  return Math.abs(a - b) <= toleranceMm;
}

/** Whether two vectors agree element-wise within the geometry tolerance. */
export function vectorsWithin(a, b, toleranceMm = GEOMETRY_TOLERANCE_MM) {
  if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
    return false;
  }
  return a.every((value, index) => within(value, b[index], toleranceMm));
}

function numbers(value, count, what, path) {
  if (
    !Array.isArray(value) ||
    value.length !== count ||
    value.some((entry) => typeof entry !== "number" || !Number.isFinite(entry))
  ) {
    throw new Error(
      `${path}: ${what} is ${JSON.stringify(value)} and PS3.3 C.7.6.2.1.1 ` +
        `requires ${count} finite decimal values. A series member with no ` +
        `geometry cannot be placed, and placing it anyway is how a volume ` +
        `comes out plausible and wrong.`,
    );
  }
  return value;
}

/**
 * The median of a list of numbers.
 *
 * An even count takes the mean of the two middle values, which is the usual
 * definition and is stated here because a nine-gap series takes the odd branch
 * and the even one would otherwise be written by whoever first met it.
 */
export function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
}

/**
 * Measure one series' geometry from its members' own attributes.
 *
 * @param {Array<{path: string, imagePositionPatient: number[],
 *   imageOrientationPatient: number[], pixelSpacing: number[]}>} members
 * @param {{toleranceMm?: number}} options
 */
export function measureSubject(members, options = {}) {
  const toleranceMm = options.toleranceMm ?? GEOMETRY_TOLERANCE_MM;
  if (!Array.isArray(members) || members.length < 2) {
    throw new Error(
      `a series needs at least two members to have a gap, and this one has ` +
        `${Array.isArray(members) ? members.length : 0}`,
    );
  }

  const first = members[0];
  const orientation = numbers(
    first.imageOrientationPatient,
    6,
    "ImageOrientationPatient (0020,0037)",
    first.path,
  );
  const spacing = numbers(
    first.pixelSpacing,
    2,
    "PixelSpacing (0028,0030)",
    first.path,
  );

  const rowCosine = orientation.slice(0, 3);
  const columnCosine = orientation.slice(3, 6);
  const normal = cross(rowCosine, columnCosine);

  // PS3.3 C.7.6.2.1.1 gives (0020,0037) as direction COSINES, so both triplets
  // are unit vectors and the standard requires them to be orthogonal. Their
  // cross product is then a unit vector too, and the projection below is a
  // distance in millimetres only because it is. A normal of another length
  // would scale every gap by an amount nobody declared, so this is refused
  // rather than normalised: a file whose cosines are not cosines is a file
  // whose geometry is not trustworthy, and silently fixing it is the quiet
  // kind of wrong.
  if (!within(norm(rowCosine), 1, toleranceMm) ||
      !within(norm(columnCosine), 1, toleranceMm) ||
      !within(norm(normal), 1, toleranceMm)) {
    throw new Error(
      `${first.path}: ImageOrientationPatient ` +
        `${JSON.stringify(orientation)} is not two perpendicular vectors of ` +
        `unit length (|row| ${norm(rowCosine)}, |col| ${norm(columnCosine)}, ` +
        `|row x col| ${norm(normal)}). PS3.3 C.7.6.2.1.1 gives them as ` +
        `direction cosines, and the projected distance below is in ` +
        `millimetres only because they are.`,
    );
  }

  // Every member must agree, because one volume carries one orientation and
  // one in-plane spacing. A series that does not agree is not one volume, and
  // averaging the disagreement away is the same defect as averaging the gaps.
  const placed = members.map((entry) => {
    const memberOrientation = numbers(
      entry.imageOrientationPatient,
      6,
      "ImageOrientationPatient (0020,0037)",
      entry.path,
    );
    const memberSpacing = numbers(
      entry.pixelSpacing,
      2,
      "PixelSpacing (0028,0030)",
      entry.path,
    );
    const position = numbers(
      entry.imagePositionPatient,
      3,
      "ImagePositionPatient (0020,0032)",
      entry.path,
    );
    if (!vectorsWithin(memberOrientation, orientation, toleranceMm)) {
      throw new Error(
        `${entry.path}: ImageOrientationPatient is ` +
          `${JSON.stringify(memberOrientation)} and ${first.path} declares ` +
          `${JSON.stringify(orientation)}. One volume carries one ` +
          `orientation, so a series that disagrees is not one volume.`,
      );
    }
    if (!vectorsWithin(memberSpacing, spacing, toleranceMm)) {
      throw new Error(
        `${entry.path}: PixelSpacing is ${JSON.stringify(memberSpacing)} and ` +
          `${first.path} declares ${JSON.stringify(spacing)}. One volume ` +
          `carries one in-plane spacing.`,
      );
    }
    return { path: entry.path, position, projectionMm: dot(position, normal) };
  });

  // Ordered by the projected position and by nothing else. Not by
  // InstanceNumber, which is Type 2 and is a label rather than a geometry, and
  // not by file name.
  const order = [...placed].sort((a, b) => a.projectionMm - b.projectionMm);
  const projectionsMm = order.map((entry) => entry.projectionMm);

  const gapsMm = [];
  for (let index = 1; index < projectionsMm.length; index += 1) {
    const gap = projectionsMm[index] - projectionsMm[index - 1];
    if (gap <= toleranceMm) {
      // The two paths and NOT their positions. This message is the one refusal
      // here that a REAL corpus row reaches, and `corpus_check.py`'s convention
      // is that a real row reports the relative path and the attribute name
      // and never the value. The paths identify which two members collided,
      // which is the whole diagnostic, and the coordinate adds nothing that a
      // sidecar does not already carry into ignored output.
      throw new Error(
        `${order[index - 1].path} and ${order[index].path} project to the ` +
          `same position on the slice normal, within ${toleranceMm} mm. Two ` +
          `members at one position is not a gap of zero, it is a series that ` +
          `cannot be a volume.`,
      );
    }
    gapsMm.push(gap);
  }

  const meanGapMm =
    gapsMm.reduce((total, gap) => total + gap, 0) / gapsMm.length;
  const maxDeviationFromMeanMm = Math.max(
    ...gapsMm.map((gap) => Math.abs(gap - meanGapMm)),
  );

  return {
    order,
    normal,
    rowCosine,
    columnCosine,
    pixelSpacing: spacing,
    projectionsMm,
    gapsMm,
    meanGapMm,
    medianGapMm: median(gapsMm),
    minGapMm: Math.min(...gapsMm),
    maxGapMm: Math.max(...gapsMm),
    maxDeviationFromMeanMm,
    voxelAxes: {
      // PS3.3 C.7.6.2.1.1: PixelSpacing is [between rows, between columns].
      // Advancing one COLUMN moves along the ROW direction cosine by the
      // between-columns spacing, and advancing one ROW moves along the COLUMN
      // direction cosine by the between-rows spacing. Getting these two the
      // other way round is the transposition the corpus's non-square
      // [0.5, 0.25] exists to catch.
      columnStepMm: scale(rowCosine, spacing[1]),
      rowStepMm: scale(columnCosine, spacing[0]),
      // The MEAN gap, which is what an averaging builder uses. Recorded under
      // a name that says it is a step so a reader can see that on a
      // non-uniform series no interior slice sits where it says.
      sliceStepMm: scale(normal, meanGapMm),
    },
  };
}
