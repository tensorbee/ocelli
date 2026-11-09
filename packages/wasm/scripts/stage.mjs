#!/usr/bin/env node
// Stage the wasm-pack outputs into this package and write the build id.
//
// npm pack and npm publish run this as the prepack lifecycle script, unless
// scripts are ignored, which packs whatever dist already holds without a
// rebuild. The build goes through bin/ocelli.sh, which is the one definition
// of the wasm-pack build that CI and a developer both get.
//
// dist/build-id.json records the package version, the git SHA of the HEAD the
// modules were built from, the git write-tree of the staged index, and
// whether the working tree has unstaged tracked changes. A release compares
// that SHA and that tree to the tag's, and the digests to a cold build of the
// same source, per docs/RELEASE.md item 6. The packages gate refuses a build
// id recorded with unstaged tracked changes, because cargo compiles the
// working tree, so those changes are in the bytes and in neither recorded
// name. A staged-but-uncommitted change is recorded as the tree it is, which
// is what lets /verify pack on a staged tree.
// The wasm build is byte-reproducible for a given SHA on one toolchain, which
// is what makes the digest comparison meaningful. Measured 2026-11-07: two
// cold builds of the same tree and one warm rebuild agree on every byte of
// both modules.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const root = path.resolve(packageDir, "..", "..");
const renderPkg = path.join(root, "crates", "ocelli-wasm", "pkg");
const decodePkg = path.join(root, "crates", "ocelli-wasm", "pkg-decode");
const dist = path.join(packageDir, "dist");

// Both out-dirs come from one build each. The render module is the default
// build, the decode module is the no-renderer split.
for (const command of ["wasm", "wasm-decode"]) {
  execFileSync(path.join(root, "bin", "ocelli.sh"), [command], {
    cwd: root,
    stdio: "inherit",
  });
}

// The four files wasm-pack emits per module. The glue and its types travel
// with the module they load, and both land beside each other, so the glue's
// relative fetch of the .wasm keeps working.
const GLUE = ["ocelli_wasm.js", "ocelli_wasm_bg.wasm", "ocelli_wasm.d.ts",
              "ocelli_wasm_bg.wasm.d.ts"];

rmSync(dist, { recursive: true, force: true });
mkdirSync(path.join(dist, "decode"), { recursive: true });
for (const name of GLUE) {
  copyFileSync(path.join(renderPkg, name), path.join(dist, name));
  copyFileSync(path.join(decodePkg, name), path.join(dist, "decode", name));
}
// The two third-party files must sit at the tarball's root for the packages
// gate to find them, so they are staged beside the manifest rather than in
// dist. They are generated from the repository root, never committed here.
for (const name of ["THIRD_PARTY_LICENSES", "THIRD_PARTY_NOTICES"]) {
  copyFileSync(path.join(root, name), path.join(packageDir, name));
}

const digest = (relative) =>
  createHash("sha256").update(readFileSync(path.join(dist, relative)))
    .digest("hex");
// Unstaged tracked modifications only. dist and the pkg out-dirs are
// untracked build artefacts, and a build that wrote them is the normal build,
// not a dirty tree. `git diff` compares the working tree with the index, so a
// staged-but-uncommitted change is not dirty: the index it was staged into is
// recorded below as the tree.
const dirty = execFileSync(
  "git", ["diff", "--name-only"], { cwd: root },
).toString().trim().length > 0;
const buildId = {
  version: JSON.parse(readFileSync(path.join(packageDir, "package.json")))
    .version,
  git_sha: execFileSync("git", ["rev-parse", "HEAD"], { cwd: root })
    .toString()
    .trim(),
  tree: execFileSync("git", ["write-tree"], { cwd: root })
    .toString()
    .trim(),
  dirty,
  modules: {
    "dist/ocelli_wasm_bg.wasm": digest("ocelli_wasm_bg.wasm"),
    "dist/decode/ocelli_wasm_bg.wasm": digest("decode/ocelli_wasm_bg.wasm"),
  },
};
writeFileSync(path.join(dist, "build-id.json"),
              JSON.stringify(buildId, null, 2) + "\n");
