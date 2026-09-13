import assert from "node:assert/strict";
import test from "node:test";

import { parsePixelRate } from "../src/runners/tier_startup_microbenchmark.mjs";

test("fill-rate runner accepts the production probe spelling", () => {
  assert.equal(parsePixelRate("pixels per second 123456\n"), 123456);
});

test("fill-rate runner refuses absent evidence", () => {
  assert.throws(() => parsePixelRate("no adapter\n"), /no positive pixel rate/);
});
