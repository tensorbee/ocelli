// Unit tests for the page server's path resolution.
//
// The server exists only to give the render page an HTTP origin, and only the
// harness's own browser talks to it. That is exactly why the containment check
// needs a test: nothing in normal operation will ever exercise it, so without
// one it is a guard nobody has watched fail.

import test from "node:test";
import assert from "node:assert/strict";
import { sep } from "node:path";

import { resolveServedPath } from "../src/server.mjs";

const BASE = `${sep}srv${sep}page${sep}dist`;

test("the root serves index.html", () => {
  assert.equal(resolveServedPath(BASE, "/"), `${BASE}${sep}index.html`);
});

test("a file inside the directory resolves under it", () => {
  assert.equal(resolveServedPath(BASE, "/app.js"), `${BASE}${sep}app.js`);
  assert.equal(
    resolveServedPath(BASE, "/wasm/charlswasm_decode.wasm"),
    `${BASE}${sep}wasm${sep}charlswasm_decode.wasm`,
  );
});

test("a path that walks out of the served directory is refused", () => {
  assert.equal(resolveServedPath(BASE, "/../../etc/passwd"), null);
  assert.equal(resolveServedPath(BASE, "/a/../../../etc/passwd"), null);
  assert.equal(resolveServedPath(BASE, "/wasm/../../secret"), null);
});

// The check is made against the resolved path, so a percent-encoded traversal
// is refused after decoding rather than passed through because it did not look
// like one before.
test("a percent-encoded traversal is refused after decoding", () => {
  assert.equal(resolveServedPath(BASE, "/..%2F..%2Fetc%2Fpasswd"), null);
});

// A malformed escape would make decodeURIComponent throw inside the request
// handler, and an uncaught throw there takes the process down.
test("a malformed percent escape is refused rather than thrown", () => {
  assert.equal(resolveServedPath(BASE, "/%zz"), null);
  assert.equal(resolveServedPath(BASE, "/%"), null);
});

// A directory that merely SHARES a prefix with the served one is not inside
// it. `/srv/page/dist-old` starts with `/srv/page/dist`.
test("a sibling directory sharing a name prefix is refused", () => {
  assert.equal(resolveServedPath(BASE, "/../dist-old/app.js"), null);
});

test("the served directory itself resolves to itself", () => {
  assert.equal(resolveServedPath(BASE, "/."), BASE);
});
