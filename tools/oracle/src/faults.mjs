// The fault catalogue: one declaration site for every injected failure.
//
// `docs/sprints/CURRENT_SPRINT.md`: "Every new guard is observed red before it
// is claimed." The four boundary assertions in this harness are exactly the
// kind that pass forever because the condition they test never arises on a
// healthy machine, so each one is aimed at by a named fault and the run is
// required to fail AT THAT BOUNDARY WITH THAT REASON.
//
// Each fault breaks ONE thing and nothing else. A fault that made the run fail
// for some other reason would prove nothing about the guard it was aimed at,
// which is why `expect` below is a fragment of the specific message and not
// just the boundary name.
//
// Where a fault mutates something, the mutation lives here beside its
// declaration, unless it can only happen inside the browser. `mutateBytes` is
// the node-side half, `pageFault` is the browser-side half, and a fault has at
// most one of them.

/** One corpus row: uncompressed Explicit VR Little Endian, twelve bits in 16. */
export const SUBJECT = "syntax/reference_mono12.dcm";

/**
 * The rows a volume fault selects: the ten uniform synthetic slices.
 *
 * One complete subject and the smallest one, so a volume fault costs one
 * volume build of ten 20 by 12 slices rather than twenty-seven full-resolution
 * CT instances. It is a `--rows` substring, so it matches exactly those ten and
 * no other manifest row.
 *
 * `SUBJECT` above touches NO series row, which is what keeps the twelve stack
 * faults from attempting a volume at all: no subject is completely selected and
 * none is partially selected either, so the volume pass is declared skipped in
 * `run.json` rather than silently absent.
 */
export const VOLUME_SUBJECT = "synthetic/ct_series_uniform/";

/**
 * Rewrite the Transfer Syntax UID to one no decoder claims.
 *
 * `1.2.840.10008.1.9.9` is the same nineteen characters as Explicit VR Little
 * Endian, so the replacement is byte for byte and the File Meta group's
 * lengths stay valid. It is a well formed UID that means nothing, which is
 * exactly the case boundary two has to refuse.
 *
 * The search is bounded to the File Meta region. PS3.10 7.1 puts the 128 byte
 * preamble, `DICM` and the whole of group 0002 at the front of the file, so a
 * match beyond that would be a Transfer Syntax UID quoted inside the data set
 * rather than the one the file is encoded in, and rewriting that would inject
 * a different fault from the one this claims to be.
 */
const FILE_META_SEARCH_BYTES = 1024;

function rewriteTransferSyntax(bytes) {
  const from = Buffer.from("1.2.840.10008.1.2.1", "latin1");
  const to = Buffer.from("1.2.840.10008.1.9.9", "latin1");
  const at = bytes.subarray(0, FILE_META_SEARCH_BYTES).indexOf(from);
  if (at < 0) {
    throw new Error(
      `the reject-syntax fault needs a row declaring Explicit VR Little ` +
        `Endian in its File Meta group, and that UID is not in the first ` +
        `${FILE_META_SEARCH_BYTES} bytes of this row`,
    );
  }
  const copy = Buffer.from(bytes);
  to.copy(copy, at);
  return copy;
}

/**
 * The injections and what each must produce.
 *
 *  - `boundary` is the guard being aimed at.
 *  - `expect` is a fragment of the message that guard produces.
 *  - `mutateBytes` is the node-side byte mutation, if any.
 *  - `mutateParams` is the node-side render-parameter mutation, if any.
 *  - `pageFault` is true when the page implements the mutation instead.
 *  - `skipRow` is the one fault that breaks the driver's own loop.
 *
 * Every entry declares the boundary it aims at, and the boundaries below are
 * the authority on which fault covers which. Some aim at a boundary's own
 * check and some at a refusal INSIDE one that no other fault reaches, and
 * `no-stack` is the important one: it is the guard that stops the previous
 * row's frame being written under this row's name, which is the
 * quietly-wrong-pixel class this project names as its dangerous defect.
 *
 * No count of them is written here. One was, twice, and it was wrong both
 * times within a round of the entry that changed it.
 */
export const FAULTS = {
  "drop-row": {
    boundary: "reached",
    row: SUBJECT,
    expect: "were never attempted",
    what: "the driver skips a row, and nothing else about the run is wrong",
    skipRow: true,
  },
  truncate: {
    boundary: "decoded",
    row: SUBJECT,
    // The reader's own message for a buffer that ends mid-element, and not the
    // boundary name. `decoded:` is the prefix of every decode failure, so it
    // would be satisfied by a corrupted corpus row or a codec regression just
    // as happily, and this fault would then report "red at decoded, as
    // required" having stopped truncating anything.
    expect: "Request more than currently allocated buffer",
    what: "the row's bytes are cut to 256, so the loader has no pixel data",
    mutateBytes: (bytes) => bytes.subarray(0, 256),
  },
  "reject-syntax": {
    boundary: "decoded",
    row: SUBJECT,
    expect: "No decoder for transfer syntax 1.2.840.10008.1.9.9",
    what: "the Transfer Syntax UID is rewritten to one no decoder claims",
    mutateBytes: rewriteTransferSyntax,
  },
  "no-render-event": {
    boundary: "presented",
    row: SUBJECT,
    expect: "no CORNERSTONE_IMAGE_RENDERED within",
    what: "the page loads and windows the image but never calls render()",
    pageFault: true,
  },
  "stale-frame": {
    boundary: "read-back",
    row: SUBJECT,
    expect: "still the sentinel colour",
    what: "the page fires IMAGE_RENDERED without drawing anything",
    pageFault: true,
  },
  "uniform-canvas": {
    boundary: "read-back",
    row: SUBJECT,
    expect: "every pixel of the frame is rgba(9,9,9,255)",
    what: "the frame is overwritten with one value after a real render",
    pageFault: true,
  },
  "no-stack": {
    boundary: "presented",
    row: SUBJECT,
    expect: "and this row is",
    what: "the page never puts the row in the viewport, so it would render whatever was there",
    pageFault: true,
  },
  "stack-throws": {
    boundary: "presented",
    row: SUBJECT,
    expect: "setStack: injected",
    what: "setStack rejects, which is where a real stack failure arrives",
    pageFault: true,
  },
  "bad-interpolation": {
    boundary: "presented",
    row: SUBJECT,
    // `constructor` and not a nonsense word, because a plain lookup into
    // cornerstone3D's numeric enum resolves it through Object.prototype and
    // returns something truthy. A guard that checked the lookup rather than
    // the name would accept this.
    expect: "and cornerstone3D names",
    what: "render-params.json asks for an interpolation resolved from Object.prototype",
    mutateParams: (params) => ({ ...params, interpolation: "constructor" }),
  },
  "bad-camera": {
    boundary: "presented",
    row: SUBJECT,
    expect: "the page implements only",
    what: "render-params.json asks for a camera mode the page does not implement",
    mutateParams: (params) => ({ ...params, camera: { mode: "fit-width" } }),
  },
  "wrong-canvas-size": {
    boundary: "read-back",
    row: SUBJECT,
    expect: "is not comparable with one at the declared size",
    what: "the frame is read back against a canvas size nobody declared",
    mutateParams: (params) => ({
      ...params,
      canvas: { ...params.canvas, width: 256, height: 256 },
    }),
  },
  "bad-voi-source": {
    boundary: "internal",
    row: SUBJECT,
    expect: 'The only sources are "file" and "none"',
    what: "an unexpected throw inside the page, which must arrive as a boundary and not as an evaluation error",
    mutateParams: (params) => ({ ...params, voi: { source: "whatever" } }),
  },

  // -------------------------------------------------------------------------
  // The volume pass, F-X007
  //
  // Four more boundaries, and every refusal the volume page or the volume half
  // of the driver makes is aimed at by exactly one of these. They select a
  // whole series directory rather than one row, which the runner needs no
  // change for because `--rows` is already a substring.
  //
  // The hooks are DIFFERENT NAMES from the stack pass's on purpose.
  // `mutateBytes`, `mutateParams` and `pageFault` are read by `renderPass` and
  // would fire during the stack pass over the same ten rows, which would leave
  // the run red at a stack boundary and prove nothing about the guard the fault
  // is aimed at. `mutateMemberBytes`, `mutateVolumeRequest`, `mutateVolumeResult`
  // and `pageVolumeFault` are read only by the volume pass, so the ten stack
  // frames of the selected rows come out normally and the failure is where it
  // is claimed to be.
  // -------------------------------------------------------------------------

  "volume-member-unparseable": {
    boundary: "volume-loaded",
    row: VOLUME_SUBJECT,
    expect: "did not parse",
    what: "one member's bytes are cut to 256, so the volume cannot read its geometry",
    // Only what the VOLUME pass sends. The same row's stack frame is rendered
    // from the untouched corpus bytes and comes out normally.
    mutateMemberBytes: (bytes) => bytes.subarray(0, 256),
  },
  "no-volume-load-event": {
    boundary: "volume-loaded",
    row: VOLUME_SUBJECT,
    expect: "no CORNERSTONE_IMAGE_VOLUME_LOADING_COMPLETED within",
    what: "the page creates the volume and never calls load(), so no frame is ever assembled",
    pageVolumeFault: true,
    // The ONE thing this fault breaks is that `load()` is never called. The
    // timeout is shortened as well, and that is not a second thing broken, it
    // is what makes the guard affordable to watch: `volume-params.json`
    // declares five minutes for a twenty-seven slice real CT series under
    // SwiftShader, and waiting that out on every oracle gate to prove a
    // timeout fires would cost five minutes to learn nothing extra. The guard
    // is "the completion event did not fire within the DECLARED timeout", and
    // it is the same guard at two seconds.
    mutateVolumeRequest: (request) => ({ ...request, loadTimeoutMs: 2_000 }),
  },
  "volume-short-load": {
    boundary: "volume-loaded",
    row: VOLUME_SUBJECT,
    expect: "and the subject declares",
    what: "one member is dropped, so the built volume has fewer slices than the subject has members",
    pageVolumeFault: true,
  },
  "volume-repeat-slice": {
    boundary: "volume-loaded",
    row: VOLUME_SUBJECT,
    // The permutation check specifically, which the slice-count check cannot
    // reach: replacing one member with a copy of another keeps the count.
    expect: "is not a permutation of",
    what: "the built volume's sorted image id list carries one member twice and another not at all",
    pageVolumeFault: true,
  },
  "volume-z-profile": {
    boundary: "volume-loaded",
    row: VOLUME_SUBJECT,
    // "steps by" and not "the z profile", which both branches of that guard
    // say. This injection changes the STEP and leaves the first value alone,
    // so the fragment names the branch it actually reaches. The first-value
    // branch is a pure function and is covered by tests/volume_test.mjs.
    expect: "steps by",
    what: "one slice of the assembled volume carries another slice's value, so the ramp is not a step of 16",
    pageVolumeFault: true,
  },
  "volume-geometry-drift": {
    boundary: "volume-geometry",
    row: VOLUME_SUBJECT,
    // The tolerance that acted, named. 2e-6 is over HLD 25.1's 1e-6 and
    // `tests/volume_test.mjs` shows 5e-7 passing, so this proves 1e-6 decided
    // and not some looser default.
    expect: "volume-truth.json declares",
    what: "one member's projected position is moved 2e-6 mm, which is over HLD 25.1's 1e-6 mm",
    mutateVolumeResult: driftOneProjectedPosition,
  },
  "bad-orientation": {
    boundary: "reformat-presented",
    row: VOLUME_SUBJECT,
    // `constructor` and not a nonsense word, for `bad-interpolation`'s reason:
    // `Enums.OrientationAxis` is a plain object on Object.prototype, so a guard
    // that looked the name up rather than checking a list would accept this.
    expect: "and this page implements",
    what: "volume-params.json asks for an orientation resolved from Object.prototype",
    mutateVolumeRequest: (request) => ({ ...request, orientations: ["constructor"] }),
  },
  "no-reformat-render-event": {
    boundary: "reformat-presented",
    row: VOLUME_SUBJECT,
    expect: "reformat-presented: no CORNERSTONE_IMAGE_RENDERED within",
    what: "the volume viewport is set up and never calls render()",
    pageVolumeFault: true,
  },
  "reformat-stale-frame": {
    boundary: "reformat-read-back",
    row: VOLUME_SUBJECT,
    expect: "is still the sentinel colour",
    what: "the page fires IMAGE_RENDERED without drawing the reformat",
    pageVolumeFault: true,
  },
  "reformat-uniform-canvas": {
    boundary: "reformat-read-back",
    row: VOLUME_SUBJECT,
    expect: "every pixel of the reformat is rgba(9,9,9,255)",
    what: "the reformat is overwritten with one value after a real render",
    pageVolumeFault: true,
  },
  "wrong-reformat-canvas-size": {
    boundary: "reformat-read-back",
    row: VOLUME_SUBJECT,
    expect: "is not comparable with one at the declared size",
    what: "the reformat is read back against a canvas size nobody declared",
    mutateVolumeRequest: (request) => ({
      ...request,
      params: {
        ...request.params,
        canvas: { ...request.params.canvas, width: 256, height: 256 },
      },
    }),
  },
};

/**
 * Move one member 2e-6 mm along the slice normal, in the result the page
 * returned.
 *
 * Node-side and on the RESULT rather than on the bytes, because the geometry
 * the driver measures comes from the attributes `page/app.mjs` read during the
 * stack pass, and perturbing the file itself would break that pass first and
 * fail at a boundary this fault is not aimed at.
 *
 * 2e-6 specifically. HLD 25.1 gives geometry as "world coordinates within 1e-6
 * mm", so this is just over it, and `tests/volume_test.mjs` shows 5e-7 passing.
 * A fault at 1 mm would have gone red against any tolerance at all, including
 * none.
 */
function driftOneProjectedPosition(result) {
  const members = result.members.map((member) => ({ ...member }));
  const target = members[4] ?? members[0];
  const position = target.attributes?.imagePositionPatient;
  if (!Array.isArray(position) || position.length !== 3) {
    throw new Error(
      `the volume-geometry-drift fault needs a member carrying a ` +
        `three-element ImagePositionPatient, and ${target.path} has ` +
        `${JSON.stringify(position)}`,
    );
  }
  // Along the slice normal (-0.6, 0.8, 0), so the whole 2e-6 mm lands on the
  // projection rather than being partly in-plane and invisible to it.
  const normal = [-0.6, 0.8, 0.0];
  target.attributes = {
    ...target.attributes,
    imagePositionPatient: position.map((value, index) => value + 2e-6 * normal[index]),
  };
  return { ...result, members };
}

/** The bytes a fault hands to the page, mutated or not. */
export function faultedBytes(name, bytes) {
  return FAULTS[name]?.mutateBytes?.(bytes) ?? bytes;
}

/** The render parameters a fault hands to the page, mutated or not. */
export function faultedParams(name, params) {
  return FAULTS[name]?.mutateParams?.(params) ?? params;
}

/** The fault name the page is told about, or null. */
export function pageFaultName(name) {
  return FAULTS[name]?.pageFault ? name : null;
}

/** Whether this fault makes the driver skip the row entirely. */
export function skipsRow(name) {
  return FAULTS[name]?.skipRow === true;
}

/** The bytes a fault hands the VOLUME page for one member, mutated or not. */
export function faultedMemberBytes(name, bytes) {
  return FAULTS[name]?.mutateMemberBytes?.(bytes) ?? bytes;
}

/** The request a fault hands the volume page, mutated or not. */
export function faultedVolumeRequest(name, request) {
  return FAULTS[name]?.mutateVolumeRequest?.(request) ?? request;
}

/** The result a fault hands the driver's geometry check, mutated or not. */
export function faultedVolumeResult(name, result) {
  return FAULTS[name]?.mutateVolumeResult?.(result) ?? result;
}

/** The fault name the VOLUME page is told about, or null. */
export function pageVolumeFaultName(name) {
  return FAULTS[name]?.pageVolumeFault ? name : null;
}
