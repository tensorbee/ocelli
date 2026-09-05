// The volume and MPR reference render page. F-X007.
//
// A SECOND page, loaded after the stack passes have finished and the stack
// page has been closed, so `page/app.mjs` is provably the same program it was
// before this story. cornerstone3D 5.8.2 defaults to `renderingEngineMode:
// ContextPool` with `webGlContextCount: 7`, so a second viewport in the same
// engine might well have been harmless, and "might well have been" is not a
// property worth spending F-011's schedule on.
//
// It exposes one object, `window.__oracleVolume`, with two entry points:
// `ready()` once and `renderSubject()` per volume subject. Every failure it can
// see comes back as a structured result naming its boundary, for the reason
// `page/app.mjs` gives: an exception crossing back into the driver arrives as a
// Playwright evaluation error with no boundary attached.
//
// THREE BOUNDARIES ARE OBSERVED HERE, and a volume has one the stack path has
// no analogue for:
//
//   volume-loaded      every member parsed, the volume built, its slice count
//                      is the member count, its sorted image ids are a
//                      permutation of the members with no repeats, and its z
//                      profile is the ramp the generator wrote. A volume
//                      assembled from the wrong slices, or from one slice ten
//                      times, renders a perfectly plausible frame that hashes
//                      stably, which is why counting the frames is not enough.
//   reformat-presented cornerstone3D's own IMAGE_RENDERED fired for the
//                      orientation, with a timeout. Never a fixed sleep.
//   reformat-read-back the pixels came back, at the declared size, not one
//                      value and not still the sentinel.
//
// The fourth, `volume-geometry`, is the driver's, because only the driver has
// `volume-truth.json`.
//
// **This page reads no DICOM attributes.** The member metadata the geometry is
// computed from is the reading `page/app.mjs` already took with `dicom-parser`
// during the stack pass, handed back down by the driver. One reader, one
// reading, cross-read once by `check_sidecars.py` under pydicom.

import {
  Enums,
  RenderingEngine,
  cache,
  eventTarget,
  getEffectiveRenderBackend,
  getRenderingCapabilities,
  getShouldUseCPURendering,
  init as coreInit,
  metaData,
  utilities,
  version as coreVersion,
  volumeLoader,
} from "@cornerstonejs/core";
import dicomImageLoader from "@cornerstonejs/dicom-image-loader";
// The reference's own metadata store. `@cornerstonejs/metadata` is already
// pinned and already reaches the bundle as a peer of `core` and of
// `dicom-image-loader`, so naming it here adds no dependency, only an import.
import { utilities as metadataUtilities } from "@cornerstonejs/metadata";

import { resolveVoi } from "../src/voi.mjs";

const RENDERING_ENGINE_ID = "ocelliOracleVolume";
const VIEWPORT_ID = "reformat";

/**
 * Painted over the viewport canvas immediately before `render()`.
 *
 * The same colour and the same reason as the stack page. A reformat plane that
 * misses the volume entirely comes back uniform background, which the
 * degeneracy check catches, and a reformat that never reached the canvas comes
 * back this magenta, which is a different failure and deserves to say so.
 */
const SENTINEL = [255, 0, 255, 255];

/**
 * The orientations this page implements, by the NAME of the
 * `Enums.OrientationAxis` member.
 *
 * Deliberately narrower than the enum. 5.8.2 also names `ACQUISITION` and four
 * `_reformat` variants, and each of those resolves its camera from the volume
 * or from the viewport's current normal rather than from a fixed patient axis,
 * which is a different and less reproducible claim. `constants/mprCameraValues.js`
 * fixes exactly these three in patient LPS axes.
 *
 * An `includes` over an array and not a lookup into the enum, for the reason
 * `page/app.mjs` gives about `InterpolationType`: a string enum is a plain
 * object on `Object.prototype`, so a plain lookup admits `constructor` and
 * `toString`. A declared parameter the renderer ignores is worse than an
 * undeclared one, because the sidecar would record it as having acted.
 */
const IMPLEMENTED_ORIENTATIONS = ["AXIAL", "SAGITTAL", "CORONAL"];

/** The blend modes this page implements, by name, for the same reason. */
const IMPLEMENTED_BLEND_MODES = ["COMPOSITE"];

const state = {
  element: null,
  engine: null,
  viewport: null,
  /** Bumped per subject and per pass, so no volume is ever served from cache. */
  serial: 0,
};

// ---------------------------------------------------------------------------
// Small helpers, the same ones the stack page uses and for the same reasons
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

function plainArray(value) {
  return value === undefined || value === null ? null : [...value];
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
    type: Enums.ViewportType.ORTHOGRAPHIC,
    element,
    defaultOptions: {
      // Declared rather than left to the acquisition plane. With no
      // orientation here, `VolumeViewport.setVolumes` sets the view plane from
      // the first volume's own acquisition plane, which would make the
      // viewport's starting state depend on the subject. Every frame calls
      // `setOrientation` explicitly afterwards, so this only fixes what the
      // viewport is between `setVolumes` and the first orientation.
      orientation: Enums.OrientationAxis.AXIAL,
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
// Boundary one, the volume is loaded
// ---------------------------------------------------------------------------

/**
 * Wait for cornerstone3D's own volume-loading-completed event.
 *
 * Never a fixed sleep, and a timeout is a failure and not a retry. The
 * listener is attached BEFORE `load()` is called, because `load()` queues its
 * requests through the image load pool and a member already parsed could
 * complete before a listener attached afterwards existed.
 */
function onceVolumeLoaded(volumeId, timeoutMs) {
  return new Promise((resolve, reject) => {
    let timer = null;
    const handler = (event) => {
      if (event.detail?.volumeId !== volumeId) {
        return;
      }
      window.clearTimeout(timer);
      eventTarget.removeEventListener(
        Enums.Events.IMAGE_VOLUME_LOADING_COMPLETED,
        handler,
      );
      resolve(event.detail);
    };
    eventTarget.addEventListener(
      Enums.Events.IMAGE_VOLUME_LOADING_COMPLETED,
      handler,
    );
    timer = window.setTimeout(() => {
      eventTarget.removeEventListener(
        Enums.Events.IMAGE_VOLUME_LOADING_COMPLETED,
        handler,
      );
      reject(
        new Error(
          `volume-loaded: no ${Enums.Events.IMAGE_VOLUME_LOADING_COMPLETED} ` +
            `within ${timeoutMs} ms. volume-params.json declares that timeout ` +
            `with its reason rather than inheriting the stack pass's, and a ` +
            `timeout is a failure and not a retry.`,
        ),
      );
    }, timeoutMs);
  });
}

/**
 * Register one member, and make its metadata available before the build.
 *
 * `generateVolumePropsFromImageIds` reads each member's Image Plane and Image
 * Pixel modules to sort the slices and size the grid, and it does that BEFORE
 * anything is loaded. For a local file that metadata comes from the reference's
 * own NATURALIZED store, which its loader populates on the file's first touch,
 * so this does the metadata half of that touch early and nothing else.
 * `addDicomPart10Instance` only records the buffer, it decodes no pixels.
 *
 * **Not `imageLoader.loadAndCacheImage`, and the difference is load-bearing.**
 * The streaming volume loader decodes each member with its OWN options: a
 * `targetBuffer` of the volume's data type and `preScale` derived from the
 * Modality LUT (PS3.3 C.11.1). `loadAndCacheImage` hands back a cached image
 * without applying them, so pre-caching a decoded image here would make the
 * reference assemble a volume from pixels it did not ask for, and this harness
 * would have changed the thing it is measuring.
 *
 * `fileManager.add` is still what mints the id, so the ids are the same shape
 * the stack pass uses and the loader's own fallback path stays available.
 */
async function parseMember(member) {
  const bytes = base64ToBytes(member.bytesBase64);
  const file = new File([bytes], `${member.id}.dcm`, {
    type: "application/dicom",
  });
  const imageId = dicomImageLoader.wadouri.fileManager.add(file);
  await metadataUtilities.addDicomPart10Instance(imageId, bytes.buffer);
  // The probe. Registering a buffer records it and parses nothing, so a member
  // that is not a DICOM file is only discovered when something asks for its
  // metadata. Asking here is what lets the refusal name the member, rather
  // than surfacing later as a destructuring error inside the reference with no
  // path attached.
  const plane = metaData.get(Enums.MetadataModules.IMAGE_PLANE, imageId);
  if (!plane || plane.imagePositionPatient === undefined) {
    throw new Error(
      `its Image Plane module (PS3.3 C.7.6.2) is not available, so the ` +
        `reference cannot place it in a volume`,
    );
  }
  return imageId;
}

/**
 * The reference's own sorted image id list is exactly the members, once each.
 *
 * The volume's version of the stack page's "the viewport is showing THIS row's
 * image". A volume built from nine of ten members, or from one member ten
 * times, renders a frame that reads back cleanly and hashes stably.
 */
function checkMembership(volume, imageIds) {
  const sorted = volume.imageIds ?? [];
  const expected = new Set(imageIds);
  const seen = new Set();
  for (const imageId of sorted) {
    if (!expected.has(imageId) || seen.has(imageId)) {
      return (
        `volume-loaded: the reference's sorted image id list is not a ` +
        `permutation of this subject's members. It carries ${imageId} ` +
        `${seen.has(imageId) ? "more than once" : "and no member has that id"}. ` +
        `A volume assembled from the wrong slices renders a plausible frame ` +
        `and hashes stably.`
      );
    }
    seen.add(imageId);
  }
  if (seen.size !== expected.size) {
    return (
      `volume-loaded: the reference sorted ${seen.size} distinct image id(s) ` +
      `and this subject declares ${expected.size} member(s)`
    );
  }
  return null;
}

// ---------------------------------------------------------------------------
// Boundary three, read back
// ---------------------------------------------------------------------------

function readBack(canvasElement, expected) {
  if (
    canvasElement.width !== expected.width ||
    canvasElement.height !== expected.height
  ) {
    throw new Error(
      `reformat-read-back: the viewport canvas is ${canvasElement.width}x` +
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

/** Coverage of the extremes, so a saturated reformat is visible in the sidecar. */
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
// Boundary two, presented
// ---------------------------------------------------------------------------

function onceRendered(element, orientation, timeoutMs) {
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
          `reformat-presented: no ${Enums.Events.IMAGE_RENDERED} within ` +
            `${timeoutMs} ms for orientation ${orientation}. Waiting on a ` +
            `fixed sleep instead of this event is the shape this defect class ` +
            `takes, so the timeout is a failure and not a retry.`,
        ),
      );
    }, timeoutMs);
  });
}

// ---------------------------------------------------------------------------
// One subject
// ---------------------------------------------------------------------------

async function renderSubjectInner(request) {
  const {
    id,
    members,
    orientations,
    params,
    blendMode,
    slabThicknessMm,
    cameraMode,
    loadTimeoutMs,
    renderTimeoutMs,
    zProfileVoxel,
    includePixels,
    fault,
  } = request;

  const stage = { reached: true, built: false, sampled: false, reformats: 0 };
  /**
   * A structured failure, carrying whatever has been learned so far.
   *
   * **`extra` is spread FIRST and the failure fields overwrite it.** The other
   * way round, which is what this was, let the partial success record's own
   * `ok: true` and `boundary: null` win, and the driver then counted a subject
   * that had failed at `reformat-presented` as one that had succeeded with no
   * frames. Measured: five of the six reformat faults reported the run green.
   * That is the quietly-wrong class this whole harness exists to refuse,
   * reached through an object spread.
   */
  const failure = (boundary, error, extra = {}) => ({
    ...extra,
    ok: false,
    boundary,
    stage,
    error,
  });

  // ---- Every member parses -------------------------------------------
  const imageIds = [];
  for (const member of members) {
    try {
      imageIds.push(await parseMember(member));
    } catch (error) {
      return failure(
        "volume-loaded",
        `member ${member.path} did not parse: ` +
          `${String(error?.message ?? error)}`,
      );
    }
  }
  const memberImageIds = [...imageIds];

  // The reference is asked to build a volume from THIS list, so a fault that
  // shortens or repeats it is injected here and nowhere deeper.
  let requested = [...imageIds];
  if (fault === "volume-short-load") {
    requested = requested.slice(0, -1);
  }

  // ---- The volume builds ----------------------------------------------
  //
  // A serial in the id, so no subject and no determinism pass is ever served a
  // volume from the cache. `createAndCacheVolume` returns the cached load
  // object for an id it already holds, and the second pass measuring the first
  // pass's volume would make the determinism claim vacuous.
  state.serial += 1;
  const volumeId = `cornerstoneStreamingImageVolume:${id}__${state.serial}`;
  let volume;
  try {
    volume = await volumeLoader.createAndCacheVolume(volumeId, {
      imageIds: requested,
    });
  } catch (error) {
    return failure(
      "volume-loaded",
      `the volume did not build: ${String(error?.message ?? error)}`,
    );
  }

  const loaded = onceVolumeLoaded(volumeId, loadTimeoutMs);
  if (fault !== "no-volume-load-event") {
    volume.load();
  }
  try {
    await loaded;
  } catch (error) {
    return failure("volume-loaded", String(error?.message ?? error));
  }

  // ---- The volume is the one this subject declares ---------------------
  const dimensions = plainArray(volume.dimensions);
  if (dimensions?.[2] !== members.length) {
    return failure(
      "volume-loaded",
      `the reference built a volume of ${dimensions?.[2]} slice(s) and the ` +
        `subject declares ${members.length} member(s). A volume missing a ` +
        `slice renders a plausible frame and hashes stably.`,
    );
  }
  if (fault === "volume-repeat-slice") {
    // The reference's own sorted list carries one member twice and another not
    // at all. Injected into the BUILT volume, for the reason
    // `volume-z-profile` gives: the guard reads the assembled volume, so that
    // is the state it has to be watched failing on.
    //
    // Not by handing `createAndCacheVolume` a repeated id, which was tried and
    // does something else entirely: `getImageIdIndex` resolves a repeat to the
    // FIRST index, `callLoadImage` then early-returns for it, `framesProcessed`
    // never reaches `totalNumFrames`, and the run hangs until the declared
    // timeout and fails there instead. That would have proved the timeout
    // guard for a second time and this one not at all.
    const corrupted = [...(volume.imageIds ?? [])];
    corrupted[corrupted.length - 1] = corrupted[0];
    volume.imageIds = corrupted;
  }
  const membership = checkMembership(volume, memberImageIds);
  if (membership !== null) {
    return failure("volume-loaded", membership);
  }
  stage.built = true;

  // ---- The z profile, where the subject's content declares one ---------
  //
  // The volume's version of the stack page's identity check, and the only one
  // that looks at the assembled VOXELS rather than at the bookkeeping around
  // them. Sampled through `voxelManager.getAtIJK`, which resolves slice k to
  // the cached image of `imageIds[k]`, so a slice relabelled into the wrong
  // slot shows here and nowhere else.
  let zProfile = null;
  if (zProfileVoxel !== null) {
    const [i, j] = zProfileVoxel;
    if (i >= dimensions[0] || j >= dimensions[1]) {
      return failure(
        "volume-loaded",
        `volume-truth.json samples the z profile at in-plane voxel ` +
          `(${i}, ${j}) and the volume is ${dimensions[0]} by ${dimensions[1]}`,
      );
    }
    const values = [];
    for (let k = 0; k < dimensions[2]; k += 1) {
      values.push(Number(volume.voxelManager.getAtIJK(i, j, k)));
    }
    if (fault === "volume-z-profile") {
      // One slice of the ASSEMBLED volume carries another slice's value, which
      // is what a builder that relabelled a slice into the wrong slot would
      // produce at this voxel. Injected into the sample rather than into the
      // file, because the guard reads the assembled volume and that is what it
      // has to be watched failing on.
      values[3] = values[4];
    }
    zProfile = { voxel: [i, j], values };
    stage.sampled = true;
  }

  // ---- The window ------------------------------------------------------
  //
  // Resolved by `src/voi.mjs`, the one function the unit tests exercise and
  // both pages execute. One volume viewport carries ONE voiRange, so the
  // window comes from one declared member and every member's own resolved
  // window is recorded beside it. `real/mr_eay131` carries fifteen different
  // windows, one per instance, so this is not a hypothetical.
  const memberWindows = [];
  for (let index = 0; index < members.length; index += 1) {
    const member = members[index];
    const image = cache.getImage(memberImageIds[index]);
    let resolved;
    try {
      resolved = resolveVoi(params.voi, {
        modality: member.attributes?.modality ?? null,
        fileWindowCenter: member.attributes?.windowCenter ?? null,
        fileWindowWidth: member.attributes?.windowWidth ?? null,
        fileVoiLutFunction: member.attributes?.voiLutFunction ?? null,
        minPixelValue: image?.minPixelValue,
        maxPixelValue: image?.maxPixelValue,
        defaults: params.modalityVoiDefaults,
      });
    } catch (error) {
      // `bad-voi-source` on the stack page proves an unexpected throw inside a
      // page arrives as a boundary. This is the same throw, and it reaches the
      // outer `renderSubject` catch as `internal` if it is not caught here, so
      // it is caught here to name the member.
      return failure(
        "volume-loaded",
        `member ${member.path}: ${String(error?.message ?? error)}`,
      );
    }
    memberWindows.push({ path: member.path, ...resolved });
  }
  // The window that ACTED, under the same key and the same shape the stack
  // sidecar uses, plus the member it was resolved from. `path` is dropped so
  // that one value does not appear twice under two names.
  const { path: _resolvedFrom, ...firstWindow } = memberWindows[0];
  void _resolvedFrom;
  const voi = { ...firstWindow, member: members[0].path };
  const membersAgree = memberWindows.every(
    (window_) =>
      window_.source === voi.source &&
      window_.windowCenter === voi.windowCenter &&
      window_.windowWidth === voi.windowWidth &&
      window_.voiLutFunction === voi.voiLutFunction,
  );

  // ---- The reference's own geometry ------------------------------------
  const referenceGeometry = {
    dimensions,
    spacing: plainArray(volume.spacing),
    origin: plainArray(volume.origin),
    direction: plainArray(volume.direction),
    dataType: volume.dataType ?? null,
    isPreScaled: volume.isPreScaled ?? null,
    scaling: volume.scaling ?? null,
    numberOfComponents: volume.numberOfComponents ?? null,
  };

  const result = {
    ok: true,
    boundary: null,
    stage,
    id,
    memberImageIds,
    referenceSortedImageIds: plainArray(volume.imageIds),
    referenceGeometry,
    zProfile,
    voi,
    memberWindows,
    voiMembersAgree: membersAgree,
    frames: [],
  };

  // ---- The viewport ----------------------------------------------------
  const viewport = state.viewport;
  try {
    await viewport.setVolumes([{ volumeId }]);
  } catch (error) {
    return failure(
      "reformat-presented",
      `setVolumes: ${String(error?.message ?? error)}`,
      result,
    );
  }

  if (!IMPLEMENTED_BLEND_MODES.includes(blendMode)) {
    return failure(
      "reformat-presented",
      `volume-params.json asks for blend mode ${JSON.stringify(blendMode)} and ` +
        `this page implements ${IMPLEMENTED_BLEND_MODES.join(", ")}`,
      result,
    );
  }
  viewport.setBlendMode(Enums.BlendModes[blendMode]);

  const interpolationNames = Object.keys(Enums.InterpolationType).filter(
    (name) => !/^\d+$/.test(name),
  );
  if (
    typeof params.interpolation !== "string" ||
    !interpolationNames.includes(params.interpolation)
  ) {
    return failure(
      "reformat-presented",
      `render-params.json asks for interpolation ` +
        `${JSON.stringify(params.interpolation)}, and cornerstone3D names ` +
        `${interpolationNames.join(", ")}`,
      result,
    );
  }

  const properties = {
    interpolationType: Enums.InterpolationType[params.interpolation],
    VOILUTFunction: voi.voiLutFunction ?? "LINEAR",
  };
  if (voi.source !== "none") {
    // cornerstone3D's own centre and width to range conversion, so no
    // arithmetic of ours enters the reference.
    properties.voiRange = utilities.windowLevel.toLowHighRange(
      voi.windowWidth,
      voi.windowCenter,
      voi.voiLutFunction,
    );
  }
  if (slabThicknessMm !== null) {
    properties.slabThickness = slabThicknessMm;
  }
  viewport.setProperties(properties);

  // ---- One frame per orientation ---------------------------------------
  for (const orientation of orientations) {
    if (!IMPLEMENTED_ORIENTATIONS.includes(orientation)) {
      return failure(
        "reformat-presented",
        `volume-params.json asks for orientation ${JSON.stringify(orientation)} ` +
          `and this page implements ${IMPLEMENTED_ORIENTATIONS.join(", ")}. A ` +
          `declared parameter the renderer ignores is worse than an undeclared ` +
          `one, because the sidecar would record it as having acted.`,
        result,
      );
    }
    if (cameraMode !== "reset") {
      return failure(
        "reformat-presented",
        `volume-params.json asks for camera mode ${JSON.stringify(cameraMode)} ` +
          `and this page implements only "reset"`,
        result,
      );
    }

    // `immediate` false, deliberately. `setOrientation` renders on its own when
    // told to, and a page that had already presented before it asked to would
    // make `no-reformat-render-event` prove nothing.
    viewport.setOrientation(Enums.OrientationAxis[orientation], false);
    viewport.resetCamera();

    const canvasElement = viewport.getCanvas();
    const sentinelContext = canvasElement.getContext("2d");
    sentinelContext.fillStyle = `rgba(${SENTINEL[0]},${SENTINEL[1]},${SENTINEL[2]},1)`;
    sentinelContext.fillRect(0, 0, canvasElement.width, canvasElement.height);

    const rendered = onceRendered(state.element, orientation, renderTimeoutMs);
    if (fault === "reformat-stale-frame") {
      // IMAGE_RENDERED without anything reaching the canvas. Without this the
      // sentinel branch below would be a guard nobody had watched fail.
      state.element.dispatchEvent(
        new CustomEvent(Enums.Events.IMAGE_RENDERED, {
          detail: { viewportId: VIEWPORT_ID, viewportStatus: "injected" },
        }),
      );
    } else if (fault !== "no-reformat-render-event") {
      viewport.render();
    }
    let renderDetail;
    try {
      renderDetail = await rendered;
    } catch (error) {
      return failure("reformat-presented", String(error?.message ?? error), result);
    }

    if (fault === "reformat-uniform-canvas") {
      const context = canvasElement.getContext("2d");
      context.fillStyle = "rgba(9,9,9,1)";
      context.fillRect(0, 0, canvasElement.width, canvasElement.height);
    }

    let rgba;
    try {
      rgba = readBack(canvasElement, params.canvas);
    } catch (error) {
      return failure("reformat-read-back", String(error?.message ?? error), result);
    }

    const uniform = uniformValue(rgba);
    if (isSentinel(uniform)) {
      return failure(
        "reformat-read-back",
        `reformat-read-back: the ${orientation} frame is still the sentinel ` +
          `colour, so ${Enums.Events.IMAGE_RENDERED} fired without anything ` +
          `reaching the viewport canvas`,
        result,
      );
    }
    if (uniform !== null && !params.allowUniform) {
      return failure(
        "reformat-read-back",
        `reformat-read-back: every pixel of the reformat is ` +
          `rgba(${uniform.join(",")}). A reformat plane that misses the volume ` +
          `reads back as uniform background, and a blank canvas hashes stably.`,
        result,
      );
    }

    const camera = viewport.getCamera();
    const frame = {
      orientation,
      renderDetail,
      camera: {
        parallelScale: camera.parallelScale,
        position: plainArray(camera.position),
        focalPoint: plainArray(camera.focalPoint),
        viewUp: plainArray(camera.viewUp),
        // A reformat is meaningless without this, and the stack sidecar does
        // not carry it.
        viewPlaneNormal: plainArray(camera.viewPlaneNormal),
        flipHorizontal: camera.flipHorizontal ?? null,
        flipVertical: camera.flipVertical ?? null,
      },
      reformat: {
        orientation,
        // The enum KEY, matching the way the orientation is written, so both
        // read the same way in a sidecar and in a frame id. The enum's own
        // value is the lower-case "orthographic".
        viewportType: "ORTHOGRAPHIC",
        blendMode,
        slabThicknessMm: viewport.getSlabThickness?.() ?? null,
        // VTK's `parallelScale` is half the viewport's height in world
        // millimetres, so this is the reformat's own scale. `downsampled` and
        // `canvasPixelsPerSourcePixel` stay STACK concepts: a reformat plane
        // has no source pixel grid to be a magnification of, and publishing a
        // number that means something else under the same name is the defect
        // this project calls quietly wrong.
        millimetresPerCanvasPixel: Number(
          ((2 * camera.parallelScale) / params.canvas.height).toFixed(6),
        ),
        cameraMode,
      },
      frame: {
        width: canvasElement.width,
        height: canvasElement.height,
        statistics: frameStatistics(rgba),
        sha256: await sha256Hex(rgba),
      },
    };
    if (includePixels) {
      frame.rawBase64 = bytesToBase64(rgba);
      frame.pngBase64 = canvasElement.toDataURL("image/png").split(",")[1];
    }
    result.frames.push(frame);
    stage.reformats += 1;
  }

  return result;
}

async function renderSubject(request) {
  try {
    return await renderSubjectInner(request);
  } catch (error) {
    return {
      ok: false,
      boundary: "internal",
      stage: { reached: true, built: false, sampled: false, reformats: 0 },
      error: `${String(error?.message ?? error)}\n${String(error?.stack ?? "")}`,
    };
  } finally {
    // Both caches, on every path. `purgeCache` empties the volume cache and
    // then the image cache, so neither a volume nor a member image survives
    // into the next subject or the next determinism pass, and eviction order
    // cannot differ between the two.
    cache.purgeCache();
  }
}

window.__oracleVolume = { ready, renderSubject };
window.__oracleVolumeLoaded = true;
