// Hand-computed geometry fixtures, from PS3.3 C.7.6.2.1.1 and the generator.
//
// Every expected number below is worked out in the comment beside it from the
// standard and from `scripts/corpus_synth.py`'s own constants, never from what
// `src/geometry.mjs` returns and never from what cornerstone3D returns. HLD
// 27.2 R2: an agent asked to test a function asserts what it does, not what it
// should do, so the arithmetic is written out first and the module is judged
// against it.
//
// The tolerance is HLD 25.1's geometry line, "world coordinates within 1e-6
// mm", and nothing here sets another.
//
//   IOP    = [0.8, 0.6, 0.0,  0.0, 0.0, -1.0]
//   row    = (0.8, 0.6,  0.0)
//   col    = (0.0, 0.0, -1.0)
//
//   normal = row x col
//     nx = row_y*col_z - row_z*col_y = 0.6*(-1) - 0.0*0.0 = -0.6
//     ny = row_z*col_x - row_x*col_z = 0.0*0.0 - 0.8*(-1) =  0.8
//     nz = row_x*col_y - row_y*col_x = 0.8*0.0 - 0.6*0.0  =  0.0
//   |normal| = sqrt(0.36 + 0.64) = 1 exactly
//
//   corpus_synth.py writes IPP_k = d_k * normal, so IPP_k . normal = d_k
//   because |normal|^2 is 1.
//
//   SERIES_SPACING 2.5, SERIES_SLICES 10, NONUNIFORM_SLICE 7,
//   NONUNIFORM_OFFSET 1.25:
//
//     uniform     d = 0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5,  20, 22.5
//     non-uniform d = 0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5
//     non-uniform gaps = 2.5 x6, 3.75, 1.25, 2.5
//     mean gap   = |22.5 - 0| / 9 = 2.5   for BOTH series
//     median gap = 2.5                    for BOTH series
//
//   PixelSpacing = [0.5, 0.25], which PS3.3 C.7.6.2.1.1 reads as
//   [between rows, between columns]. Advancing one ROW moves along the COLUMN
//   direction cosine, which is the second IOP triplet, so
//
//     rowStep    = PixelSpacing[0] * col = 0.5  * (0, 0, -1)    = (0, 0, -0.5)
//     columnStep = PixelSpacing[1] * row = 0.25 * (0.8, 0.6, 0) = (0.2, 0.15, 0)
//     sliceStep  = meanGap        * normal = 2.5 * (-0.6, 0.8, 0) = (-1.5, 2, 0)

import test from "node:test";
import assert from "node:assert/strict";

import {
  GEOMETRY_TOLERANCE_MM,
  cross,
  measureSubject,
  norm,
} from "../src/geometry.mjs";

const IOP = [0.8, 0.6, 0.0, 0.0, 0.0, -1.0];
const NORMAL = [-0.6, 0.8, 0.0];
const PIXEL_SPACING = [0.5, 0.25];

/** The projected distances corpus_synth.py places each slice at. */
const UNIFORM_D = [0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5, 20, 22.5];
const NONUNIFORM_D = [0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5];

/**
 * One member, built the way `case_series` writes it.
 *
 * `f"{distance * axis:.6f}"` in the generator, so the position is the six
 * decimal rounding of `d * normal` and not `d * normal` itself. Reproduced
 * here rather than idealised, because the six decimals are what the file
 * carries and what `dicom-parser` will read back.
 */
function member(path, distance, { orientation = IOP, spacing = PIXEL_SPACING } = {}) {
  return {
    path,
    imagePositionPatient: NORMAL.map((axis) => Number((distance * axis).toFixed(6))),
    imageOrientationPatient: orientation,
    pixelSpacing: spacing,
  };
}

function series(distances, prefix = "synthetic/ct_series_x") {
  return distances.map((distance, index) =>
    member(`${prefix}/slice_${String(index).padStart(3, "0")}.dcm`, distance),
  );
}

function close(actual, expected, what) {
  assert.ok(
    Math.abs(actual - expected) <= GEOMETRY_TOLERANCE_MM,
    `${what}: ${actual} is not within ${GEOMETRY_TOLERANCE_MM} of ${expected}`,
  );
}

function closeVector(actual, expected, what) {
  assert.equal(actual.length, expected.length, `${what}: wrong length`);
  actual.forEach((value, index) =>
    close(value, expected[index], `${what}[${index}]`),
  );
}

// HLD 25.1's geometry line is the only tolerance this story applies, and it is
// asserted rather than described so a later widening is a diff.
test("the geometry tolerance is HLD 25.1's 1e-6 mm", () => {
  assert.equal(GEOMETRY_TOLERANCE_MM, 1e-6);
});

test("the slice normal is the cross product PS3.3 C.7.6.2.1.1 implies", () => {
  closeVector(cross([0.8, 0.6, 0.0], [0.0, 0.0, -1.0]), NORMAL, "normal");
  close(norm(NORMAL), 1, "|normal|");
});

// The cross product is anti-commutative, so a reader who took col x row would
// get the normal pointing the other way and every projection negated. The
// order is row THEN column, which is the order the two triplets appear in
// (0020,0037).
test("the cross product is taken row x col and not col x row", () => {
  closeVector(
    cross([0.0, 0.0, -1.0], [0.8, 0.6, 0.0]),
    [0.6, -0.8, 0.0],
    "col x row",
  );
});

test("the uniform series projects onto the normal at the generator's distances", () => {
  const measured = measureSubject(series(UNIFORM_D, "synthetic/ct_series_uniform"));
  closeVector(measured.normal, NORMAL, "normal");
  closeVector(measured.projectionsMm, UNIFORM_D, "projections");
  closeVector(measured.gapsMm, new Array(9).fill(2.5), "gaps");
  close(measured.meanGapMm, 2.5, "mean gap");
  close(measured.medianGapMm, 2.5, "median gap");
  close(measured.minGapMm, 2.5, "min gap");
  close(measured.maxGapMm, 2.5, "max gap");
  close(measured.maxDeviationFromMeanMm, 0, "max deviation from the mean");
});

// The whole point of the non-uniform series. Slice 7 sits at 18.75 rather than
// 17.5, so the gap into it is 3.75 and the gap out of it is 1.25, and the mean
// and the median are both still 2.5.
test("the non-uniform series has one displaced slice and two odd gaps", () => {
  const measured = measureSubject(
    series(NONUNIFORM_D, "synthetic/ct_series_nonuniform"),
  );
  closeVector(measured.projectionsMm, NONUNIFORM_D, "projections");
  closeVector(
    measured.gapsMm,
    [2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 3.75, 1.25, 2.5],
    "gaps",
  );
  close(measured.meanGapMm, 2.5, "mean gap");
  close(measured.medianGapMm, 2.5, "median gap");
  close(measured.minGapMm, 1.25, "min gap");
  close(measured.maxGapMm, 3.75, "max gap");
  // 3.75 - 2.5 = 1.25, which is exactly half a nominal gap.
  close(measured.maxDeviationFromMeanMm, 1.25, "max deviation from the mean");
});

// The reference computes |d_last - d_first| / (N - 1) and never compares one
// gap with another, so the two series hand it the same number. That is a fact
// about cornerstone3D 5.8.2 and it is asserted here on OUR OWN arithmetic, so
// the harness can show the reference agreeing with a mean it should not have
// used.
test("the mean gap alone cannot separate the two series", () => {
  const uniform = measureSubject(series(UNIFORM_D));
  const nonuniform = measureSubject(series(NONUNIFORM_D));
  close(uniform.meanGapMm, nonuniform.meanGapMm, "mean gap");
  close(uniform.medianGapMm, nonuniform.medianGapMm, "median gap");
  assert.notEqual(uniform.minGapMm, nonuniform.minGapMm);
});

// THE TRANSPOSITION FIXTURE. PS3.3 C.7.6.2.1.1 gives PixelSpacing as
// [between rows, between columns], and advancing one row moves along the
// COLUMN direction cosine. The corpus's deliberately non-square [0.5, 0.25] is
// what makes the wrong index visible: with the two swapped, rowStep would be
// (0, 0, -0.25) and columnStep (0.4, 0.3, 0).
test("PixelSpacing[0] multiplies the column cosine and [1] the row cosine", () => {
  const { voxelAxes } = measureSubject(series(UNIFORM_D));
  closeVector(voxelAxes.rowStepMm, [0.0, 0.0, -0.5], "rowStep");
  closeVector(voxelAxes.columnStepMm, [0.2, 0.15, 0.0], "columnStep");
  closeVector(voxelAxes.sliceStepMm, [-1.5, 2.0, 0.0], "sliceStep");

  // And the transposed answers are NOT what came back, stated positively so
  // this fixture fails for the transposition specifically rather than for any
  // difference at all.
  assert.ok(Math.abs(voxelAxes.rowStepMm[2] - -0.25) > GEOMETRY_TOLERANCE_MM);
  assert.ok(Math.abs(voxelAxes.columnStepMm[0] - 0.4) > GEOMETRY_TOLERANCE_MM);
});

// The through-plane step is the MEAN gap, which is what a builder that
// averages would use, and recording it makes the averaging visible rather than
// implied. On the non-uniform series it is 2.5 and no slice but the endpoints
// sits where it says.
test("the slice step is the mean gap along the normal, for both series", () => {
  const nonuniform = measureSubject(series(NONUNIFORM_D));
  closeVector(nonuniform.voxelAxes.sliceStepMm, [-1.5, 2.0, 0.0], "sliceStep");
});

// On BOTH corpus series the mean and the median are exactly 2.5, so neither
// this fixture nor the corpus can tell a mean from a median. Measured: swapping
// `meanGapMm` for `median(gapsMm)` in `voxelAxes.sliceStepMm` left the whole
// suite green. So the choice is asserted on a series where the two differ,
// which the corpus does not contain and does not need to.
//
//   d    = 0, 2.5, 5, 12.5
//   gaps = 2.5, 2.5, 7.5
//   mean   = 12.5 / 3 = 4.1666666666666667
//   median = 2.5
//   sliceStep = mean * (-0.6, 0.8, 0) = (-2.5, 3.3333333333333335, 0)
test("the slice step is the MEAN and not the median, on a series where they differ", () => {
  const measured = measureSubject(series([0, 2.5, 5, 12.5]));
  close(measured.meanGapMm, 12.5 / 3, "mean gap");
  close(measured.medianGapMm, 2.5, "median gap");
  closeVector(
    measured.voxelAxes.sliceStepMm,
    [(-0.6 * 12.5) / 3, (0.8 * 12.5) / 3, 0],
    "sliceStep",
  );
  // And explicitly not the median's answer, which is (-1.5, 2, 0).
  assert.ok(
    Math.abs(measured.voxelAxes.sliceStepMm[0] - -1.5) > GEOMETRY_TOLERANCE_MM,
  );
});

test("members are ordered by their projection, whatever order they arrive in", () => {
  const shuffled = [...series(NONUNIFORM_D)];
  const swap = shuffled[2];
  shuffled[2] = shuffled[7];
  shuffled[7] = swap;
  const measured = measureSubject(shuffled);
  closeVector(measured.projectionsMm, NONUNIFORM_D, "projections");
  assert.deepEqual(
    measured.order.map((entry) => entry.path),
    series(NONUNIFORM_D).map((entry) => entry.path),
  );
});

// Refusals. Each one is a thing that would otherwise produce a plausible
// number from data that cannot support it.

test("a series of fewer than two members has no gap to measure", () => {
  assert.throws(() => measureSubject(series([0])), /at least two/);
});

test("members that disagree about the image orientation are refused", () => {
  const members = series(UNIFORM_D);
  members[4] = member(members[4].path, UNIFORM_D[4], {
    orientation: [1, 0, 0, 0, 1, 0],
  });
  assert.throws(() => measureSubject(members), /ImageOrientationPatient/);
});

test("members that disagree about the pixel spacing are refused", () => {
  const members = series(UNIFORM_D);
  members[4] = member(members[4].path, UNIFORM_D[4], { spacing: [1, 1] });
  assert.throws(() => measureSubject(members), /PixelSpacing/);
});

test("a member with no position, orientation or spacing is refused", () => {
  for (const field of [
    "imagePositionPatient",
    "imageOrientationPatient",
    "pixelSpacing",
  ]) {
    const members = series(UNIFORM_D);
    members[3] = { ...members[3], [field]: null };
    assert.throws(() => measureSubject(members), /slice_003/, field);
  }
});

// Two direction cosines that are not perpendicular unit vectors give a normal
// that is not unit length, and every projection would then be scaled by an
// amount nobody declared. PS3.3 C.7.6.2.1.1 requires them to be unit vectors.
test("direction cosines that are not unit and perpendicular are refused", () => {
  const members = series(UNIFORM_D).map((entry) => ({
    ...entry,
    imageOrientationPatient: [1, 0, 0, 1, 0, 0],
  }));
  assert.throws(() => measureSubject(members), /unit length/);
});

test("two members at the same position are refused, because the gap is zero", () => {
  const members = series([0, 2.5, 2.5, 7.5]);
  assert.throws(() => measureSubject(members), /same position/);
});
