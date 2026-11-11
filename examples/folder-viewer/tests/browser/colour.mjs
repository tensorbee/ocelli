// F-239 AC16, AC19, AC21, AC22 and AC26. Synthetic bytes only.
// PS3.3 2026d C.7.6.3.1.2 defines YBR_FULL's forward matrix and the
// Y1,Y2,Cb,Cr 4:2:2 wire order. C.7.6.3.1.3 defines RGB plane order.
// C.7.6.3.1.5 defines palette input clamping and descriptor depth.
// https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.3.html
// Windows use the approved cornerstone parity policy, reusing the formula
// of C.11.2.1.2.1 per channel. C.11.2.1.2.2 itself limits file VOI to mono.
// https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html
// Literal bytes below are the approved plan's hand values, not viewer output.
// Palette fallback and 12-bit descriptors are explicitly R1 interoperability
// behavior. Current PS3.3 requires 8/16 and equal RGB descriptor depths.
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { ensureViewerBinary, longEl, shortEl, startViewer } from "../../../../tools/bench/src/viewer_work.mjs";
import { loadPlaywright, stopByPid } from "./aspect.mjs";

function iccSentence(spec) {
  return (spec.icc || spec.profile) && !spec.iccApplied
    ? "This image's " + (spec.iccKind || "malformed") + " ICC profile was not applied." : "";
}
const profile = (kind) => readFileSync(new URL(`../fixtures/icc_${kind}.icc`, import.meta.url));
const COLOUR_NOTE = "This image is colour, so the file's windows and the presets do not apply. Drag to window each channel.";
const RGB = [10, 20, 30, 200, 100, 50];
export const COLOUR_CASES = [
  { name: "rgb-interleaved", columns: 2, samples: RGB, rgb: RGB },
  { name: "rgb-planar", columns: 2, planar: 1, samples: [10, 200, 20, 100, 30, 50], rgb: RGB },
  { name: "ybr-full", columns: 1, photo: "YBR_FULL", samples: [100, 150, 90], rgb: [47, 120, 139] },
  { name: "ybr-422", columns: 2, photo: "YBR_FULL_422", samples: [50, 200, 100, 160], rgb: [95, 37, 0, 245, 187, 150] },
  { name: "palette-offset", columns: 5, samples: [9, 10, 12, 13, 99], palette: { first: 10, bits: 16, channels: [[0, 85, 170, 255], [255, 170, 85, 0], [10, 20, 30, 40]] }, rgb: [0, 255, 10, 0, 255, 10, 170, 85, 30, 255, 0, 40, 255, 0, 40] },
  { name: "palette-16", columns: 5, samples: [0, 1, 2, 3, 4], palette: { bits: 16, channels: [[0, 255, 256, 32768, 65535], [65535, 32768, 256, 255, 0], [65535, 32768, 256, 255, 0]] }, rgb: [0, 255, 255, 0, 128, 128, 1, 1, 1, 128, 0, 0, 255, 0, 0] },
  ...[16, 12].map((bits) => ({ name: `palette-fallback-${bits}`, columns: 3, samples: [0, 1, 2], palette: { bits, channels: [[0, 100, 255], [0, 100, 255], [0, 100, 255]] }, rgb: [0, 0, 0, 100, 100, 100, 255, 255, 255] })),
  { name: "palette-wide-blue", columns: 3, samples: [0, 1, 2], palette: { bits: 16, channels: [[0, 100, 255], [0, 100, 255], [0, 100, 256]] }, rgb: [0, 0, 0, 0, 0, 0, 0, 0, 1] },
  { name: "rgb-12", columns: 1, bits: 12, allocated: 16, samples: [4095, 2048, 15], rgb: [255, 128, 0] },
  { name: "rgb-icc", columns: 2, samples: RGB, rgb: RGB, icc: true },
  { name: "icc-gamma1", columns: 4, samples: [64,64,64,128,128,128,192,192,192,255,255,255], rgb: [137,137,137,188,188,188,225,225,225,255,255,255], profile: profile("gamma1"), iccApplied: true, windows: [{ center: 128, width: 10, rgb: [255,255,255,255,255,255,255,255,255,255,255,255], inverse: [0,0,0,0,0,0,0,0,0,0,0,0] }] },
  { name: "icc-gamma22", columns: 4, samples: [64,64,64,128,128,128,192,192,192,255,255,255], rgb: [62,62,62,129,129,129,193,193,193,255,255,255], profile: profile("gamma22"), iccApplied: true },
  { name: "icc-srgb-para", columns: 4, samples: [64,64,64,128,128,128,192,192,192,255,255,255], rgb: [64,64,64,128,128,128,192,192,192,255,255,255], profile: profile("srgb_para"), iccApplied: true },
  { name: "icc-swap", columns: 2, samples: [255,0,0,0,255,0], rgb: [0,255,0,255,0,0], profile: profile("swap"), iccApplied: true },
  { name: "icc-lut", columns: 2, samples: RGB, rgb: RGB, profile: profile("lut"), iccKind: "lookup-table" },
  { name: "icc-12", columns: 3, bits: 12, allocated: 16, samples: [4095,4095,4095,2048,2048,2048,0,0,0], rgb: [255,255,255,188,188,188,0,0,0], profile: profile("gamma1"), iccApplied: true },
  { name: "window-100", columns: 1, samples: [100, 30, 200], rgb: [100, 30, 200], windows: [{ center: 128, width: 100, rgb: [57, 0, 255], inverse: [198, 255, 0] }] },
  { name: "window-10", columns: 1, samples: [125, 128, 130], rgb: [125, 128, 130], windows: [{ center: 128, width: 10, rgb: [57, 142, 198], inverse: [198, 113, 57] }] },
  { name: "window-mid", columns: 1, samples: [128, 128, 128], rgb: [128, 128, 128], windows: [{ center: 128, width: 100, rgb: [129, 129, 129], inverse: [126, 126, 126] }] },
  { name: "window-identity", columns: 1, samples: [100, 100, 100], rgb: [100, 100, 100], windows: [{ center: 127, width: 255, rgb: [100, 100, 100] }] },
  { name: "window-threshold", columns: 1, samples: [127, 128, 0], rgb: [127, 128, 0], windows: [{ center: 128, width: 2, rgb: [0, 255, 0] }] },
  { name: "window-tie", columns: 1, samples: [130, 130, 130], rgb: [130, 130, 130], windows: [{ center: 128.5, width: 511, rgb: [128, 128, 128], inverse: [126, 126, 126] }, { center: 130.5, width: 100, rgb: [128, 128, 128] }] },
];
const CT = { name: "ct-control", columns: 2, mono: true, allocated: 16, bits: 16, samples: [224, 1064], rgb: [0, 0, 0, 128, 128, 128] };

function u16(value) { const out = Buffer.alloc(2); out.writeUInt16LE(value); return out; }
function u32(value) { const out = Buffer.alloc(4); out.writeUInt32LE(value); return out; }
function words(values) { return Buffer.concat(values.map(u16)); }
function padded(value, fill = 32) { const out = Buffer.from(value, "ascii"); return out.length % 2 ? Buffer.concat([out, Buffer.from([fill])]) : out; }
const text = (value) => padded(String(value));
const uid = (value) => padded(value, 0);

// PS3.10 file meta is Explicit VR LE, as is the native dataset. No patient
// attributes are generated. Each case has distinct SOP/Series identities.
export function colourFile(spec, index) {
  const sopClass = spec.mono ? "1.2.840.10008.5.1.4.1.1.2"
    : spec.frames ? "1.2.840.10008.5.1.4.1.1.7.4" : "1.2.840.10008.5.1.4.1.1.7";
  const sop = `2.25.239000${index}`;
  const meta = Buffer.concat([
    longEl(2, 1, "OB", Buffer.from([0, 1])), shortEl(2, 2, "UI", uid(sopClass)),
    shortEl(2, 3, "UI", uid(sop)), shortEl(2, 0x10, "UI", uid("1.2.840.10008.1.2.1")),
    shortEl(2, 0x12, "UI", uid("2.25.239")),
  ]);
  const palette = spec.palette;
  const samples = spec.mono || palette ? 1 : 3;
  const allocated = spec.allocated || 8;
  const bits = spec.bits || allocated;
  let pixels = allocated === 16 ? words(spec.samples) : Buffer.from(spec.samples);
  if (pixels.length % 2) pixels = Buffer.concat([pixels, Buffer.from([0])]);
  const elements = [
    shortEl(8, 0x16, "UI", uid(sopClass)), shortEl(8, 0x18, "UI", uid(sop)),
    shortEl(8, 0x60, "CS", text(spec.mono ? "CT" : "OT")), shortEl(8, 0x103e, "LO", text(spec.name)),
    shortEl(0x20, 0x0d, "UI", uid("2.25.239001")), shortEl(0x20, 0x0e, "UI", uid(spec.seriesUid || `2.25.239100${index}`)),
    shortEl(0x20, 0x11, "IS", text(index)), shortEl(0x20, 0x13, "IS", text(spec.instanceNumber || 1)),
    shortEl(0x20, 0x32, "DS", text("0\\0\\0")), shortEl(0x20, 0x37, "DS", text("1\\0\\0\\0\\1\\0")),
    shortEl(0x20, 0x52, "UI", uid("2.25.239002")),
    shortEl(0x28, 2, "US", u16(samples)), shortEl(0x28, 4, "CS", text(spec.mono ? "MONOCHROME2" : palette ? "PALETTE COLOR" : spec.photo || "RGB")),
    ...(samples === 3 ? [shortEl(0x28, 6, "US", u16(spec.planar || 0))] : []),
    ...(spec.frames ? [shortEl(0x28, 8, "IS", text(spec.frames))] : []),
    shortEl(0x28, 0x10, "US", u16(spec.rows || 1)), shortEl(0x28, 0x11, "US", u16(spec.columns)),
    shortEl(0x28, 0x30, "DS", text("1\\1")),
    shortEl(0x28, 0x100, "US", u16(allocated)), shortEl(0x28, 0x101, "US", u16(bits)),
    shortEl(0x28, 0x102, "US", u16(bits - 1)), shortEl(0x28, 0x103, "US", u16(0)),
    // Deliberately nonsensical colour VOI must be ignored at initial display.
    shortEl(0x28, 0x1050, "DS", text(spec.mono ? "40" : "500\\1000")),
    shortEl(0x28, 0x1051, "DS", text(spec.mono ? "400" : "10\\100")),
    ...(spec.mono ? [shortEl(0x28, 0x1052, "DS", text(-1024)), shortEl(0x28, 0x1053, "DS", text(1))] : []),
    shortEl(0x28, 0x1056, "CS", text(spec.mono ? "LINEAR" : "SIGMOID")),
  ];
  if (palette) {
    palette.channels.forEach((channel, at) => elements.push(shortEl(0x28, 0x1101 + at, "US", words([channel.length, palette.first || 0, palette.bits]))));
    palette.channels.forEach((channel, at) => elements.push(longEl(0x28, 0x1201 + at, "OW", words(channel))));
  }
  // Presence alone is F239's contract. This synthetic marker is no profile.
  if (spec.icc || spec.profile) elements.push(longEl(0x28, 0x2000, "OB", spec.profile || Buffer.from("TEST")));
  elements.push(longEl(0x7fe0, 0x10, allocated === 16 ? "OW" : "OB", pixels));
  return Buffer.concat([Buffer.alloc(128), Buffer.from("DICM"), shortEl(2, 0, "UL", u32(meta.length)), meta, ...elements]);
}

export function makeColourFixtures(scratch) {
  const fixtures = [...COLOUR_CASES, CT, { ...CT, name: "ct-control-2" }].map((spec, index) => {
    const directory = join(scratch, spec.name);
    mkdirSync(directory);
    const path = join(directory, `${spec.name}.dcm`);
    writeFileSync(path, colourFile(spec, index + 1));
    return { ...spec, path, directory };
  });
  const mixed = join(scratch, "mixed");
  mkdirSync(mixed);
  for (const name of ["rgb-interleaved", "rgb-planar", "ybr-422", "palette-offset", "rgb-icc", "icc-gamma1", "icc-lut", CT.name, "ct-control-2"]) {
    const fixture = fixtures.find((entry) => entry.name === name);
    copyFileSync(fixture.path, join(mixed, `${name}.dcm`));
  }
  // Reuse independently specified native RGB bytes. Distinct frames make a
  // frame-zero-only canvas fail, including in Four images.
  const frameRgbs = [RGB, [200, 100, 50, 10, 20, 30], [100, 30, 200, 125, 128, 130]];
  const multiframe = { name: "rgb-cine", columns: 2, frames: 3, samples: frameRgbs.flat(), rgb: frameRgbs[0], frameRgbs };
  multiframe.directory = join(scratch, multiframe.name);
  mkdirSync(multiframe.directory);
  writeFileSync(join(multiframe.directory, `${multiframe.name}.dcm`), colourFile(multiframe, 90));
  // One unusual series with mixed image kinds exercises the consumer's
  // per-image window units. It intentionally shares a series identity.
  const mixedSeries = { directory: join(scratch, "mixed-series"), specs: [] };
  mkdirSync(mixedSeries.directory);
  [CT, COLOUR_CASES[0], CT, COLOUR_CASES[1]].forEach((base, index) => {
    const spec = { ...base, name: `mixed-${index + 1}-${base.mono ? "ct" : "rgb"}`, seriesUid: "2.25.239999", instanceNumber: index + 1 };
    writeFileSync(join(mixedSeries.directory, `${spec.name}.dcm`), colourFile(spec, 91 + index));
    mixedSeries.specs.push(spec);
  });
  // A two-row image permits a normal rectangle above the existing one-pixel
  // minimum height, enclosing only the two centres in row zero.
  const region = { name: "rgb-region", rows: 2, columns: 2, samples: [...RGB, ...RGB], rgb: [...RGB, ...RGB] };
  region.directory = join(scratch, region.name);
  mkdirSync(region.directory);
  writeFileSync(join(region.directory, `${region.name}.dcm`), colourFile(region, 95));
  return { fixtures, mixed, multiframe, mixedSeries, region };
}

function instrument() {
  window.__colour = { requests: [], serial: 0, paints: [] };
  const fetchOriginal = window.fetch.bind(window);
  window.fetch = async function (input, init) {
    const path = new URL(typeof input === "string" ? input : input.url, location.href).pathname;
    if (path !== "/preview") return fetchOriginal(input, init);
    const headers = new Headers(init?.headers);
    const request = { sequence: window.__colour.requests.length, generation: currentGeneration, id: headers.get("X-Ocelli-File"), frame: headers.get("X-Ocelli-Frame") || "0", center: headers.get("X-Ocelli-Window-Center"), width: headers.get("X-Ocelli-Window-Width"), preset: headers.get("X-Ocelli-Window-Preset"), invert: headers.get("X-Ocelli-Invert") || "0" };
    window.__colour.requests.push(request);
    const response = await fetchOriginal(input, init);
    request.status = response.status;
    request.channels = response.headers.get("X-Ocelli-Channels");
    const arrayBuffer = response.arrayBuffer.bind(response);
    response.arrayBuffer = async () => {
      const result = await arrayBuffer();
      request.bytes = Array.from(new Uint8Array(result));
      request.paintFloor = window.__colour.serial;
      return result;
    };
    return response;
  };
  const put = CanvasRenderingContext2D.prototype.putImageData;
  CanvasRenderingContext2D.prototype.putImageData = function (...args) {
    const result = put.apply(this, args);
    if (this.canvas.matches(".cell canvas.view")) window.__colour.paints.push({ serial: ++window.__colour.serial, cell: Number(this.canvas.closest(".cell").dataset.cell) });
    return result;
  };
}

function rgba(rgb) { return rgb.flatMap((value, at) => at % 3 === 2 ? [value, 255] : [value]); }
async function checkpoint(page) { return page.evaluate(() => window.__colour.requests.length); }
async function canvasPixels(page, cell = 0) {
  return page.locator(`.cell[data-cell="${cell}"] canvas.view`).evaluate((canvas) => ({ width: canvas.width, height: canvas.height, rgba: Array.from(canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data) }));
}

// A new image identity or new generation alone is insufficient. Require an
// exact successful response consumed by page code, then a later active-canvas
// paint and exact backing RGBA. Corrupt bytes cannot be hidden by old pixels.
export async function waitColourPaint(page, spec, after, window = {}, cell = 0) {
  const wanted = { after, name: `${spec.name}.dcm`, center: window.center == null ? null : String(window.center), width: window.width == null ? null : String(window.width), invert: window.invert ? "1" : "0", preset: window.preset || null, frame: String(spec.frame || 0), cell };
  await page.waitForFunction((want) => {
    const vp = viewports[want.cell];
    const image = currentImage(vp);
    if (!image || image.file.name !== want.name || String(image.frame) !== want.frame) return false;
    const id = filePosts.get(image.file);
    return window.__colour.requests.some((request) => request.sequence >= want.after && request.generation === currentGeneration && request.id === id && request.frame === want.frame && request.center === want.center && request.width === want.width && request.invert === want.invert && request.preset === want.preset && request.bytes) || Boolean(vp.dom.fault && !vp.dom.fault.hidden);
  }, wanted);
  const response = await page.evaluate((want) => {
    const image = currentImage(viewports[want.cell]);
    const id = filePosts.get(image.file);
    return window.__colour.requests.find((request) => request.sequence >= want.after && request.generation === currentGeneration && request.id === id && request.frame === want.frame && request.center === want.center && request.width === want.width && request.invert === want.invert && request.preset === want.preset && request.bytes) || null;
  }, wanted);
  assert(response, `${spec.name}: no successful consumed preview for selected source`);
  assert.equal(response.status, 200, `${spec.name}: preview status`);
  assert.equal(response.channels, spec.mono ? "1" : "3", `${spec.name}: channel count`);
  const expected = window.rgb || spec.rgb;
  assert.deepEqual(response.bytes, spec.mono ? expected.filter((_, at) => at % 3 === 0) : expected, `${spec.name}: consumed preview channel bytes`);
  await page.waitForFunction(({ floor, cell: target }) => window.__colour.paints.some((paint) => paint.cell === target && paint.serial > floor), { floor: response.paintFloor, cell });
  assert.deepEqual(await canvasPixels(page, cell), { width: spec.columns, height: spec.rows || 1, rgba: rgba(expected) }, `${spec.name}: new decoded canvas`);
}

async function openFixture(page, spec) {
  const after = await checkpoint(page);
  await page.locator("#folder").setInputFiles(spec.directory);
  await waitColourPaint(page, spec, after);
  await page.waitForFunction(() => activeViewport().scale > 0);
}
async function moveImage(page, from, to) {
  const [a, b] = await page.evaluate(([start, end]) => [start, end].map(([x, y]) => imageToClient(activeViewport(), { x, y })), [from, to]);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 8 });
  await page.mouse.up();
}
async function setWindow(page, spec, window) {
  const after = await checkpoint(page);
  await page.evaluate(async ({ center, width }) => {
    dropCache();
    await chooseWindow(activeViewport(), { override: { center, width, fn: "LINEAR" } });
  }, window);
  await waitColourPaint(page, spec, after, window);
}

async function thumbnail(page, spec) {
  // With square spacing the image occupies the whole 128-pixel width.
  // Sample the independently placed centre of every source column, so a
  // black placeholder cannot satisfy palettes whose first entry is black.
  const samples = Array.from({ length: spec.columns }, (_, column) => ({
    x: Math.floor((column + 0.5) * 128 / spec.columns),
    rgba: spec.rgb.slice(column * 3, column * 3 + 3).concat(255),
  }));
  await page.waitForFunction((wanted) => {
    const canvas = document.querySelector("#series-list canvas.thumb");
    if (!canvas) return false;
    return wanted.every((sample) => {
      const value = canvas.getContext("2d").getImageData(sample.x, 64, 1, 1).data;
      return sample.rgba.every((channel, at) => channel === value[at]);
    });
  }, samples);
}
async function readout(page, x, expected) {
  const point = await page.evaluate((column) => imageToClient(activeViewport(), { x: column + 0.5, y: 0.5 }), x);
  await page.mouse.move(point.x, point.y);
  await page.mouse.move(point.x + 0.1, point.y);
  await page.waitForFunction((text) => !document.getElementById("cross-read").hidden && document.getElementById("cross-read").textContent.includes(text), expected);
}
async function colourRegion(page, spec) {
  await page.locator("#tool-rect").click();
  const response = page.waitForResponse((entry) => new URL(entry.url()).pathname === "/measure" && entry.ok());
  await moveImage(page, [0.05, 0.05], [1.95, 1.2]);
  const result = await (await response).json();
  assert.equal(result.stats.unit, "RGB");
  const channels = Object.fromEntries(Object.entries(result.stats.channels).map(([name, stats]) => [name, Object.fromEntries(["count", "mean", "sd", "min", "max"].map((key) => [key, stats[key]]))]));
  assert.equal(result.stats.padding_excluded, 0);
  assert.deepEqual(channels, {
    r: { count: 2, mean: 105, sd: 95, min: 10, max: 200 },
    g: { count: 2, mean: 60, sd: 40, min: 20, max: 100 },
    b: { count: 2, mean: 40, sd: 10, min: 30, max: 50 },
  }, "AC13 unwindowed population statistics per channel");
  await page.waitForFunction(() => [...notes.values()].flat().some((note) => note.result?.stats?.unit === "RGB"));
  const handText = "R mean 105.0, SD 95.0, min 10, max 200, G mean 60.0, SD 40.0, min 20, max 100, B mean 40.0, SD 10.0, min 30, max 50";
  assert((await page.locator("#marks").textContent()).includes(handText), "visible per-channel region statistics");
  await setWindow(page, spec, { center: 128, width: 100, rgb: [0, 0, 0, 255, 57, 0, 0, 0, 0, 255, 57, 0] });
  assert.deepEqual(await page.evaluate(() => [...notes.values()].flat().find((note) => note.result?.stats)?.result.stats.channels), result.stats.channels, "window leaves region statistics unchanged");
  await readout(page, 1, "R 200.0 G 100.0 B 50.0");
}

// Link tests use real controls and actual drags. Copying a window is a
// transport invariant, so each recipient must draw the same specified window
// while the other kind's literal pixels and state remain untouched.
async function expectCell(page, spec, cell, rgb = spec.rgb) {
  await page.waitForFunction(({ expected, cell: target }) => {
    const canvas = viewports[target].dom.canvas;
    const bytes = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    return bytes.length === expected.length && expected.every((value, at) => value === bytes[at]);
  }, { expected: rgba(rgb), cell });
  assert.deepEqual(await canvasPixels(page, cell), { width: spec.columns, height: spec.rows || 1, rgba: rgba(rgb) });
}
async function dragWidth(page, cell, pixels) {
  await page.locator(`.cell[data-cell="${cell}"]`).click();
  await page.locator("#tool-wl").click();
  const box = await page.locator(`.cell[data-cell="${cell}"] canvas.view`).boundingBox();
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + pixels, y, { steps: 8 });
  await page.mouse.up();
}
async function linkedWindows(page, data) {
  await page.reload();
  const rgb = data.fixtures.find((entry) => entry.name === "rgb-interleaved");
  const rgb2 = data.fixtures.find((entry) => entry.name === "rgb-planar");
  const ct = data.fixtures.find((entry) => entry.name === "ct-control");
  const ct2 = data.fixtures.find((entry) => entry.name === "ct-control-2");
  const first = await checkpoint(page);
  await page.locator("#folder").setInputFiles(data.mixed);
  await waitColourPaint(page, rgb, first);
  await page.locator("#layout").click();
  await page.locator('#layout-picker [data-columns="2"][data-rows="2"]').click();
  for (const [cell, spec] of [[1, rgb2], [2, ct], [3, ct2]]) {
    const before = await checkpoint(page);
    await page.evaluate(() => dropCache());
    // A layout can already have assigned this series. Dropping it again
    // deliberately keeps its view, so click its row in the chosen cell to
    // request a fresh initial preview rather than accepting retained pixels.
    await page.locator(`.cell[data-cell="${cell}"]`).click();
    await page.locator("#series-list button").filter({ has: page.getByText(spec.name, { exact: true }) }).click();
    await waitColourPaint(page, spec, before, {}, cell);
  }
  await page.locator("#link").click();
  await page.locator("#link-window").click();
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#link-window").getAttribute("aria-checked"), "true", "AC28 window linking really enabled");
  const beforeRgb = await checkpoint(page);
  await dragWidth(page, 0, -121.875); // 256 + (-121.875 * 256/200) = 100.
  const rgbWindow = { center: 128, width: 100, rgb: [0, 0, 0, 255, 57, 0] };
  await waitColourPaint(page, rgb, beforeRgb, rgbWindow, 0);
  await waitColourPaint(page, rgb2, beforeRgb, rgbWindow, 1);
  await expectCell(page, ct, 2);
  await expectCell(page, ct2, 3);
  assert.deepEqual(await page.evaluate(() => [2, 3].map((cell) => viewports[cell].overrideWindow)), [null, null], "colour window never changes either CT state");
  await page.locator("#reset").click();
  await expectCell(page, rgb, 0);
  await expectCell(page, rgb2, 1);
  assert.deepEqual(await page.evaluate(() => [0, 1].map((cell) => viewports[cell].overrideWindow)), [null, null], "colour reset propagates defaults");
  const beforeCT = await checkpoint(page);
  await dragWidth(page, 2, -150); // 400 + (-150 * 400/200) = 100, centre remains 40.
  // PS3.3 C.11.2.1.2.1 LINEAR: modality 40 gives
  // ((40 - (40 - .5)) / (100 - 1) + .5) * 255 = 128.7878 -> 129.
  // Modality -800 is below the window, so it remains 0.
  const ctWindow = { center: 40, width: 100, rgb: [0, 0, 0, 129, 129, 129] };
  await waitColourPaint(page, ct, beforeCT, ctWindow, 2);
  await waitColourPaint(page, ct2, beforeCT, ctWindow, 3);
  await expectCell(page, rgb, 0);
  await expectCell(page, rgb2, 1);
  assert.deepEqual(await page.evaluate(() => [0, 1].map((cell) => viewports[cell].overrideWindow)), [null, null], "CT window never changes either RGB state");
  await page.locator("#reset").click();
  await expectCell(page, ct, 2);
  await expectCell(page, ct2, 3);
  assert.deepEqual(await page.evaluate(() => [2, 3].map((cell) => viewports[cell].overrideWindow)), [null, null], "CT reset propagates defaults");
}
async function expectTiles(page, expected) {
  await page.waitForFunction((wanted) => {
    const nodes = [...document.querySelectorAll(".cell.active .tiles canvas")];
    return wanted.every((rgb, index) => {
      const canvas = nodes[index];
      if (!canvas || canvas.width !== rgb.length / 3 || canvas.height !== 1) return false;
      const bytes = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
      return rgb.every((value, at) => value === bytes[Math.floor(at / 3) * 4 + at % 3]);
    });
  }, expected);
}
async function mixedTilesAndPrefetch(page, data) {
  await page.reload();
  const { directory, specs } = data.mixedSeries;
  const first = await checkpoint(page);
  await page.locator("#folder").setInputFiles(directory);
  await waitColourPaint(page, specs[0], first);
  assert.equal(await page.locator("#series-list button").count(), 1, "mixed fixture is one series");
  const after = await checkpoint(page);
  await page.evaluate(() => dropCache());
  await page.keyboard.press("2");
  const lung = [94, 94, 94, 236, 236, 236];
  await waitColourPaint(page, specs[0], after, { preset: "lung", rgb: lung });
  await page.waitForFunction((since) => {
    const image = seriesOf(activeViewport()).images[1];
    const id = filePosts.get(image.file);
    return window.__colour.requests.some((request) => request.sequence >= since && request.id === id && request.bytes);
  }, after);
  const prefetched = await page.evaluate((since) => {
    const image = seriesOf(activeViewport()).images[1];
    const id = filePosts.get(image.file);
    return window.__colour.requests.filter((request) => request.sequence >= since && request.id === id && request.bytes);
  }, after);
  assert(prefetched.length > 0, "opposite-kind colour prefetch was consumed");
  for (const request of prefetched) {
    assert.equal(request.preset, null, "colour prefetch has no CT preset");
    assert.equal(request.center, null, "colour prefetch has no CT centre");
    assert.equal(request.width, null, "colour prefetch has no CT width");
    assert.deepEqual(request.bytes, RGB, "colour prefetch keeps initial RGB bytes");
  }
  assert.equal(await page.evaluate(() => {
    const vp = activeViewport();
    const image = seriesOf(vp).images[1];
    return cacheKey(image, vp) === cacheKey(image, PLAIN_VIEW);
  }), true, "opposite-kind prefetch key is its own default window");
  await page.evaluate(() => dropCache());
  await page.locator("#tile-mode").click();
  await expectTiles(page, [lung, RGB, lung, RGB]);
  const rgbRequests = await page.evaluate((since) => {
    const series = seriesOf(activeViewport());
    const ids = new Set(series.images.filter((image) => image.info.colour).map((image) => filePosts.get(image.file)));
    return window.__colour.requests.filter((request) => request.sequence >= since && ids.has(request.id));
  }, after);
  assert(rgbRequests.length > 0);
  assert(rgbRequests.every((request) => request.preset === null && request.center === null && request.width === null), "all mixed colour tiles and prefetches keep their own window kind");
}
async function multiframeColour(page, spec) {
  await page.reload();
  await openFixture(page, spec);
  assert.equal(await page.evaluate(() => seriesOf(activeViewport()).images.length), 3, "AC14 colour cine lists every frame");
  for (let frame = 1; frame < spec.frames; frame += 1) {
    const after = await checkpoint(page);
    await page.evaluate(() => dropCache());
    await page.keyboard.press("ArrowDown");
    await waitColourPaint(page, { ...spec, frame, rgb: spec.frameRgbs[frame] }, after);
  }
  await page.keyboard.press("ArrowDown"); // Loop to frame zero before Four images.
  await expectCell(page, spec, 0);
  await page.locator("#tile-mode").click();
  await expectTiles(page, spec.frameRgbs);
}

// With the same literal pixels still on the old canvas, hold all new preview
// replies, then supply correctly sized zero bytes. Readiness must remain
// pending while held and reject those new bytes after release.
async function corruptedReplyControl(page, spec) {
  await page.reload();
  await openFixture(page, spec);
  const after = await checkpoint(page);
  let release;
  const barrier = new Promise((resolve) => { release = resolve; });
  let held;
  const entered = new Promise((resolve) => { held = resolve; });
  await page.route("**/preview", async (route) => {
    const response = await route.fetch();
    held();
    await barrier;
    await route.fulfill({ response, body: Buffer.alloc(spec.rgb.length) });
  });
  let outcome = "pending";
  let failure;
  const next = waitColourPaint(page, spec, after).then(() => { outcome = "accepted"; }, (error) => { outcome = "rejected"; failure = error; });
  try {
    await page.evaluate(() => { dropCache(); showIndex(activeViewport(), activeViewport().viewIndex); });
    let timer;
    try {
      await Promise.race([entered, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("controlled preview reply was never held")), 20000); })]);
    } finally { clearTimeout(timer); }
    // Repaint retained, matching bytes while the new response is held.
    // This deliberately advances the canvas counter too, challenging a
    // generation-only or paint-counter-only readiness predicate.
    await page.evaluate((expected) => {
      const canvas = document.querySelector(".cell.active canvas.view");
      const context = canvas.getContext("2d");
      const image = context.createImageData(canvas.width, canvas.height);
      image.data.set(expected);
      context.putImageData(image, 0, 0);
    }, rgba(spec.rgb));
    assert.deepEqual((await canvasPixels(page)).rgba, rgba(spec.rgb), "matching retained pixels exist while reply is held");
    assert.equal(outcome, "pending", "held reply must not accept the old identical canvas");
    assert.equal(await page.evaluate((since) => window.__colour.requests.some((request) => request.sequence >= since && request.bytes), after), false, "held reply is not consumed");
    release();
    await next;
    assert.equal(outcome, "rejected", "corrupt decoded reply must be rejected even with old matching pixels");
    assert.match(failure?.message || "", /consumed preview channel bytes/, "control fails at actual decoded bytes, not an unrelated timeout");
  } finally {
    release();
    await page.unroute("**/preview");
    await next.catch(() => {});
  }
}

export async function observeColour(url, chromium, data) {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    page.setDefaultTimeout(20000);
    await page.addInitScript(instrument);
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(url);
    await page.keyboard.press("c");
    for (const spec of data.fixtures.filter((entry) => !entry.mono)) {
      await openFixture(page, spec);
      await thumbnail(page, spec);
      assert.equal(await page.locator("#window-note").textContent(), COLOUR_NOTE, `${spec.name}: colour window sentence`);
      assert.equal(await page.locator("#window-list button").count(), 0, `${spec.name}: no preset or file window offered`);
      const corner = await page.locator(".cell.active .overlay.br").textContent();
      const warning = iccSentence(spec);
      assert.equal(corner.includes("ICC profile was not applied"), Boolean(warning), `${spec.name}: ICC sentence in corner`);
      if (warning) assert(corner.includes(warning), `${spec.name}: exact ICC kind`);
      const applied = await page.evaluate(() => currentImage(activeViewport()).info.icc_applied);
      assert.equal(applied, Boolean(spec.iccApplied), `${spec.name}: inspect ICC disposition`);
      if (spec.icc || spec.profile) {
        const box = await page.locator(".cell.active canvas.view").boundingBox();
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        const help = await page.locator("#sentence").textContent();
        assert.equal(help.includes("ICC profile was not applied"), Boolean(warning), "ICC sentence in help");
        if (warning) assert(help.includes(warning), "ICC kind in help");
      }
      for (const window of spec.windows || []) {
        await setWindow(page, spec, window);
        if (window.inverse) {
          const after = await checkpoint(page);
          await page.locator("#invert").click();
          await waitColourPaint(page, spec, after, { ...window, invert: true, rgb: window.inverse });
          const undo = await checkpoint(page);
          await page.locator("#invert").click();
          // A cache hit is permitted on undo. Its actual pixels must return.
          await page.waitForFunction(() => !activeViewport().invert);
          await page.waitForFunction((expected) => {
            const canvas = document.querySelector(".cell.active canvas.view");
            const bytes = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
            return expected.every((value, at) => value === bytes[at]);
          }, rgba(window.rgb));
          assert.deepEqual((await canvasPixels(page)).rgba, rgba(window.rgb), `${spec.name}: invert undo`);
          assert((await checkpoint(page)) >= undo);
        }
      }
    }
    const rgb = data.fixtures.find((entry) => entry.name === "rgb-interleaved");
    await openFixture(page, rgb);
    await readout(page, 1, "R 200.0 G 100.0 B 50.0");
    await openFixture(page, data.region);
    await colourRegion(page, data.region);
    await openFixture(page, data.fixtures.find((entry) => entry.name === "ybr-full"));
    await readout(page, 0, "R 46.7 G 119.6 B 139.0");
    const windowCase = data.fixtures.find((entry) => entry.name === "window-100");
    await openFixture(page, windowCase);
    // Current viewer mouse contract: width/200 codes per CSS pixel, so
    // 256 -> 100 is -121.875 CSS pixels. No pixel expectation is read back.
    await page.locator("#tool-wl").click();
    const box = await page.locator(".cell.active canvas.view").boundingBox();
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    const afterDrag = await checkpoint(page);
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x - 121.875, y, { steps: 8 });
    await page.mouse.up();
    await waitColourPaint(page, windowCase, afterDrag, windowCase.windows[0]);

    // Mixed CT and RGB share one folder but keep distinct window units.
    const afterMixed = await checkpoint(page);
    await page.locator("#folder").setInputFiles(data.mixed);
    await waitColourPaint(page, rgb, afterMixed);
    for (const name of ["ybr-422", "palette-offset", "rgb-icc", "icc-gamma1", "icc-lut", "rgb-interleaved"]) {
      const spec = data.fixtures.find((entry) => entry.name === name);
      const beforeSelection = await checkpoint(page);
      await page.evaluate(() => dropCache());
      await page.locator("#series-list button").filter({ has: page.getByText(name, { exact: true }) }).click();
      await waitColourPaint(page, spec, beforeSelection);
      const corner = await page.locator(".cell.active .overlay.br").textContent();
      const warning = iccSentence(spec);
      assert.equal(corner.includes("ICC profile was not applied"), Boolean(warning), `${name}: ICC state follows selection within mixed folder`);
      if (warning) assert(corner.includes(warning), `${name}: selected ICC kind`);
    }
    await page.locator("#layout").click();
    await page.locator('#layout-picker [data-columns="2"][data-rows="1"]').click();
    const ct = data.fixtures.find((entry) => entry.mono);
    const afterCT = await checkpoint(page);
    await page.evaluate(() => dropCache());
    await page.locator("#series-list button").filter({ has: page.getByText(ct.name, { exact: true }) }).dragTo(page.locator('.cell[data-cell="1"]'));
    await waitColourPaint(page, ct, afterCT, {}, 1);
    const rgbBefore = await canvasPixels(page);
    await page.locator('.cell[data-cell="1"]').click();
    const lung = await checkpoint(page);
    await page.keyboard.press("2");
    await waitColourPaint(page, ct, lung, { preset: "lung", rgb: [94, 94, 94, 236, 236, 236] }, 1);
    assert.deepEqual(await canvasPixels(page), rgbBefore, "CT window leaves adjacent colour pixels unchanged");
    await linkedWindows(page, data);
    await mixedTilesAndPrefetch(page, data);
    await multiframeColour(page, data.multiframe);
    await corruptedReplyControl(page, rgb);
    assert.deepEqual(errors, [], "browser page errors");
    return { chromium: browser.version(), fixtures: data.fixtures.length, corruptedReplyRejected: true };
  } finally { await browser.close(); }
}

async function main() {
  const scratch = mkdtempSync(join(tmpdir(), "ocelli-colour-"));
  let server;
  try {
    const data = makeColourFixtures(scratch);
    const { chromium } = await loadPlaywright();
    const binary = await ensureViewerBinary("release", true);
    server = await startViewer(binary);
    const seen = await observeColour(server.url, chromium, data);
    console.log(JSON.stringify(seen));
  } finally {
    if (server) await stopByPid(server.pid);
    rmSync(scratch, { recursive: true, force: true });
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((error) => { console.error(error.stack || error.message); process.exitCode = 1; });
