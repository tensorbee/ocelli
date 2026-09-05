// Unit tests for the version walk that the pin check depends on.
//
// Every dependency of the reference is version-checked, and six of the sixteen
// are reached by this walk because they expose no `./package.json`. A walk that
// read the wrong file would record a version nobody installed, which reads as a
// check and is not one.
//
// The trap is real and is present in the tree, in five of the
// `@cornerstonejs/*` packages: a `dist/esm/package.json` carrying nothing but
// `{"type": "module"}`. Those five are reached by the direct route today and
// none of the six that walk contains one, so the trap is built here rather
// than met in production. It would be met the day an exports map moves a
// package from one route to the other.

import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { versionFromPackageRoot } from "../src/pins.mjs";

/**
 * A package tree shaped like the ones this actually walks:
 *
 *   <root>/package.json          {"name": "@scope/thing", "version": "1.2.3"}
 *   <root>/dist/esm/package.json {"type": "module"}
 *   <root>/dist/esm/index.js
 */
async function tree(name, version, { moduleMarker = true } = {}) {
  const base = await mkdtemp(join(tmpdir(), "ocelli-pins-test-"));
  const root = join(base, "node_modules", name);
  const inner = join(root, "dist", "esm");
  await mkdir(inner, { recursive: true });
  await writeFile(
    join(root, "package.json"),
    JSON.stringify({ name, version }),
  );
  if (moduleMarker) {
    await writeFile(
      join(inner, "package.json"),
      JSON.stringify({ type: "module" }),
    );
  }
  return { base, root, inner };
}

test("the version comes from the manifest that names the package", async () => {
  const { base, root, inner } = await tree("thing", "1.2.3");
  try {
    assert.equal(versionFromPackageRoot(inner, "thing"), "1.2.3");
    assert.equal(versionFromPackageRoot(root, "thing"), "1.2.3");
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// The whole point. `dist/esm/package.json` is found FIRST on the way up and
// has neither a name nor a version. A walk that took it would record
// `undefined` as the installed version of a package that decides pixels.
test("a type-module marker on the way up is walked past, not read", async () => {
  const { base, inner } = await tree("thing", "1.2.3");
  try {
    assert.equal(versionFromPackageRoot(inner, "thing"), "1.2.3");
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// A scoped package sits one directory deeper, and the manifest that names it
// is not the first one with a name either: `node_modules/@scope` has none, but
// a sibling package's manifest could be reached by a walk that ignored names.
test("a scoped package finds its own manifest and not a neighbour's", async () => {
  const { base, inner } = await tree("@cornerstonejs/codec-charls", "1.2.5");
  try {
    await writeFile(
      join(base, "node_modules", "package.json"),
      JSON.stringify({ name: "@cornerstonejs/codec-charls", version: "9.9.9" }),
    );
    assert.equal(
      versionFromPackageRoot(inner, "@cornerstonejs/codec-charls"),
      "1.2.5",
      "the nearer manifest wins, so a walk cannot climb past its own package",
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("a manifest naming a different package is not accepted", async () => {
  const { base, inner } = await tree("thing", "1.2.3");
  try {
    assert.throws(
      () => versionFromPackageRoot(inner, "other-thing"),
      /no package.json naming other-thing/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// Not "returns undefined". A manifest with the right name and no version is
// not an answer, and treating it as one would record `undefined` as a pin.
test("a manifest with the right name and no version is not an answer", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-pins-test-"));
  try {
    const root = join(base, "thing");
    await mkdir(root, { recursive: true });
    await writeFile(join(root, "package.json"), JSON.stringify({ name: "thing" }));
    assert.throws(() => versionFromPackageRoot(root, "thing"), /no package.json naming/);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("a walk that reaches nothing terminates rather than looping", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-pins-test-"));
  try {
    assert.throws(
      () => versionFromPackageRoot(base, "nothing-is-here"),
      /no package.json naming nothing-is-here/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});
