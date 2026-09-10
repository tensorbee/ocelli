import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  parseDecodeFrame,
  requireReleaseBinary,
} from "../src/runners/decode_frame.mjs";

test("decode frame result retains the measured value and evidence", () => {
  const result = parseDecodeFrame(
    '{"value":0.25,"iterations":31,"warmup_iterations":1,' +
      '"range_ms":[0.2,0.3],"checksum":42}',
  );
  assert.equal(result.value, 0.25);
  assert.deepEqual(result.range_ms, [0.2, 0.3]);
  assert.equal(result.checksum, 42);
});

test("decode frame result refuses an absent measurement", () => {
  assert.throws(
    () => parseDecodeFrame(
      '{"value":0,"iterations":31,"range_ms":[0,0],"checksum":42}',
    ),
    /positive finite duration/,
  );
});

test("decode frame result refuses incomplete measurement evidence", () => {
  const invalid = [
    [
      '{"value":0.25,"iterations":0,"range_ms":[0.2,0.3],"checksum":42}',
      /iteration count/,
    ],
    [
      '{"value":0.25,"iterations":31,"range_ms":[0.2],"checksum":42}',
      /finite observed range/,
    ],
    [
      '{"value":0.25,"iterations":31,"range_ms":[0.2,0.3],"checksum":0}',
      /output checksum/,
    ],
  ];

  for (const [record, refusal] of invalid) {
    assert.throws(() => parseDecodeFrame(record), refusal);
  }
});

test("decode frame no-build mode requires the release executable", async () => {
  const directory = await mkdtemp(join(tmpdir(), "ocelli-decode-frame-"));
  try {
    await assert.rejects(
      requireReleaseBinary(join(directory, "absent")),
      /release executable and refuses a substitute/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
