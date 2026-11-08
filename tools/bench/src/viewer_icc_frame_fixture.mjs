// The two ICC workloads of subject viewer.colour_frame, F-240 AC15's additive
// extension of F-239's instrument. F-239's own two fixtures and their module
// are untouched: this file only adds an 8-bit RGB frame carrying the AC2 curv
// gamma 0x0233 profile and a 16-bit RGB frame carrying a derived two-entry
// curv table profile, both 8192 by 8192 native Part 10 like the old pair.
//
// The profile bytes are the project-authored synthetic inputs from frozen ICC
// input tree 81987bd7c13f8e981b62838e18d8afa21b9b7ce4, copied under
// tests/fixtures/f240-icc/ beside a provenance record that names the source
// tree, the SHA-256 of every asset and the tag table of every profile. The
// bytes are verified against that record before use, and no third-party
// profile and no product ICC readback is involved anywhere here.
//
// The 16-bit workload's profile is not stored. It is derived at load time from
// icc_gamma1.icc by rewriting each of the three TRC tags in place: the
// tag-table size field goes from 14 to 16 and the count-1 u8Fixed8 gamma
// payload becomes a count-2 table over [0, 65535]. The TRC slots are 16 bytes
// apart, so alignment is preserved. ICC.1 7.2.18 also requires the
// nonzero profile ID to describe the changed profile. A two-entry
// curv table is the AC4 shape, and it is a supported matrix/TRC profile, not
// the unsupported A2B lookup-table kind.
//
// Validation uses the precomputed hand vectors only at the marker pixels.
// Each workload's sampled positions are deterministic neutral markers whose
// stored value is one of the supplied hand codes, so the runner can check
// exact reply bytes against inputs/independent-hand-vectors.json without a
// second ICC pipeline. Every other pixel is the varied native background, and
// the 16-bit background spans the full encoding range so the 65536-entry TRC
// table is indexed well beyond a few hot entries.

import { createHash } from "node:crypto";
import { mkdir, open, stat } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { Buffer } from "node:buffer";
import { join } from "node:path";

import { benchPath } from "./paths.mjs";
import { longEl, shortEl } from "./viewer_work.mjs";
import { EDGE, replyBytes, windowedByte } from "./viewer_colour_frame_fixture.mjs";

/** Pixels per streamed write. 65536 pixels are 384 KiB of RGB16 or 192 KiB of RGB8. */
const ICC_CHUNK_PIXELS = 65536;

const SC_SOP_CLASS = "1.2.840.10008.5.1.4.1.1.7";
const EXPLICIT_VR_LE = "1.2.840.10008.1.2.1";
const IMPLEMENTATION_UID = "2.25.216";

/** Where the owned generated assets live, under the tests' fixture directory. */
const ASSET_DIR = ["tests", "fixtures", "f240-icc"];

/**
 * The TRC table the viewer builds per parsed profile, one f32 per device code
 * per channel: 3 * 2^bits * 4 bytes, 3072 at 8 bits and 786432 at 16.
 */
function trcTableBytesFor(bitsAllocated) {
  return 3 * 2 ** bitsAllocated * 4;
}

/**
 * The two workloads. `cacheMiB` is the explicit child budget, and it is set on
 * the 16-bit ICC workload alone: its 384 MiB encoded and 384 MiB decoded
 * samples cannot be held under the 256 MiB default, so the lead's amendment
 * authorises 512 MiB for this one workload. Every other workload passes null
 * and inherits whatever the process environment already does.
 */
export const ICC_WORKLOADS = {
  icc8: {
    id: "icc8",
    file: "icc8-8192.dcm",
    photometric: "RGB",
    samplesPerPixel: 3,
    bitsAllocated: 8,
    bitsStored: 8,
    channels: 3,
    icc: true,
    profileKind: "curv gamma 0x0233 (563/256)",
    profileBytes: 560,
    trcTableBytes: trcTableBytesFor(8),
    cacheMiB: null,
  },
  icc16: {
    id: "icc16",
    file: "icc16-8192.dcm",
    photometric: "RGB",
    samplesPerPixel: 3,
    bitsAllocated: 16,
    bitsStored: 16,
    channels: 3,
    icc: true,
    profileKind: "curv table [0, 65535] derived from icc_gamma1.icc",
    profileBytes: 560,
    trcTableBytes: trcTableBytesFor(16),
    cacheMiB: 512,
  },
};

/**
 * The neutral marker positions, in hand-code order: ascending codes map onto
 * this list in order. The 8-bit workload uses the first three, one per
 * supplied hand code, and the 16-bit workload all five, which keeps code 15
 * and the high 16-bit codes covered even at the small test edges.
 */
const ICC_MARKER_POSITIONS = [
  [0, 0],
  [1, 1],
  [2, 1],
  [1, 2],
  [2, 2],
];

let provenanceCache = null;
let gamma22Cache = null;
let gamma1Cache = null;
let linear16Cache = null;

function provenance() {
  if (provenanceCache === null) {
    provenanceCache = JSON.parse(
      readFileSync(benchPath(...ASSET_DIR, "profile-provenance.json"), "utf8"),
    );
  }
  return provenanceCache;
}

/** One asset's bytes, refused unless they match the recorded SHA-256. */
function verifiedAsset(name) {
  const record = provenance().bench_assets[name];
  if (!record || typeof record.sha256 !== "string") {
    throw new Error(`no provenance record for the ${name} asset`);
  }
  const bytes = readFileSync(benchPath(...ASSET_DIR, name));
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (digest !== record.sha256) {
    throw new Error(
      `the ${name} asset is ${digest}, not its recorded ${record.sha256}`,
    );
  }
  return bytes;
}

function gamma22Profile() {
  if (gamma22Cache === null) gamma22Cache = verifiedAsset("icc_gamma22.icc");
  return gamma22Cache;
}

function gamma1Profile() {
  if (gamma1Cache === null) gamma1Cache = verifiedAsset("icc_gamma1.icc");
  return gamma1Cache;
}

/**
 * The 16-bit workload's profile: icc_gamma1.icc with each TRC tag rewritten in
 * place from a count-1 gamma to a count-2 table over [0, 65535], the AC4
 * encoding. The nonzero profile ID is recomputed per ICC.1:2022 7.2.18.
 * All other bytes stay unchanged, including the original colourants.
 */
export function linear16TableProfile() {
  if (linear16Cache === null) {
    const derived = Buffer.from(gamma1Profile());
    const count = derived.readUInt32BE(128);
    const tags = [];
    for (let index = 0; index < count; index += 1) {
      const entry = 132 + 12 * index;
      tags.push({
        signature: derived.toString("latin1", entry, entry + 4),
        entry,
        offset: derived.readUInt32BE(entry + 4),
        size: derived.readUInt32BE(entry + 8),
      });
    }
    let rewritten = 0;
    for (const tag of tags) {
      if (tag.signature !== "rTRC" && tag.signature !== "gTRC" && tag.signature !== "bTRC") {
        continue;
      }
      if (derived.toString("latin1", tag.offset, tag.offset + 4) !== "curv") {
        throw new Error(`${tag.signature} does not hold a curv payload`);
      }
      if (derived.readUInt32BE(tag.offset + 8) !== 1) {
        throw new Error(`${tag.signature} does not hold a count-1 gamma`);
      }
      // The slot must have room for the 16-byte table without moving a
      // neighbour: the next tag starts at or beyond offset + 16.
      const next = tags
        .filter((other) => other.offset > tag.offset)
        .reduce((smallest, other) => Math.min(smallest, other.offset), Infinity);
      if (next < tag.offset + 16) {
        throw new Error(`${tag.signature} has no aligned room for a two-entry table`);
      }
      derived.writeUInt32BE(16, tag.entry + 8);
      derived.writeUInt32BE(2, tag.offset + 8);
      derived.writeUInt16BE(0, tag.offset + 12);
      derived.writeUInt16BE(0xffff, tag.offset + 14);
      rewritten += 1;
    }
    if (rewritten !== 3) {
      throw new Error(`rewrote ${rewritten} TRC tags of icc_gamma1.icc, expected 3`);
    }
    // ICC.1:2022 7.2.18: the nonzero profile ID is MD5 of the full profile
    // with flags, rendering intent and profile ID set to zero for hashing.
    const forId = Buffer.from(derived);
    forId.fill(0, 44, 48);
    forId.fill(0, 64, 68);
    forId.fill(0, 84, 100);
    createHash("md5").update(forId).digest().copy(derived, 84);
    linear16Cache = derived;
  }
  return linear16Cache;
}

const HAND_VECTORS = JSON.parse(
  readFileSync(benchPath(...ASSET_DIR, "hand-vectors.json"), "utf8"),
);
const HAND_ROWS_8 = HAND_VECTORS.rows.filter((row) => row.bits === 8);
const HAND_ROWS_16 = HAND_VECTORS.rows.filter((row) => row.bits === 16);

// The marker scheme covers the 3 supplied 8-bit codes and the 5 supplied
// 16-bit codes. A different row count means the hand data changed shape, and
// the binding has to be redone rather than silently truncated.
if (HAND_ROWS_8.length !== 3 || HAND_ROWS_16.length !== 5) {
  throw new Error(
    `the hand vectors carry ${HAND_ROWS_8.length} 8-bit and ` +
      `${HAND_ROWS_16.length} 16-bit rows, expected 3 and 5`,
  );
}
if (ICC_MARKER_POSITIONS.length !== HAND_ROWS_16.length) {
  throw new Error("the marker positions do not cover the 16-bit hand codes");
}

/** The marker positions of one workload, in ascending hand-code order. */
export function iccMarkers(workloadId) {
  if (workloadId === ICC_WORKLOADS.icc8.id) return ICC_MARKER_POSITIONS.slice(0, HAND_ROWS_8.length);
  if (workloadId === ICC_WORKLOADS.icc16.id) return ICC_MARKER_POSITIONS;
  throw new Error(`unknown ICC frame workload ${JSON.stringify(workloadId)}`);
}

/** The hand codes one workload's markers carry, in marker order. */
export function iccMarkerCodes(workloadId) {
  return (workloadId === ICC_WORKLOADS.icc8.id ? HAND_ROWS_8 : HAND_ROWS_16)
    .map((row) => row.code);
}

/** The neutral code stored at a marker pixel, or null off the markers. */
export function iccMarkerCode(workloadId, x, y) {
  const markers = iccMarkers(workloadId);
  const index = markers.findIndex(([mx, my]) => mx === x && my === y);
  if (index === -1) return null;
  return iccMarkerCodes(workloadId)[index];
}

/**
 * One stored background channel of one ICC workload at one pixel, written
 * without an intermediate array because the real frame is 67 million pixels.
 * The 8-bit channels are rgbStored's, R = x & 255, G = y & 255 and
 * B = (x + y) & 255, and the 16-bit channels are icc16Stored's, full range.
 */
function backgroundChannel(workloadId, x, y, channel) {
  if (workloadId === ICC_WORKLOADS.icc8.id) {
    return channel === 0 ? x & 255 : channel === 1 ? y & 255 : (x + y) & 255;
  }
  return ((channel === 0 ? x : channel === 1 ? y : x + y) * 8) % 65536;
}

/**
 * The stored 16-bit background sample of one pixel, full range: 8192 distinct
 * codes per channel spread across 0 to 65528, so the 65536-entry TRC table is
 * indexed well beyond a few hot entries.
 */
export function icc16Stored(x, y) {
  return [
    backgroundChannel(ICC_WORKLOADS.icc16.id, x, y, 0),
    backgroundChannel(ICC_WORKLOADS.icc16.id, x, y, 1),
    backgroundChannel(ICC_WORKLOADS.icc16.id, x, y, 2),
  ];
}

/** The stored triple of one ICC workload at one pixel. */
export function iccStored(workloadId, x, y) {
  const code = iccMarkerCode(workloadId, x, y);
  if (code !== null) return [code, code, code];
  return [
    backgroundChannel(workloadId, x, y, 0),
    backgroundChannel(workloadId, x, y, 1),
    backgroundChannel(workloadId, x, y, 2),
  ];
}

/** The decoded sample bytes one workload holds at one frame edge. */
export function iccSampleBytes(workloadId, edge = EDGE) {
  const spec = ICC_WORKLOADS[workloadId];
  if (!spec) {
    throw new Error(`unknown ICC frame workload ${JSON.stringify(workloadId)}`);
  }
  return edge * edge * spec.samplesPerPixel * (spec.bitsAllocated / 8);
}

/** The TRC table bytes one workload's parsed profile charges: 3 * 2^bits * 4. */
export function iccTrcTableBytes(workloadId) {
  const spec = ICC_WORKLOADS[workloadId];
  if (!spec) {
    throw new Error(`unknown ICC frame workload ${JSON.stringify(workloadId)}`);
  }
  return spec.trcTableBytes;
}

function handRow(workloadId, code) {
  const bits = ICC_WORKLOADS[workloadId]?.bitsAllocated;
  const row = HAND_VECTORS.rows.find((candidate) => candidate.bits === bits && candidate.code === code);
  if (!row) {
    throw new Error(`no hand vector row for ${workloadId} code ${code}`);
  }
  return row;
}

/** The unrounded ICC display value of one hand code, three channels. */
export function iccExpectedDisplay(workloadId, code) {
  return handRow(workloadId, code).icc_display;
}

/** The windowed reply bytes of one hand code under one window. */
export function iccWindowBytes(workloadId, code, window) {
  const found = handRow(workloadId, code).windows.find(
    (candidate) => candidate.center === window.center && candidate.width === window.width,
  );
  if (!found) {
    throw new Error(
      `no hand vector window ${window.center}/${window.width} for ` +
        `${workloadId} code ${code}`,
    );
  }
  return found.bytes;
}

/**
 * Where the runner samples an ICC reply: one absolute offset per checked byte
 * at the workload's markers, with the hand vectors' expected bytes. Positions
 * at or beyond a smaller edge drop out, which keeps every marker inside the
 * small test edges.
 */
export function iccSampleChecks(workloadId, window, edge = EDGE) {
  const checks = [];
  for (const [x, y] of iccMarkers(workloadId)) {
    if (x >= edge || y >= edge) continue;
    const base = (y * edge + x) * 3;
    const expected = iccWindowBytes(workloadId, iccMarkerCode(workloadId, x, y), window);
    for (let channel = 0; channel < 3; channel += 1) {
      checks.push({ offset: base + channel, expected: expected[channel] });
    }
  }
  return checks;
}

/**
 * The IEC 61966-2-1 encoding the F-240 plan transcribes. It exists here only
 * to model the wrongly indexed 16-bit reply the negative control rejects: the
 * correct marker bytes always come from the hand vectors, never from this
 * file, so this is not a second ICC pipeline.
 */
function srgbEncode(linear) {
  return linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
}

/**
 * One reply byte for the small fake viewer, by variant. `icc` is the hand
 * vector's byte. `unprofiled` is what the frame would reply with the profile
 * ignored, the device byte windowed. `indexed8` is the ICC reply computed as
 * if the 16-bit code had been indexed into the TRC by its high byte only, the
 * wrong implementation the negative control must catch. Non-marker pixels
 * reply 0, because the runner samples the markers only.
 */
export function iccReplyByte(workloadId, x, y, channel, window, variant = "icc") {
  const code = iccMarkerCode(workloadId, x, y);
  if (code === null) return 0;
  if (variant === "icc") {
    return iccWindowBytes(workloadId, code, window)[channel];
  }
  if (variant === "unprofiled") {
    const device = workloadId === ICC_WORKLOADS.icc8.id ? code : code >> 8;
    return windowedByte(device, window.center, window.width);
  }
  if (variant === "indexed8") {
    if (workloadId !== ICC_WORKLOADS.icc16.id) {
      throw new Error("the 8-bit workload has no 16-bit indexing to get wrong");
    }
    return windowedByte(255 * srgbEncode((code >> 8) / 255), window.center, window.width);
  }
  throw new Error(`unknown marker byte variant ${JSON.stringify(variant)}`);
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

async function writeAll(handle, buffer) {
  let written = 0;
  while (written < buffer.length) {
    const { bytesWritten } = await handle.write(buffer, written, buffer.length - written);
    if (bytesWritten <= 0) {
      throw new Error("the ICC fixture write made no progress");
    }
    written += bytesWritten;
  }
}

/**
 * The background pixels of one chunk, `count` pixels from `start`, with the
 * workload's neutral markers written in as part of the same pass. The markers
 * in this chunk are collected once, so the loop per pixel is three channel
 * writes and one map lookup, and the streamed payload stays bounded whatever
 * the edge is.
 */
function fillBackground(chunk, workloadId, edge, start, count, bytesPerSample) {
  const markers = new Map();
  for (const [x, y] of iccMarkers(workloadId)) {
    const pixel = y * edge + x;
    if (pixel >= start && pixel < start + count) {
      markers.set(pixel - start, iccMarkerCode(workloadId, x, y));
    }
  }
  let x = start % edge;
  let y = Math.floor(start / edge);
  for (let index = 0; index < count; index += 1) {
    const markerCode = markers.get(index);
    for (let channel = 0; channel < 3; channel += 1) {
      const at = index * 3 * bytesPerSample + channel * bytesPerSample;
      const value = markerCode === undefined
        ? backgroundChannel(workloadId, x, y, channel)
        : markerCode;
      if (bytesPerSample === 1) chunk[at] = value;
      else chunk.writeUInt16LE(value, at);
    }
    x += 1;
    if (x === edge) {
      x = 0;
      y += 1;
    }
  }
}

async function streamIccPixels(handle, edge, bytesPerSample, workloadId) {
  const total = edge * edge;
  const chunkPixels = Math.min(ICC_CHUNK_PIXELS, total);
  const chunk = Buffer.alloc(chunkPixels * 3 * bytesPerSample);
  let start = 0;
  while (start < total) {
    const count = Math.min(chunkPixels, total - start);
    fillBackground(chunk, workloadId, edge, start, count, bytesPerSample);
    await writeAll(handle, chunk.subarray(0, count * 3 * bytesPerSample));
    start += count;
  }
}

async function writeIccFixture(path, sopUid, dataset, pixelVr, edge, bytesPerSample, workloadId) {
  const meta = metaElements(sopUid);
  const bytesPerPixel = 3 * bytesPerSample;
  const pixelLength = edge * edge * bytesPerPixel;
  const handle = await open(path, "w");
  try {
    await writeAll(handle, preamble());
    await writeAll(handle, shortEl(0x0002, 0x0000, "UL", u32(meta.length)));
    await writeAll(handle, meta);
    await writeAll(handle, dataset);
    await writeAll(handle, longHeader(0x7fe0, 0x0010, pixelVr, pixelLength));
    await streamIccPixels(handle, edge, bytesPerSample, workloadId);
  } finally {
    await handle.close();
  }
  const { size } = await stat(path);
  return size;
}

function iccDataset(sopUid, seriesUid, edge, bitsAllocated, profile) {
  return Buffer.concat([
    shortEl(0x0008, 0x0016, "UI", ui(SC_SOP_CLASS)),
    shortEl(0x0008, 0x0018, "UI", ui(sopUid)),
    shortEl(0x0008, 0x0060, "CS", textPad("OT")),
    shortEl(0x0010, 0x0010, "PN", textPad("SYNTHETIC")),
    shortEl(0x0020, 0x000e, "UI", ui(seriesUid)),
    shortEl(0x0020, 0x0011, "IS", textPad("1")),
    shortEl(0x0020, 0x0013, "IS", textPad("1")),
    shortEl(0x0028, 0x0002, "US", u16(3)),
    shortEl(0x0028, 0x0004, "CS", textPad("RGB")),
    shortEl(0x0028, 0x0006, "US", u16(0)),
    shortEl(0x0028, 0x0010, "US", u16(edge)),
    shortEl(0x0028, 0x0011, "US", u16(edge)),
    shortEl(0x0028, 0x0100, "US", u16(bitsAllocated)),
    shortEl(0x0028, 0x0101, "US", u16(bitsAllocated)),
    shortEl(0x0028, 0x0102, "US", u16(bitsAllocated - 1)),
    shortEl(0x0028, 0x0103, "US", u16(0)),
    longEl(0x0028, 0x2000, "OB", profile),
  ]);
}

/**
 * Write both ICC fixtures under `dir` and return their specs, with the file
 * size the request's Content-Length names and the marker positions and hand
 * codes the runner validates against. `edge` exists for the fixture tests, so
 * the Part 10 shape, the markers and the derived profile can be checked on a
 * frame that is four pixels wide. The runner passes no edge and measures 8192.
 */
export async function writeIccFrameFixtures(dir, edge = EDGE) {
  await mkdir(dir, { recursive: true });
  const icc8Sop = "2.25.2163";
  const icc16Sop = "2.25.2164";
  const icc8Size = await writeIccFixture(
    join(dir, ICC_WORKLOADS.icc8.file),
    icc8Sop,
    iccDataset(icc8Sop, "2.25.21613", edge, 8, gamma22Profile()),
    "OB",
    edge,
    1,
    ICC_WORKLOADS.icc8.id,
  );
  const icc16Size = await writeIccFixture(
    join(dir, ICC_WORKLOADS.icc16.file),
    icc16Sop,
    iccDataset(icc16Sop, "2.25.21614", edge, 16, linear16TableProfile()),
    "OW",
    edge,
    2,
    ICC_WORKLOADS.icc16.id,
  );
  const spec = (workloadId, fileBytes) => ({
    ...ICC_WORKLOADS[workloadId],
    markers: iccMarkers(workloadId),
    codes: iccMarkerCodes(workloadId),
    path: join(dir, ICC_WORKLOADS[workloadId].file),
    fileBytes,
    edge,
    replyBytes: replyBytes(edge),
  });
  return {
    icc8: spec(ICC_WORKLOADS.icc8.id, icc8Size),
    icc16: spec(ICC_WORKLOADS.icc16.id, icc16Size),
  };
}
