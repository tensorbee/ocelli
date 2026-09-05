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
};

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
