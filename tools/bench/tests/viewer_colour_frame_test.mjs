// Hand checks for the viewer.colour_frame instrument. The expected bytes are
// computed here from the rules the F-239 plan cites, not read back from the
// runner, and the negative cases each name the wrong implementation they
// reject. Nothing here launches a viewer, builds the release binary, touches
// an 8192 pixel frame or a GPU: the fixtures run at edge 4 and the HTTP
// controls are a local fake server.

import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { readFile, mkdtemp, mkdir, rm, stat, writeFile, chmod } from "node:fs/promises";
import http from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  assertCensusStable,
  assertHeldBudget,
  assertHeldCensus,
  assertIccApplied,
  assertRelease,
  consumePreview,
  holdFile,
  IDENTITY_WINDOW,
  measureWorkload,
  requestedCacheMib,
  SampleWindow,
  startEnv,
  startRssSampler,
  validatePreview,
  WINDOW_STEPS,
  withFixtures,
} from "../src/runners/viewer_colour_frame.mjs";
import {
  CHECK_POSITIONS,
  EDGE,
  expectedSamples,
  paletteEntry,
  paletteIndex,
  replyBytes,
  rgbStored,
  sampleChecks,
  windowedByte,
  WORKLOADS,
  writeColourFrameFixtures,
} from "../src/viewer_colour_frame_fixture.mjs";
import {
  icc16Stored,
  iccExpectedDisplay,
  iccMarkerCode,
  iccMarkerCodes,
  iccMarkers,
  iccReplyByte,
  iccSampleBytes,
  iccSampleChecks,
  iccStored,
  iccTrcTableBytes,
  iccWindowBytes,
  linear16TableProfile,
  writeIccFrameFixtures,
} from "../src/viewer_icc_frame_fixture.mjs";
import { loadRegistry, MEASURABLE_STATUS } from "../src/registry.mjs";

/** The F-239 edge-4 golden digests, from the task inputs' old-fixture-goldens.json. */
const GOLDENS = {
  rgb8: {
    sha256: "9747f1110ccce243d54dacf9d131719f31d73ac757f030a5cf1e134851965d28",
    bytes: 524,
  },
  palette16: {
    sha256: "759af30545c9ec4624a66669db197b62ca3c82902365bfbdb59a4909916b900b",
    bytes: 393802,
  },
};

/** The provenance record shipped beside the ICC assets, read once. */
const PROVENANCE = JSON.parse(
  readFileSync(new URL("./fixtures/f240-icc/profile-provenance.json", import.meta.url), "utf8"),
);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function refuses(fn, text) {
  assert.throws(fn, (error) => error instanceof Error && String(error.message).includes(text));
}

async function rejectsMessage(workload, text) {
  // A function is wrapped, so a synchronous throw and a rejection take the
  // same path. assert.rejects routes a sync throw straight out in this node.
  // A promise is taken as it is.
  const pending = typeof workload === "function"
    ? Promise.resolve().then(workload)
    : workload;
  await assert.rejects(pending, (error) =>
    String(error?.message ?? error).includes(text),
  );
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function listen(handler) {
  const server = http.createServer(handler);
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { server, port: server.address().port };
}

function close(server) {
  return new Promise((resolve) => {
    server.closeAllConnections();
    server.close(() => resolve());
  });
}

const LONG_VRS = new Set(["OB", "OW", "OF", "SQ", "UT", "UN"]);

/** Every element of one explicit VR little endian buffer, by tag. */
function elements(buffer) {
  const found = new Map();
  let offset = 132;
  while (offset + 8 <= buffer.length) {
    const group = buffer.readUInt16LE(offset);
    const element = buffer.readUInt16LE(offset + 2);
    const vr = buffer.toString("ascii", offset + 4, offset + 6);
    let length;
    let header;
    if (LONG_VRS.has(vr)) {
      length = buffer.readUInt32LE(offset + 8);
      header = 12;
    } else {
      length = buffer.readUInt16LE(offset + 6);
      header = 8;
    }
    const tag = `${group.toString(16).padStart(4, "0")},${element.toString(16).padStart(4, "0")}`;
    found.set(tag, buffer.subarray(offset + header, offset + header + length));
    offset += header + length;
  }
  return found;
}

function textOf(buffer) {
  return buffer.toString("latin1").replaceAll("\0", "").trim();
}

function usOf(buffer, index = 0) {
  return buffer.readUInt16LE(index * 2);
}

test("the colour frame row is registered against its story and no threshold", () => {
  const loaded = loadRegistry();
  const row = loaded.find(({ subject }) => subject.id === "viewer.colour_frame");
  assert.ok(row, "viewer.colour_frame is not in the tracked registry");
  assert.equal(row.subject.unit, "ms");
  assert.equal(row.subject.subject_story, "F-239");
  assert.deepEqual(row.subject.feeds, ["F-239"]);
  assert.deepEqual(row.subject.tiers, ["C"]);
  // The row's measurability follows the backlog row, whatever it is today.
  // Asserting `pending` here would break the day F-239 starts.
  assert.equal(row.subjectExists, MEASURABLE_STATUS[row.storyStatus]);
  // AC31 gives the subject no threshold, so the row carries none to alter.
  assert.equal(
    Object.keys(row.subject).filter((key) => /threshold/i.test(key)).length,
    0,
  );
  // The runner file the row names is the one this file checks.
  assert.match(row.subject.definition, /8192 by 8192/);
});

test("the fixtures are Part 10 with the declared image attributes", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-frame-"));
  try {
    const fixtures = await writeColourFrameFixtures(dir, 4);
    // Hand computed at edge 4. 132 bytes of preamble and DICM, the group
    // length element, 110 bytes of file meta, 210 bytes of RGB dataset, the
    // 12 byte pixel data header and 48 bytes of pixels.
    assert.equal(fixtures.rgb8.fileBytes, 524);
    // The palette dataset carries three 131072 byte tables, so the same
    // frame with one sample per pixel and three tables is 393802 bytes.
    assert.equal(fixtures.palette16.fileBytes, 393802);
    assert.equal(fixtures.rgb8.replyBytes, 48);
    assert.equal(fixtures.palette16.replyBytes, 48);

    const rgb = await readFile(fixtures.rgb8.path);
    assert.equal(rgb.subarray(128, 132).toString("ascii"), "DICM");
    const rgbTags = elements(rgb);
    // PS3.5 7.1: the meta group length covers the meta elements that follow.
    assert.equal(rgbTags.get("0002,0000").readUInt32LE(0), 110);
    assert.equal(textOf(rgbTags.get("0002,0010")), "1.2.840.10008.1.2.1");
    assert.equal(usOf(rgbTags.get("0028,0002")), 3);
    assert.equal(textOf(rgbTags.get("0028,0004")), "RGB");
    assert.equal(usOf(rgbTags.get("0028,0006")), 0);
    assert.equal(usOf(rgbTags.get("0028,0010")), 4);
    assert.equal(usOf(rgbTags.get("0028,0011")), 4);
    assert.equal(usOf(rgbTags.get("0028,0100")), 8);
    assert.equal(usOf(rgbTags.get("0028,0101")), 8);
    assert.equal(usOf(rgbTags.get("0028,0102")), 7);
    assert.equal(usOf(rgbTags.get("0028,0103")), 0);
    const rgbPixels = rgbTags.get("7fe0,0010");
    assert.equal(rgbPixels.length, 48);
    for (const [x, y] of [[0, 0], [1, 0], [0, 1], [3, 3]]) {
      const at = (y * 4 + x) * 3;
      assert.deepEqual(
        [...rgbPixels.subarray(at, at + 3)],
        rgbStored(x, y),
        `the RGB pixel at ${x},${y} is not the written pattern`,
      );
      assert.deepEqual([...rgbPixels.subarray(at, at + 3)], expectedSamples("rgb8", x, y));
    }

    const palette = await readFile(fixtures.palette16.path);
    assert.equal(palette.subarray(128, 132).toString("ascii"), "DICM");
    const paletteTags = elements(palette);
    assert.equal(usOf(paletteTags.get("0028,0002")), 1);
    assert.equal(textOf(paletteTags.get("0028,0004")), "PALETTE COLOR");
    assert.equal(usOf(paletteTags.get("0028,0100")), 16);
    assert.equal(usOf(paletteTags.get("0028,0101")), 16);
    assert.equal(usOf(paletteTags.get("0028,0102")), 15);
    assert.equal(usOf(paletteTags.get("0028,0103")), 0);
    // PS3.3 C.7.6.3.1.5: an entry count of zero is 65536 entries, first
    // mapped input 0, 16-bit entries, the same on all three channels.
    for (const tag of ["0028,1101", "0028,1102", "0028,1103"]) {
      const descriptor = paletteTags.get(tag);
      assert.equal(descriptor.length, 6);
      assert.deepEqual([usOf(descriptor, 0), usOf(descriptor, 1), usOf(descriptor, 2)], [0, 0, 16]);
    }
    for (const [tag, stride] of [["0028,1201", 7], ["0028,1202", 13], ["0028,1203", 29]]) {
      const table = paletteTags.get(tag);
      assert.equal(table.length, 65536 * 2);
      for (const index of [0, 1, 5, 65535]) {
        assert.equal(table.readUInt16LE(index * 2), (stride * index) % 65536, tag);
      }
    }
    const palettePixels = paletteTags.get("7fe0,0010");
    assert.equal(palettePixels.length, 32);
    for (const [x, y] of [[0, 0], [1, 0], [0, 1], [3, 3]]) {
      assert.equal(
        palettePixels.readUInt16LE((y * 4 + x) * 2),
        paletteIndex(x, y),
        `the palette index at ${x},${y} is not the written pattern`,
      );
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("expectedSamples is the colour stage output, hand computed", () => {
  // The 8-bit sample passes as it is: 300 and 200 reduce by 256, and the sum
  // wraps once.
  assert.deepEqual(expectedSamples("rgb8", 300, 200), [44, 200, 244]);
  assert.deepEqual(expectedSamples("rgb8", 8191, 8191), [255, 255, 254]);
  assert.deepEqual(expectedSamples("rgb8", 0, 0), [0, 0, 0]);
  // Index 40000 comes from pixel (0, 8000), because 5 * 8000 is 40000. The
  // entries are 7 * 40000 = 280000, 13 * 40000 = 520000 and 29 * 40000 =
  // 1160000, each reduced by 65536, then shifted by the descriptor's 16
  // bits: 17856, 61248 and 45888 become 69, 239 and 179.
  assert.deepEqual(expectedSamples("palette16", 0, 8000), [69, 239, 179]);
  assert.deepEqual(expectedSamples("palette16", 0, 0), [0, 0, 0]);
  assert.equal(paletteIndex(0, 8000), 40000);
  assert.deepEqual(paletteEntry(40000), [17856, 61248, 45888]);
  refuses(() => expectedSamples("monochrome2", 0, 0), "unknown colour frame workload");
});

test("windowedByte is the LINEAR colour window over 0 to 255", () => {
  // F-239 AC19's rows: width 10 and centre 128 over stored 125, 128, 130.
  assert.equal(windowedByte(125, 128, 10), 57);
  assert.equal(windowedByte(128, 128, 10), 142);
  assert.equal(windowedByte(130, 128, 10), 198);
  // The same AC19 row at width 2: 127 128 0 gives 0 255 0.
  assert.equal(windowedByte(127, 128, 2), 0);
  assert.equal(windowedByte(128, 128, 2), 255);
  assert.equal(windowedByte(0, 128, 2), 0);
  // The initial window is the identity on every display value.
  for (let value = 0; value <= 255; value += 1) {
    assert.equal(windowedByte(value, 128, 256), value, `identity at ${value}`);
  }
  // The step windows clamp the values beyond them, exactly.
  assert.equal(windowedByte(240, 100, 200), 255);
  assert.equal(windowedByte(8, 108, 200), 0);
});

test("every validated sample sits away from a rounding tie", () => {
  // The runner computes the expected byte in f64 and the viewer applies the
  // window in f32. A validated value near a tie could flip between the two,
  // so every sample the instrument checks must sit at least 0.002 from one,
  // which is two hundred times the f32 error at these magnitudes. Clamped
  // samples compare integers exactly and are excluded.
  const windows = [IDENTITY_WINDOW, ...WINDOW_STEPS];
  for (const workloadId of Object.keys(WORKLOADS)) {
    for (const window of windows) {
      const lower = window.center - 0.5 - (window.width - 1) / 2;
      const upper = window.center - 0.5 + (window.width - 1) / 2;
      for (const [x, y] of CHECK_POSITIONS) {
        const raw = expectedSamples(workloadId, x, y);
        for (let channel = 0; channel < 3; channel += 1) {
          const value = raw[channel];
          if (value <= lower || value > upper) continue;
          const scaled =
            ((value - (window.center - 0.5)) / (window.width - 1) + 0.5) * 255;
          const tie = Math.floor(scaled) + 0.5;
          assert.ok(
            Math.abs(scaled - tie) >= 0.002,
            `${workloadId} at ${x},${y} channel ${channel} window ` +
              `${window.center}/${window.width} scaled to ${scaled}, ` +
              `${Math.abs(scaled - tie)} from a tie`,
          );
        }
      }
    }
  }
});

test("sampleChecks names one absolute offset per checked byte", () => {
  const checks = sampleChecks("rgb8", IDENTITY_WINDOW);
  assert.equal(CHECK_POSITIONS.length, 10);
  assert.equal(checks.length, 30);
  const offsets = new Set(checks.map((check) => check.offset));
  assert.equal(offsets.size, checks.length);
  for (const check of checks) {
    assert.ok(check.offset >= 0 && check.offset < replyBytes(EDGE));
    assert.ok(check.expected >= 0 && check.expected <= 255);
  }
  // A smaller edge drops the positions it does not contain, which leaves
  // the two positions inside a 4 by 4 frame.
  const tiny = sampleChecks("rgb8", IDENTITY_WINDOW, 4);
  assert.equal(tiny.length, 6);
  assert.ok(tiny.every((check) => check.offset < 48));
});

test("SampleWindow checks bytes across chunk boundaries", () => {
  const window = new SampleWindow([
    { offset: 2, expected: 7 },
    { offset: 5, expected: 9 },
    { offset: 6, expected: 1 },
  ]);
  window.push(Buffer.from([0, 0, 7, 0, 0]), 0);
  // The check at offset 5 straddles the boundary between the two chunks.
  window.push(Buffer.from([9, 1, 0, 0]), 5);
  assert.deepEqual(window.result(), { count: 3 });
});

test("SampleWindow refuses a corrupted sample and names the offset", () => {
  const window = new SampleWindow([{ offset: 5, expected: 9 }]);
  window.push(Buffer.alloc(4, 0), 0);
  // Offset 5 is the second byte of the second chunk, and it holds 8.
  refuses(() => window.push(Buffer.from([0, 8, 0, 0]), 4), "offset 5 was 8, expected 9");
});

test("SampleWindow refuses a reply that ends before every sample", () => {
  const window = new SampleWindow([{ offset: 5, expected: 9 }]);
  window.push(Buffer.alloc(4, 0), 0);
  refuses(() => window.result(), "before the sample at offset 5");
});

test("SampleWindow refuses a check its chunks already passed", () => {
  const window = new SampleWindow([{ offset: 5, expected: 9 }]);
  refuses(() => window.push(Buffer.alloc(16, 9), 16), "passed before it arrived");
});

function fakeReplyHeaders(window, overrides = {}) {
  return {
    "content-type": "application/octet-stream",
    "x-ocelli-channels": "3",
    "x-ocelli-width": "2",
    "x-ocelli-height": "2",
    "x-ocelli-window-center": String(window?.center ?? 128),
    "x-ocelli-window-width": String(window?.width ?? 256),
    ...overrides,
  };
}

test("consumePreview consumes the whole reply and validates the samples", async () => {
  const body = Buffer.alloc(12);
  body[0] = 5;
  body[11] = 255;
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, fakeReplyHeaders());
    response.end(body);
  });
  try {
    const checks = [
      { offset: 0, expected: 5 },
      { offset: 11, expected: 255 },
    ];
    const outcome = await consumePreview({ port, path: "/preview", checks });
    assert.equal(outcome.status, 200);
    assert.equal(outcome.bodyBytes, 12);
    assert.equal(outcome.validated, 2);
    assert.ok(outcome.elapsed_ms >= 0);
    validatePreview(outcome, {
      checks,
      channels: 3,
      replyBytes: 12,
      edge: 2,
      window: IDENTITY_WINDOW,
      label: "fake",
    });
  } finally {
    await close(server);
  }
});

test("a reply with the wrong channel count is refused", async () => {
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, fakeReplyHeaders(IDENTITY_WINDOW, { "x-ocelli-channels": "1" }));
    response.end(Buffer.alloc(4));
  });
  try {
    const outcome = await consumePreview({ port, path: "/preview", checks: [] });
    await rejectsMessage(
      () => validatePreview(outcome, { checks: [], channels: 3, replyBytes: 4, edge: 2, window: IDENTITY_WINDOW, label: "fake" }),
      'X-Ocelli-Channels "1"',
    );
  } finally {
    await close(server);
  }
});

test("a reply of the wrong length is refused even with the right channels", async () => {
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, fakeReplyHeaders());
    response.end(Buffer.alloc(11));
  });
  try {
    const outcome = await consumePreview({ port, path: "/preview", checks: [] });
    await rejectsMessage(
      () => validatePreview(outcome, { checks: [], channels: 3, replyBytes: 12, edge: 2, window: IDENTITY_WINDOW, label: "fake" }),
      "replied 11 body bytes, expected 12",
    );
  } finally {
    await close(server);
  }
});

test("a corrupted sample rejects the request itself", async () => {
  const body = Buffer.alloc(12);
  body[11] = 254;
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, fakeReplyHeaders());
    response.end(body);
  });
  try {
    await rejectsMessage(
      consumePreview({ port, path: "/preview", checks: [{ offset: 11, expected: 255 }] }),
      "offset 11 was 254, expected 255",
    );
  } finally {
    await close(server);
  }
});

test("a refusal answers with its code and is not a figure", async () => {
  const { server, port } = await listen((request, response) => {
    response.writeHead(400, { "x-ocelli-error": "colour-unavailable" });
    response.end("this build cannot draw the frame");
  });
  try {
    const outcome = await consumePreview({ port, path: "/preview", checks: [] });
    await rejectsMessage(
      () => validatePreview(outcome, { checks: [], channels: 3, replyBytes: 12, edge: 2, window: IDENTITY_WINDOW, label: "fake" }),
      "answered 400 with colour-unavailable",
    );
  } finally {
    await close(server);
  }
});

test("a reply that closes before it ends rejects", async () => {
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, fakeReplyHeaders());
    response.write(Buffer.alloc(4));
    setTimeout(() => response.destroy(), 10);
  });
  try {
    // The socket's death surfaces as a request error or as the close before
    // the end, by timing. Either way the request rejects instead of hanging.
    await assert.rejects(
      consumePreview({ port, path: "/preview", checks: [] }),
      (error) => /aborted|closed before it ended/.test(String(error.message)),
    );
  } finally {
    await close(server);
  }
});

test("a viewer that is not there rejects", async () => {
  const { server, port } = await listen(() => {});
  const closedPort = port;
  await close(server);
  await assert.rejects(consumePreview({ port: closedPort, path: "/preview", checks: [] }));
});

test("a body preview streams the file and an id preview carries no body", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-frame-"));
  const captured = [];
  const { server, port } = await listen((request, response) => {
    const record = { path: request.url, headers: { ...request.headers }, bodyBytes: 0 };
    captured.push(record);
    request.on("data", (chunk) => {
      record.bodyBytes += chunk.length;
    });
    request.on("end", () => {
      response.writeHead(200, fakeReplyHeaders());
      response.end(Buffer.alloc(12));
    });
  });
  try {
    const fixtures = await writeColourFrameFixtures(dir, 2);
    await consumePreview({
      port,
      path: "/preview",
      bodyPath: fixtures.rgb8.path,
      bodyLength: fixtures.rgb8.fileBytes,
      checks: [],
    });
    await consumePreview({
      port,
      path: "/preview",
      headers: { "X-Ocelli-File": "1.2" },
      checks: [],
    });
    assert.equal(captured[0].bodyBytes, fixtures.rgb8.fileBytes);
    assert.equal(captured[0].headers["content-length"], String(fixtures.rgb8.fileBytes));
    assert.equal(captured[0].headers["x-ocelli-file"], undefined);
    // The held form is a POST by id with an empty body, which is what the
    // server's own rule requires: an id and a body cannot go together.
    assert.equal(captured[1].headers["x-ocelli-file"], "1.2");
    assert.equal(captured[1].headers["content-length"], "0");
    assert.equal(captured[1].bodyBytes, 0);
  } finally {
    await close(server);
    await rm(dir, { recursive: true, force: true });
  }
});

test("holdFile takes the id the server names and refuses an unheld file", async () => {
  const { server, port } = await listen((request, response) => {
    response.writeHead(200, { "x-ocelli-file": "5.a", "x-ocelli-held": "1" });
    response.end("{}");
  });
  try {
    assert.deepEqual(await holdFile({ port, bodyPath: null, bodyLength: null }), { id: "5.a", report: {} });
  } finally {
    await close(server);
  }
  const unheld = await listen((request, response) => {
    response.writeHead(200, { "x-ocelli-held": "0" });
    response.end("{}");
  });
  try {
    await rejectsMessage(
      holdFile({ port: unheld.port, bodyPath: null, bodyLength: null }),
      "did not hold the posted file",
    );
  } finally {
    await close(unheld.server);
  }
  const unnamed = await listen((request, response) => {
    response.writeHead(200, { "x-ocelli-held": "1" });
    response.end("{}");
  });
  try {
    await rejectsMessage(
      holdFile({ port: unnamed.port, bodyPath: null, bodyLength: null }),
      "named no file id",
    );
  } finally {
    await close(unnamed.server);
  }
});

test("the release guard refuses every other build report", () => {
  assertRelease("release");
  refuses(() => assertRelease("debug"), "measures the release build only");
  refuses(() => assertRelease(undefined), "measures the release build only");
  refuses(() => assertRelease("release-profile"), "measures the release build only");
});

test("the rss sampler keeps its readings and refuses an empty peak", async () => {
  // `ps` is not exercised here on purpose. The sampler's contract is what
  // the check proves, with a reader standing in for the subprocess, and the
  // real reading is what AC31 takes on the F-216 host.
  let value = 100;
  const rising = async () => {
    await delay(2);
    value += 10;
    return value;
  };
  const sampler = startRssSampler(1, 5, rising);
  await delay(60);
  const rss = await sampler.stop();
  assert.ok(rss.samples >= 2, `only ${rss.samples} samples in 60 ms`);
  assert.ok(rss.peak_bytes > 1024);
  assert.equal(rss.failures, 0);
  assert.equal(rss.period_ms, 5);
  // Stopping twice is one outcome.
  const again = await sampler.stop();
  assert.equal(again.peak_bytes, rss.peak_bytes);
  assert.equal(again.samples, rss.samples);
});

test("the rss sampler counts failed reads and never overlaps them", async () => {
  let calls = 0;
  let active = 0;
  let deepest = 0;
  const flaky = async () => {
    calls += 1;
    active += 1;
    deepest = Math.max(deepest, active);
    await delay(2);
    active -= 1;
    return calls % 2 === 0 ? null : calls * 100;
  };
  const sampler = startRssSampler(1, 5, flaky);
  await delay(60);
  const rss = await sampler.stop();
  assert.ok(rss.samples >= 2, `only ${rss.samples} good reads`);
  assert.ok(rss.failures >= 1, "the failed reads were not counted");
  assert.equal(deepest, 1, "two reads ran at once");

  const empty = startRssSampler(1, 5, async () => null);
  await rejectsMessage(() => empty.stop(), "no rss sample was taken");
});

test("the nine window steps leave the identity behind", () => {
  assert.equal(WINDOW_STEPS.length, 9);
  const centres = new Set(WINDOW_STEPS.map((step) => step.center));
  assert.equal(centres.size, 9);
  for (const step of WINDOW_STEPS) {
    assert.equal(step.width, 200);
    assert.notDeepEqual(step, IDENTITY_WINDOW);
  }
});

test("withFixtures removes the fixtures on failure and on success", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-frame-"));
  try {
    const failing = join(dir, "failing");
    await rejectsMessage(
      withFixtures(failing, async () => {
        const check = await stat(join(failing, WORKLOADS.rgb8.file));
        assert.ok(check.size > 0);
        throw new Error("the workload broke");
      }, 4),
      "the workload broke",
    );
    await assert.rejects(() => stat(join(failing, WORKLOADS.rgb8.file)));

    const kept = join(dir, "kept");
    const fileBytes = await withFixtures(kept, async (fixtures) => fixtures.rgb8.fileBytes, 4);
    assert.ok(fileBytes > 0);
    await assert.rejects(() => stat(join(kept, WORKLOADS.rgb8.file)));
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

// Final library ruling R2. These exact halves distinguish the new contract
// from the older plan supplied to the initial instrument implementation.
test("colour validation rounds exact ties to even under R2", () => {
  assert.equal(windowedByte(130, 128.5, 511), 128);
  assert.equal(windowedByte(130, 130.5, 100), 128);
  assert.equal(windowedByte(129.999, 128.5, 511), 128);
  assert.equal(windowedByte(130.001, 128.5, 511), 129);
});

test("RSS rejects invalid observations and preserves every actual reading", async () => {
  const invalid = startRssSampler(1, 1, async () => 0);
  await delay(5);
  await rejectsMessage(() => invalid.stop(), "no rss sample was taken");
  let next = 100;
  const sampler = startRssSampler(1, 1, async () => next++);
  await delay(6);
  const actual = await sampler.stop();
  assert(Array.isArray(actual.readings_kb), "actual RSS readings are preserved in the record");
  assert.equal(actual.samples, actual.readings_kb.length);
  assert(actual.samples > 0);
  assert.deepEqual(actual.readings_kb, Array.from({ length: actual.samples }, (_, index) => 100 + index));
  assert.equal(actual.peak_bytes, Math.max(...actual.readings_kb) * 1024);
});

test("fixture cleanup covers a partial generation failure at the small test edge", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-partial-"));
  try {
    await mkdir(join(dir, WORKLOADS.palette16.file));
    let ran = false;
    await rejectsMessage(withFixtures(dir, () => { ran = true; }, 4), "EISDIR");
    assert.equal(ran, false);
    await assert.rejects(() => stat(dir), { code: "ENOENT" });
  } finally { await rm(dir, { recursive: true, force: true }); }
});

// A real tiny HTTP child proves the lifecycle and transaction order without
// building a viewer, decoding a largest frame, or opening a browser/GPU.
async function fakeColourViewer(dir, mode, workloadId = "rgb8") {
  const binary = join(dir, "fake-viewer.mjs");
  const trace = join(dir, "viewer-trace.json");
  const fixtureModule = new URL("../src/viewer_colour_frame_fixture.mjs", import.meta.url).href;
  const script = `#!/usr/bin/env node
import http from "node:http";
import { writeFileSync } from "node:fs";
import { expectedReplyByte } from ${JSON.stringify(fixtureModule)};
const requests = [];
let previews = 0;
const server = http.createServer((request, response) => {
  let bytes = 0;
  request.on("data", chunk => { bytes += chunk.length; });
  request.on("end", () => {
    requests.push({ method: request.method, path: request.url, bytes, file: request.headers["x-ocelli-file"] || null });
    if (request.url === "/held") {
      response.end(JSON.stringify({ build: "release", budget: 268435456 }));
    } else if (request.url === "/file") {
      response.writeHead(200, { "X-Ocelli-File": "1.a", "X-Ocelli-Held": "1" });
      response.end(JSON.stringify({ colour: true, monochrome: false, rows: 4, columns: 4, icc: false }));
    } else if (request.url === "/preview") {
      const source = request.headers["x-ocelli-file"] ? (previews++ === 0 ? "decoded" : "held") : "body";
      const window = { center: Number(request.headers["x-ocelli-window-center"]), width: Number(request.headers["x-ocelli-window-width"]) };
      const body = Buffer.alloc(48);
      for (let y = 0; y < 4; y++) for (let x = 0; x < 4; x++) for (let c = 0; c < 3; c++) body[(y * 4 + x) * 3 + c] = expectedReplyByte(${JSON.stringify(workloadId)}, x, y, c, window);
      response.writeHead(200, { "X-Ocelli-Source": ${JSON.stringify(mode)} === "wrong-warm-source" && previews > 1 ? "decoded" : source, "X-Ocelli-Channels": "3", "X-Ocelli-Width": "4", "X-Ocelli-Height": "4", "X-Ocelli-Window-Center": String(window.center), "X-Ocelli-Window-Width": String(window.width) });
      response.end(body);
    } else { response.writeHead(404); response.end(); }
  });
});
server.listen(0, "127.0.0.1", () => console.log("http://127.0.0.1:" + server.address().port + "/"));
process.on("SIGTERM", () => {
  writeFileSync(${JSON.stringify(trace)}, JSON.stringify({ pid: process.pid, requests }));
  server.closeAllConnections();
  server.close(() => process.exit(0));
});
`;
  await writeFile(binary, script);
  await chmod(binary, 0o755);
  return { binary, trace };
}

function tinyRss() {
  const report = { peak_bytes: 65536, samples: 1, readings_kb: [64], failures: 0, period_ms: 20 };
  return { stop: async () => report };
}

for (const mode of ["normal", "wrong-warm-source"]) {
  test(`first preview decodes, later windows stay held, and child cleanup works: ${mode}`, async () => {
    const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-protocol-"));
    try {
      const fixtures = await writeColourFrameFixtures(dir, 4);
      const actor = await fakeColourViewer(dir, mode);
      const action = measureWorkload({ binary: actor.binary, spec: fixtures.rgb8, sampleRss: tinyRss });
      if (mode === "wrong-warm-source") {
        await rejectsMessage(action, "expected held");
      } else {
        const result = await action;
        assert.equal(result.first_source, "decoded");
        assert.deepEqual(result.stepSources, Array(9).fill("held"));
        assert.equal(result.stepSamples.length, 9);
        assert.equal(result.postReport.colour, true);
        assert.equal(result.heldAfterPost.budget, 268435456);
      }
      const trace = JSON.parse(await readFile(actor.trace, "utf8"));
      const posts = trace.requests.filter(entry => entry.method === "POST");
      assert.equal(posts[0].path, "/file");
      assert.equal(posts[0].bytes, fixtures.rgb8.fileBytes);
      assert(posts.slice(1).every(entry => entry.path === "/preview" && entry.bytes === 0 && entry.file === "1.a"));
      assert.equal(posts.length, mode === "normal" ? 11 : 3);
      assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after success or source refusal");
    } finally { await rm(dir, { recursive: true, force: true }); }
  });
}

// ---------------------------------------------------------------------------
// F-240 AC15's additive ICC extension. The expected bytes are the precomputed
// hand vectors, never product output, and every control below runs at edge 4
// against a local fake child: no 8192 generation, no release build, no
// benchmark, no Cargo, no browser and no GPU.
// ---------------------------------------------------------------------------

test("the F239 fixture bytes are unchanged against the golden digests", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-golden-"));
  try {
    const fixtures = await writeColourFrameFixtures(dir, 4);
    assert.equal(fixtures.rgb8.fileBytes, GOLDENS.rgb8.bytes);
    assert.equal(sha256(await readFile(fixtures.rgb8.path)), GOLDENS.rgb8.sha256);
    assert.equal(fixtures.palette16.fileBytes, GOLDENS.palette16.bytes);
    assert.equal(sha256(await readFile(fixtures.palette16.path)), GOLDENS.palette16.sha256);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("the ICC fixtures are Part 10 with the profiles and the neutral markers", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-fix-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    // Hand computed at edge 4: 254 bytes of preamble, group length and meta,
    // a 782 byte dataset carrying the 560 byte profile, the 12 byte pixel
    // data header, then 48 or 96 bytes of pixels.
    assert.equal(fixtures.icc8.fileBytes, 1096);
    assert.equal(fixtures.icc16.fileBytes, 1144);
    assert.equal(fixtures.icc8.replyBytes, 48);
    assert.equal(fixtures.icc16.replyBytes, 48);

    const icc8 = await readFile(fixtures.icc8.path);
    assert.equal(icc8.subarray(128, 132).toString("ascii"), "DICM");
    const tags8 = elements(icc8);
    assert.equal(textOf(tags8.get("0002,0010")), "1.2.840.10008.1.2.1");
    assert.equal(usOf(tags8.get("0028,0002")), 3);
    assert.equal(textOf(tags8.get("0028,0004")), "RGB");
    assert.equal(usOf(tags8.get("0028,0006")), 0);
    assert.equal(usOf(tags8.get("0028,0100")), 8);
    assert.equal(usOf(tags8.get("0028,0101")), 8);
    assert.equal(usOf(tags8.get("0028,0102")), 7);
    const profile8 = tags8.get("0028,2000");
    assert.equal(profile8.length, 560);
    assert.equal(sha256(profile8), PROVENANCE.bench_assets["icc_gamma22.icc"].sha256,
      "the 8-bit workload embeds the recorded gamma22 bytes verbatim");
    const pixels8 = tags8.get("7fe0,0010");
    assert.equal(pixels8.length, 48);
    // The markers are neutral and carry the three 8-bit hand codes.
    assert.deepEqual(fixtures.icc8.markers, [[0, 0], [1, 1], [2, 1]]);
    assert.deepEqual(fixtures.icc8.codes, [64, 128, 192]);
    for (const [index, [x, y]] of fixtures.icc8.markers.entries()) {
      const at = (y * 4 + x) * 3;
      assert.deepEqual([...pixels8.subarray(at, at + 3)], [fixtures.icc8.codes[index], fixtures.icc8.codes[index], fixtures.icc8.codes[index]]);
      assert.deepEqual([...pixels8.subarray(at, at + 3)], iccStored("icc8", x, y));
      assert.equal(iccMarkerCode("icc8", x, y), fixtures.icc8.codes[index]);
    }
    // Background pixels are the native F-239 pattern, markers aside.
    assert.deepEqual([...pixels8.subarray((0 * 4 + 3) * 3, (0 * 4 + 3) * 3 + 3)], [...rgbStored(3, 0)]);
    assert.deepEqual([...pixels8.subarray((3 * 4 + 3) * 3, (3 * 4 + 3) * 3 + 3)], [...rgbStored(3, 3)]);

    const icc16 = await readFile(fixtures.icc16.path);
    assert.equal(icc16.subarray(128, 132).toString("ascii"), "DICM");
    const tags16 = elements(icc16);
    assert.equal(usOf(tags16.get("0028,0002")), 3);
    assert.equal(textOf(tags16.get("0028,0004")), "RGB");
    assert.equal(usOf(tags16.get("0028,0006")), 0);
    assert.equal(usOf(tags16.get("0028,0100")), 16);
    assert.equal(usOf(tags16.get("0028,0101")), 16);
    assert.equal(usOf(tags16.get("0028,0102")), 15);
    assert.equal(usOf(tags16.get("0028,0103")), 0);
    const profile16 = tags16.get("0028,2000");
    assert.equal(profile16.length, 560);
    assert.equal(sha256(profile16), PROVENANCE.bench_assets["icc_linear16_table.icc"].sha256,
      "the 16-bit workload embeds the derived two-entry table profile");
    const pixels16 = tags16.get("7fe0,0010");
    assert.equal(pixels16.length, 96);
    assert.deepEqual(fixtures.icc16.markers, [[0, 0], [1, 1], [2, 1], [1, 2], [2, 2]]);
    assert.deepEqual(fixtures.icc16.codes, [15, 16384, 32768, 49152, 65535]);
    for (const [index, [x, y]] of fixtures.icc16.markers.entries()) {
      const at = (y * 4 + x) * 6;
      for (const channel of [0, 1, 2]) {
        assert.equal(pixels16.readUInt16LE(at + channel * 2), fixtures.icc16.codes[index]);
      }
      assert.deepEqual(iccStored("icc16", x, y), [fixtures.icc16.codes[index], fixtures.icc16.codes[index], fixtures.icc16.codes[index]]);
    }
    // The background is native, varied and full range away from the markers.
    assert.deepEqual(icc16Stored(0, 3), [0, 24, 24]);
    assert.deepEqual(icc16Stored(3, 3), [24, 24, 48]);
    assert.deepEqual(icc16Stored(8191, 8191), [65528, 65528, 65520]);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("the stored background is the native pattern across the whole coordinate range", () => {
  // The streamed writer writes channels directly for speed, so bind the fast
  // path back to the spec-level pattern over a spread that crosses the 8-bit
  // wrap and reaches the 16-bit top of range. Marker positions are excluded:
  // those pixels carry the neutral hand codes, not the background.
  const positions = [
    [1, 0], [0, 1], [255, 0], [0, 255], [256, 300], [300, 256],
    [8191, 0], [0, 8191], [4096, 4096], [2731, 5461], [5461, 2731],
    [8191, 8191], [8190, 8191], [8191, 8190], [3, 3],
  ];
  for (const [x, y] of positions) {
    assert.deepEqual(iccStored("icc8", x, y), [...rgbStored(x, y)],
      `the icc8 background at ${x},${y} is not rgbStored's pattern`);
    assert.deepEqual(iccStored("icc16", x, y), [...icc16Stored(x, y)],
      `the icc16 background at ${x},${y} is not the full-range pattern`);
    // And the 16-bit channel values really do span the encoding range.
    for (const value of iccStored("icc16", x, y)) {
      assert.ok(value >= 0 && value <= 65535);
    }
  }
  assert.deepEqual(icc16Stored(8191, 0), [65528, 0, 65528]);
  assert.deepEqual(icc16Stored(0, 8191), [0, 65528, 65528]);
});

test("the derived 16-bit profile rewrites its TRCs and bound profile ID", async () => {
  const gamma1 = await readFile(new URL("./fixtures/f240-icc/icc_gamma1.icc", import.meta.url));
  const derived = linear16TableProfile();
  assert.equal(derived.length, gamma1.length);
  // Every differing byte sits in the regions the provenance record names, so
  // alignment and all other bytes besides the bound profile ID are preserved.
  const differing = [];
  for (let offset = 0; offset < derived.length; offset += 1) {
    if (derived[offset] !== gamma1[offset]) differing.push(offset);
  }
  const expected = new Set();
  for (const [from, to] of PROVENANCE.bench_assets["icc_linear16_table.icc"].changed_byte_regions_vs_icc_gamma1) {
    for (let offset = from; offset <= to; offset += 1) expected.add(offset);
  }
  assert.deepEqual(differing, [...expected].sort((left, right) => left - right),
    "the derivation changed exactly the recorded byte regions");
  // The tag table now names 16-byte TRC tags holding count-2 tables.
  const count = derived.readUInt32BE(128);
  const seen = [];
  for (let index = 0; index < count; index += 1) {
    const entry = 132 + 12 * index;
    const signature = derived.toString("latin1", entry, entry + 4);
    const offset = derived.readUInt32BE(entry + 4);
    const size = derived.readUInt32BE(entry + 8);
    if (!signature.endsWith("TRC")) continue;
    seen.push(signature);
    assert.equal(size, 16, `${signature} tag size`);
    assert.equal(derived.toString("latin1", offset, offset + 4), "curv");
    assert.equal(derived.readUInt32BE(offset + 8), 2, `${signature} entry count`);
    assert.deepEqual([derived.readUInt16BE(offset + 12), derived.readUInt16BE(offset + 14)], [0, 65535], `${signature} entries`);
  }
  assert.deepEqual(seen, ["rTRC", "gTRC", "bTRC"]);
  // The colourants and white point are the gamma1 bytes.
  for (const signature of ["rXYZ", "gXYZ", "bXYZ", "wtpt"]) {
    const entry = 132 + 12 * [["rXYZ", "gXYZ", "bXYZ", "rTRC", "gTRC", "bTRC", "wtpt", "desc", "cprt"].indexOf(signature)];
    const offset = derived.readUInt32BE(entry + 4);
    const size = derived.readUInt32BE(entry + 8);
    assert.deepEqual(derived.subarray(offset, offset + size), gamma1.subarray(offset, offset + size), signature);
  }
});

test("the derived profile ID describes the modified profile under ICC.1 7.2.18", () => {
  const profile = linear16TableProfile();
  const forId = Buffer.from(profile);
  forId.fill(0, 44, 48);
  forId.fill(0, 64, 68);
  forId.fill(0, 84, 100);
  const expected = createHash("md5").update(forId).digest();
  // Independently derived from the frozen gamma1 bytes before this repair.
  assert.equal(expected.toString("hex"), "ef3a144416bbf68ae488f9978642e3b1");
  assert.deepEqual(profile.subarray(84, 100), expected);
  assert.equal(sha256(profile), "94977cfdf14ee323cfc5709c2382942f21f67073f3525a30c3c5d46a7b101158");
});

test("the hand vectors are the windowed display values under R2 and cover the markers", async () => {
  // The copy under the fixtures directory is the task input's bytes.
  assert.equal(
    sha256(await readFile(new URL("./fixtures/f240-icc/hand-vectors.json", import.meta.url))),
    PROVENANCE.bench_assets["hand-vectors.json"].sha256,
  );
  const windows = [IDENTITY_WINDOW, ...WINDOW_STEPS];
  for (const workloadId of ["icc8", "icc16"]) {
    const codes = iccMarkerCodes(workloadId);
    for (const code of codes) {
      const display = iccExpectedDisplay(workloadId, code);
      for (const window of windows) {
        const bytes = iccWindowBytes(workloadId, code, window);
        for (let channel = 0; channel < 3; channel += 1) {
          assert.equal(
            bytes[channel],
            windowedByte(display[channel], window.center, window.width),
            `${workloadId} code ${code} at ${window.center}/${window.width} channel ${channel}`,
          );
        }
      }
    }
    // The markers carry the codes in order and sit inside the small test edge.
    const markers = iccMarkers(workloadId);
    assert.equal(markers.length, codes.length);
    markers.forEach(([x, y], index) => {
      assert.ok(x < 4 && y < 4, `marker ${x},${y} must sit inside the edge-4 test frame`);
      assert.equal(iccMarkerCode(workloadId, x, y), codes[index]);
      assert.deepEqual(iccStored(workloadId, x, y), [codes[index], codes[index], codes[index]]);
    });
  }
  // Every sampled value sits away from a rounding tie. The measured minimum
  // across the marker set is 0.0014883, at 16-bit code 16384 and centre 100,
  // which is about a hundred times the f32 error at these magnitudes.
  let minimum = Infinity;
  for (const workloadId of ["icc8", "icc16"]) {
    for (const code of iccMarkerCodes(workloadId)) {
      for (const value of iccExpectedDisplay(workloadId, code)) {
        for (const window of windows) {
          const lower = window.center - 0.5 - (window.width - 1) / 2;
          const upper = window.center - 0.5 + (window.width - 1) / 2;
          if (value <= lower || value > upper) continue;
          const scaled = ((value - (window.center - 0.5)) / (window.width - 1) + 0.5) * 255;
          const tie = Math.floor(scaled) + 0.5;
          minimum = Math.min(minimum, Math.abs(scaled - tie));
        }
      }
    }
  }
  assert.ok(minimum >= 0.001, `a sampled value sits ${minimum} from a tie`);
});

test("iccSampleChecks names the marker offsets and the hand bytes", () => {
  const at = (x, y) => (y * EDGE + x) * 3;
  const atMarker = (checks, x, y) =>
    checks.filter((check) => check.offset >= at(x, y) && check.offset < at(x, y) + 3)
      .map((check) => check.expected);
  const identity = iccSampleChecks("icc16", IDENTITY_WINDOW);
  assert.equal(identity.length, 15);
  assert.deepEqual(identity.slice(0, 3), [{ offset: 0, expected: 1 }, { offset: 1, expected: 1 }, { offset: 2, expected: 1 }]);
  assert.deepEqual(atMarker(identity, 2, 2), [255, 255, 255]);
  assert.deepEqual(atMarker(identity, 1, 1), [137, 137, 137]);
  // The nine steps come from the same rows.
  const stepped = iccSampleChecks("icc16", { center: 108, width: 200 });
  assert.deepEqual(atMarker(stepped, 1, 1), [165, 165, 165]);
  assert.deepEqual(atMarker(stepped, 0, 0), [0, 0, 0]);
  // The 8-bit workload samples its three markers.
  const icc8 = iccSampleChecks("icc8", IDENTITY_WINDOW);
  assert.equal(icc8.length, 9);
  assert.deepEqual(icc8.slice(0, 3), [{ offset: 0, expected: 62 }, { offset: 1, expected: 62 }, { offset: 2, expected: 62 }]);
  assert.deepEqual(atMarker(iccSampleChecks("icc8", { center: 100, width: 200 }), 0, 0), [79, 79, 79]);
  assert.deepEqual(atMarker(iccSampleChecks("icc8", { center: 100, width: 200 }), 2, 1), [248, 248, 248]);
  // Every marker survives the small test edge, code 15 and the high codes
  // included, and every offset is inside the reply.
  for (const workloadId of ["icc8", "icc16"]) {
    const tiny = iccSampleChecks(workloadId, IDENTITY_WINDOW, 4);
    assert.ok(tiny.every((check) => check.offset < 48));
    assert.equal(tiny.length, workloadId === "icc8" ? 9 : 15);
  }
  refuses(() => iccSampleChecks("icc16", { center: 7, width: 9 }), "no hand vector window");
  refuses(() => iccSampleChecks("icc7", IDENTITY_WINDOW), "unknown ICC frame workload");
});

test("sample bytes and TRC tables fix the held census minimum", () => {
  assert.equal(iccSampleBytes("icc8"), 8192 * 8192 * 3);
  assert.equal(iccSampleBytes("icc16"), 8192 * 8192 * 3 * 2);
  assert.equal(iccSampleBytes("icc8", 4), 48);
  assert.equal(iccSampleBytes("icc16", 4), 96);
  // 3 * 2^bits * 4 bytes of table per parsed profile.
  assert.equal(iccTrcTableBytes("icc8"), 3 * 256 * 4);
  assert.equal(iccTrcTableBytes("icc8"), 3072);
  assert.equal(iccTrcTableBytes("icc16"), 3 * 65536 * 4);
  assert.equal(iccTrcTableBytes("icc16"), 786432);
  refuses(() => iccSampleBytes("icc7"), "unknown ICC frame workload");
  refuses(() => iccTrcTableBytes("icc7"), "unknown ICC frame workload");
});

test("the fake reply variants model the wrong implementations the controls reject", () => {
  const identity = { center: 128, width: 256 };
  // The default variant is the hand vector's byte.
  assert.equal(iccReplyByte("icc8", 0, 0, 0, identity), 62);
  assert.equal(iccReplyByte("icc16", 0, 0, 0, identity), 1);
  // Unprofiled replies the device byte windowed: the raw sample, or the
  // 16-bit value shifted down, never the profile.
  assert.equal(iccReplyByte("icc8", 0, 0, 0, identity, "unprofiled"), 64);
  assert.equal(iccReplyByte("icc16", 0, 0, 0, identity, "unprofiled"), 0);
  assert.equal(iccReplyByte("icc16", 2, 2, 0, identity, "unprofiled"), 255);
  // Wrongly 8-bit indexed 16-bit replies. Under the identity window only the
  // code 15 marker tells the wrong indexing from the hand bytes, and it does:
  // 0 against 1.
  assert.equal(iccReplyByte("icc16", 0, 0, 0, identity, "indexed8"), 0);
  assert.equal(iccReplyByte("icc16", 1, 1, 0, identity, "indexed8"), 137);
  assert.equal(iccReplyByte("icc16", 2, 2, 0, identity, "indexed8"), 255);
  // Non-marker pixels reply 0 in every variant, because the runner samples
  // the markers only.
  for (const variant of ["icc", "unprofiled", "indexed8"]) {
    assert.equal(iccReplyByte("icc16", 3, 0, 0, identity, variant), 0);
  }
  refuses(() => iccReplyByte("icc8", 0, 0, 0, identity, "indexed8"), "no 16-bit indexing to get wrong");
  refuses(() => iccReplyByte("icc8", 0, 0, 0, identity, "nonsense"), "unknown marker byte variant");
});

test("the ICC metadata, budget and census guards refuse their wrong inputs", () => {
  assertIccApplied("icc8", { icc_applied: true, icc_kind: "" });
  refuses(() => assertIccApplied("icc8", {}), "does not carry icc_applied true");
  refuses(() => assertIccApplied("icc8", { icc_applied: false, icc_kind: "" }), "does not carry icc_applied true");
  refuses(() => assertIccApplied("icc8", { icc_applied: true, icc_kind: "lookup-table" }), "icc_kind");

  assertHeldBudget({ id: "icc16", cacheMiB: 512 }, 512 * 1048576);
  refuses(
    () => assertHeldBudget({ id: "icc16", cacheMiB: 512 }, 268435456),
    "requested OCELLI_CACHE_MIB=512",
  );
  // No explicit request, no refusal: inherited budgets are recorded only.
  assertHeldBudget({ id: "icc8", cacheMiB: null }, 123);
  assert.equal(requestedCacheMib({}), null);
  assert.equal(requestedCacheMib({ cacheMiB: null }), null);
  assert.equal(requestedCacheMib({ cacheMiB: 512 }), 512);
  assert.deepEqual(startEnv({}), {});
  assert.deepEqual(startEnv({ cacheMiB: null }), {});
  assert.deepEqual(startEnv({ cacheMiB: 512 }), { OCELLI_CACHE_MIB: "512" });

  const census = { used: 786528, frames: 1 };
  assertHeldCensus({ id: "icc16" }, census, 786528, "after the first decode");
  refuses(
    () => assertHeldCensus({ id: "icc16" }, { used: 786527, frames: 1 }, 786528, "after the first decode"),
    "below the 786528 bytes of decoded samples plus TRC tables",
  );
  refuses(() => assertHeldCensus({ id: "icc16" }, { frames: 1 }, 786528, "after the first decode"), "no used byte count");
  refuses(() => assertHeldCensus({ id: "icc16" }, { used: 786528 }, 786528, "after the first decode"), "no frame count");

  assertCensusStable({ id: "icc16" }, { used: 10, frames: 1 }, { used: 10, frames: 1 });
  refuses(() => assertCensusStable({ id: "icc16" }, { used: 10, frames: 1 }, { used: 11, frames: 1 }), "held byte count moved across the warm draws");
  refuses(() => assertCensusStable({ id: "icc16" }, { used: 10, frames: 1 }, { used: 10, frames: 2 }), "held frame count moved across the warm draws");
});

// A real tiny ICC child, the existing fake-viewer pattern with the two
// ICC workloads: it applies the requested env to its reported budget, keeps
// icc_applied true in every mode, and writes its request trace on SIGTERM.
async function fakeIccViewer(dir, workloadId, mode) {
  const binary = join(dir, "fake-icc-viewer.mjs");
  const trace = join(dir, "viewer-trace.json");
  const helperModule = new URL("../src/viewer_icc_frame_fixture.mjs", import.meta.url).href;
  const script = `#!/usr/bin/env node
import http from "node:http";
import { writeFileSync } from "node:fs";
import { iccReplyByte, iccSampleBytes, iccTrcTableBytes } from ${JSON.stringify(helperModule)};
const workload = ${JSON.stringify(workloadId)};
const mode = ${JSON.stringify(mode)};
const requests = [];
let previews = 0;
function budget() {
  if (mode === "wrong-budget") return 268435456;
  const mib = Number(process.env.OCELLI_CACHE_MIB);
  return Number.isSafeInteger(mib) && mib > 0 ? mib * 1048576 : 268435456;
}
function heldBody() {
  const base = iccSampleBytes(workload, 4) + iccTrcTableBytes(workload) + 64;
  const used = mode === "held-undercount" ? Math.floor(iccTrcTableBytes(workload) / 2)
    : mode === "held-growth" ? base + previews * 4096 : base;
  return JSON.stringify({ build: "release", budget: budget(), used, files: 1, frames: 1, inflight: 0, workspace: 0, free: 0, grey: 0, grey_replies: 0, kept: 0 });
}
const variant = mode === "unprofiled-bytes" ? "unprofiled" : mode === "indexed8-bytes" ? "indexed8" : "icc";
const server = http.createServer((request, response) => {
  let bytes = 0;
  request.on("data", chunk => { bytes += chunk.length; });
  request.on("end", () => {
    requests.push({ method: request.method, path: request.url, bytes, file: request.headers["x-ocelli-file"] || null });
    if (request.url === "/held") {
      response.end(heldBody());
    } else if (request.url === "/file") {
      response.writeHead(200, { "X-Ocelli-File": "2.b", "X-Ocelli-Held": "1" });
      response.end(JSON.stringify({ colour: true, monochrome: false, rows: 4, columns: 4, icc: true, icc_applied: true, icc_kind: "" }));
    } else if (request.url === "/preview") {
      const drawn = previews++;
      const source = drawn === 0 ? "decoded" : "held";
      const window = { center: Number(request.headers["x-ocelli-window-center"]), width: Number(request.headers["x-ocelli-window-width"]) };
      const body = Buffer.alloc(48);
      for (let y = 0; y < 4; y++) for (let x = 0; x < 4; x++) for (let c = 0; c < 3; c++) body[(y * 4 + x) * 3 + c] = iccReplyByte(workload, x, y, c, window, variant);
      response.writeHead(200, { "X-Ocelli-Source": source, "X-Ocelli-Channels": "3", "X-Ocelli-Width": "4", "X-Ocelli-Height": "4", "X-Ocelli-Window-Center": String(window.center), "X-Ocelli-Window-Width": String(window.width) });
      response.end(body);
    } else { response.writeHead(404); response.end(); }
  });
});
server.listen(0, "127.0.0.1", () => console.log("http://127.0.0.1:" + server.address().port + "/"));
process.on("SIGTERM", () => {
  writeFileSync(${JSON.stringify(trace)}, JSON.stringify({ pid: process.pid, requests }));
  server.closeAllConnections();
  server.close(() => process.exit(0));
});
`;
  await writeFile(binary, script);
  await chmod(binary, 0o755);
  return { binary, trace };
}

for (const workloadId of ["icc8", "icc16"]) {
  test(`the ICC workload applies the profile and holds it across the warm draws: ${workloadId}`, async () => {
    const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-run-"));
    try {
      const fixtures = await writeIccFrameFixtures(dir, 4);
      const spec = fixtures[workloadId];
      const actor = await fakeIccViewer(dir, workloadId, "normal");
      const result = await measureWorkload({ binary: actor.binary, spec, sampleRss: tinyRss });
      assert.equal(result.first_source, "decoded");
      assert.deepEqual(result.stepSources, Array(9).fill("held"));
      assert.equal(result.stepSamples.length, 9);
      // The profile was applied, not merely present.
      assert.equal(result.postReport.icc_applied, true);
      assert.equal(result.postReport.icc_kind, "");
      // The census holds the samples plus the TRC tables, and does not move.
      assert.equal(result.minimumHeldBytes, iccSampleBytes(workloadId, 4) + iccTrcTableBytes(workloadId));
      assert.ok(result.censusAfterFirst.used >= result.minimumHeldBytes);
      assert.equal(result.censusAfterFirst.frames, 1);
      assert.equal(result.censusAfterFirst.used, result.censusAfterWindows.used);
      assert.equal(result.censusAfterFirst.frames, result.censusAfterWindows.frames);
      // Only the 16-bit workload asked for a budget, and its child ran with it.
      assert.equal(requestedCacheMib(spec), workloadId === "icc16" ? 512 : null);
      if (workloadId === "icc16") {
        assert.equal(result.heldAfterPost.budget, 512 * 1048576);
      }
      const trace = JSON.parse(await readFile(actor.trace, "utf8"));
      const posts = trace.requests.filter(entry => entry.method === "POST");
      assert.equal(posts[0].path, "/file");
      assert.equal(posts[0].bytes, spec.fileBytes);
      assert(posts.slice(1).every(entry => entry.path === "/preview" && entry.bytes === 0 && entry.file === "2.b"));
      assert.equal(posts.length, 11);
      assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the run");
    } finally { await rm(dir, { recursive: true, force: true }); }
  });
}

test("unprofiled marker bytes are refused while the metadata stays applied", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-neg1-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    const actor = await fakeIccViewer(dir, "icc8", "unprofiled-bytes");
    const action = measureWorkload({ binary: actor.binary, spec: fixtures.icc8, sampleRss: tinyRss });
    // icc_applied stayed true and the headers stayed right, so the refusal is
    // the sample checker rejecting the bytes, not a missing flag: the device
    // byte 64 is not the profile's 62 at the first marker.
    await rejectsMessage(action, "the reply byte at offset 0 was 64, expected 62");
    const trace = JSON.parse(await readFile(actor.trace, "utf8"));
    const posts = trace.requests.filter(entry => entry.method === "POST");
    assert.equal(posts[0].path, "/file");
    assert.equal(posts[0].bytes, fixtures.icc8.fileBytes);
    assert.equal(posts[1].path, "/preview");
    assert.equal(posts[1].bytes, 0);
    assert.equal(posts.length, 2, "no window step and no re-inspection after the refusal");
    assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the refusal");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("wrongly 8-bit indexed 16-bit marker bytes are refused", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-neg2-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    const actor = await fakeIccViewer(dir, "icc16", "indexed8-bytes");
    const action = measureWorkload({ binary: actor.binary, spec: fixtures.icc16, sampleRss: tinyRss });
    // A 16-bit input indexed by its high byte replies 0 at code 15, against
    // the hand vector's 1, with every header and the metadata still correct.
    await rejectsMessage(action, "the reply byte at offset 0 was 0, expected 1");
    const trace = JSON.parse(await readFile(actor.trace, "utf8"));
    assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the refusal");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("a child that ignores the requested budget is refused", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-neg3-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    const actor = await fakeIccViewer(dir, "icc16", "wrong-budget");
    const action = measureWorkload({ binary: actor.binary, spec: fixtures.icc16, sampleRss: tinyRss });
    await rejectsMessage(action, "requested OCELLI_CACHE_MIB=512");
    const trace = JSON.parse(await readFile(actor.trace, "utf8"));
    assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the refusal");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("per-draw held growth across the warm draws is refused", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-neg4-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    const actor = await fakeIccViewer(dir, "icc16", "held-growth");
    const action = measureWorkload({ binary: actor.binary, spec: fixtures.icc16, sampleRss: tinyRss });
    await rejectsMessage(action, "held byte count moved across the warm draws");
    const trace = JSON.parse(await readFile(actor.trace, "utf8"));
    assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the refusal");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("a held census below samples plus TRC tables is refused", async () => {
  const dir = await mkdtemp(join(tmpdir(), "ocelli-colour-icc-neg5-"));
  try {
    const fixtures = await writeIccFrameFixtures(dir, 4);
    const actor = await fakeIccViewer(dir, "icc8", "held-undercount");
    const action = measureWorkload({ binary: actor.binary, spec: fixtures.icc8, sampleRss: tinyRss });
    await rejectsMessage(action, "below the 3120 bytes of decoded samples plus TRC tables");
    const trace = JSON.parse(await readFile(actor.trace, "utf8"));
    assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH", "owned child exited after the refusal");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

// Census controls for every refusal in the AC31 measurement boundary.
// These use tiny HTTP reports and exact fixed metadata, never a measurement.
test("validatePreview refuses a wrong source, missing or wrong numeric headers and missed checks", () => {
  const checks = [{ offset: 0, expected: 0 }];
  const options = { checks, channels: 3, replyBytes: 12, edge: 2,
    window: IDENTITY_WINDOW, label: "control", source: "held" };
  const good = { status: 200, headers: fakeReplyHeaders(IDENTITY_WINDOW,
    { "x-ocelli-source": "held" }), bodyBytes: 12, validated: 1 };
  validatePreview(good, options);
  refuses(() => validatePreview({ ...good, headers: { ...good.headers,
    "x-ocelli-source": "decoded" } }, options), "expected held");
  for (const name of ["x-ocelli-width", "x-ocelli-height",
    "x-ocelli-window-center", "x-ocelli-window-width"]) {
    const missing = { ...good.headers };
    delete missing[name];
    refuses(() => validatePreview({ ...good, headers: missing }, options), `replied no ${name}`);
    refuses(() => validatePreview({ ...good, headers: { ...good.headers,
      [name]: "999" } }, options), `replied ${name}`);
  }
  refuses(() => validatePreview({ ...good, validated: 0 }, options), "validated 0 of 1 samples");
});

test("holdFile refuses an HTTP failure and every non-object JSON report", async () => {
  for (const [status, body, words] of [
    [400, "refused", "refused to hold the file"],
    [200, "null", "the file report is not an object"],
    [200, "[]", "the file report is not an object"],
    [200, '"text"', "the file report is not an object"],
  ]) {
    const { server, port } = await listen((request, response) => {
      response.writeHead(status, { "x-ocelli-file": "5.a", "x-ocelli-held": "1" });
      response.end(body);
    });
    try { await rejectsMessage(holdFile({ port, bodyPath: null, bodyLength: null }), words); }
    finally { await close(server); }
  }
});

test("fixture controls refuse an unknown workload, odd long value and stalled write", async () => {
  refuses(() => expectedSamples("not-a-workload", 0, 0), "unknown colour frame workload");
  // These two defensive leaf guards have no invalid public caller: the
  // fixture builder always uses even payload lengths and real FileHandles.
  // Export them only from a test-owned module copy, preserving production API.
  const fixtureUrl = new URL("../src/viewer_colour_frame_fixture.mjs", import.meta.url);
  const original = await readFile(fixtureUrl, "utf8");
  const localImport = 'from "./viewer_work.mjs"';
  assert.equal(original.split(localImport).length, 2);
  const source = original.replace(localImport,
    `from ${JSON.stringify(new URL("../src/viewer_work.mjs", import.meta.url).href)}`)
    + "\nexport { longHeader, writeAll };\n";
  const { longHeader, writeAll } = await import(
    `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
  // PS3.5 7.1.2 requires even Value Length. The zero/even control remains valid.
  assert.equal(longHeader(0x7fe0, 0x0010, "OB", 2).length, 12);
  refuses(() => longHeader(0x7fe0, 0x0010, "OB", 1), "a long element has a bad length");
  const writes = [];
  await writeAll({ async write(buffer, offset, length) {
    writes.push([offset, length]); return { bytesWritten: Math.min(2, length) };
  } }, Buffer.from([1, 2, 3]));
  assert.deepEqual(writes, [[0, 3], [2, 1]]);
  for (const bytesWritten of [0, -1]) {
    await rejectsMessage(writeAll({ async write() { return { bytesWritten }; } },
      Buffer.from([1])), "the fixture write made no progress");
  }
});

// F-240 review D1. Observe actual HTTP order for both established AC31
// workloads. ICC-only census traffic must not enter their measured sequence.
for (const workloadId of ["rgb8", "palette16"]) {
  test(`untagged AC31 HTTP protocol remains unchanged: ${workloadId}`, async () => {
    const dir = await mkdtemp(join(tmpdir(), "ocelli-ac31-order-"));
    try {
      const fixtures = await writeColourFrameFixtures(dir, 4);
      const actor = await fakeColourViewer(dir, "normal", workloadId);
      const result = await measureWorkload({ binary: actor.binary,
        spec: fixtures[workloadId], sampleRss: tinyRss });
      const trace = JSON.parse(await readFile(actor.trace, "utf8"));
      // F-239's established wire protocol: build/start metadata, untimed
      // ingestion and its metadata, first decode plus nine held previews,
      // then end metadata. No GET is inserted among the measured replies.
      assert.deepEqual(trace.requests.map(entry => `${entry.method} ${entry.path}`), [
        "GET /held", "GET /held", "POST /file", "GET /held",
        ...Array(10).fill("POST /preview"), "GET /held",
      ]);
      assert.equal(result.censusAfterFirst, null);
      assert.equal(result.censusAfterWindows, null);
      assert.throws(() => process.kill(trace.pid, 0), error => error.code === "ESRCH");
    } finally { await rm(dir, { recursive: true, force: true }); }
  });
}
