// The two synthetic fixtures of subject viewer.colour_frame, F-239 AC31's
// instrument. One 8-bit RGB frame and one 16-bit PALETTE COLOR frame, each
// 8192 by 8192 native Part 10, which is MAX_PIXELS in the folder viewer's
// preview. They are written per run under the gitignored bench out directory
// and removed again by the runner's finally, so nothing DICOM is ever staged.
//
// Both pixel payloads are position based and deterministic, so the runner can
// validate representative reply bytes against the pattern it wrote instead of
// against a length and a status alone. The expected values are the colour
// stage's output: the stored sample for the RGB file, and for the palette
// file the table entry indexed by the stored value and then shifted right by 8 for the
// 16-bit descriptor, F-239's plan item 11. The request's colour window
// sits on top, which is the identity at the initial 128 and 256 and the
// LINEAR rule of PS3.3 C.11.2.1.2 over 0..=255 for every step.
//
// The writers stream a bounded chunk at a time. The 8-bit RGB pixel value
// alone is 192 MiB, so no code path here ever holds a whole payload.

import { mkdir, open, stat } from "node:fs/promises";
import { Buffer } from "node:buffer";
import { join } from "node:path";

import { longEl, shortEl } from "./viewer_work.mjs";

/** The frame edge both fixtures share, the viewer's largest admitted size. */
export const EDGE = 8192;

/** Pixels per streamed write. 65536 pixels are 192 KiB of RGB or 128 KiB of palette. */
const CHUNK_PIXELS = 65536;

const SC_SOP_CLASS = "1.2.840.10008.5.1.4.1.1.7";
const EXPLICIT_VR_LE = "1.2.840.10008.1.2.1";
const IMPLEMENTATION_UID = "2.25.216";
const PALETTE_ENTRIES = 65536;

/**
 * The two workloads. `replyBytes` is the colour reply's size, three bytes per
 * pixel with no alpha, and `channels` is the X-Ocelli-Channels the reply must
 * carry. Both workloads reply in RGB.
 */
export const WORKLOADS = {
  rgb8: {
    id: "rgb8",
    file: "rgb8-8192.dcm",
    photometric: "RGB",
    samplesPerPixel: 3,
    bitsAllocated: 8,
    bitsStored: 8,
    channels: 3,
  },
  palette16: {
    id: "palette16",
    file: "palette16-8192.dcm",
    photometric: "PALETTE COLOR",
    samplesPerPixel: 1,
    bitsAllocated: 16,
    bitsStored: 16,
    channels: 3,
  },
};

/** The stored RGB triple of the 8-bit workload at one pixel. */
export function rgbStored(x, y) {
  return [x & 255, y & 255, (x + y) & 255];
}

/** The stored palette index of the 16-bit workload at one pixel. */
export function paletteIndex(x, y) {
  return (3 * x + 5 * y) % PALETTE_ENTRIES;
}

/** The three 16-bit table entries at one index, PS3.3 C.7.9's three tables. */
export function paletteEntry(index) {
  return [
    (7 * index) % PALETTE_ENTRIES,
    (13 * index) % PALETTE_ENTRIES,
    (29 * index) % PALETTE_ENTRIES,
  ];
}

/**
 * The colour stage's display-scale output at one pixel, before any window.
 * The 8-bit sample passes as it is, and a palette entry of 16 bits shifts
 * right by 8, the rule of F-239's plan item 11 decided from the header.
 */
export function expectedSamples(workloadId, x, y) {
  if (workloadId === WORKLOADS.rgb8.id) {
    return rgbStored(x, y);
  }
  if (workloadId === WORKLOADS.palette16.id) {
    return paletteEntry(paletteIndex(x, y)).map((entry) => entry >> 8);
  }
  throw new Error(`unknown colour frame workload ${JSON.stringify(workloadId)}`);
}

/**
 * The LINEAR colour window over 0..=255, the rule F-239's plan item 8 takes
 * from cornerstone3D 5.8.2 and PS3.3 C.11.2.1.2, and the byte rule the render
 * boundary applies, which rounds a colour tie to even under ruling R2.
 * Validation only. No measured
 * figure passes through here.
 */
export function windowedByte(value, center, width) {
  const lower = center - 0.5 - (width - 1) / 2;
  const upper = center - 0.5 + (width - 1) / 2;
  if (value <= lower) return 0;
  if (value > upper) return 255;
  const scaled = ((value - (center - 0.5)) / (width - 1) + 0.5) * 255;
  const below = Math.floor(scaled);
  const fraction = scaled - below;
  if (fraction < 0.5) return below;
  if (fraction > 0.5) return below + 1;
  return below % 2 === 0 ? below : below + 1;
}

/** The reply byte at one pixel and channel under one requested window. */
export function expectedReplyByte(workloadId, x, y, channel, window) {
  const raw = expectedSamples(workloadId, x, y)[channel];
  return windowedByte(raw, window.center, window.width);
}

/**
 * Where the runner samples the reply. Spread over the frame, corners and
 * centre included, and small enough that the check cost is nothing against a
 * 67 megapixel reply. Positions at or beyond a smaller edge drop out, which
 * is what the fixture test at edge 4 exercises.
 */
export const CHECK_POSITIONS = [
  [0, 0],
  [1, 1],
  [255, 0],
  [0, 255],
  [8191, 8191],
  [8191, 0],
  [0, 8191],
  [4096, 4096],
  [2731, 5461],
  [5461, 2731],
];

/**
 * One expected reply byte per checked position and channel, as absolute body
 * offsets into the row-major three-byte reply.
 */
export function sampleChecks(workloadId, window, edge = EDGE) {
  const checks = [];
  for (const [x, y] of CHECK_POSITIONS) {
    if (x >= edge || y >= edge) continue;
    const base = (y * edge + x) * 3;
    for (let channel = 0; channel < 3; channel += 1) {
      checks.push({
        offset: base + channel,
        expected: expectedReplyByte(workloadId, x, y, channel, window),
      });
    }
  }
  return checks;
}

/** The colour reply's length, three bytes per pixel and no alpha. */
export function replyBytes(edge = EDGE) {
  return edge * edge * 3;
}

function u16(value) {
  const bytes = Buffer.alloc(2);
  bytes.writeUInt16LE(value);
  return bytes;
}

function u32(value) {
  const bytes = Buffer.alloc(4);
  bytes.writeUInt32LE(value);
  return bytes;
}

function textPad(text) {
  const bytes = Buffer.from(text, "latin1");
  if (bytes.length % 2 === 0) return bytes;
  return Buffer.concat([bytes, Buffer.from(" ")]);
}

function ui(text) {
  const bytes = Buffer.from(text, "latin1");
  if (bytes.length % 2 === 0) return bytes;
  return Buffer.concat([bytes, Buffer.from([0])]);
}

/** The 12-byte header of a long-form element, for a value streamed after it. */
function longHeader(group, element, vr, length) {
  if (length % 2 !== 0) throw new Error("a long element has a bad length");
  return Buffer.concat([
    u16(group),
    u16(element),
    Buffer.from(vr, "ascii"),
    Buffer.from([0, 0]),
    u32(length),
  ]);
}

function preamble() {
  return Buffer.concat([Buffer.alloc(128), Buffer.from("DICM")]);
}

function metaElements(sopUid) {
  return Buffer.concat([
    longEl(0x0002, 0x0001, "OB", Buffer.from([0, 1])),
    shortEl(0x0002, 0x0002, "UI", ui(SC_SOP_CLASS)),
    shortEl(0x0002, 0x0003, "UI", ui(sopUid)),
    shortEl(0x0002, 0x0010, "UI", ui(EXPLICIT_VR_LE)),
    shortEl(0x0002, 0x0012, "UI", ui(IMPLEMENTATION_UID)),
  ]);
}

function paletteTable(stride) {
  const table = Buffer.alloc(PALETTE_ENTRIES * 2);
  for (let index = 0; index < PALETTE_ENTRIES; index += 1) {
    table.writeUInt16LE((stride * index) % PALETTE_ENTRIES, index * 2);
  }
  return table;
}

async function writeAll(handle, buffer) {
  let written = 0;
  while (written < buffer.length) {
    const { bytesWritten } = await handle.write(buffer, written, buffer.length - written);
    if (bytesWritten <= 0) {
      throw new Error("the fixture write made no progress");
    }
    written += bytesWritten;
  }
}

/**
 * Fill one chunk of the 8-bit RGB payload, `count` pixels from `start`, as
 * interleaved R G B, Planar Configuration 0.
 */
function fillRgb(chunk, edge, start, count) {
  let x = start % edge;
  let y = Math.floor(start / edge);
  for (let index = 0; index < count; index += 1) {
    const at = index * 3;
    chunk[at] = x & 255;
    chunk[at + 1] = y & 255;
    chunk[at + 2] = (x + y) & 255;
    x += 1;
    if (x === edge) {
      x = 0;
      y += 1;
    }
  }
}

/**
 * Fill one chunk of the 16-bit palette payload, `count` stored samples from
 * `start`, one little-endian sample per pixel.
 */
function fillPalette(chunk, edge, start, count) {
  let x = start % edge;
  let y = Math.floor(start / edge);
  for (let index = 0; index < count; index += 1) {
    chunk.writeUInt16LE(paletteIndex(x, y), index * 2);
    x += 1;
    if (x === edge) {
      x = 0;
      y += 1;
    }
  }
}

async function streamPixels(handle, edge, bytesPerPixel, fill) {
  const total = edge * edge;
  const chunkPixels = Math.min(CHUNK_PIXELS, total);
  const chunk = Buffer.alloc(chunkPixels * bytesPerPixel);
  let start = 0;
  while (start < total) {
    const count = Math.min(chunkPixels, total - start);
    fill(chunk, edge, start, count);
    await writeAll(handle, chunk.subarray(0, count * bytesPerPixel));
    start += count;
  }
}

async function writeFixture(path, sopUid, dataset, pixelVr, edge, bytesPerPixel, fill) {
  const meta = metaElements(sopUid);
  const pixelLength = edge * edge * bytesPerPixel;
  const handle = await open(path, "w");
  try {
    await writeAll(handle, preamble());
    await writeAll(handle, shortEl(0x0002, 0x0000, "UL", u32(meta.length)));
    await writeAll(handle, meta);
    await writeAll(handle, dataset);
    await writeAll(handle, longHeader(0x7fe0, 0x0010, pixelVr, pixelLength));
    await streamPixels(handle, edge, bytesPerPixel, fill);
  } finally {
    await handle.close();
  }
  const { size } = await stat(path);
  return size;
}

/**
 * Write both fixtures under `dir` and return their specs, with the file size
 * the request's Content-Length names. `edge` exists for the fixture test, so
 * the Part 10 shape and the patterns can be checked on a frame that is four
 * pixels wide. The runner passes no edge and measures 8192.
 */
export async function writeColourFrameFixtures(dir, edge = EDGE) {
  await mkdir(dir, { recursive: true });
  const rgbDataset = Buffer.concat([
    shortEl(0x0008, 0x0016, "UI", ui(SC_SOP_CLASS)),
    shortEl(0x0008, 0x0018, "UI", ui("2.25.2161")),
    shortEl(0x0008, 0x0060, "CS", textPad("OT")),
    shortEl(0x0010, 0x0010, "PN", textPad("SYNTHETIC")),
    shortEl(0x0020, 0x000e, "UI", ui("2.25.21611")),
    shortEl(0x0020, 0x0011, "IS", textPad("1")),
    shortEl(0x0020, 0x0013, "IS", textPad("1")),
    shortEl(0x0028, 0x0002, "US", u16(3)),
    shortEl(0x0028, 0x0004, "CS", textPad("RGB")),
    shortEl(0x0028, 0x0006, "US", u16(0)),
    shortEl(0x0028, 0x0010, "US", u16(edge)),
    shortEl(0x0028, 0x0011, "US", u16(edge)),
    shortEl(0x0028, 0x0100, "US", u16(8)),
    shortEl(0x0028, 0x0101, "US", u16(8)),
    shortEl(0x0028, 0x0102, "US", u16(7)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
  ]);
  const rgbSize = await writeFixture(
    join(dir, WORKLOADS.rgb8.file),
    "2.25.2161",
    rgbDataset,
    "OB",
    edge,
    3,
    fillRgb,
  );

  // PS3.3 C.7.6.3.1.5: a declared entry count of zero means 65536 entries,
  // which is what a full 16-bit table is. Each channel's descriptor is
  // entries 0, first mapped input 0, 16-bit entries.
  const descriptor = Buffer.concat([u16(0), u16(0), u16(16)]);
  const paletteDataset = Buffer.concat([
    shortEl(0x0008, 0x0016, "UI", ui(SC_SOP_CLASS)),
    shortEl(0x0008, 0x0018, "UI", ui("2.25.2162")),
    shortEl(0x0008, 0x0060, "CS", textPad("OT")),
    shortEl(0x0010, 0x0010, "PN", textPad("SYNTHETIC")),
    shortEl(0x0020, 0x000e, "UI", ui("2.25.21612")),
    shortEl(0x0020, 0x0011, "IS", textPad("1")),
    shortEl(0x0020, 0x0013, "IS", textPad("1")),
    shortEl(0x0028, 0x0002, "US", u16(1)),
    shortEl(0x0028, 0x0004, "CS", textPad("PALETTE COLOR")),
    shortEl(0x0028, 0x0010, "US", u16(edge)),
    shortEl(0x0028, 0x0011, "US", u16(edge)),
    shortEl(0x0028, 0x0100, "US", u16(16)),
    shortEl(0x0028, 0x0101, "US", u16(16)),
    shortEl(0x0028, 0x0102, "US", u16(15)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    shortEl(0x0028, 0x1101, "US", descriptor),
    shortEl(0x0028, 0x1102, "US", descriptor),
    shortEl(0x0028, 0x1103, "US", descriptor),
    longEl(0x0028, 0x1201, "OW", paletteTable(7)),
    longEl(0x0028, 0x1202, "OW", paletteTable(13)),
    longEl(0x0028, 0x1203, "OW", paletteTable(29)),
  ]);
  const paletteSize = await writeFixture(
    join(dir, WORKLOADS.palette16.file),
    "2.25.2162",
    paletteDataset,
    "OW",
    edge,
    2,
    fillPalette,
  );

  return {
    rgb8: {
      ...WORKLOADS.rgb8,
      path: join(dir, WORKLOADS.rgb8.file),
      fileBytes: rgbSize,
      edge,
      replyBytes: replyBytes(edge),
    },
    palette16: {
      ...WORKLOADS.palette16,
      path: join(dir, WORKLOADS.palette16.file),
      fileBytes: paletteSize,
      edge,
      replyBytes: replyBytes(edge),
    },
  };
}
