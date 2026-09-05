// Build the static reference page from the installed cornerstone3D packages.
//
// The plan's step 2 says the render page is static. It is built here, once per
// run, rather than served by a dev server, so the bytes the browser executes
// are a file on disk that can be inspected after a divergence.
//
// Three things have to end up in `page/dist`:
//
// 1. `app.js`, the bundled page entry.
// 2. `decodeImageFrameWorker.js`, under exactly that name and beside `app.js`.
//    `@cornerstonejs/dicom-image-loader`'s `init` starts its decode worker with
//    `new Worker(new URL('./decodeImageFrameWorker.js', import.meta.url))`, and
//    `import.meta.url` inside the bundle is the bundle's own URL.
// 3. The four codec `.wasm` binaries, under the names the decoders ask
//    `locateFile` for. The decoders resolve them against the `wasmBasePath`
//    the page passes to `init`, so they are looked up by name and not by the
//    bare-specifier `new URL` the sources fall back to.
//
// Nothing here is committed: `.gitignore`'s `dist/` rule matches at any depth,
// so `tools/oracle/page/dist` is ignored, and the whole tree is rebuilt from
// `node_modules` on every run.

import { mkdir, rm, copyFile, readFile } from "node:fs/promises";
import { builtinModules, createRequire } from "node:module";
import { dirname, join } from "node:path";

import * as esbuild from "esbuild";

import { isEntryPoint, oraclePath } from "./src/paths.mjs";

const require = createRequire(import.meta.url);

export const PAGE_SOURCE = oraclePath("page");
export const PAGE_DIST = oraclePath("page", "dist");

/**
 * Codec wasm binaries, keyed by the file name `locateFile` is asked for.
 *
 * These binaries decide the decoded pixels for every JPEG, JPEG-LS, JPEG 2000
 * and HTJ2K row. `run.mjs` pins the four packages by version in
 * `PINNED_CODECS`, which is a second list of the same four things, so
 * `checkPins` compares the two: a codec copied into the page and not pinned,
 * or pinned and not copied, is refused. `codecPackages` below is what makes
 * the comparison possible, by reducing each subpath specifier to its package
 * name.
 */
export const CODEC_WASM = {
  "charlswasm_decode.wasm": "@cornerstonejs/codec-charls/decodewasm",
  "openjpegwasm_decode.wasm": "@cornerstonejs/codec-openjpeg/decodewasm",
  "libjpegturbowasm_decode.wasm": "@cornerstonejs/codec-libjpeg-turbo-8bit/decodewasm",
  "openjphjs.wasm": "@cornerstonejs/codec-openjph/wasm",
};

/**
 * The four codec package names, from the specifiers above.
 *
 * A scoped subpath specifier is `@scope/name/subpath`, so the package name is
 * the first TWO segments. An unscoped one is `name/subpath`, so it is the
 * first. Taking two unconditionally would fail closed, because the resulting
 * non-existent name matches no pin, but the refusal would then name a package
 * that does not exist. Derived rather than written down again, because a third
 * hand-maintained list of the same four packages is what this exists to
 * prevent.
 */
export function packageNameOf(specifier) {
  const segments = specifier.split("/");
  return segments.slice(0, segments[0].startsWith("@") ? 2 : 1).join("/");
}

export function codecPackages() {
  return Object.values(CODEC_WASM).map(packageNameOf).sort();
}

// The loader's `init` starts its worker with
// `new Worker(new URL('./decodeImageFrameWorker.js', import.meta.url))`, so
// the built worker has to carry exactly that name beside `app.js`. The source
// is not one of the ten subpaths the package exports, so it is reached through
// the one of them that locates the package root, `./package.json`.
const WORKER_ENTRY = join(
  dirname(require.resolve("@cornerstonejs/dicom-image-loader/package.json")),
  "dist",
  "esm",
  "decodeImageFrameWorker.js",
);

// The four codec glue files are Emscripten output built for both node and the
// browser. Their node branch is dead here, but it still names `fs`, `path` and
// friends, and esbuild resolves an import before it works out that nothing
// reaches it. Stubbing them empties the dead branch rather than shipping a
// polyfill for a code path the browser never enters. If one of these ever IS
// entered, the failure is an immediate `undefined is not a function` at the
// call site, not a silently wrong pixel.
const EMPTY_STUB = "module.exports = {};";

// `events` is the one that cannot be empty. `xmlbuilder2`, which vtk.js pulls
// in, does `class XMLBuilderCBImpl extends EventEmitter` at module scope, and
// extending `undefined` throws while the bundle is still evaluating, before
// anything of ours runs. So this one gets a real, small EventEmitter. It is
// never used: the class that extends it is on a code path the reference render
// does not enter.
const EVENT_EMITTER_STUB = `
class EventEmitter {
  constructor() { this._listeners = new Map(); }
  on(name, listener) {
    const existing = this._listeners.get(name) ?? [];
    existing.push(listener);
    this._listeners.set(name, existing);
    return this;
  }
  once(name, listener) {
    const wrapper = (...args) => { this.off(name, wrapper); listener(...args); };
    return this.on(name, wrapper);
  }
  off(name, listener) {
    const existing = this._listeners.get(name) ?? [];
    const at = existing.indexOf(listener);
    if (at >= 0) { existing.splice(at, 1); }
    return this;
  }
  removeListener(name, listener) { return this.off(name, listener); }
  removeAllListeners(name) {
    if (name === undefined) { this._listeners.clear(); } else { this._listeners.delete(name); }
    return this;
  }
  listeners(name) { return [...(this._listeners.get(name) ?? [])]; }
  emit(name, ...args) {
    const existing = this._listeners.get(name) ?? [];
    for (const listener of [...existing]) { listener(...args); }
    return existing.length > 0;
  }
}
module.exports = EventEmitter;
module.exports.EventEmitter = EventEmitter;
module.exports.default = EventEmitter;
`;

const STUB_CONTENTS = { events: EVENT_EMITTER_STUB };

const NODE_BUILTIN_STUB = {
  name: "node-builtin-stub",
  setup(build) {
    const names = builtinModules
      .filter((name) => !name.startsWith("_"))
      .map((name) => name.replace(/[.\\+*?[^\]$(){}=!<>|:-]/g, "\\$&"));
    const filter = new RegExp(`^(node:)?(${names.join("|")})$`);
    build.onResolve({ filter }, (args) => ({
      path: args.path,
      namespace: "node-builtin-stub",
    }));
    build.onLoad({ filter: /.*/, namespace: "node-builtin-stub" }, (args) => ({
      contents: STUB_CONTENTS[args.path.replace(/^node:/, "")] ?? EMPTY_STUB,
      loader: "js",
    }));
  },
};

export async function buildPage() {
  await rm(PAGE_DIST, { recursive: true, force: true });
  await mkdir(join(PAGE_DIST, "wasm"), { recursive: true });

  await esbuild.build({
    entryPoints: [
      { in: join(PAGE_SOURCE, "app.mjs"), out: "app" },
      { in: WORKER_ENTRY, out: "decodeImageFrameWorker" },
    ],
    outdir: PAGE_DIST,
    bundle: true,
    format: "esm",
    platform: "browser",
    target: "chrome120",
    // Read at a divergence, so it stays readable. Size is not a budget here,
    // this bundle never ships.
    minify: false,
    sourcemap: false,
    logLevel: "warning",
    define: { "process.env.NODE_ENV": '"production"' },
    plugins: [NODE_BUILTIN_STUB],
  });

  for (const [name, specifier] of Object.entries(CODEC_WASM)) {
    await copyFile(require.resolve(specifier), join(PAGE_DIST, "wasm", name));
  }

  await copyFile(
    join(PAGE_SOURCE, "index.html"),
    join(PAGE_DIST, "index.html"),
  );

  return PAGE_DIST;
}

if (isEntryPoint(import.meta.url)) {
  const dist = await buildPage();
  const html = await readFile(join(dist, "index.html"), "utf8");
  process.stdout.write(`built ${dist} (${html.length} bytes of html)\n`);
}
