# F-228, The Content Security Policy Ocelli needs, measured and documented

**Status**: approved
**Epic ref**: E38.34, proposed, see "Proposed backlog row"
**Sprint**: S46
**Estimate**: 2w

Story row, to be filed by the sprint lead in `docs/sprints/BACKLOG.md` and `docs/sprints/allocation.json` before implementation starts: "Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn". Layer All, depends on F-213 and F-215, milestone M20.

## Proposed backlog row

```text
| F-228 | E38.34 | S46 | Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn | All | 2w | F-213, F-215 | pending |
```

E38.33 onward is free on `8729656` (`grep -c 'E38\.3[3-9]'` prints 0 in both ledgers). F-227 takes E38.33 and this row takes E38.34. The sibling plans of the same round take other epics. If another story is filed into E38 first, the lead allocates in F-ID order and corrects the number. Allocation notes: "Operator issue list of 2026-11-05, legacy Cornerstone codecs loaded from blob: URLs hung under the page CSP. Example half is parallel safe. Folder viewer half is the last viewer story."

## Normative source, transcribed

### The operator's context, 2026-11-05, relayed by the sprint lead

"Legacy Cornerstone's bundle loaded codecs from blob: URLs and the codec worker inherited the page CSP, so compressed series hung on Loading." The operator asked for the exact minimal policy an embedding page needs, verified rather than asserted, a browser test under a strict policy with no `unsafe-inline`, no `blob:` and no `unsafe-eval`, and a probe that a weaker policy is not needed.

### `docs/hld/07-concurrency-and-typescript.md` section 9, verbatim

|  |
|----|
| **Start single-threaded per worker.** N decode workers, each holding its own WebAssembly instance; one render worker owning the GPUDevice and every OffscreenCanvas; the main thread doing DOM, events and tool UI. No SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain. |

|  |
|----|
| Shared-memory threading costs a pinned nightly toolchain, -Z build-std, dual threaded and non-threaded builds with feature detection, and COOP/COEP headers — a permanent tax on the build. |

### Section 10, the bullet this story touches

"Framework bindings, DICOMweb fetch and authentication"

### `docs/hld/21-worker-protocol.md` section 24, verbatim

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

|  |
|----|
| Three roles: the main thread, N decode workers each with its own WebAssembly instance, and one render worker owning the GPUDevice and every OffscreenCanvas. Decode workers never touch the GPU; the render worker never decodes. |

### `docs/hld/11-decision-log.md`, D5

`| D5 | Single-threaded, one instance per worker | wasm-bindgen-rayon from the start | Stays on stable Rust |`

So Ocelli needs no SharedArrayBuffer and therefore no COOP or COEP header. That is a header an embedder does not have to send, and it is stated because it is the first thing an embedder asks after CSP.

### What the HLD says about CSP

Nothing. `grep -rn -i "content.security\|csp\|wasm-unsafe\|blob:" docs/hld/*.md` prints nothing. No test or document in the tree mentions CSP (`grep -rn -i "content-security\|csp" docs examples tools packages scripts bin README.md`, excluding `node_modules` and `dist`, prints nothing).

### The tree, read on `8729656`

- `packages/core/package.json` `files` is `dist`, the two licences, `NOTICE` and `README.md`. `@ocelli/core` ships no wasm module and constructs no worker. `openDecodePool(owner, count, factory)` in `packages/core/src/pool.ts` takes the embedder's factory, as `more.html` `#worker` shows with `(index) => new Worker(url)`.
- `examples/viewer-react/src/core-worker-client.ts:51` constructs `new Worker(new URL("./core-worker.ts", import.meta.url), { type: "module" })`.
- `examples/viewer-react/src/core-worker.ts:122` fetches `__OCELLI_WASM_URL__` as a probe, then `await import(/* @vite-ignore */ __OCELLI_WASM_URL__)`, then the wasm-bindgen default export.
- `examples/viewer-react/vite.config.ts` defines `__OCELLI_WASM_URL__` as `/@fs` plus the absolute path of `crates/ocelli-wasm/pkg/ocelli_wasm.js`, and the built worker `dist/assets/core-worker-*.js` carries that absolute path.
- `crates/ocelli-wasm/pkg/ocelli_wasm.js`, the wasm-bindgen `--target web` glue, locates the module with `new URL('ocelli_wasm_bg.wasm', import.meta.url)`, fetches it, and calls `WebAssembly.instantiateStreaming`, falling back to `WebAssembly.instantiate` on a wrong MIME type. It contains no `new Function`, no `eval(` and no `createObjectURL`.
- No `blob:`, `createObjectURL`, `eval(` or `new Function` in the non-test sources of `packages/core/src`, `packages/react/src` and `examples/viewer-react/src`, or in `examples/viewer-react/dist/assets/*.js`. `packages/core/src/capabilities.ts` calls `WebAssembly.validate`, which CSP does not gate, and `instance-bytes.ts` constructs a `WebAssembly.Memory`, which CSP does not gate either.
- `node_modules/dcmjs/build/dcmjs.es.js`, a runtime dependency of `@ocelli/core`, contains three `new Function(` calls, all inside its bundled ndarray constructor compiler (lines 18368, 18401, 18572), reached through `ndarray(...)` from dcmjs's segmentation adapters. `@ocelli/core` calls `dcmjs.data` and `dcmjs.utilities.TID1500` only (`secondary-capture.ts`, `tid2010.ts`, `tid1500.ts`). Evidence E5 shows those paths run under the strict policy.
- **The wasm module decodes nothing.** `crates/ocelli-wasm/Cargo.toml` has no `ocelli-codec` dependency, its exports (`pkg/ocelli_wasm.d.ts`) are the session, ring, canvas and tier functions, and `more.html` `#worker` says "Not built: decoding that envelope in the worker, its frame cache and a render of that frame. The example worker publishes one stored sample to prove the path." In this repository a compressed frame is decoded only in native code, by the folder viewer's local process and by `ocelli-example-server`.
- The folder viewer's three pages are `include_str!` constants in `serve.rs`. `page.html` has one inline `<script>`, one `<style>` block, one `style` attribute (`<svg ... style="position:absolute">`, line 536) and a `data:` favicon. `use.html` has one inline `<script>` and one `<style>`. `more.html` has one `<style>` and one `style` attribute (line 460). No page carries an inline event handler attribute. F-213 adds an inline `<script type="importmap">` and module loads from `/lib/`.
- `docs/sprints/CURRENT_SPRINT.md` records that the `@ocelli/core` tarball carries no wasm module and `/release` would refuse. This story does not solve that. Its documentation says the module is served from wherever the embedder hosts `ocelli_wasm.js` and `ocelli_wasm_bg.wasm`, which today is a `bin/ocelli.sh wasm` build of `crates/ocelli-wasm/pkg`.

### External definitions, cited and not quoted

The specification text is not in the repository. A reviewer checks these at the source.

- W3C Content Security Policy Level 3: the `'wasm-unsafe-eval'` source keyword gates WebAssembly compilation and instantiation without allowing `eval`. `worker-src` governs `Worker` construction and falls back to `child-src`, then `script-src`, then `default-src`. Module scripts and dynamic `import()` are governed by `script-src`. `fetch` is governed by `connect-src`. Inline scripts and styles are allowed by a `'sha256-...'` source over the element's text.
- WHATWG HTML, worker policy containers: a worker loaded from a network URL takes its policy from its own response, and a worker from a local scheme such as `blob:` or `data:` inherits its creator's policy. Evidence E1 and E2 measure exactly this in the pinned Chromium.
- The WebAssembly JavaScript Interface, the host hook a CSP uses to refuse compilation, which surfaces as a `CompileError`.
- RFC 4648 section 4, base64, with its section 10 test vectors, and FIPS 180-4 SHA-256, with the `abc` example digest `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`.

## Evidence, measured 2026-11-05 by the plan author

Host Apple M4 Max, macOS 27.0.1. Playwright 1.62.1 from `tools/oracle/node_modules`, its bundled Chromium 151.0.7922.34, headless, default flags. Every response was supplied by `page.route` on an invented origin, so no port was bound and no server was started. Scripts lived in the session scratchpad and are not tracked. These are design-time observations that size the probe. They are not the permanent probe and they pass no acceptance row.

| ID | Observation |
|----|-------------|
| E1 | A module worker loaded from a URL compiled an 8-byte empty wasm module. Page and worker both with `'wasm-unsafe-eval'`: worker ok. Both without: `CompileError`. Page without, worker response with no CSP header at all: worker ok. Page with, worker response without: `CompileError`. Page with no header, worker without: `CompileError`. **The worker's own response policy governs its wasm compile. The page's does not reach it** |
| E2 | A `blob:` module worker. Page `script-src 'self' 'wasm-unsafe-eval'; worker-src 'self' blob:`: compile ok. Page without `'wasm-unsafe-eval'`: compile blocked inside the blob worker. Page without `blob:` in `worker-src`: construction refused, `securitypolicyviolation` with `effectiveDirective` `worker-src` and `blockedURI` `blob`. **A blob worker inherits the page policy. That is the legacy Cornerstone failure, reproduced** |
| E3 | The same compile in the page: blocked without `'wasm-unsafe-eval'`, allowed with it |
| E4 | The existing untracked `examples/viewer-react/dist` and `crates/ocelli-wasm/pkg` (both 2026-11-05 05:16, source identity not pinned) served with one header on every response. No header: `core ready`, `Tier C`, `Canvas published`. `default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'`: the same three, and no CSP console error. Without `'wasm-unsafe-eval'`: `core not built`, `Canvas not published`. `worker-src 'none'`: `core not built` and the console error "Creating a worker from ... violates ... worker-src 'none'". `connect-src 'none'`: `core not built`, `Canvas not published`. With `--enable-unsafe-webgpu --enable-gpu --use-angle=metal` the tier is still C, with or without the header |
| E5 | Under the E4 policy, the example's "Synthetic documents" button (TID 2010 key object and Secondary Capture through dcmjs) showed `Pixels 0, 1, 2, 3`, the same as without the header |
| E6 | `examples/folder-viewer/src/page.html` served with `default-src 'none'; script-src 'self' <sha256 of its script>; style-src 'self' <sha256 of its style>; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`, hashes from `node:crypto`. The page booted (`showIndex` defined) and logged one error, "Applying inline style violates ... style-src", from the `svg` `style` attribute. One character added inside the script: the script was blocked and the page did not boot. Without `data:` in `img-src`: the favicon was refused. The page contains no carriage return |

## What the specification does not cover

Every item is Answered: lead decision under the operator's delegation of 2026-11-05.

1. **The policy an embedder needs, for the path that exists.** Four directives and nothing else:

   ```text
   default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'
   ```

   `script-src 'self'` loads the page bundle, the module worker's own imports, and the glue through `import()`. `'wasm-unsafe-eval'` lets the worker compile `ocelli_wasm_bg.wasm`, and it is narrower than `'unsafe-eval'`, which also allows `eval` and is never needed. `worker-src 'self'` allows the URL-loaded module worker and nothing from `blob:` or `data:`. `connect-src 'self'` covers the glue's `fetch` of the module and the worker's probe. A DICOMweb service on another origin is added to `connect-src`, and a module hosted on a CDN is added to both `script-src` and `connect-src`. Everything else an embedding page needs, its styles and images and fonts, is the page's own business and outside this claim. `default-src 'none'` is in the tested header so the probe proves Ocelli needs nothing beyond the three.
2. **Send it on every response, the worker script included.** E1 shows that in Chromium 151 the worker's wasm compile obeys the worker response's header, not the page's. An embedder who sets the policy only on the HTML document leaves the worker unrestricted in Chromium, and one who sets a policy only on the worker without `'wasm-unsafe-eval'` breaks it. "The same header on every response" is correct under both the response model and the inheritance model, so it is the documented rule.
3. **Never `blob:` or `data:` for an Ocelli worker.** E2 shows a blob worker inherits the page policy, so a strict page breaks it and loosening the page for it weakens every script on the page. Ocelli constructs workers from URLs only, and the embedder's `openDecodePool` factory should too. The documentation says so.
4. **COOP and COEP are not needed**, decision D5.
5. **The compressed frame.** The operator asked for a compressed frame decoded and rendered under the policy. The wasm module decodes nothing today, so no compressed frame can be decoded in a worker by any policy. The plan proves the two halves that exist. (a) The worker and wasm path under the embedder policy, through the built example viewer, which publishes one stored sample, plus a WADO-RS instance fetched cross-origin and handed to the worker. (b) Compressed frames decoded and drawn under a strict policy where Ocelli does decode them, the folder viewer, whose native process decodes and whose page draws. AC9 guards the gap: when `ocelli-wasm` gains an `ocelli-codec` dependency, a test fails until the probe is extended to decode in the worker. This plan does not claim a wasm decode it cannot observe.
6. **The folder viewer gets a CSP, with hashes, not nonces.** Yes, because it is the page users open, the page that draws compressed frames, and the page that renders header text from untrusted files, which is exactly what a strict `script-src` protects. Hashes, because every page is a compile-time constant, so a hash allows only the bytes the binary already contains, and a nonce would need an unguessable per-response value and a templating step for no added protection here. The hashes are computed by the process from its own page constants on first use, not tracked by hand, so the many viewer stories that edit `page.html` cannot leave a stale hash. The browser probe recomputes them independently with `node:crypto`.
7. **The folder viewer's policy**, per HTML page:

   ```text
   default-src 'none'; script-src 'self' <hash of each inline script>; style-src 'self' <hash of each style block>; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
   ```

   No `'wasm-unsafe-eval'`, because the page loads no wasm. `img-src data:` for the inline favicon only. The two `style` attributes move into the stylesheet as classes, so no `'unsafe-hashes'` and no `'unsafe-inline'` is needed. `base-uri`, `form-action` and `frame-ancestors` are page hygiene that `default-src` does not cover. They are in this page's policy and not in the embedder's minimum.
8. **Where the probes live.** Beside what they test, in the pattern of `packages/core/cpu-canvas.mjs`, which loads Playwright from `tools/oracle/node_modules` and is an explicit target check named by `/verify`, not a gate. `examples/viewer-react/csp-probe.mjs` and `examples/folder-viewer/csp-probe.mjs`. The pure parts, the policy text and the doc check, run in `npm run test` under the `packages` gate. No gate is added and no workflow file is edited.
9. **Responses are supplied by `page.route`** for the example probe, on invented origins `http://ocelli.test` and `http://dicomweb.test`, so the probe binds no port, and so each variation is one changed header. The folder viewer probe starts the release viewer through F-216's `startViewer` on `OCELLI_PORT` 0, which refuses 8742 and 8765, and stops it by PID.
10. **Browsers.** Chromium, the pinned Playwright build, is the acceptance browser. Firefox and WebKit run when their Playwright builds are installed and are otherwise reported as not run. A not-run browser is not a pass and is not claimed.

## Problem and reproducer

What is absent: no statement anywhere of the policy Ocelli needs, no test under any policy, and two `style` attributes that would force `'unsafe-inline'` on the folder viewer. E2 reproduces the legacy failure in 20 lines. E4 shows the example already runs under the minimal policy, which nothing yet keeps true.

```bash
bin/ocelli.sh wasm
npm run build -w @ocelli/example-viewer
node examples/viewer-react/csp-probe.mjs
cargo build --release -p ocelli-folder-viewer
node examples/folder-viewer/csp-probe.mjs
```

## Scope, API and compatibility

### Example half, parallel safe

- `examples/viewer-react/src/csp-policy.ts` (new): `export const EMBEDDING_POLICY: readonly (readonly [string, readonly string[]])[]`, the four directives of item 1 in order, `export const FORBIDDEN_SOURCES = ["'unsafe-inline'", "'unsafe-eval'", "'unsafe-hashes'", "'strict-dynamic'", "blob:", "data:", "*"]`, and `export function policyHeader(extraConnect: readonly string[] = []): string`, which appends origins to `connect-src` and refuses a forbidden source.
- `examples/viewer-react/src/csp-policy.test.ts` (new, vitest): AC1, AC2, AC9 and AC10.
- `examples/viewer-react/csp-probe.mjs` (new): AC3 to AC8. It imports `./src/csp-policy.ts` under Node 24's type stripping and refuses with a sentence when that is unavailable, never with a copy of the policy. It refuses with a sentence when `dist/` or `crates/ocelli-wasm/pkg` is absent. It maps `/@fs/<absolute path>` to files under the repository root only, serves `.wasm` as `application/wasm`, and applies the variation's header to every response.

### Folder viewer half, serialised

- `examples/folder-viewer/src/csp.rs` (new): `pub(crate) fn page_policy(page: &'static str) -> &'static str`, cached per page in a `OnceLock`, built from every `<script>` element without a `src` attribute and every `<style>` element, hashing the exact text between the start tag's `>` and the next end tag, SHA-256 through the workspace `sha2`, encoded by a local RFC 4648 encoder.
- `examples/folder-viewer/Cargo.toml`: `sha2 = { workspace = true }`. It is already in the lockfile and the licence allowlist, `crates/ocelli-codec` uses it.
- `serve.rs`: the `/`, `/use` and `/more` replies carry `Content-Security-Policy: <page_policy(...)>`. Other replies are unchanged.
- `page.html` and `more.html`: the two `style` attributes become classes in each page's `<style>` block.
- `examples/folder-viewer/tests/preview.rs`: AC11 and AC12, appended.
- `examples/folder-viewer/csp-probe.mjs` (new): AC13 to AC16.

Non-goals: packing the wasm module into `@ocelli/core` (the CURRENT_SPRINT finding), a wasm decode, Trusted Types, Subresource Integrity, CORS policy beyond what the DICOMweb stub needs, `report-to` infrastructure, COOP and COEP, and any policy for `ocelli-example-server` or the Vite dev server (whose React refresh preamble is an inline script and is a development tool only).

Compatibility: the folder viewer's pages gain a header and lose two attributes, with the same rendering. No library API changes.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Embedding page | `EMBEDDING_POLICY` text in README, `use.html` and `more.html` | Same header on every response | A working worker and wasm path, or a named directive missing |
| Example page bundle | `dist/index.html`, `assets/index-*.js` | `script-src 'self'` | React mounts |
| Example module worker | `assets/core-worker-*.js` by URL | `worker-src 'self'`, its own response's policy | `core ready`, `Canvas published` |
| wasm glue and module | `crates/ocelli-wasm/pkg` | `script-src` for `import()`, `connect-src` for `fetch`, `'wasm-unsafe-eval'` for compile | `Tier` shown |
| WADO-RS fetch | `postWadoInstance` through `DicomwebClient` | `connect-src` lists the service origin | "The instance bytes were handed to the worker." |
| dcmjs document paths | `writeKeyObject`, `writeSecondaryCapture`, TID 1500 | No `'unsafe-eval'` | Document text equal to the control |
| Folder viewer pages | `serve.rs` with `csp.rs` | Hashes from the binary's own constants | Pages boot, compressed frames draw |

## State transitions and invariants

Not applicable as a state machine. The probe's variations are independent page loads. Invariant across consumers: the embedder policy text exists once, in `csp-policy.ts`, and every marked block quotes it (AC2). The folder viewer's hashes are a pure function of its compiled pages.

## Ownership, commit and recovery

Not applicable. No resource is held. The folder viewer probe owns one child process, started on an ephemeral port and stopped by PID in a `finally` block, which is F-216's runner rule.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `policyHeader()` is exactly `default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'`, contains no forbidden source, and `policyHeader(["https://pacs.example"])` adds that origin to `connect-src` only. A forbidden source passed in is refused | `csp-policy.test.ts`, "the embedding policy is the four directives" | A policy grown by `'unsafe-eval'` or `blob:` to make something pass | implementation |
| AC2 | Superseded by the marker rule in the lead amendments after review pass 4 | `csp-policy.test.ts`, "every document quotes the one policy" | A marked block that drifts from `policyHeader()` | implementation |
| AC3 | Built example, release wasm, the AC1 header on every response. The page shows `core ready`, `Canvas published` and a `Tier` text equal to the no-header control run, the worker canvas's screenshot pixel equals the control's, and no console message contains "Content Security Policy" | `csp-probe.mjs` case `strict` against case `control` | A probe that only checks the page loaded | external, the probe on the operator's Mac, Chromium 151 |
| AC4 | Minimality, each from the AC3 header with one change, on every response. Without `'wasm-unsafe-eval'`: `core not built` and `Canvas not published`. `worker-src 'none'`: `core not built` and a console violation naming `worker-src`. `connect-src 'none'`: `core not built`. `script-src 'none'`: React does not mount, `#root` is empty. Each case's named failure is asserted, not only the absence of success | `csp-probe.mjs` cases `no-wasm`, `no-worker`, `no-connect`, `no-script` | A minimality claim that passes because the page broke for another reason | external, as AC3 |
| AC5 | Where the worker's policy comes from. Page with the AC3 header and the worker script with it minus `'wasm-unsafe-eval'`: `core not built`. Page minus `'wasm-unsafe-eval'` and every other response with the full header: `core ready`. Recorded in the run output with the browser version, and the documentation's "every response" rule cites it | `csp-probe.mjs` cases `worker-strict`, `page-strict` | Documenting a page-only header | external, as AC3 |
| AC6 | The legacy failure, reproduced as a negative control. A probe-local page under the AC3 header constructs a `blob:` module worker that compiles the 8-byte empty module: construction is refused with a `worker-src` violation. With `blob:` added to `worker-src` and `'wasm-unsafe-eval'` removed from the page, the blob worker's compile fails with `CompileError` | `csp-probe.mjs` case `blob-control` | Claiming blob workers are safe under a strict page | external, as AC3 |
| AC7 | `@ocelli/core`'s dcmjs paths under the AC3 header. "Synthetic documents" shows `Pixels 0, 1, 2, 3`, and "Show measurement" shows the same TID 1500 line as the control | `csp-probe.mjs` case `documents` | A policy that holds only until a document is written | external, as AC3 |
| AC8 | DICOMweb `connect-src`. The base URL field set to `http://dicomweb.test`, served by a route that answers the WADO-RS instance request with CORS headers and the multipart content type `packages/core/src/dicomweb.ts` accepts, wrapping the bytes of `corpus/data/syntax/rle_lossless.dcm`. With `policyHeader(["http://dicomweb.test"])`: "The instance bytes were handed to the worker." With `policyHeader()`: "The instance request was refused." and a console violation naming `connect-src` | `csp-probe.mjs` cases `dicomweb`, `dicomweb-unlisted`. Corpus absent makes both not run, never passed | Documenting `connect-src 'self'` alone for a remote archive | external, as AC3 |
| AC9 | The decode gap is guarded. A test reads `crates/ocelli-wasm/Cargo.toml` and fails, with a sentence naming this plan, when any dependency table names `ocelli-codec`, because the probe would then need a worker decode case | `csp-policy.test.ts`, "the wasm module still decodes nothing" | A wasm decode landing under a policy nobody re-measured | implementation |
| AC10 | No source Ocelli ships constructs a blob or data worker or evaluates strings. Non-test files under `packages/core/src`, `packages/react/src`, `examples/viewer-react/src` and `crates/ocelli-wasm/pkg/ocelli_wasm.js` when present contain no `createObjectURL`, `blob:`, `eval(` or `new Function`. Positive control: the scanner flags a string containing `new Function(` | `csp-policy.test.ts`, "no shipped source needs a weaker policy" | A future `new Worker(URL.createObjectURL(...))` | implementation |
| AC11 | `page_policy`. RFC 4648 section 10 vectors, `f` `Zg==`, `fo` `Zm8=`, `foo` `Zm9v`, `foobar` `Zm9vYmFy`. SHA-256 of `abc` encodes as `ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=`. A two-script page gives two `script-src` hashes, a `src` script gives none, and the policy has no `'unsafe-inline'`, `'unsafe-eval'`, `blob:` or `'wasm-unsafe-eval'` | `csp.rs` unit tests | Hashing the tag instead of the text. A base64 with the wrong padding | implementation |
| AC12 | The three pages contain no carriage return, no `style` attribute and no `on<event>=` attribute, and `/`, `/use` and `/more` replies carry the header for their own page | `the_pages_carry_their_own_policy` in `tests/preview.rs` | A new inline handler that only `'unsafe-inline'` would allow | implementation |
| AC13 | Folder viewer pages under their header. `/`, `/use` and `/more` load with no CSP console message, `/` boots, and each header's hashes equal `node:crypto` hashes of that page's inline elements. A route that changes one character of the inline script while keeping the header: the script is blocked and the page does not boot | `examples/folder-viewer/csp-probe.mjs` cases `pages`, `tampered` | A hash list that allows any script, or a page that boots only because hashes are absent | external, the probe, release viewer |
| AC14 | Compressed frames decoded and drawn under the header. Each lossless greyscale row `j2k_lossless`, `htj2k_lossless`, `htj2k_lossless_rpcl`, `jpeg_lossless_p14`, `jpeg_lossless_p14_sv1`, `jpegls_lossless` and `rle_lossless` from `corpus/data/syntax` is opened alone through the folder input. The active viewport's grey bytes, read with `getImageData` at backing size, equal those of `explicit_vr_le.dcm`, which `scripts/corpus_synth.py` `SYNTAX_REFERENCE` names as the uncompressed case each must decode back to, opened in the same page, and no CSP console message appears | `csp-probe.mjs` case `compressed`. Corpus absent makes it not run, never passed | A probe that checks only uncompressed input | external, the probe, corpus present |
| AC15 | The library path under the header. F-213's region window, which imports `@ocelli/core` from `/lib/` through the inline import map, changes the window, and F-215's TID 1500 save produces a Playwright download that F-215's load reads back | `csp-probe.mjs` cases `region`, `save` | An import map whose hash was never computed | external, the probe |
| AC16 | The folder viewer needs no `'wasm-unsafe-eval'`, shown by AC13 to AC15 passing under a header without it | Inspection of the run output | Copying the embedder policy onto a page that loads no wasm | external |

Semantic RED. AC12 runs red on today's pages for the two `style` attributes and the missing header. AC13 runs red on today's binary for the missing header, through the `tampered` case booting. AC4 to AC6 are red-by-design cases inside one run, and each asserts its own named failure. Mutations after GREEN, each run and seen red: drop `'wasm-unsafe-eval'` from `EMBEDDING_POLICY` (AC3), add `'unsafe-eval'` (AC1), hash the start tag (AC11, AC13), skip the importmap element (AC15), and add `ocelli-codec` to `ocelli-wasm` in a scratch copy (AC9).

## Approach

1. Example half: `csp-policy.ts`, its vitest file, the probe, then the three documents. This half may start now.
2. Folder viewer half, last in the viewer order: `csp.rs`, the header in `serve.rs`, the two classes, the Rust tests, the probe.
3. LLD.

Rejected alternatives: `'unsafe-eval'` instead of `'wasm-unsafe-eval'` (allows `eval`), `blob:` workers (E2), a page-only header (E1), nonces for the folder viewer (item 6), hand-tracked hashes (stale on the next page edit), moving the folder viewer's inline script into a served file (every pending viewer story edits that script, and the hash route needs no restructuring), and a new gate (item 8).

## Dependencies, write ownership and resources

Declared dependencies F-213 and F-215, for the folder viewer half only. The example half writes `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts` and `examples/viewer-react/csp-probe.mjs`, which no other S46 story writes, plus the three documents, which every story edits for its own section. It may run now, in parallel, with its document edits merged three-way as usual.

The folder viewer half writes `serve.rs`, `page.html`, `more.html`, `Cargo.toml`, `lib.rs` and `tests/preview.rs`, which every viewer story writes. **It is the last viewer story**: F-217, F-216, F-210, F-211, F-208, F-209, F-212, F-213, F-227, F-214, F-215, then this half. Last because its probe must cover F-213's import map and `/lib/` modules and F-215's download, and because its hash route then sees the final pages. F-223 is independent and may run before or after it.

Write set: `examples/viewer-react/src/csp-policy.ts` (new), `examples/viewer-react/src/csp-policy.test.ts` (new), `examples/viewer-react/csp-probe.mjs` (new), `examples/folder-viewer/src/csp.rs` (new), `examples/folder-viewer/src/lib.rs`, `examples/folder-viewer/src/serve.rs`, `examples/folder-viewer/Cargo.toml`, `examples/folder-viewer/src/page.html`, `examples/folder-viewer/src/more.html`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/tests/preview.rs`, `examples/folder-viewer/csp-probe.mjs` (new), `README.md`, `docs/lld/typescript-packaging.md`, and `Cargo.lock` if the `sha2` edge changes it.

Resources: Chromium through Playwright 1.62.1, the release wasm build, the built example, the release folder viewer, `corpus/data/syntax` for AC8 and AC14. No GPU. Never ports 8742 or 8765.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate packages
bin/ocelli.sh gate lint
bin/ocelli.sh gate types
bin/ocelli.sh test ocelli-folder-viewer
bin/ocelli.sh clippy ocelli-folder-viewer
bin/ocelli.sh gate licences
bin/ocelli.sh wasm
npm run build -w @ocelli/example-viewer
node examples/viewer-react/csp-probe.mjs
cargo build --release -p ocelli-folder-viewer
node examples/folder-viewer/csp-probe.mjs
```

Final profile `bin/ocelli.sh gate --sprint`. The two probes are explicit target checks in the sense of `.claude/commands/verify.md`, kept with their command, Chromium version and per-case result in the feature evidence. They are not gate membership. Evidence is invalidated by any change to the example's worker, its Vite config, the wasm glue version, `crates/ocelli-wasm`, the folder viewer's pages or `serve.rs`, the Playwright pin, or the corpus syntax rows. A case that did not run is reported as not run.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The probe reads canvas pixels in the test harness only
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): no CSP directive governs WebGPU or its shaders, so the policy is the same. Not observed. E4 resolved tier C headless, also with WebGPU flags, and the example worker publishes through the CPU canvas. The probe records the resolved tier and does not claim A
- Tier B (WebGL2): the same, not observed
- Tier C (CPU): full. AC3 observes tier C publication under the policy

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1, AC2, AC9 and AC10, the policy text, the documents, the decode guard and the source scan | `examples/viewer-react/src/csp-policy.test.ts` |
| unit | AC11, RFC 4648 and FIPS 180-4 vectors and the page policy shape | `examples/folder-viewer/src/csp.rs` |
| contract | AC12, page hygiene and the reply header | `examples/folder-viewer/tests/preview.rs` |
| browser | AC3 to AC8, the example under the policy and its minimality | `examples/viewer-react/csp-probe.mjs` |
| browser | AC13 to AC16, the folder viewer under its policy, compressed rows against their uncompressed reference | `examples/folder-viewer/csp-probe.mjs` |

No `fixture` row. This story does no pixel arithmetic. AC14 compares decoded grey against the same page's grey for the uncompressed reference, which is a decode identity, and the arithmetic behind both is F-206's and F-216's fixtures.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no deployment row. The legacy Cornerstone behaviour this story answers is an operator report, and cornerstone3D 5.8.2 is not measured here.

## Deviations

None.

## LLD impact

`docs/lld/typescript-packaging.md`: a new section "The Content Security Policy an embedder needs", with the four directives, the every-response rule and E1, the blob rule and E2, the decode gap and AC9's guard, and the note that the tarball does not carry the module today. The folder viewer's policy is described by the folder-viewer half, which adds it when that policy exists.

## User-facing documentation impact

- `README.md`, "Start the example viewer": a short subsection with the policy in a fenced block, "send the same header on every response, the worker script included", "add your DICOMweb origin to `connect-src`", "no `blob:`, no `'unsafe-eval'`, no COOP or COEP", and the probe command.
- `examples/folder-viewer/src/use.html`, chapter 5.1 "Load the core in a worker": the policy, the every-response rule, the blob warning for an `openDecodePool` factory, and the DICOMweb and CDN additions.
- `examples/folder-viewer/src/more.html`, `#worker`: one paragraph with the policy, and one sentence that this page itself is served under a hash policy with no `'wasm-unsafe-eval'`, because it decodes in its local process.

## Open questions

None open.

- Q1, which directives. Answered: lead decision under the operator's delegation of 2026-11-05. The four of item 1, measured in E4 and kept by AC3 and AC4.
- Q2, page or every response. Answered: lead decision under the operator's delegation of 2026-11-05. Every response, item 2, E1, AC5.
- Q3, the compressed frame. Answered: lead decision under the operator's delegation of 2026-11-05. The wasm module decodes nothing, so the worker path is proven with what it does and compressed decode is proven on the folder viewer, item 5, AC9 and AC14. The operator may prefer to hold the worker decode case until a wasm decode story exists, which is what AC9 enforces.
- Q4, folder viewer CSP. Answered: lead decision under the operator's delegation of 2026-11-05. Yes, hashes computed from the compiled pages, last in the viewer order, items 6 and 7.
- Q5, the tarball has no wasm module. Answered: lead decision under the operator's delegation of 2026-11-05. Out of scope. It stays the CURRENT_SPRINT operator question, and this story's documentation names where the module is served from today.

### Lead amendments after review pass 1

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-228-working-pass-1.md`.

- **AC2 never skips.** The example half writes the policy text into `README.md` and `docs/lld/typescript-packaging.md` now, and AC2 checks those two exactly, red or green. The Use book quote lands with the folder-viewer half, which adds it to AC2. A skipped test is not an answer here.
- **`data:` is a forbidden source.** The AC10 scan and the forbidden list include `data:` beside `blob:`, `createObjectURL`, `eval(` and `new Function(`, and the scan walks every shipped source directory found under `packages/*/src` and `examples/viewer-react/src`, not a list written by hand.
- **`worker-src 'self'` stays although it falls back to `script-src`.** It keeps a later widening of `script-src` from widening workers too. The minimality case for it asserts the fallback rather than a failure, and says so.
- **`policyHeader` refuses what would widen the policy.** A scheme-only source such as `https:` or `http:`, and any entry containing `;`, `,`, whitespace or a quote, is refused with an error.

### Lead amendments after review pass 4

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-228-working-pass-4.md`. AC2 drew a different finding in four straight passes because it tried to recognise the header in free prose. That is replaced by a marker rule.

- **Each quote is a marked block.** A document quotes the policy in a block preceded, on its own line, by the marker `<!-- ocelli:csp -->`. The line after the block opening, in Markdown a fenced block opened by ```` ```text ```` and in HTML a `<pre>` whose code starts on the next line, is the header line. AC2 checks that each marked block's header line equals the expected header exactly, and that README.md holds exactly two marked blocks and `docs/lld/typescript-packaging.md` exactly one. The folder-viewer half adds the Use book's block and its count.
- **Prose is not parsed.** No rule inspects lines outside marked blocks. The documents' sentences about the policy are checked by review, as every other sentence is.

### Folder-half integration facts, 2026-11-08

The complete F215/F225 source stack was applied three-way before runtime wiring. The main page now has an import map and a main script, so both are hashed. The native CSP module uses the same target gating as the native serve module. Production wiring replaces the AC11 preparation slice's test-only declaration. The folder browser probe has explicit Node and browser callback globals in ESLint, without changing its wasm-memory allowance list or enforcement rules.

The native response stores one of the three HTML page identities, and both its body and policy derive from that identity. Adding an optional string slice made the existing Reply error variants fail result_large_err. The page identity keeps the existing error path without introducing boxed errors or changing its callers.
