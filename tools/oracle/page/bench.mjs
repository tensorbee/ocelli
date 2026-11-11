// The F-223 comparator page: the smallest cornerstone3D 5.8.2 page that does
// what the folder viewer does, so the local viewer and the incumbent can be
// measured on one machine. It exposes exactly one object, `window.__bench`,
// whose contract the comparison runner drives:
//
//   ready()        resolves after core init, loader init and enableElement
//   metadata()     actual { core_version, renderer, cache_bytes, workers,
//                  hardware_concurrency }, nothing invented
//   open(files)    parses every file's header, groups them into series and
//                  writes the series sentence; resolves there, not after the
//                  first render. Called with no argument it returns the
//                  current open promise.
//   selected()     the current image's { name, size, columns, rows, index,
//                  frames } for the pixel precondition, in memory only
//   snapshot()     { openedAt, committedAt, commitSequence, index, blank,
//                  generation }, all clocks `performance.now()`
//   windowChange() applies one VOI window and resolves at the first
//                  requestAnimationFrame after that window's IMAGE_RENDERED
//   capture()      reads the current image back at its native size for the
//                  pixel check only
//   renderer()     the unmasked WEBGL renderer string, or a refusal
//
// Two rules hold throughout. No identifying value leaves the page: file
// names and sizes exist only in `selected()` and `capture()` return values
// and in nothing this page writes to the DOM, the console or an error, and
// every error message is a fixed sentence. And the page is a measurement
// subject, not a product: it holds no state it cannot re-derive from a fresh
// `open`, and one StackViewport is recreated per open so no event from an
// earlier folder can be mistaken for this one's.

import {
  Enums,
  RenderingEngine,
  cache,
  getWebWorkerManager,
  imageLoader,
  metaData,
  utilities,
  version as coreVersion,
  init as coreInit,
} from "@cornerstonejs/core";
import dicomImageLoader from "@cornerstonejs/dicom-image-loader";

import { groupBenchSeries, selectLargestSeries, loaderVoiValue } from "../src/bench-series.mjs";

const RENDERING_ENGINE_ID = "ocelliBench";
const VIEWPORT_ID = "bench";
const VIEW_ID = "view";
const FOLDER_ID = "folder";
const STATUS_ID = "status";

/**
 * Painted over the viewport canvas immediately before every deliberate
 * render, exactly as the reference page does. A canvas the render pipeline
 * never reached still reads back as this colour, which is how `capture`
 * refuses an IMAGE_RENDERED that presented nothing.
 */
const SENTINEL = [255, 0, 255, 255];

/**
 * How long any single commit may take to arrive. It exists so a refused
 * observation is an error rather than a hang, and it is a backstop, not a
 * timing parameter: nothing here measures against it.
 */
const COMMIT_TIMEOUT_MS = 30000;

// The loader resolves its codec `.wasm` binaries against `wasmBasePath`, and
// build-page copies them beside the built pages. This document sits at the
// dist root, so stripping its last path segment is the same origin-relative
// `wasm/` the reference driver passes.
const WASM_BASE = window.location.href.replace(/[^/]*$/, "") + "wasm/";

const view = document.getElementById(VIEW_ID);
const folderInput = document.getElementById(FOLDER_ID);
const statusElement = document.getElementById(STATUS_ID);

const state = {
  generation: 0,
  engine: null,
  viewport: null,
  viewportId: null,
  countingViewportId: null,
  stack: null,
  imageIds: null,
  stackPromise: null,
  openPromise: null,
};

let index = -1;
let presentedIndex = -1;
let blank = true;
let openedAt = 0;
let committedAt = 0;
let commitSequence = 0;

const commitWaiters = new Set();

class OpenSuperseded extends Error {
  constructor() {
    super("open: superseded by a newer open");
  }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function bytesToBase64(bytes) {
  // Same walk as the reference page's: String.fromCharCode.apply blows the
  // argument limit on a multi-megabyte frame, so this chunks at 0x8000.
  const chunkSize = 0x8000;
  let binary = "";
  for (let chunk = 0; chunk < bytes.length; chunk += chunkSize) {
    binary += String.fromCharCode.apply(
      null,
      bytes.subarray(chunk, chunk + chunkSize),
    );
  }
  return btoa(binary);
}

/** The one RGBA value every pixel carries, or `null` when they differ. */
function uniformValue(rgba) {
  const first = [rgba[0], rgba[1], rgba[2], rgba[3]];
  for (let pixel = 4; pixel < rgba.length; pixel += 4) {
    if (
      rgba[pixel] !== first[0] ||
      rgba[pixel + 1] !== first[1] ||
      rgba[pixel + 2] !== first[2] ||
      rgba[pixel + 3] !== first[3]
    ) {
      return null;
    }
  }
  return first;
}

function isSentinel(value) {
  return (
    value !== null &&
    value[0] === SENTINEL[0] &&
    value[1] === SENTINEL[1] &&
    value[2] === SENTINEL[2] &&
    value[3] === SENTINEL[3]
  );
}

function paintSentinel(canvasElement) {
  const context = canvasElement.getContext("2d");
  context.fillStyle = `rgba(${SENTINEL[0]},${SENTINEL[1]},${SENTINEL[2]},1)`;
  context.fillRect(0, 0, canvasElement.width, canvasElement.height);
}

function nextFrame() {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => resolve());
  });
}

/**
 * The next IMAGE_RENDERED counted for the current viewport, never one that
 * has already landed. The caller registers the waiter before it renders, so
 * the commit it awaits is that render's own, and `afterSequence` is the
 * sequence captured at entry, so an event counted for an earlier action
 * cannot satisfy it twice.
 */
function waitCommit(afterSequence, what) {
  return new Promise((resolve, reject) => {
    const waiter = { afterSequence, settle: null, fail: null };
    const timer = window.setTimeout(() => {
      commitWaiters.delete(waiter);
      reject(new Error(`${what}: no IMAGE_RENDERED within ${COMMIT_TIMEOUT_MS} ms`));
    }, COMMIT_TIMEOUT_MS);
    waiter.settle = (timestamp, sequence) => {
      window.clearTimeout(timer);
      commitWaiters.delete(waiter);
      resolve({ committedAt: timestamp, commitSequence: sequence });
    };
    waiter.fail = (error) => {
      window.clearTimeout(timer);
      commitWaiters.delete(waiter);
      reject(error);
    };
    commitWaiters.add(waiter);
  });
}

function onImageRendered(event) {
  // Only the current generation's viewport counts. Earlier viewports are
  // disabled and their ids retired, so an event that carries one is refused
  // here rather than being allowed to clear `blank` for a newer source.
  if (state.countingViewportId === null) {
    return;
  }
  if (event.detail?.viewportId !== state.countingViewportId) {
    return;
  }
  commitSequence += 1;
  committedAt = window.performance.now();
  const imageId = state.viewport.getCornerstoneImage()?.imageId;
  presentedIndex = state.imageIds?.indexOf(imageId) ?? -1;
  blank = false;
  for (const waiter of [...commitWaiters]) {
    if (commitSequence > waiter.afterSequence) {
      waiter.settle(committedAt, commitSequence);
    }
  }
}

/**
 * Wait until the current source has settled: its stack presented, and every
 * commit that belonged to an earlier action counted. A window change or a
 * capture that starts from here measures its own render, not one that was
 * already in flight.
 */
async function ensureSettled(what) {
  if (state.stack === null || state.stack.length === 0) {
    throw new Error(`${what}: no image is displayed`);
  }
  const stackPromise = state.stackPromise;
  if (stackPromise !== null) {
    try {
      await stackPromise;
    } catch {
      throw new Error(`${what}: no image is displayed`);
    }
  }
  if (blank) {
    await waitCommit(commitSequence, what);
  }
}

function readWindow(request, what) {
  const { center, width, function: voiFunction = "LINEAR" } = request ?? {};
  if (
    typeof center !== "number" ||
    !Number.isFinite(center) ||
    typeof width !== "number" ||
    !Number.isFinite(width)
  ) {
    throw new Error(`${what}: centre and width must be finite numbers`);
  }
  if (voiFunction !== "LINEAR") {
    throw new Error(`${what}: only the LINEAR VOI LUT function is implemented`);
  }
  return { center, width };
}

// ---------------------------------------------------------------------------
// Renderer identity
// ---------------------------------------------------------------------------

let cachedRenderer;

function unmaskedRenderer() {
  if (cachedRenderer !== undefined) {
    return cachedRenderer;
  }
  const scratch = document.createElement("canvas");
  const context =
    scratch.getContext("webgl2") ?? scratch.getContext("webgl");
  const extension = context?.getExtension("WEBGL_debug_renderer_info");
  cachedRenderer =
    context && extension
      ? (context.getParameter(extension.UNMASKED_RENDERER_WEBGL) ?? null)
      : null;
  return cachedRenderer;
}

function renderer() {
  const value = unmaskedRenderer();
  if (value === null) {
    throw new Error(
      "renderer: the browser reported no unmasked WebGL renderer through WEBGL_debug_renderer_info",
    );
  }
  return value;
}

function metadata() {
  const registry = getWebWorkerManager().workerRegistry?.dicomImageLoader;
  return {
    core_version: coreVersion,
    renderer: unmaskedRenderer(),
    cache_bytes: cache.getMaxCacheSize(),
    workers: registry ? registry.instances.filter(Boolean).length : 0,
    hardware_concurrency: navigator.hardwareConcurrency,
  };
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

async function ready() {
  try {
    coreInit();
    // Shipped defaults: no maxWebWorkers, so the loader registers
    // floor(hardwareConcurrency / 2) decode workers, and no cache override,
    // so core keeps its own 3 GiB limit. Both actuals are what metadata()
    // reports.
    dicomImageLoader.init({ wasmBasePath: WASM_BASE });
    // The grouping reads its attributes through core.metaData, and the
    // provider that answers from the loader's own header cache is the one
    // wadouri exports. Registered after init, so the decode route itself
    // stays the loader's shipped default.
    // wadouri returns a VOI string, but loader5.8.2 createImage indexes [0].
    // See the same installed-source defect in oracle app.mjs fileWindow.
    const provider = dicomImageLoader.wadouri.metaData.metaDataProvider;
    metaData.addProvider((type, imageId) => {
      const value = provider(type, imageId);
      return type === Enums.MetadataModules.VOI_LUT && value
        ? { ...value, voiLUTFunction: loaderVoiValue(value.voiLUTFunction) } : value;
    });

    state.engine = new RenderingEngine(RENDERING_ENGINE_ID);
    const viewportId = viewportIdForGeneration(state.generation);
    state.engine.enableElement({
      viewportId,
      type: Enums.ViewportType.STACK,
      element: view,
      defaultOptions: { background: [0, 0, 0] },
    });
    state.viewportId = viewportId;
    state.countingViewportId = viewportId;
    state.viewport = state.engine.getViewport(viewportId);
    folderInput.disabled = false;
  } catch {
    throw new Error("ready: the comparator page could not be initialised");
  }
}

function viewportIdForGeneration(generation) {
  return `${VIEWPORT_ID}:${generation}`;
}

// ---------------------------------------------------------------------------
// Open
// ---------------------------------------------------------------------------

/**
 * One open, from the file list to the series sentence.
 *
 * The promise is resolved the moment the sentence is written. The selected
 * series' stack is presented after that, asynchronously, because the metric
 * that ends at the sentence and the metric that ends at the first commit are
 * different measurements.
 */
async function runOpen(files, generation, resolve, reject) {
  try {
    // The previous generation's viewport goes away first. Its pending
    // renders are dropped because its id is no longer in the engine, and the
    // commit handler above counts this generation's id only.
    if (state.viewportId !== null) {
      state.engine.disableElement(state.viewportId);
    }
    view.replaceChildren();
    view.style.width = "";
    view.style.height = "";
    state.viewport = null;
    state.viewportId = null;
    state.countingViewportId = null;
    state.stack = null;
    state.imageIds = null;
    state.stackPromise = null;
    index = -1;
    presentedIndex = -1;
    blank = true;
    openedAt = 0;
    committedAt = 0;
    for (const waiter of [...commitWaiters]) {
      waiter.fail(
        new Error("commit: the open changed before the image was presented"),
      );
    }
    // The loader's own caches are released. `fileManager` is deliberately
    // not purged: its indexes name the cache keys every provider reads, and
    // reusing them after a purge could hand this open a data set parsed for
    // the previous one. New files take fresh indexes instead.
    dicomImageLoader.wadouri.dataSetCacheManager.purge();
    cache.purgeCache();

    const rows = [];
    let unsupported = 0;
    for (let fileIndex = 0; fileIndex < files.length; fileIndex += 1) {
      if (generation !== state.generation) {
        throw new OpenSuperseded();
      }
      statusElement.textContent = `Reading ${fileIndex + 1} of ${files.length}`;
      const file = files[fileIndex];
      const imageId = dicomImageLoader.wadouri.fileManager.add(file);
      const uri = dicomImageLoader.wadouri.parseImageId(imageId).url;
      let dataSet;
      try {
        dataSet = await dicomImageLoader.wadouri.dataSetCacheManager.load(
          uri,
          dicomImageLoader.wadouri.loadFileRequest,
          imageId,
        );
      } catch {
        unsupported += 1;
        continue;
      }
      if (generation !== state.generation) {
        throw new OpenSuperseded();
      }
      const seriesModule =
        metaData.get(Enums.MetadataModules.GENERAL_SERIES, imageId) ?? {};
      const imageModule =
        metaData.get(Enums.MetadataModules.GENERAL_IMAGE, imageId) ?? {};
      // PS3.3 C.7.6.6, Value 1 of (0028,0008), VR IS. The image pixel module
      // does not carry it, and the loader's own parsed data set is where the
      // frame addressing reads it from.
      const reported = Number(dataSet.intString("x00280008"));
      rows.push({
        file,
        imageId,
        seriesUid: seriesModule.seriesInstanceUID,
        seriesNumber: seriesModule.seriesNumber,
        seriesDescription: seriesModule.seriesDescription,
        instanceNumber: imageModule.instanceNumber,
        frames: Number.isFinite(reported) ? reported : undefined,
      });
    }

    if (generation !== state.generation) {
      throw new OpenSuperseded();
    }

    const seriesList = groupBenchSeries(rows);
    const largest = selectLargestSeries(seriesList);
    const notRead = unsupported
      ? ` ${unsupported} file${unsupported === 1 ? "" : "s"} could not be read.`
      : "";
    statusElement.textContent = largest
      ? `${seriesList.length} series.${notRead}`
      : `No DICOM series in that folder.${notRead}`;
    openedAt = window.performance.now();
    resolve();

    if (largest !== null) {
      const presenting = presentSeries(largest, generation);
      state.stackPromise = presenting;
      presenting.catch(() => {
        if (generation === state.generation) {
          statusElement.textContent =
            "The selected series could not be displayed.";
        }
      });
      await presenting;
    }
  } catch (error) {
    reject(
      error instanceof OpenSuperseded
        ? error
        : new Error("open: the folder could not be opened"),
    );
  }
}

/**
 * Present the selected series in a viewport created for this generation.
 *
 * `setStack` itself renders the first image once it is loaded; the explicit
 * reset and render after it fold into the same frame, so the first commit
 * the harness sees is the first image at its own window.
 */
async function presentSeries(series, generation) {
  const viewportId = viewportIdForGeneration(generation);
  state.engine.enableElement({
    viewportId,
    type: Enums.ViewportType.STACK,
    element: view,
    defaultOptions: { background: [0, 0, 0] },
  });
  const viewport = state.engine.getViewport(viewportId);
  state.viewport = viewport;
  state.viewportId = viewportId;
  state.countingViewportId = viewportId;
  const imageIds = series.images.map((entry) => entry.imageId);
  state.stack = series.images;
  state.imageIds = imageIds;
  index = 0;
  blank = true;
  await viewport.setStack(imageIds, 0);
  if (generation !== state.generation) {
    return;
  }
  viewport.resetCamera();
  viewport.render();
  schedulePrefetch(0, imageIds);
}

function schedulePrefetch(currentIndex, imageIds) {
  // The folder viewer's window, next two and previous one, issued through
  // the core call its own prefetch utility uses. A repeated call reuses the
  // cached or in-flight load object, so it is free.
  for (const offset of [1, 2, -1]) {
    const target = currentIndex + offset;
    if (target < 0 || target >= imageIds.length) {
      continue;
    }
    imageLoader.loadAndCacheImage(imageIds[target]).catch(() => {});
  }
}

function open(files) {
  if (files === undefined) {
    if (state.openPromise === null) {
      return Promise.reject(new Error("open: no folder open is in progress"));
    }
    return state.openPromise;
  }
  if (state.engine === null) {
    return Promise.reject(new Error("open: the page is not ready"));
  }
  const list = Array.from(files);
  const previous = state.openPromise;
  state.generation += 1;
  const generation = state.generation;
  state.openPromise = new Promise((resolve, reject) => {
    runOpen(list, generation, resolve, reject);
  });
  if (previous !== null) {
    // An orphaned open refuses into the void rather than as an unhandled
    // rejection. A caller that held it has already been superseded.
    previous.catch(() => {});
  }
  return state.openPromise;
}

// ---------------------------------------------------------------------------
// Observation
// ---------------------------------------------------------------------------

function selected() {
  const stack = state.stack;
  if (stack === null || index < 0 || index >= stack.length) {
    return null;
  }
  const entry = stack[index];
  const pixel =
    metaData.get(Enums.MetadataModules.IMAGE_PIXEL, entry.imageId) ?? {};
  return {
    name: entry.file.name,
    size: entry.file.size,
    columns: pixel.columns ?? null,
    rows: pixel.rows ?? null,
    index,
    frames: entry.frames,
  };
}

function snapshot() {
  const canvas = state.viewport?.getCanvas();
  const box = canvas?.getBoundingClientRect();
  const hidden = !canvas || canvas.hidden || view.hidden || !box?.width || !box?.height ||
    window.getComputedStyle(canvas).visibility !== "visible";
  return {
    openedAt,
    committedAt,
    commitSequence,
    index: presentedIndex,
    blank: blank || Boolean(hidden),
    generation: state.generation,
  };
}

// ---------------------------------------------------------------------------
// Window change and capture
// ---------------------------------------------------------------------------

async function windowChange(request) {
  const { center, width } = readWindow(request, "windowChange");
  const startedAt = window.performance.now();
  await ensureSettled("windowChange");
  const viewport = state.viewport;
  const waited = waitCommit(commitSequence, "windowChange");
  // The same conversion the reference page applies, with the same argument
  // order: width, then centre, then the function.
  viewport.setProperties({
    voiRange: utilities.windowLevel.toLowHighRange(
      width,
      center,
      Enums.VOILUTFunctionType.LINEAR,
    ),
    VOILUTFunction: Enums.VOILUTFunctionType.LINEAR,
  });
  viewport.render();
  const commit = await waited;
  await nextFrame();
  return {
    elapsed_ms: window.performance.now() - startedAt,
    committedAt: commit.committedAt,
    commitSequence: commit.commitSequence,
  };
}

async function capture(request) {
  const { center, width } = readWindow(request, "capture");
  await ensureSettled("capture");
  const viewport = state.viewport;
  const imageId = viewport.getCurrentImageId();
  const pixel =
    metaData.get(Enums.MetadataModules.IMAGE_PIXEL, imageId) ?? {};
  const columns = pixel.columns;
  const rows = pixel.rows;
  if (
    typeof columns !== "number" ||
    typeof rows !== "number" ||
    columns < 1 ||
    rows < 1
  ) {
    throw new Error("capture: the displayed image declares no pixel dimensions");
  }

  // Native size at device pixel ratio 1: the canvas backing store follows
  // the element, and the engine re-reads the element on resize.
  view.style.width = `${columns}px`;
  view.style.height = `${rows}px`;
  state.engine.resize();
  viewport.setProperties({
    interpolationType: Enums.InterpolationType.NEAREST,
    voiRange: utilities.windowLevel.toLowHighRange(
      width,
      center,
      Enums.VOILUTFunctionType.LINEAR,
    ),
    VOILUTFunction: Enums.VOILUTFunctionType.LINEAR,
  });
  viewport.resetCamera();

  const canvasElement = viewport.getCanvas();
  paintSentinel(canvasElement);
  const waited = waitCommit(commitSequence, "capture");
  viewport.render();
  await waited;
  await nextFrame();

  if (canvasElement.width !== columns || canvasElement.height !== rows) {
    throw new Error(
      `capture: the canvas is ${canvasElement.width}x${canvasElement.height} ` +
        `and the image is ${columns}x${rows}`,
    );
  }
  const image = canvasElement
    .getContext("2d")
    .getImageData(0, 0, canvasElement.width, canvasElement.height);
  const rgba = new Uint8Array(image.data.buffer.slice(0));
  if (isSentinel(uniformValue(rgba))) {
    throw new Error(
      "capture: the canvas kept the sentinel colour, so IMAGE_RENDERED fired without anything reaching it",
    );
  }
  const entry =
    state.stack.find((candidate) => candidate.imageId === imageId) ?? null;
  if (entry === null) {
    throw new Error("capture: the displayed image is not the selected one");
  }
  return {
    width: canvasElement.width,
    height: canvasElement.height,
    rgba_base64: bytesToBase64(rgba),
    name: entry.file.name,
    size: entry.file.size,
  };
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

view.addEventListener(Enums.Events.IMAGE_RENDERED, onImageRendered);

// The harness drives scroll by dispatching ArrowDown keydown events, and the
// folder viewer binds its own at window level, so this one lives there too:
// an event dispatched at document or on any element inside the page reaches
// it by bubbling, which a listener on #view alone would miss.
window.addEventListener("keydown", (event) => {
  if (event.key !== "ArrowDown") {
    return;
  }
  event.preventDefault();
  const stack = state.stack;
  const viewport = state.viewport;
  if (stack === null || stack.length === 0 || viewport === null) {
    return;
  }
  const next = Math.min(index + 1, stack.length - 1);
  if (next === index) {
    return;
  }
  const generation = state.generation;
  index = next;
  // A pending request retains the previous visible canvas. Only an actual
  // render advances the presented index observed by the collector.
  viewport
    .setImageIdIndex(next)
    .then(() => {
      if (generation === state.generation && state.imageIds !== null) {
        schedulePrefetch(next, state.imageIds);
      }
    })
    .catch(() => {
      // A failed request retains the prior visible image and presented index.
      // No requested index is invented as a new presented frame.
    });
});

folderInput.addEventListener("change", () => {
  const files = Array.from(folderInput.files ?? []);
  if (files.length > 0) {
    open(files);
  }
});

window.__bench = {
  ready,
  metadata,
  open,
  selected,
  snapshot,
  windowChange,
  capture,
  renderer,
};
window.__benchLoaded = true;
