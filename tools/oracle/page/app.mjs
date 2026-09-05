// The reference render page. Runs inside headless Chromium, driven by run.mjs.
//
// It exposes exactly one object, `window.__oracle`, with two entry points:
// `ready()` once, and `render()` per corpus row. Every failure it can see is
// returned as a structured result rather than thrown into a console nobody
// reads, because the sprint's named defect is a page that "can start, load a
// test runner and exit successfully without decoding every corpus row,
// presenting a frame or reading back the rendered pixels".
//
// Three of the four boundaries are observed here. Boundary one, that every row
// was attempted, is the driver's, because only the driver has the manifest.

import {
  Enums,
  RenderingEngine,
  cache,
  getEffectiveRenderBackend,
  getRenderingCapabilities,
  getShouldUseCPURendering,
  imageLoader,
  init as coreInit,
  metaData,
  utilities,
  version as coreVersion,
} from "@cornerstonejs/core";
import dicomImageLoader from "@cornerstonejs/dicom-image-loader";
import dicomParser from "dicom-parser";

import { resolveVoi } from "../src/voi.mjs";

const RENDERING_ENGINE_ID = "ocelliOracle";
const VIEWPORT_ID = "reference";

/**
 * Painted over the viewport canvas immediately before `render()`.
 *
 * A blank canvas reads back perfectly and hashes stably, which is exactly why
 * "the pixels came back" is not the same claim as "something was drawn". If
 * cornerstone3D does not overwrite the canvas, the readback is still this
 * colour and the driver refuses the row.
 */
const SENTINEL = [255, 0, 255, 255];

const state = {
  element: null,
  engine: null,
  viewport: null,
};

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function base64ToBytes(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function bytesToBase64(bytes) {
  // String.fromCharCode.apply blows the argument limit on a 1 MB frame, so
  // this walks in chunks. 0x8000 is well inside every engine's limit.
  const chunkSize = 0x8000;
  let binary = "";
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode.apply(
      null,
      bytes.subarray(index, index + chunkSize),
    );
  }
  return btoa(binary);
}

async function sha256Hex(bytes) {
  const buffer = await crypto.subtle.digest(
    "SHA-256",
    bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
  );
  return [...new Uint8Array(buffer)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function numbersFrom(text) {
  if (text === undefined || text === null || text === "") {
    return null;
  }
  const parts = String(text)
    .split("\\")
    .map((part) => Number(part.trim()));
  return parts.some((value) => !Number.isFinite(value)) ? null : parts;
}

function firstOrNull(values) {
  return values && values.length > 0 ? values[0] : null;
}

// ---------------------------------------------------------------------------
// An independent read of the file's own attributes
// ---------------------------------------------------------------------------

/**
 * Read the pixel-module and VOI attributes straight from the bytes.
 *
 * Deliberately NOT taken from cornerstone3D's metadata providers. HLD section
 * 11 diffs metadata alongside pixels "because a wrong rescale slope can still
 * produce a plausible image", and a sidecar that transcribed the reference's
 * own reading could not show the reference reading it wrong. The result is
 * cross-checked against pydicom by `check_sidecars.py`.
 *
 * Returns `{ attributes, error }`. A parse failure is recorded, not thrown:
 * whether the row decodes is cornerstone3D's answer to give, not this
 * function's.
 */
function readAttributes(bytes) {
  let dataSet;
  try {
    dataSet = dicomParser.parseDicom(bytes);
  } catch (error) {
    return { attributes: null, error: String(error?.message ?? error) };
  }
  const string = (tag) => {
    const value = dataSet.string(tag);
    return value === undefined || value === "" ? null : value;
  };
  const uint16 = (tag) => {
    const value = dataSet.uint16(tag);
    return value === undefined ? null : value;
  };
  const decimals = (tag) => numbersFrom(dataSet.string(tag));
  const decimal = (tag) => firstOrNull(decimals(tag));

  return {
    error: null,
    attributes: {
      // PS3.10 7.1 File Meta, then PS3.3 C.12.1 SOP Common and C.7.3.1
      // General Series.
      transferSyntaxUID: string("x00020010"),
      sopClassUID: string("x00080016"),
      modality: string("x00080060"),
      // PS3.3 C.7.6.3, Image Pixel.
      photometricInterpretation: string("x00280004"),
      samplesPerPixel: uint16("x00280002"),
      planarConfiguration: uint16("x00280006"),
      rows: uint16("x00280010"),
      columns: uint16("x00280011"),
      bitsAllocated: uint16("x00280100"),
      bitsStored: uint16("x00280101"),
      highBit: uint16("x00280102"),
      pixelRepresentation: uint16("x00280103"),
      // PS3.3 C.7.6.6, Multi-frame Module. Not Image Pixel.
      numberOfFrames: decimal("x00280008"),
      // PS3.3 C.11.1, Modality LUT.
      rescaleSlope: decimal("x00281053"),
      rescaleIntercept: decimal("x00281052"),
      rescaleType: string("x00281054"),
      // PS3.3 C.11.2, VOI LUT.
      windowCenter: decimals("x00281050"),
      windowWidth: decimals("x00281051"),
      voiLutFunction: string("x00281056"),
      // PS3.3 C.7.6.2, Image Plane.
      pixelSpacing: decimals("x00280030"),
      imagePositionPatient: decimals("x00200032"),
      imageOrientationPatient: decimals("x00200037"),
      sliceThickness: decimal("x00180050"),
      // PS3.3 C.7.6.1.1.5.
      lossyImageCompression: string("x00282110"),
      lossyImageCompressionMethod: string("x00282114"),
    },
  };
}

/**
 * The window the FILE asks for, and where it was found.
 *
 * Two sources, in order, because neither alone covers the corpus:
 *
 * 1. The top level tags, read independently above. This is the answer for
 *    every single-frame instance.
 * 2. cornerstone3D's own `voiLutModule`, which is where a multi-frame
 *    instance's per-frame functional groups (PS3.3 C.7.6.16.2.10) surface.
 *    `synthetic/ct_multiframe_perframe.dcm` exists to trap a reader that
 *    looks only at the top level, and it has no top level VOI at all.
 *
 * NOT taken from the loaded image's `voiLUTFunction`. In cornerstone3D 5.8.2,
 * `createImage.js` computes it as
 * `(voiLutModule.voiLUTFunction?.length && voiLutModule.voiLUTFunction[0])`,
 * which indexes a STRING and yields "L" for a file saying "LINEAR". Feeding
 * that back into the reference's own `toLowHighRange` throws "Invalid VOI LUT
 * function". The harness reads the module, not the derived field.
 */
function fileWindow(attributes, cornerstoneMetadata) {
  const asArray = (value) =>
    value === undefined || value === null
      ? null
      : Array.isArray(value)
        ? value
        : [value];
  const asString = (value) =>
    Array.isArray(value) ? (value[0] ?? null) : (value ?? null);

  if (attributes?.windowCenter && attributes?.windowWidth) {
    return {
      origin: "file-top-level",
      windowCenter: attributes.windowCenter,
      windowWidth: attributes.windowWidth,
      voiLutFunction: attributes.voiLutFunction ?? null,
    };
  }
  const module_ = cornerstoneMetadata?.voiLutModule;
  if (module_ && !module_.error && module_.windowCenter && module_.windowWidth) {
    return {
      origin: "file-functional-groups",
      windowCenter: asArray(module_.windowCenter),
      windowWidth: asArray(module_.windowWidth),
      voiLutFunction: asString(module_.voiLUTFunction),
    };
  }
  return {
    origin: "absent",
    windowCenter: null,
    windowWidth: null,
    voiLutFunction: null,
  };
}

/** The modules cornerstone3D itself resolved, recorded beside the file's own. */
function readCornerstoneMetadata(imageId) {
  const modules = {};
  for (const name of [
    "imagePixelModule",
    "modalityLutModule",
    "voiLutModule",
    "imagePlaneModule",
    "generalSeriesModule",
    "sopCommonModule",
  ]) {
    try {
      modules[name] = metaData.get(name, imageId) ?? null;
    } catch (error) {
      modules[name] = { error: String(error?.message ?? error) };
    }
  }
  return modules;
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

async function ready(setup) {
  const { canvas, background, wasmBasePath } = setup;

  const capabilities = getRenderingCapabilities();

  coreInit();
  dicomImageLoader.init({
    // One worker, so decode order is fixed and two runs cannot interleave
    // differently. This is a reference renderer, not a throughput test.
    maxWebWorkers: 1,
    wasmBasePath,
  });

  const element = document.getElementById("viewport");
  element.style.width = `${canvas.width}px`;
  element.style.height = `${canvas.height}px`;
  state.element = element;

  state.engine = new RenderingEngine(RENDERING_ENGINE_ID);
  state.engine.enableElement({
    viewportId: VIEWPORT_ID,
    type: Enums.ViewportType.STACK,
    element,
    defaultOptions: {
      background: [
        background[0] / 255,
        background[1] / 255,
        background[2] / 255,
      ],
    },
  });
  state.viewport = state.engine.getViewport(VIEWPORT_ID);

  return {
    versions: {
      cornerstoneCore: coreVersion,
      userAgent: navigator.userAgent,
    },
    rendering: {
      // The plan's step 2: the adapter string is recorded in run.json so the
      // SwiftShader choice is visible in every output rather than implied by a
      // launch flag nobody reads.
      renderer: capabilities.renderer,
      webgl: capabilities.webgl,
      webgl2: capabilities.webgl2,
      maxTextureSize: capabilities.maxTextureSize,
      softwareRasterizer: capabilities.softwareRasterizer,
      norm16: capabilities.norm16,
      norm16Linear: capabilities.norm16Linear,
      float: capabilities.float,
      floatLinear: capabilities.floatLinear,
      halfFloat: capabilities.halfFloat,
      useCPURendering: getShouldUseCPURendering(),
      effectiveRenderBackend: getEffectiveRenderBackend(),
      devicePixelRatio: window.devicePixelRatio,
    },
  };
}

// ---------------------------------------------------------------------------
// Boundary three, presented
// ---------------------------------------------------------------------------

function onceRendered(element, timeoutMs) {
  return new Promise((resolve, reject) => {
    let timer = null;
    const handler = (event) => {
      window.clearTimeout(timer);
      element.removeEventListener(Enums.Events.IMAGE_RENDERED, handler);
      resolve({
        viewportId: event.detail?.viewportId ?? null,
        viewportStatus: event.detail?.viewportStatus ?? null,
      });
    };
    element.addEventListener(Enums.Events.IMAGE_RENDERED, handler);
    timer = window.setTimeout(() => {
      element.removeEventListener(Enums.Events.IMAGE_RENDERED, handler);
      reject(
        new Error(
          `presented: no ${Enums.Events.IMAGE_RENDERED} within ${timeoutMs} ms. ` +
            `Waiting on a fixed sleep instead of this event is the shape this ` +
            `defect class takes, so the timeout is a failure and not a retry.`,
        ),
      );
    }, timeoutMs);
  });
}

// ---------------------------------------------------------------------------
// Boundary four, read back
// ---------------------------------------------------------------------------

function readBack(canvasElement, expected) {
  if (
    canvasElement.width !== expected.width ||
    canvasElement.height !== expected.height
  ) {
    throw new Error(
      `read back: the viewport canvas is ${canvasElement.width}x` +
        `${canvasElement.height} and render-params.json declares ` +
        `${expected.width}x${expected.height}. A frame at a size nobody ` +
        `declared is not comparable with one at the declared size.`,
    );
  }
  const context = canvasElement.getContext("2d");
  const image = context.getImageData(0, 0, canvasElement.width, canvasElement.height);
  return new Uint8Array(image.data.buffer.slice(0));
}

/** Whether every pixel of the frame carries one value. */
function uniformValue(rgba) {
  const first = [rgba[0], rgba[1], rgba[2], rgba[3]];
  for (let index = 4; index < rgba.length; index += 4) {
    if (
      rgba[index] !== first[0] ||
      rgba[index + 1] !== first[1] ||
      rgba[index + 2] !== first[2] ||
      rgba[index + 3] !== first[3]
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

/** Coverage of the extremes, so a saturated frame is visible in the sidecar. */
function frameStatistics(rgba) {
  let atFloor = 0;
  let atCeiling = 0;
  let opaque = 0;
  const pixels = rgba.length / 4;
  for (let index = 0; index < rgba.length; index += 4) {
    const r = rgba[index];
    if (r === 0 && rgba[index + 1] === 0 && rgba[index + 2] === 0) {
      atFloor += 1;
    } else if (r === 255 && rgba[index + 1] === 255 && rgba[index + 2] === 255) {
      atCeiling += 1;
    }
    if (rgba[index + 3] === 255) {
      opaque += 1;
    }
  }
  return {
    pixels,
    black: atFloor,
    white: atCeiling,
    opaque,
    blackFraction: atFloor / pixels,
    whiteFraction: atCeiling / pixels,
  };
}

// ---------------------------------------------------------------------------
// One row
// ---------------------------------------------------------------------------

/**
 * Render one row.
 *
 * Every failure this page can see comes back as a structured result naming the
 * boundary it happened at. An exception crossing back into the driver would
 * arrive as a Playwright evaluation error with no boundary attached, so
 * `render` below turns even an unexpected one into a result the run record
 * can carry.
 */
async function renderRow(request) {
  const { id, bytesBase64, params, timeoutMs, includePixels, fault } = request;
  const bytes = base64ToBytes(bytesBase64);
  const stage = { reached: true, decoded: false, presented: false, readBack: false };

  const independent = readAttributes(bytes);

  // Boundary two, decoded. A new imageId every row: reusing one would hit
  // cornerstone3D's image cache and hand back the previous row's pixels.
  const file = new File([bytes], `${id}.dcm`, { type: "application/dicom" });
  const imageId = dicomImageLoader.wadouri.fileManager.add(file);

  let image;
  try {
    image = await imageLoader.loadAndCacheImage(imageId);
  } catch (error) {
    return {
      ok: false,
      boundary: "decoded",
      stage,
      imageId,
      attributes: independent.attributes,
      attributesError: independent.error,
      error: String(error?.message ?? error),
    };
  }
  stage.decoded = true;

  const cornerstoneMetadata = readCornerstoneMetadata(imageId);

  // The window, resolved by the one tested function that also runs under node.
  const window_ = fileWindow(independent.attributes, cornerstoneMetadata);
  const voi = {
    ...resolveVoi(params.voi, {
      modality:
        independent.attributes?.modality ??
        cornerstoneMetadata.generalSeriesModule?.modality ??
        null,
      fileWindowCenter: window_.windowCenter,
      fileWindowWidth: window_.windowWidth,
      fileVoiLutFunction: window_.voiLutFunction,
      minPixelValue: image.minPixelValue,
      maxPixelValue: image.maxPixelValue,
      defaults: params.modalityVoiDefaults,
    }),
    origin: window_.origin,
  };

  const viewport = state.viewport;
  try {
    if (fault === "stack-throws") {
      // Injected at the one place a real `setStack` rejection would arrive.
      throw new Error("injected: setStack rejected");
    }
    if (fault !== "no-stack") {
      // Index 0 of a one-image stack, and it is not a multi-frame frame
      // number. A wadouri imageId addresses frame 0 unless `?frame=N` is
      // appended, so `synthetic/ct_multiframe_perframe.dcm` renders its first
      // frame and its other two are not reference-rendered by this story.
      // Recorded in docs/lld/oracle.md rather than left to be inferred.
      //
      // `no-stack` SKIPS this rather than throwing, so the row never reaches
      // the viewport and the identity check below is what catches it. Throwing
      // here would exercise the catch instead, which is `stack-throws`.
      await viewport.setStack([imageId], 0);
    }
  } catch (error) {
    return {
      ok: false,
      boundary: "presented",
      stage,
      imageId,
      attributes: independent.attributes,
      attributesError: independent.error,
      cornerstoneMetadata,
      // `presented`, not `decoded`. The image loaded, `stage.decoded` is
      // already true, and a row counted in `decoded` cannot also have failed
      // there. It matters beyond the printed line: `entryFor` matches on the
      // boundary, so a real rejection here labelled `decoded` would be
      // accountable by an `unsupported.json` entry meant for a decode failure.
      error: `setStack: ${String(error?.message ?? error)}`,
    };
  }

  if (viewport.getCurrentImageId() !== imageId) {
    return {
      ok: false,
      boundary: "presented",
      stage,
      imageId,
      error:
        `the viewport is showing ${viewport.getCurrentImageId()} and this row ` +
        `is ${imageId}. A frame from the previous row would read back cleanly.`,
    };
  }

  if (params.camera?.mode !== "reset") {
    return {
      ok: false,
      boundary: "presented",
      stage,
      imageId,
      error:
        `render-params.json asks for camera mode ` +
        `${JSON.stringify(params.camera?.mode)} and the page implements only ` +
        `"reset". A declared parameter the renderer ignores is worse than an ` +
        `undeclared one, because the sidecar would record it as having acted.`,
    };
  }

  // `Object.hasOwn` and a NAME check, not a lookup. `Enums.InterpolationType`
  // is a TypeScript numeric enum, which is a plain object on
  // Object.prototype AND is reverse mapped, so a plain lookup admitted
  // `constructor`, `toString` and `"0"` alike. The sibling guard above compares
  // `camera.mode` to an exact string and cannot be defeated, and the reason
  // stated there applies here word for word: a declared parameter the renderer
  // ignores is worse than an undeclared one, because the sidecar would record
  // it as having acted.
  const interpolationNames = Object.keys(Enums.InterpolationType).filter(
    (name) => !/^\d+$/.test(name),
  );
  if (
    typeof params.interpolation !== "string" ||
    !interpolationNames.includes(params.interpolation)
  ) {
    return {
      ok: false,
      boundary: "presented",
      stage,
      imageId,
      error:
        `render-params.json asks for interpolation ` +
        `${JSON.stringify(params.interpolation)}, and cornerstone3D names ` +
        `${interpolationNames.join(", ")}`,
    };
  }
  const properties = {
    interpolationType: Enums.InterpolationType[params.interpolation],
  };
  // ALWAYS set, even where there is no window. `setStack` resets `voiRange`,
  // `interpolationType`, `invert` and both flips for each new image, and it
  // does NOT reset `VOILUTFunction`. Leaving it unset would let a colour row
  // inherit the previous monochrome row's function. That is harmless while
  // every corpus row resolves LINEAR, and it is the shape of a defect the day
  // one does not.
  properties.VOILUTFunction = voi.voiLutFunction ?? "LINEAR";
  if (voi.source !== "none") {
    // cornerstone3D's own centre and width to range conversion, so no
    // arithmetic of ours enters the reference.
    properties.voiRange = utilities.windowLevel.toLowHighRange(
      voi.windowWidth,
      voi.windowCenter,
      voi.voiLutFunction,
    );
  }
  viewport.setProperties(properties);
  viewport.resetCamera();

  // Boundary three, presented.
  const canvasElement = viewport.getCanvas();
  const sentinelContext = canvasElement.getContext("2d");
  sentinelContext.fillStyle = `rgba(${SENTINEL[0]},${SENTINEL[1]},${SENTINEL[2]},1)`;
  sentinelContext.fillRect(0, 0, canvasElement.width, canvasElement.height);

  const rendered = onceRendered(state.element, timeoutMs);
  if (fault === "stale-frame") {
    // IMAGE_RENDERED without anything reaching the canvas. This is what the
    // sentinel exists to catch, and without this injection the sentinel branch
    // would be a guard nobody had watched fail.
    state.element.dispatchEvent(
      new CustomEvent(Enums.Events.IMAGE_RENDERED, {
        detail: { viewportId: VIEWPORT_ID, viewportStatus: "injected" },
      }),
    );
  } else if (fault !== "no-render-event") {
    viewport.render();
  }
  let renderDetail;
  try {
    renderDetail = await rendered;
  } catch (error) {
    return {
      ok: false,
      boundary: "presented",
      stage,
      imageId,
      attributes: independent.attributes,
      attributesError: independent.error,
      cornerstoneMetadata,
      error: String(error?.message ?? error),
    };
  }
  stage.presented = true;

  // Boundary four, read back.
  if (fault === "uniform-canvas") {
    const context = canvasElement.getContext("2d");
    context.fillStyle = "rgba(9,9,9,1)";
    context.fillRect(0, 0, canvasElement.width, canvasElement.height);
  }

  let rgba;
  try {
    rgba = readBack(canvasElement, params.canvas);
  } catch (error) {
    return {
      ok: false,
      boundary: "read-back",
      stage,
      imageId,
      attributes: independent.attributes,
      attributesError: independent.error,
      cornerstoneMetadata,
      error: String(error?.message ?? error),
    };
  }

  const uniform = uniformValue(rgba);
  if (isSentinel(uniform)) {
    return {
      ok: false,
      boundary: "read-back",
      stage,
      imageId,
      error:
        `read back: the frame is still the sentinel colour, so ` +
        `${Enums.Events.IMAGE_RENDERED} fired without anything reaching the ` +
        `viewport canvas`,
    };
  }
  if (uniform !== null && !params.allowUniform) {
    return {
      ok: false,
      boundary: "read-back",
      stage,
      imageId,
      attributes: independent.attributes,
      attributesError: independent.error,
      cornerstoneMetadata,
      error:
        `read back: every pixel of the frame is rgba(${uniform.join(",")}). A ` +
        `blank canvas reads back perfectly and hashes stably, so a uniform ` +
        `frame is a failure unless render-params.json declares allowUniform.`,
    };
  }
  stage.readBack = true;

  const camera = viewport.getCamera();
  const result = {
    ok: true,
    boundary: null,
    stage,
    imageId,
    attributes: independent.attributes,
    attributesError: independent.error,
    cornerstoneMetadata,
    voi,
    renderDetail,
    camera: {
      parallelScale: camera.parallelScale,
      position: camera.position ? [...camera.position] : null,
      focalPoint: camera.focalPoint ? [...camera.focalPoint] : null,
      viewUp: camera.viewUp ? [...camera.viewUp] : null,
      flipHorizontal: camera.flipHorizontal ?? null,
      flipVertical: camera.flipVertical ?? null,
    },
    image: {
      rows: image.rows,
      columns: image.columns,
      color: image.color ?? null,
      minPixelValue: image.minPixelValue,
      maxPixelValue: image.maxPixelValue,
      slope: image.slope ?? null,
      intercept: image.intercept ?? null,
      preScale: image.preScale ?? null,
      numberOfComponents: image.numberOfComponents ?? null,
      dataType: image.voxelManager?.getConstructor?.()?.name ?? null,
    },
    frame: {
      width: canvasElement.width,
      height: canvasElement.height,
      statistics: frameStatistics(rgba),
      sha256: await sha256Hex(rgba),
    },
  };

  if (includePixels) {
    result.rawBase64 = bytesToBase64(rgba);
    result.pngBase64 = canvasElement.toDataURL("image/png").split(",")[1];
  }

  return result;
}

async function render(request) {
  try {
    return await renderRow(request);
  } catch (error) {
    return {
      ok: false,
      boundary: "internal",
      stage: { reached: true, decoded: false, presented: false, readBack: false },
      error: `${String(error?.message ?? error)}\n${String(error?.stack ?? "")}`,
    };
  } finally {
    // In a `finally`, so it runs on every path and not only the one that
    // succeeded. A row that failed AFTER its image was cached would otherwise
    // leave it there for the rest of the run, which is exactly what the purge
    // exists to prevent, on the path most likely to need it. One corpus row
    // takes that path today: the YBR one loads and then fails at read back.
    //
    // The corpus is small, but 91 decoded frames held live is not a property
    // anybody asked for, and a growing cache changes eviction order between
    // the two determinism passes.
    cache.purgeCache();
  }
}

window.__oracle = { ready, render };
window.__oracleLoaded = true;
