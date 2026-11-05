# F-223, Comparative cost of the local folder viewer and cornerstone3D 5.8.2 on one machine

**Status**: approved
**Epic ref**: E38.32
**Sprint**: S46
**Estimate**: 3w

Story row to be filed by the sprint lead in `docs/sprints/BACKLOG.md` and `docs/sprints/allocation.json` before implementation starts, because `scripts/bench_check.py` resolves every story it reads through the allocation: "Comparative cost of the local folder viewer and cornerstone3D 5.8.2 on the same studies and machine: open, first image, scroll, window change, peak memory and CPU over every process each side runs, behind a one-frame pixel agreement check, with a regenerated tracked summary". Layer All, depends on F-216, milestone M20. `E38.32` is unused today, `grep -c 'E38\.32' docs/sprints/BACKLOG.md docs/sprints/allocation.json` prints 0 for both.

The operator's request, 2026-11-05, verbatim: "perform some comparative performance benchmark on how this tool performs against cornerstone, like memory usage, time taken, etc., to make a case".

## Normative source, transcribed

The HLD does not specify a comparative benchmark, an incumbent measurement page or a published summary. `grep -n -i 'cornerstone' docs/hld/*.md` finds no performance, speed or memory claim about cornerstone3D except one sentence in section 26 of `26-differentiating-capabilities.md` about whole-volume WebGL textures, which concerns volumes and is not measured here. The sources below are the rules this plan keeps.

### `docs/hld/23-performance-rules.md`, section 26, last rule

|  |
|----|
| Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer. |

### `docs/spikes/A7-tier-c.md`, section A7.3, the only written method that compares against an incumbent

"**The budget is set relative to the viewer the estate already permits**, not from an invented absolute.

1. Measure the incumbent viewer on a representative GPU-less session. Record its interactive cost and its idle cost as fractions of one vCPU.
2. **Ocelli's budget is no worse than those two numbers.**
3. Record Ocelli's observed figures and gate on regression beyond a tolerance, the same mechanism `ci/wasm-size-budget.json` already uses for binary size."

And, from the same file: "Do not invent a number."

This story takes the method's shape, the incumbent measured the same way on the same machine, and sets no budget. It is not the A7.3 measurement, because A7.3 names a representative GPU-less session and this story runs on a GPU-capable browser by lead decision.

### `docs/hld/11-decision-log.md`, section 14, decision D14

| **\#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |

Applied to cost: every figure in the output is a measured median with its spread, a difference is stated only where the measured spreads separate, and nothing is extrapolated.

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, the two monochrome bullets the pixel check evaluates

|  |
|----|
| **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference ≤ 1 LSB on at least 99.9% of pixels; zero pixels differing by more than 2. |
| **Systematic bias, monochrome:** signed mean difference over the informative region within 0.1 of one display code, evaluated only where input identity, declared parameters and geometry already agree. |

`tools/oracle/src/tolerance.rs` carries these as `MONOCHROME_ONE_LSB = 1`, `MONOCHROME_MAX_ABS_DIFF = 2`, `MONOCHROME_SIGNED_MEAN_BIAS = 0.1`, with `monochrome_predicate` and `bias_bound`, and adds `INFORMATIVE_FRACTION_FLOOR = 0.10`, which its comment marks "**Not from 25.1.**" and explains is the floor below which a class-one verdict cannot mean anything. This plan reuses those constants and functions and defines none of its own.

### Project instructions on tone, `CLAUDE.md`

"**cornerstone3D is our reference oracle, and we read it and learn from it.** It is MIT licensed and in bounds. Phase 1 targets feature parity with **5.8.2** and every frame is diffed against it before merge, which is a debt to acknowledge rather than a rivalry to advertise. Keep that tone in anything public."

### `README.md`, "What we are trying to add"

"**Every benchmark we publish names its tier**, because a number without the hardware it ran on is not reproducible and, on a mixed estate, is not honest either."

### `docs/lld/benchmarks.md`, the harness's own rule

"So this harness compares a figure against a figure **it recorded**, on the machine that recorded it, and against nothing else." And: "A duration belongs to a CPU, an operating system build and a browser build."

The comparison mode keeps that rule. Both sides are measured in the same run, on the same machine, with the same browser build, and nothing is compared against a figure from another run or another host.

## Facts measured for this plan

Measured on 2026-11-05 on `sprint/s46` at `b13812c`, and in the F-216 worktree at `8b00223`, read only. Host Apple M4 Max, 16 CPU cores, 40 GPU cores, 51,539,607,552 bytes, macOS 27.0.1 (26A434).

1. **There is no comparison mode.** `tools/bench/run.mjs` `parseArgs` throws `unknown argument --compare-cornerstone. Try --help.` for any flag it does not list, and `tools/bench/package.json` says "There is no cornerstone3D here and there is no bundler."
2. **The F-216 Ocelli instrument exists only in its worktree.** `tools/bench/src/viewer_work.mjs` there exports `ensureReleaseBinary`, `startViewer` (spawns the release binary with `OCELLI_PORT=0` and refuses a bound port of 8742 or 8765), `stopViewer`, `ensureWorkloads` (1,693 synthetic 512 by 512 16-bit CT files and one 3000 by 2900 DX, stored value 50 everywhere), and `measureOpen`, `measureFirstImage`, `measureScroll`, `measureWindow`, `measureResident`. Its five `viewer.*` rows in `subjects.json` carry `"tiers": ["C"]` and `"subject_story": null`.
3. **F-216 samples memory of the server only.** `measureResident` polls `ps -o rss= -p <server pid>` every 20 ms and reads `performance.memory.usedJSHeapSize` in the page. No Chromium process is counted. Lead decision 2 needs every process on both sides, so this story adds its own collector and reuses none of that one.
4. **The Ocelli page's hooks.** `examples/folder-viewer/src/page.html` in the F-216 worktree defines the page globals `requestGrey`, `drawGrey`, `showIndex`, `currentImage`, `postPreview`, `overrideWindow`, `activePreset` and `viewIndex`, posts each file to `/file` with four workers, groups by `seriesUid`, sorts series by number then description then key and images by instance number then file name then frame, and selects the series with the most images. `drawGrey` paints a canvas whose backing store is the image's own width and height.
5. **How the oracle loads cornerstone3D.** `tools/oracle/build-page.mjs` `buildPage()` runs one esbuild pass over `page/app.mjs`, `page/volume.mjs` and the loader's `decodeImageFrameWorker.js`, copies the four codec `.wasm` binaries named in `CODEC_WASM`, and writes everything into `tools/oracle/page/dist` after `rm -rf` of that directory. `src/server.mjs` `serveDirectory(root)` serves it on 127.0.0.1 on an ephemeral port. `run.mjs` launches Chromium with `--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader --force-device-scale-factor=1 --disable-lcd-text --hide-scrollbars`. `checkPins()` in `run.mjs` compares `PINNED`, `PINNED_CODECS` and `PINNED_TOOLS_PEERS` against the installed versions and is not exported.
6. **The oracle page's render route.** `page/app.mjs` calls `coreInit()`, `dicomImageLoader.init({ maxWebWorkers: 1, wasmBasePath })`, `enableElement` with `ViewportType.STACK`, adds the file through `dicomImageLoader.wadouri.fileManager.add(file)`, `setStack([imageId], 0)`, sets `VOILUTFunction`, `interpolationType` and `voiRange` from `utilities.windowLevel.toLowHighRange`, paints a sentinel over the canvas, calls `render()`, waits for `Enums.Events.IMAGE_RENDERED`, and reads the canvas back with `getImageData`. A canvas still showing the sentinel after the event is refused, so the oracle already proves the canvas is written before `IMAGE_RENDERED` reaches a listener.
7. **cornerstone3D 5.8.2 defaults.** `@cornerstonejs/core` `cache/cache.js` sets `this._maxCacheSize = 3 * ONE_GB`. `@cornerstonejs/dicom-image-loader` `init.js` uses `options?.maxWebWorkers || getReasonableWorkerCount()`, which is `Math.max(1, Math.floor(navigator.hardwareConcurrency / 2))`, so 8 on this host. `wadouri` exports `dataSetCacheManager` with `load(uri, loadRequest, imageId)` and `loadFileRequest`.
8. **The pseudo-PHI folder, attributes only.** 1,696 files: 1,693 `.dcm`, one `LICENSE`, two `.DS_Store`. 612 MiB on disk. 22 studies, 26 series. Every `.dcm` is Implicit VR Little Endian (`1.2.840.10008.1.2`) or Explicit VR Little Endian (`1.2.840.10008.1.2.1`), so **no compressed transfer syntax is present and no codec cost is compared on these studies.** By series: five PT series of 128 by 128 (267, 267, 175, 175 and 174 images), CT 512 by 512 MONOCHROME2 with a file window (152 and 107 images), one PT RGB series of 512 by 512 (145), MR 256 by 256 (60, 26, 26, 19), MG 1421 by 1024 (57), MR 512 by 512 (19), DX 2022 by 2022 (7), CT 665 by 512 (5), and single-image CR, DX, MG and CT objects. The largest DX by pixel count is 2330 rows by 2846 columns, 13,265,590 bytes. The largest file is an MG of 3328 by 2560 at 17,042,396 bytes. Read with `.venv/bin/python3` and pydicom 3.0.2 with `stop_before_pixels=True`. No identifying value was printed or recorded.
9. **macOS gives a per-process kernel peak.** `/usr/bin/footprint -f bytes -p <pid> -p <pid>` prints, per process, `phys_footprint: <n> B` and `phys_footprint_peak: <n> B`, then `Summary Footprint: <n> B`, which for two unrelated processes was their sum. Two tiny processes took 0.02 s of wall time.
10. **Playwright 1.62.1 ships both builds at revision 1234.** `~/Library/Caches/ms-playwright` holds `chromium-1234` and `chromium_headless_shell-1234`, browser version 151.0.7922.34. `tools/bench/src/browser_session.mjs` already reads `SystemInfo.getProcessInfo` through `browser.newBrowserCDPSession()` for its CPU collector.
11. **No mechanical lock serialises browser and GPU jobs.** `bin/ocelli.sh` and `scripts/*.py` contain no lock file. `.claude/commands/run-sprint.md` section 3 lists "**the GPU**, for any story running the oracle or a browser tier test" as an exclusive resource, which is a scheduling rule.
12. **The tracked summary's directory is not prose-checked today.** `scripts/prose_check.py` `INCLUDE_PREFIXES` names `docs/sprints/`, `docs/lld/`, `docs/runbooks/` and `docs/spikes/`, and no `docs/benchmarks/`.

## What the specification does not cover

1. **Where the comparison lives in the harness.** `subjects.json` rows each carry one value, one unit and a per-host baseline with a two-sided tolerance, and `runSubjects` runs every row that has a runner. A comparison carries two sides, N trials each, a pixel precondition, and inputs that exist on one machine. As registry rows with runners it would make the default `bin/ocelli.sh bench` fail on every host without the pseudo-PHI folder or the oracle install, and it would write a baseline for an external library's figures. So the comparison is a mode, `--compare-cornerstone`, whose definitions live in a tracked `tools/bench/comparison.json` checked by `scripts/bench_check.py`, and whose code lives under `tools/bench/src/` outside `src/runners/`. It writes no baseline and refuses `--compare` and `--accept`. Each of its metrics names the `viewer.*` subject it mirrors, so the two definitions cannot drift apart unseen. `AGENTS.md`'s sentence "the subjects it will ever measure are listed in `tools/bench/subjects.json`" gains the comparison file beside it.
2. **The comparator page.** Lead decision 1 fixes the packages. The page itself is a new file, `tools/oracle/page/bench.mjs` with `bench.html`, built by the oracle's `buildPage` so it uses exactly the pinned packages and codecs and the oracle's pin check. It is the smallest page that does what the folder viewer does: a folder input, header parse of every file, grouping into series, selection of the largest series, one `StackViewport`, ArrowDown to the next image, and a window entry point. It loads `@cornerstonejs/core` and `@cornerstonejs/dicom-image-loader` only. `@cornerstonejs/tools` is not loaded, as in the oracle.
3. **How the comparator reads headers.** Through `dicomImageLoader.wadouri.dataSetCacheManager.load(uri, wadouri.loadFileRequest, imageId)` after `fileManager.add(file)`, which is the loader's own route for a file `imageId`, then series and instance attributes through cornerstone3D's `metaData.get`. That route keeps each parsed file in memory, which is the library's behaviour and is what the memory figure measures. The summary says so, and says that an application with a header-only reader would hold less. The Ocelli side holds posted bytes in its 256 MiB LRU, which is F-216's behaviour.
4. **Prefetch.** The folder viewer prefetches the next 2 and previous 1 images (F-216 Open question B's answer). `StackViewport` prefetches nothing, and the prefetch utility lives in `@cornerstonejs/tools`, which lead decision 1 excludes. The comparator page therefore issues the same policy itself through `imageLoader.loadAndCacheImage`, the core call that utility uses. The summary names the matched policy.
5. **Worker count, cache sizes and other defaults.** Each side runs at its shipped defaults: cornerstone3D 3 GiB cache and `floor(hardwareConcurrency / 2)` decode workers, Ocelli 256 MiB server cache, 64 MiB page grey cache and 2 decode permits. The output records every one of them beside the memory figure, because a memory comparison at different budgets is a comparison of defaults and the reader must be told.
6. **Timing endpoints that both sides have.** F-216's ends are Ocelli page functions that cornerstone3D does not have. Each metric here starts at the same harness action on both sides and ends at the first `requestAnimationFrame` callback after the side's commit, where Ocelli's commit is the return of `putImageData` on `#view` and cornerstone3D's is `IMAGE_RENDERED` for the viewport (fact 6). F-223 figures are therefore not F-216 figures and are never compared with `ci/bench-baseline.json`.
7. **Cold and warm.** Cold is a freshly launched Chromium, a new Ocelli server process and a new page. Warm is a new page in the same browser and, for Ocelli, the same server, after one completed trial of the same workload. The operating system file cache is warm in both, because purging it needs root. One discarded priming pass reads every input file once before trial one, so neither side pays first disk touch. The summary says so.
8. **Memory collector.** On macOS, for every process in a side's inventory, `footprint -f bytes` is called with every pid in one call, repeatedly, at least 250 ms apart. Two figures are reported. The **sampled concurrent peak** is the largest per-sample sum of `phys_footprint`, a lower bound at the sampling interval. The **sum of per-process peaks** is the sum over pids of the largest `phys_footprint_peak` seen for that pid, an upper bound because peaks of different processes need not coincide. The inventory is the side's browser process and its descendants by `ps -A -o pid=,ppid=`, unioned with every pid `SystemInfo.getProcessInfo` reports, plus the Ocelli server pid on the Ocelli side. The cornerstone side's static file server is harness infrastructure, not product, so it is excluded and its own figure is recorded separately. Memory the GPU driver does not attribute to a process is counted on neither side, and the summary says so. On any other platform the memory metric is `unavailable` with the reason "no collector for <platform>", and is never estimated.
9. **CPU.** The operator's "etc." is answered with one more figure: CPU seconds summed over the same inventory across the memory session, from `ps -o time=` at the session's start and end. A process that appears or exits between the two snapshots makes that session's CPU figure invalid, which is the rule `.claude/commands/design.md` states for CPU cost ("A changed process set or invalid counter refuses the observation"). The session runs in one page so renderers do not churn.
10. **The pixel agreement check.** Lead decision 5 needs a comparator, and `tools/oracle`'s `ocelli-compare` has `gate`, `identity`, `mutations` and `census`, all over a rendered corpus run directory. This plan adds `ocelli-compare pair`, which takes two raw RGBA8 frames of stated size and prints the 25.1 verdict from the existing `frame::difference`, `monochrome_predicate`, `bias_bound` and `INFORMATIVE_FRACTION_FLOOR`. No tolerance is defined or moved. The frames are written to a temporary directory outside the repository and removed afterwards, because a rendered pseudo-PHI image is a picture of the input.
11. **It is not the oracle's verdict.** The pair check compares one frame of the folder viewer's native CPU `LutChain` against one cornerstone3D frame at one window. It is a precondition for comparing speed, and it is not a golden test, not a corpus run and not D7's renderer comparison. The oracle still has no Ocelli renderer to compare against.
12. **Which inputs, selected by attribute and never by path.** Five workloads, each a selection rule over header attributes. Synthetic CT and synthetic DX are F-216's `ensureWorkloads`. Pseudo full is every file under the root. Pseudo CT is the CT series with the most images among series whose every image is MONOCHROME2, 512 by 512, carrying Window Center and Window Width, ties broken by the smaller digest of the series' sorted content digests. Pseudo DX is the DX object with the most pixels, ties broken the same way. On today's folder these are 152 images and 2330 by 2846 (fact 8). A rule that selects nothing makes that workload's metrics `unavailable` with the rule's sentence as the reason, and no other input is substituted. Each pseudo subset is materialised as a directory of symbolic links in a fresh directory under the operating system's temporary directory, removed in a `finally` block. If Playwright's `setInputFiles` does not follow a link, which the implementer observes once and records, the subset is copied there instead.
13. **No identifying value leaves the run.** The harness reads, from every pseudo input, Patient Name, Patient ID, Patient Birth Date, Accession Number, Institution Name, Referring Physician Name, Study, Series and SOP Instance UIDs, Study, Series and Acquisition Dates, and every path component below the root, into an in-memory deny set, using `dicom-parser` from the oracle install. Every output, the ignored JSON, the plain report and both tracked files, is scanned against the deny set before it is written, and a hit refuses the write. The published JSON also has a closed schema, so a value cannot arrive through a field nobody expected. The input identity recorded is the SHA-256 of the sorted list of per-file content SHA-256 digests, which identifies the input set for a rerun and reveals nothing.
14. **Exclusive use.** Before launching anything the mode lists processes with `ps -A -o pid=,command=` and refuses when any process outside its own tree has `ms-playwright` in its command line, naming the count and never killing anything. It also refuses a second comparison through an exclusive `mkdir` of `tools/bench/out/compare/lock`, removed in a `finally` block. This is a check, not a lock on the oracle, which takes none (fact 11).
15. **Browser mode and GPU.** Both sides launch the full Chromium build, revision 1234, through Playwright `chromium.launch({ channel: "chromium" })` with `--force-device-scale-factor=1`, a 1600 by 1000 viewport and no SwiftShader flag. Each page reports `WEBGL_debug_renderer_info` `UNMASKED_RENDERER_WEBGL` from a scratch canvas. The run refuses when the two sides' renderer strings or `browser.version()` strings differ, and refuses a renderer string naming SwiftShader unless `--software` was passed, in which case every output names SwiftShader in its first paragraph. If the implementer's first probe shows headless Chromium on this host does not reach the hardware adapter, the mode launches headed with the same viewport, and records `headed` in the output. That choice is evidence, recorded, not a question.
16. **Order and repetition.** Trials alternate sides, A then B then A, with the starting side recorded and flipped between workloads, so slow drift lands on both sides equally. N is 9 for page ready, open, first image and window change, 5 for scroll, and 3 sessions for memory and CPU. The figures are the median, the 25th and 75th percentiles by linear interpolation between order statistics (the method numpy calls `linear`), the minimum and the maximum. A metric with fewer valid trials than N is `invalid` and has no median.
17. **When a difference is stated.** Only when the two sides' interquartile ranges do not overlap. Otherwise the summary prints both medians and "no separation at N trials". The ratio printed is Ocelli's median over cornerstone3D's, to three significant figures, with no adjective.
18. **The architectural difference is stated in every output.** The first paragraph of the plain report and of the tracked summary, and a top-level `architecture` field in both JSON files, carry one fixed text, given in the Scope section, that the renderer of either output cannot omit because the closed schema requires it.

## Problem and reproducer

Source identity: `sprint/s46` at `b13812c`, plus F-216 as implemented in `ocelli-wt-f-216` at `8b00223`, which this story depends on.

1. `node tools/bench/run.mjs --compare-cornerstone` prints `unknown argument --compare-cornerstone. Try --help.` and exits 2 (fact 1).
2. No page in the tree drives cornerstone3D on a folder of files. The oracle page renders one file per `render()` call at a fixed canvas and pins one worker for determinism, which is the opposite of a throughput setting (facts 5 and 6).
3. No collector in the tree counts a browser's process memory. F-216 counts the server only (fact 3).
4. The operator has no measured basis for any statement about cost relative to cornerstone3D, and the README promises that every published benchmark names its tier.

This is a new instrument. What is absent is the comparator page, the mode, the collector, the pair check, the workload selection, the publication path and its guards.

## Scope, API and compatibility

In scope: the comparison mode in `tools/bench`, the comparator page in `tools/oracle`, `ocelli-compare pair`, the two tracked output files, their floor checks, and the documentation lines below.

Non-goals: a budget or threshold of any kind, a baseline entry, an A7.3 answer, OHIF, `@cornerstonejs/tools`, the Ocelli wasm core or WebGPU path (no viewer page drives it), volume or MPR, compressed transfer syntaxes beyond what the inputs contain, network transfer, a Linux or Windows memory collector, and any change to the folder viewer itself.

### Command line

```text
bin/ocelli.sh bench --compare-cornerstone --pseudo <dir> [--publish] [--software] [--no-build] [--out <dir>]
```

| Flag | Meaning | Refusals |
|---|---|---|
| `--compare-cornerstone` | Run the comparison and nothing else. `runSubjects` is not called | With `--compare`, `--accept`, `--subject`, `--story`, `--why`, `--tolerance` or `--list`: exit 2 naming the conflict |
| `--pseudo <dir>` | The pseudo-PHI root. Also read from `OCELLI_PSEUDO_DIR` when the flag is absent | Absent from both, not a directory, or no Part 10 file under it: exit 2 before any launch. The message names the flag and never the directory's contents |
| `--publish` | Also write the two tracked files | Only with `--compare-cornerstone`. Refused with `--no-build`, because a publication must name a binary and a page this run built. Refused when any metric is `invalid` or the pixel check did not pass |
| `--software` | Permit a SwiftShader renderer and name it | Only with `--compare-cornerstone` |
| `--no-build` | Reuse the release binary and the built page | As today, and refused with `--publish` |
| `--out <dir>` | Where the ignored record goes, default `tools/bench/out/compare` | As today |

`bin/ocelli.sh bench` gains one check: with `--compare-cornerstone`, `tools/oracle/node_modules` must exist, and its absence prints `(cd tools/oracle && npm ci)` and exits 2. The existing `tools/bench/node_modules` check is unchanged.

### Outputs

| Path | Tracked | Content |
|---|---|---|
| `tools/bench/out/compare/comparison.json` | no, under the ignored `tools/bench/out/` | Every trial's raw figures, the inventory per sample, the pair verdict, machine, versions, defaults, refusals |
| `tools/bench/out/compare/comparison.txt` | no | The plain report, rendered from the JSON above |
| `docs/benchmarks/cornerstone-comparison.json` | yes, with `--publish` | The published subset under a closed schema: machine, versions, defaults, workloads by label, per metric and side the median, quartiles, minimum, maximum, N and state, the pair verdict, `architecture`, `not_shown`, input identity |
| `docs/benchmarks/cornerstone-comparison.md` | yes, with `--publish` | Rendered from the tracked JSON by `renderSummary`, byte for byte, with a first line naming the command that wrote it and "do not edit" |

The fixed `architecture` text, carried verbatim in every output:

> The two sides do different work in different places. The Ocelli folder viewer parses and decodes each file in a native process on this machine and sends 8-bit grey frames over loopback to the page, which draws them on a 2D canvas on the CPU. The cornerstone3D page parses and decodes in the browser, in dicom-image-loader's WebAssembly workers, and renders through WebGL. Memory and CPU are therefore counted over every process each side runs: for Ocelli the native process and every Chromium process, and for cornerstone3D every Chromium process, including the GPU process. This measures the local folder viewer example, not the Ocelli browser core, which has no viewer page to measure yet.

The fixed acknowledgement, second paragraph of the summary:

> cornerstone3D 5.8.2 is the reference this project learns from and validates against. This page records cost on one machine, for one set of inputs, at each side's shipped defaults. It is a measurement, not a ranking.

The `not_shown` list, fixed and rendered as its own section: one machine and one browser build, loopback only and no network, a warm operating system file cache, different default budgets on the two sides, a comparator page written for this measurement rather than an application such as OHIF, the matched prefetch policy, uncompressed inputs only so no codec cost, a 2D canvas against WebGL, and the tier: Ocelli tier C on the CPU, cornerstone3D on the named WebGL renderer.

### Metric definitions, fixed now in `tools/bench/comparison.json`

| Metric | Mirrors | Start | End | Cold | Warm | Workloads | N |
|---|---|---|---|---|---|---|---|
| `page_ready` | none | Ocelli: spawn of the release binary. cornerstone3D: `page.goto` of the built page | First rAF after the page reports ready: Ocelli `load` with `#folder` enabled, cornerstone3D `__bench.ready()` resolved after `coreInit`, loader `init` and `enableElement` | yes | unavailable, "a warm page load measures the browser cache, not either library" | none | 9 |
| `open` | `viewer.open` | `setInputFiles` on the side's folder input | First rAF after the side's series sentence is written, which needs every file's header parsed and grouped | yes | yes | synthetic CT, pseudo full | 9 |
| `first_image` | `viewer.first_image` | The series sentence | First rAF after the first commit of the selected series' first image | yes | yes | synthetic CT, synthetic DX, pseudo CT, pseudo DX | 9 |
| `scroll` | `viewer.scroll_rate` | First ArrowDown, one per rAF | 3 s or 150 keys, whichever is first, then 300 ms settle. Reports presented frames per second, keys delivered and blank samples. Fewer than 120 keys is invalid | unavailable, "scroll is measured after the first image" | yes | synthetic CT, pseudo CT | 5 |
| `window_change` | `viewer.window_step` | Harness `performance.now()` before the side's window entry point: Ocelli `overrideWindow` then `showIndex(viewIndex)`, cornerstone3D `setProperties({ voiRange, VOILUTFunction })` then `render()` | First rAF after the commit | unavailable, "a window change needs a drawn image" | yes | all four image workloads | 9 steps, centres 40 to 48, width 400, `LINEAR` |
| `peak_memory` | `viewer.resident_memory` | Session start, before the page loads | Session end: open pseudo full, first image, scroll pseudo CT, 9 window changes on pseudo CT and on pseudo DX, in one page | yes | n/a | the session | 3 sessions |
| `session_cpu` | none | Same session | Same session | yes | n/a | the session | 3 sessions |

A presented frame and a blank sample keep F-216's meaning: a presented frame is a drawn image index that changed since the previous rAF callback, and a blank sample is a rAF callback at which the side shows no image (Ocelli `stage.hidden`, cornerstone3D no `IMAGE_RENDERED` since `setStack`).

Clock: `performance.now()` in each page for durations, rounded with the harness's existing `atClockPrecision` to four decimal places. Process figures use `ps` and `footprint` from Node.

### Pixel agreement, a precondition

Before any timing trial, on pseudo CT's first image by the shared ordering, at centre 40, width 400, `LINEAR`:

1. The Ocelli page draws it. The harness reads `#view` with `getImageData`, whose size is the image's own (fact 4).
2. The comparator page sizes its element to the image's columns and rows at DPR 1, sets `interpolationType` `NEAREST` and the window through `toLowHighRange` as the oracle page does, calls `resetCamera`, paints the oracle's sentinel, renders, waits for `IMAGE_RENDERED`, refuses a sentinel canvas, and reads the canvas back.
3. Both sides report the selected file's name and size into the harness's memory. A mismatch refuses with "the two sides selected different images" and nothing about which.
4. `ocelli-compare pair --reference <cs.raw> --candidate <ocelli.raw> --width W --height H` prints a JSON verdict: `within_one_lsb_fraction`, `count_over_max`, `max_abs_diff`, `signed_mean_bias_informative`, `informative_fraction`, `passes`. `passes` is the monochrome predicate over the full frame and the bias bound over the informative region, and requires `informative_fraction >= INFORMATIVE_FRACTION_FLOOR`.
5. A failed or low-information verdict sets the run state to `refused_pixels`, writes the verdict numbers and no timing figure, and exits 1.

```rust
// tools/oracle/src/tolerance.rs
pub struct PairVerdict {
    pub within_one_lsb_fraction: f64,
    pub count_over_max: u64,
    pub max_abs_diff: u8,
    pub signed_mean_bias_informative: f64,
    pub informative_fraction: f64,
    pub low_information: bool,
    pub passes: bool,
}
/// # Errors
/// When the frames differ in size or the image region is empty.
pub fn pair_verdict(reference: &Frame, candidate: &Frame) -> Result<PairVerdict, ToleranceError>;
```

It calls `difference(reference, candidate, Rect::full(w, h), ChannelSet::Monochrome)`, then `monochrome_predicate` over `full`, `bias_bound` over `informative` and `informative_fraction()`. `ToleranceError` gains one variant for a frame error, through `#[from]`. No constant changes.

### JavaScript surface

```js
// tools/bench/src/comparison.mjs, pure, no browser, no process, floor-tested
export function loadComparison(text)                 // parse and validate comparison.json
export function trialOrder(sides, n, startSide)      // ["ocelli","cornerstone",...]
export function summarise(values, declaredN)         // { state, n, median, p25, p75, min, max }
export function separated(a, b)                      // IQR overlap rule
export function parseProcessTree(psText, rootPid)    // Set of descendant pids
export function parseFootprint(text)                 // Map pid -> { footprint, peak }
export function memoryFigures(samples)               // { sampled_peak, sum_of_peaks }
export function cpuFigure(before, after)             // seconds, or invalid when inventories differ
export function selectWorkloads(headers)             // the five rules over header records
export function inputIdentity(contentDigests)        // sha256 hex
export function scanForDenied(text, denySet)         // array of hit kinds, never the hit itself
export function publishedRecord(run)                 // closed-schema subset, throws on unknown key
export function renderSummary(published)             // the tracked Markdown
export function renderReport(run)                    // the plain report
export function otherBrowserJobs(psText, ownPids)    // count of ms-playwright processes outside our tree

// tools/bench/src/comparison_run.mjs, the orchestration
export async function runComparison(options)         // returns { state, record }
```

`run.mjs` imports `runComparison` with `await import` inside `main`, as `wasm_cold_start.mjs` does with Playwright, so the floor suites load `run.mjs` without the oracle install.

### Compatibility

`bin/ocelli.sh bench` without the new flag behaves as today and runs nothing new. `buildPage()` keeps its default output directory and gains `buildPage({ dist })`. `app.js` and `volume.js` are separate esbuild entry points with no code splitting, so a third entry point does not change their bytes, and AC15 measures that. `checkPins` becomes an export with an unchanged body. `ocelli-compare`'s four existing commands are unchanged.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `bin/ocelli.sh bench --compare-cornerstone` | `run.mjs` `main`, then `runComparison` | Refusals before any launch, lock, exclusive check | Exit code, the ignored record |
| Ocelli side | F-216 `ensureReleaseBinary`, `startViewer`, `stopViewer`, `ensureWorkloads` in `viewer_work.mjs` | Release profile only, `OCELLI_PORT=0`, 8742 and 8765 refused as F-216 does | Server pid and URL |
| Ocelli page timing | Page globals in fact 4, wrapped by an init script as F-216 does | No change to `page.html` | Commit timestamps |
| cornerstone3D side | `tools/oracle/build-page.mjs` `buildPage({ dist: tools/bench/out/compare/page })`, `src/server.mjs` `serveDirectory`, `run.mjs` `checkPins` | Pinned packages only, refuses drift | Page origin |
| Comparator page | `tools/oracle/page/bench.mjs` | Loader defaults, matched prefetch | `window.__bench` with `ready`, `open`, `selected`, `windowChange`, `capture`, `renderer` |
| Pixel check | `ocelli-compare pair`, built `--release -p ocelli-oracle --bin ocelli-compare` | Frames in a temporary directory outside the repo | Verdict JSON |
| Header reads for selection and deny set | `dicom-parser` 1.8.21 resolved from `tools/oracle/package.json` with `createRequire` | Headers only, `untilTag: "x7fe00010"` | Header records in memory |
| Memory and CPU | `footprint`, `ps`, CDP `SystemInfo.getProcessInfo` | macOS only for memory | Samples |
| `scripts/bench_check.py` | `comparison.json`, `subjects.json`, the tracked pair | Floor | Refusals |
| `tools/bench/tests/comparison_test.mjs` | `comparison.mjs` | Floor | Test results |

Units: milliseconds for durations, frames per second for scroll, bytes for memory, seconds for CPU. Rounding: durations by `atClockPrecision`, ratios to three significant figures, bytes reported exactly and rendered in MiB to one decimal. Selection is owned by `selectWorkloads`. No pixel arithmetic is done in JavaScript, and the grey values compared are each side's own output.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Idle | Flags valid, prerequisites present | `prepared`: lock held, page built, binary built, workloads selected, deny set loaded | Any refusal: exit 2, lock not taken or released, nothing written |
| `prepared` | Other `ms-playwright` process present | Refused, exit 2, count named | Lock released |
| `prepared` | Renderer or browser version differs between sides | Refused, exit 2, both strings recorded in the ignored record | Lock released, browsers closed, server stopped |
| `prepared` | Pixel check passes | `checked` | |
| `prepared` | Pixel check fails or is low-information | `refused_pixels`, verdict written, exit 1 | No timing figure written. A published pair already tracked is left as it was |
| `checked` | All trials run | `measured`, every metric `measured`, `unavailable` with reason, or `invalid` | |
| `measured` | `--publish` and no `invalid` metric | Deny-set scan, schema check, then both tracked files written | A hit or schema failure refuses the write and leaves the tracked pair unchanged |
| any | Error or signal | `finally`: browsers closed, servers stopped by the handle the harness holds, temp directory removed, lock removed | The error is carried into the ignored record when it can be written |

Invariants: the two sides never share a browser instance. Every timing figure in an output was taken in a run whose pixel check passed. Every output was scanned against the deny set of the inputs it describes. A tracked summary always renders from the tracked JSON beside it.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Lock directory | `mkdir` without `recursive` | The run | Exists | Second run refused. A stale lock after a crash is named with the command to remove it, never removed automatically |
| Temporary workload and frame directory | `mkdtemp` under `os.tmpdir()` | The run | n/a | Removed in `finally`. Its path is never written to an output |
| Release binary and built page | F-216 `ensureReleaseBinary`, `buildPage({ dist })` | Digest of each recorded | Recorded in the record | `--no-build` reuses and marks the record not publishable |
| Ocelli server | `startViewer` | Child handle and pid | URL printed | Stopped by handle in `finally` |
| Chromium instances | `chromium.launch` | Browser handle | `browser.version()` | Closed in `finally` |
| Ignored record | `writeFile` after the run | `started` timestamp | File written | Overwritten by the next run |
| Tracked pair | `--publish` | Input identity and run timestamp inside | Both files written, JSON first | Written only after both pass the scan and schema. A failure leaves both unchanged, so they never disagree |

An identical repeated run produces a new record with new figures, and that is correct, since each run is a new observation. Nothing is cached between runs except the built binary and page, whose digests are recorded.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `parseArgs` accepts `--compare-cornerstone`, `--pseudo`, `--publish`, `--software` and refuses every conflict in the flag table with exit 2 | `parseArgs refuses each comparison conflict` in `run_test.mjs` | A mode that silently also runs `runSubjects`, or `--publish` with `--no-build` | implementation |
| AC2 | Missing `--pseudo`, an empty root, a missing oracle install and a pin drift each refuse before any browser launch | `comparison_test.mjs` with an injected launcher that throws if called, plus `bin/ocelli.sh bench --compare-cornerstone` without `tools/oracle/node_modules` | A check after launch that leaves a browser running | implementation |
| AC3 | `otherBrowserJobs` counts `ms-playwright` processes outside our tree from fixture `ps` text, and the mode refuses on a non-zero count and never signals a process | `other browser jobs refuse the run` | A run beside the oracle, which reads as a slow side | implementation |
| AC4 | `selectWorkloads` over fixture header records: picks the largest qualifying CT series, breaks a tie by digest, picks the DX with most pixels, and returns `unavailable` with the rule sentence when nothing qualifies | `workload selection is by attribute`, with a fixture where the largest CT series lacks a window | Selecting by path, by first found, or substituting PT when no CT qualifies | implementation |
| AC5 | `summarise` over hand-computed sets: `[1,2,3,4,5,6,7,8,9]` gives median 5, p25 3, p75 7. `[10,20,30,40]` gives median 25, p25 17.5, p75 32.5. Fewer than N gives `invalid` | `quartiles are linear between order statistics` | Median of an unsorted list, or nearest-rank quartiles | implementation |
| AC6 | `separated` is true only when interquartile ranges do not overlap, including the touching case `p75(a) == p25(b)` being not separated | `a difference is stated only when spreads separate` | A rule on medians alone | implementation |
| AC7 | `trialOrder` alternates and flips start per workload | `trials alternate sides` | All A then all B | implementation |
| AC8 | `parseProcessTree` over fixture `ps` text returns the root and every descendant at depth 3, and excludes a sibling. `parseFootprint` over the recorded shape in fact 9 reads `phys_footprint` and `phys_footprint_peak` per pid. `memoryFigures` gives the hand-computed lower and upper figures for a three-sample fixture where peaks do not coincide | `memory counts every process in the inventory` | Counting the server only, which is F-216's scope, or the browser process only | implementation |
| AC9 | `cpuFigure` is `invalid` when the pid sets differ and the hand-computed sum otherwise | `cpu refuses a changed inventory` | Summing whatever pids are present at the end | implementation |
| AC10 | `ocelli-compare pair` over hand-built frames: identical gives `passes` true. One pixel at +3 gives `count_over_max` 1 and false. Every informative pixel at +1 gives `within_one_lsb_fraction` 1.0 and bias 1.0 and false. A frame with 5 informative pixels in 100 gives `low_information` true and false. Sizes that differ refuse | `tools/oracle/tests/pair_fixture.rs`, values derived from 25.1's text | A bias over the full frame, which a clipped background dilutes, or a passing low-information frame | implementation |
| AC11 | A failed pair verdict sets `refused_pixels`, writes the verdict and no timing figure, exits 1 | `runComparison` with an injected verdict, in `comparison_test.mjs` | Timing published beside a picture that differs | implementation |
| AC12 | `scanForDenied` finds a name, a UID, a date and a path component planted in a fixture record, returns hit kinds without the values, and `--publish` refuses the write | `outputs refuse an identifying value` | A scan of the Markdown only, or an error message that prints the hit | implementation |
| AC13 | `publishedRecord` throws on an unknown key and on a missing `architecture` or `not_shown` | `the published record has a closed schema` | A free-text field that could carry a header value | implementation |
| AC14 | `renderSummary` output contains the `architecture` text, the acknowledgement, the tier sentence, the machine, the Chromium and package versions, the defaults table and the `not_shown` section, contains no em-dash and no semicolon outside code, and contains none of a fixed list of adjectives (faster, slower, blazing, superior, outperforms, dramatically, significantly, best, better, worse) | `the summary is measured and acknowledging`, plus `prose_check.scan` over the rendered text | A template that drops the caveat when numbers favour one side | implementation |
| AC15 | Adding `bench.mjs` to `buildPage` leaves `app.js` and `volume.js` byte-identical, and `bin/ocelli.sh oracle` passes | SHA-256 of both files before and after, recorded in the handoff, and the oracle run | A shared chunk that moves the reference page | integrated, GPU exclusive |
| AC16 | `comparison.json` validated by `bench_check.py`: required fields, F-223 resolves through the allocation, every `mirrors` names a `subjects.json` id, N and workloads match this plan | `scripts/tests/test_bench_check.py` negative cases for each | A definition edited in one file and not the other | implementation |
| AC17 | When the tracked pair exists, `renderSummary(tracked json)` equals the tracked Markdown, and when F-223 is done the pair must exist | `the tracked summary is regenerated, not typed` in `comparison_test.mjs`, `check_comparison` in `bench_check.py` | A hand-edited summary | implementation, then integrated after publication |
| AC18 | Equipped run, both sides, every workload: both browsers report the same version and renderer, viewport 1600 by 1000 at DPR 1, pixel check passes, every metric `measured` or `unavailable` with reason | `bin/ocelli.sh bench --compare-cornerstone --pseudo <root>` on the host in Facts, record kept in the handoff by digest | A side launched with different flags, or SwiftShader unnamed | external, operator's host, serialised with every browser and GPU job |
| AC19 | Publication: `--publish` on that run writes the tracked pair, the deny-set scan passed, the temporary directory is gone, and `git status` shows no other new file. `bin/ocelli.sh gate content prose bench` passes on the staged pair | The equipped run plus the gates | Pseudo-PHI frames or paths left in the tree | external, then integrated |
| AC20 | Documentation lines below land, and `AGENTS.md`, `docs/lld/benchmarks.md` and `docs/lld/oracle.md` name the mode | `bin/ocelli.sh gate prose skills` | A mode nobody is told how to run | implementation |

A pixel refusal on the equipped run is a recorded outcome, not a passing AC18. If it happens, a story is filed for the divergence with the verdict numbers, and AC18 and AC19 stay open with that story named, so F-223 is carried rather than completed.

## Approach

1. `tools/oracle`: export `checkPins`, add the `dist` parameter to `buildPage`, add `page/bench.html` and `page/bench.mjs` as a third entry point. Measure AC15 before anything else changes.
2. `tools/oracle/src/tolerance.rs` and `src/bin/ocelli-compare.rs`: `pair_verdict` and the `pair` command, then `tests/pair_fixture.rs`. Each fixture value is computed by hand from 25.1's text and written as a literal.
3. `tools/bench/comparison.json`, then `src/comparison.mjs` and `tests/comparison_test.mjs` together, each pure function with its hand-computed fixture.
4. `src/comparison_run.mjs`: prerequisites, lock, exclusive check, build, selection with symbolic links, deny set, two browsers, pixel check, alternated trials, memory sessions, record, publication.
5. `run.mjs` flags and `bin/ocelli.sh` arm and test list, `tools/bench/package.json` test script and its `$why_no_other_dependency` note, `scripts/bench_check.py` `check_comparison` with its tests, `scripts/prose_check.py` prefix.
6. The equipped run, then `--publish`, then documentation.

Rejected: registry rows with runners (fact-based reason in decision 1), OHIF (lead decision 1), a JavaScript copy of the 25.1 predicate (a second copy of the tolerance, the defect class the oracle's comments warn about), `ps -o rss=` summed over Chromium (counts shared pages once per process, and footprint gives a kernel peak), loading `@cornerstonejs/tools` for prefetch (lead decision 1), and running both sides in one browser (an inventory that cannot be split).

## Dependencies, write ownership and resources

Declared dependency F-216, which must be done: this story imports its `viewer_work.mjs` helpers, mirrors its `viewer.*` subjects and drives its page. Source-level dependencies: the oracle's pinned install, `ocelli-oracle`'s comparator, and the sprint lead filing the F-223 backlog and allocation rows first.

Write set:

- `tools/bench/comparison.json` (new), `tools/bench/src/comparison.mjs` (new), `tools/bench/src/comparison_run.mjs` (new), `tools/bench/tests/comparison_test.mjs` (new), `tools/bench/run.mjs`, `tools/bench/tests/run_test.mjs` (appended), `tools/bench/package.json`
- `tools/oracle/build-page.mjs`, `tools/oracle/run.mjs` (export only), `tools/oracle/page/bench.html` (new), `tools/oracle/page/bench.mjs` (new), `tools/oracle/src/tolerance.rs`, `tools/oracle/src/bin/ocelli-compare.rs`, `tools/oracle/tests/pair_fixture.rs` (new)
- `bin/ocelli.sh` (bench arm test list and the oracle install check), `scripts/bench_check.py`, `scripts/tests/test_bench_check.py`, `scripts/prose_check.py`
- `docs/benchmarks/cornerstone-comparison.json` and `.md` (new, generated)
- `docs/lld/benchmarks.md`, `docs/lld/oracle.md`, `AGENTS.md` (one sentence), `README.md`, `examples/folder-viewer/src/use.html`

Shared files and order: `tools/bench/subjects.json` is read, not written. `tools/bench/run.mjs`, `tools/bench/package.json`, `bin/ocelli.sh` and `scripts/bench_check.py` are also written by F-216, so this story starts after F-216 integrates. `tools/oracle/*` has no other writer in S46. `README.md` is an exclusive resource under run-sprint section 3.

Exclusive resources: the equipped run and AC15's oracle run each need the GPU and the browser alone, per run-sprint section 3, and the mode's own check refuses a concurrent Playwright browser. Ports 8742 and 8765 are never used, and the static server refuses either if the operating system hands one back.

## Verification and evidence reuse

Focused: `bin/ocelli.sh gate bench`, `bin/ocelli.sh test ocelli-oracle`, `bin/ocelli.sh clippy ocelli-oracle`, `(cd tools/oracle && npm test)`, `bin/ocelli.sh gate prose content provenance skills`. Final: `bin/ocelli.sh gate --sprint` in the sprint verification.

Mutation checks, HLD 27.3. Each applied, the named test observed red, then reverted.

1. `separated` compares medians only. AC6 goes red.
2. `pair_verdict` takes the bias over `image` instead of `informative`. AC10's clipped-background fixture goes red.
3. `runComparison` skips the verdict check. AC11 goes red.
4. `scanForDenied` skips UIDs. AC12 goes red.
5. `parseProcessTree` returns direct children only. AC8's depth-3 fixture goes red.
6. `renderSummary` omits `not_shown` when Ocelli's medians are lower. AC14 goes red.
7. `summarise` uses nearest rank. AC5's four-value case goes red.

Equipped-host procedure for AC18 and AC19: host in Facts, nothing else running, `OCELLI_PSEUDO_DIR` set, `bin/ocelli.sh bench --compare-cornerstone`, read the record, then the same with `--publish`. Workload: the five rules. Release build of `ocelli-folder-viewer` and of `ocelli-compare`. No threshold, because lead decision 4 asks for figures and A7.3 says not to invent one.

Invalidates evidence: any change to `examples/folder-viewer/src`, `tools/oracle/page`, `tools/oracle/package.json` pins, `tools/bench/src/comparison*.mjs`, `tools/bench/comparison.json`, the Playwright pin, the release profile, the inputs (the input identity changes), or the host. A run that did not happen is a missing observation and does not become a passed row.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The Ocelli folder viewer receives grey bytes from its local process, as F-206, F-207 and F-216 accepted. The harness reads one canvas from each page for the pixel check, outside any product boundary.
- Render-loop allocation: none added. No product code changes.
- unsafe: none
- Tier A (WebGPU): n/a. The folder viewer opens no device. cornerstone3D's WebGL renderer is a property of the comparator, recorded by name, and not an Ocelli tier.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): full. The Ocelli side measured is the CPU `LutChain` in the native process, F-216's path, and the summary names tier C.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | `pair_verdict` against hand-computed frames for 25.1's within-one fraction, over-two count, informative bias and the information floor | `tools/oracle/tests/pair_fixture.rs` |
| unit | Quartiles, separation, trial order, process tree, footprint parse, memory and CPU figures, workload selection, deny-set scan, closed schema, rendering | `tools/bench/tests/comparison_test.mjs`, registered in `npm test` and the `bench` arm |
| unit | Flag parsing and refusals | `tools/bench/tests/run_test.mjs`, appended |
| contract | `comparison.json` against the registry and the allocation, and the tracked pair | `scripts/tests/test_bench_check.py` |
| browser | AC15, AC18, AC19 | `bin/ocelli.sh oracle`, the equipped comparison run |

No `fixture` row is needed for pixel arithmetic in the harness, because the harness does none. The grey values compared are each side's own output, and the one piece of arithmetic over them, the 25.1 verdict, is the oracle's.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` counts viewport types, tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation representations, core events and adapters. A cost comparison is on none of those axes.

## Deviations

none. The story sets no budget, changes no tolerance and writes no baseline. D-04 already covers a GPU-and-browser observation outside the CI floor, D-07 the tier C naming, and D-11 the 5.8.2 pin the comparator uses.

## LLD impact

`docs/lld/benchmarks.md` gains a section "Comparing against cornerstone3D": why it is a mode and not registry rows, the endpoints, the collector and its two bounds, the alternation and separation rules, the pixel precondition, the deny set and closed schema, and what the numbers do not show. `docs/lld/oracle.md` gains the third page and `ocelli-compare pair`, and states that neither is a reference run.

## User-facing documentation impact

- `README.md`, "Standing on cornerstone3D": one paragraph saying that a measured comparison of the local folder viewer against cornerstone3D 5.8.2 is recorded in `docs/benchmarks/cornerstone-comparison.md`, with the machine and tier named there, and the command that regenerates it. The paragraph carries no figure, because a figure in prose beside the file that holds it goes stale on the next run.
- `examples/folder-viewer/src/use.html`, chapter 16 "Testing and the oracle": one paragraph on `bin/ocelli.sh bench --compare-cornerstone`, that it needs the oracle install, a GPU-capable browser and a local study folder, that it refuses to compare speed when the two pictures differ, and where the summary lives.
- `examples/folder-viewer/src/more.html`: no change. It lists capabilities, and a benchmark is not one. Its `#worker` section already says the folder viewer is not the wasm path, which the summary repeats.

`use.html` joins the write set for that edit.

## Open questions

No material question remains. Each lead decision below was given by the sprint lead and is recorded as answered.

1. Comparator. Answered: lead decision under the operator's request of 2026-11-05. A bare cornerstone3D 5.8.2 page with `@cornerstonejs/core` `StackViewport` and `@cornerstonejs/dicom-image-loader` with its pinned codecs, exactly the packages pinned in `tools/oracle/package.json`, built by the oracle's page build. Not OHIF.
2. Ocelli side and process accounting. Answered: lead decision under the operator's request of 2026-11-05. The folder viewer as shipped after F-216, its architectural difference stated in every output, and memory counted over every process on both sides.
3. Inputs. Answered: lead decision under the operator's request of 2026-11-05. F-216's synthetic series and studies from the pseudo-PHI folder, which stays outside git, with no identifying value in any output.
4. Metrics. Answered: lead decision under the operator's request of 2026-11-05. F-216's five subjects where both sides can do them, cold and warm, N trials with median and spread, one Chromium binary and window size, GPU and CPU named, and unavailable with a reason rather than an estimate.
5. Pixel sanity. Answered: lead decision under the operator's request of 2026-11-05. The first CT image at one window on both sides, and the comparison refused beyond the 25.1 tolerance.
6. Output. Answered: lead decision under the operator's request of 2026-11-05. JSON and a plain report under an ignored directory, and a tracked summary in the measured, acknowledging tone, regenerated by the harness. This plan adds the tracked JSON beside the summary so regeneration is checkable on the floor.
7. Harness placement. Answered: lead decision under the operator's request of 2026-11-05. In `tools/bench`, through `bin/ocelli.sh`, not a floor gate, with floor unit tests for its logic. This plan chose the `--compare-cornerstone` mode, for the reason in decision 1 of "What the specification does not cover".
8. Exclusive GPU. Answered: lead decision under the operator's request of 2026-11-05. The run is serialised with every other browser or GPU job, and the mode refuses when it sees another Playwright browser.

Decisions this plan made locally, each stated above so a reviewer can challenge it: the header route through `dataSetCacheManager` (3), the matched prefetch policy (4), shipped defaults on both sides (5), the rAF-after-commit endpoint (6), the warm file cache (7), the two memory bounds (8), the CPU figure (9), the `pair` command (10), attribute selection (12), the deny set and closed schema (13), the browser mode probe (15), and the separation rule (17).
