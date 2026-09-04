<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Differentiating capabilities (Part III)

**Source**: bootstrap import from `Ocelli-HLD.docx`, Part III, sections 30 to 37. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## Part III, Differentiating capabilities

Parity with cornerstone3D is table stakes, not a product. A survey of the open-source field and thirteen commercial products found eight capabilities that are unclaimed — four of them by anyone, commercial included. They are Phase 1.5 in the backlog: 352 engineer-weeks, roughly 81 engineer-months, to be re-estimated once Phase 1 evidence exists.

What matters architecturally is §38: four of these need a hook inside Phase 1, each costing a few weeks now and a rewrite later.

|  |
|----|
| **THE REFRAMING FACT** The acknowledged performance leader in this market is not a browser application. Its own documentation states that it does not run in a browser - zero-footprint there means no data at rest. The fastest product in the category concluded the browser was the wrong container. Several of the capabilities below exist specifically to prove that conclusion out of date. |

## 30. Out-of-core volume streaming

Cornerstone3D loads whole volumes into one WebGL texture and runs out of memory; the shipped mitigation is overlapping partitions swapped on scroll. Neuroglancer solved this properly at petabyte scale a decade ago and nobody brought the design to DICOM.

- **Residency** is the view frustum intersected with the current level of detail, recomputed per frame against a byte budget. GPU memory becomes a configured number rather than a function of series size.

- **Level selection** compares projected voxel size against screen pixel size.

- **Bricks** are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is ocelli-cache with a brick key — no second cache.

- **Prefetch** runs along the scroll axis, which is where the next request almost always is.

- **Time is another chunk axis.** This is what makes 4D a first-class data model instead of a special case of the memory problem.

### 30.1 The detail that is usually wrong

Sample count varies per level, so **opacity must be corrected** or apparent brightness changes when the level does. Against a fixed reference step:

```text
// alpha' = 1 - (1 - alpha) ^ (ds / ds_ref)
fn correct_opacity(alpha: f32, ds: f32, ds_ref: f32) -> f32 {
    1.0 - (1.0 - alpha).powf(ds / ds_ref)
}
```

*Omit this and the volume visibly brightens or dims as the LOD changes under the user's hand — which reads as a rendering bug and is very hard to diagnose after the fact.*

|  |
|----|
| **THE CLAIM THIS BUYS** A leading CPU-only server-side renderer, sold to several major imaging vendors, publishes its memory constant: roughly 40% above dataset size. A bounded-residency client renderer has no dataset-proportional footprint at all. That is a stronger claim than beating 1.4x, it is checkable, and it is the direct answer to the reason server-side rendering exists. |

## 31. The compute subsystem

WebGPU's rendering advantage over WebGL2 is modest, and NiiVue's maintainers are right to say so. The compute advantage is not modest, and as of September 2026 essentially nobody in medical imaging is taking it.

```text
// ocelli-compute/src/lib.rs
pub trait Kernel {
    fn tier(&self) -> Tier; // A = WebGPU only, B = has fallback
    fn workgroup(&self, caps: &Caps) -> [u32; 3];
    fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
}
```

- **Shares the renderer's device.** ocelli-compute never creates a wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot share textures, which would defeat the entire point.

- **Every tier-A kernel declares a fallback** — CPU, or a worker — so a feature degrades rather than fails on WebGL2. A kernel with no fallback marks its feature unavailable; it never silently produces a different answer.

- **Workgroup sizes come from \`Caps\`**, never hardcoded. A hardcoded 256 is a portability bug waiting for a device that reports less.

- **Buffer pools by size class**, no per-dispatch allocation — the same discipline as §20.

- **Results stay resident on the GPU** wherever the consumer is the renderer. A segmentation mask should never round-trip through JavaScript to be drawn.

Initial kernel set: region growing and interactive segmentation, denoise and gradient filtering, per-frame histogram and ROI statistics, full-resolution MPR and oblique resampling, and registration refinement.

## 32. Prompted segmentation, in the browser

SAM2-class models running on WebGPU exist. DICOM viewers exist. Nobody has wired them together — the most obviously unbuilt thing in the whole survey.

ONNX Runtime Web on the WebGPU execution provider, with the model's input tensor bound to a GPU buffer the renderer already owns. **That bridge is the entire reason to do this inside the engine rather than beside it:** no decode, no CPU copy, no re-quantisation, and the model sees exactly the voxels the radiologist sees.

- Prompts: point, box, and propagation through the third dimension.

- Output lands in a labelmap texture and serialises to DICOM SEG carrying model identity, version, the prompt itself, and operator.

- **No pixel data leaves the browser.** That is a compliance argument as much as a performance one, and it is the argument server-side inference cannot make.

## 33. Multi-monitor

A radiologist reads on two to four calibrated displays. One commercial viewer ships a browser extension solely to place windows across them, and it requires every connected display to have matching resolution. That fragility is a large part of why the market leader left the browser.

- The **Window Management API** enumerates displays and places viewport windows across them; the layout persists per user.

- Each window is its own render surface sharing one session. State synchronises through the same delta mechanism §37 uses.

|  |
|----|
| **BE HONEST ABOUT THE CEILING** A browser can report pixel ratio, colour depth and extension state. It cannot perform DICOM Part 14 greyscale calibration - that is not reachable from a web page at all. The calibration trait reports what it can measure and defers the rest to the desktop target, which is the strongest reason that target exists. Claiming calibrated diagnostic display in a browser would be false and, in this market, dangerous. |

## 34. Standards-native annotations

dwv and SLIM independently converged on the same conclusion: the annotation model should **be** the standard rather than export to it. The field's recurring complaint is that annotations which cannot leave the viewer that made them are worthless as training data.

- The in-memory type **is** a DICOM SR content-item tree. The drawing layer renders from it; it does not own it.

- Concepts coded with **SNOMED CT and UCUM** from a small typed library, which makes every measurement training-data-grade by construction.

- **Two tiers**, following SLIM: SR for semantics, a bulk binary representation once object counts pass a few thousand. The moment a model annotates every nucleus or every nodule, SR is the wrong container.

- **GSPS is written as well as read.** BlueLight does this; cornerstone3D does not.

- **Provenance on every annotation**: tool, engine version, algorithm and version where one was involved, and operator.

|  |
|----|
| **LICENCE WARNING** dwv is GPL-3.0. It is the most interesting of the open-source viewers architecturally and it is the one nobody on this project may read - reimplementing from a GPL-3 source risks a derivative work, and agent exposure weakens any clean-room position exactly as it would with Horos. Take the idea from DICOM PS3.3 and PS3.16, which is where dwv took it from. BlueLight and dicom-microscopy-viewer are MIT, NiiVue is BSD-2, Neuroglancer is Apache-2.0; all four are safe to read. |

## 35. Whole-slide imaging in the same scene graph

Imaging Data Commons runs two separate viewer forks — OHIF for radiology, SLIM for pathology — with different UIs. Only one research group ships both halves on one archive. Correlating a lesion on CT with its biopsy is a real oncology workflow with no good tool.

- The viewport model gains a **\`Pyramid\` source** alongside Stack and Volume.

- **Pyramids are discovered, not assumed.** DICOM VL Whole Slide Microscopy objects carry no pyramid awareness; it is reconstructed at runtime from instance metadata, and must tolerate variable downsampling factors between levels, inconsistent tile sizes, missing frames and mixed compression within one slide. Radiology's regular-grid assumption does not hold here.

- **Frame-level DICOMweb retrieval** — pull the individual frames that fill viewport tiles, never whole instances. Mandatory at gigapixel scale.

- **ICC colour management is a correctness requirement.** A real pipeline stage after the LUT chain: scanner RGB → CIEXYZ → sRGB via the embedded profile. Radiology has windowing; pathology has colour management, and getting it wrong changes diagnoses.

- **Optical paths drive additive blending** for multiplexed immunofluorescence.

- A slide layer and a volume layer **share the annotation coordinate model** through the frame of reference. That is what makes correlation possible rather than merely adjacent.

## 36. Attestable rendering

Given series instance UIDs, a presentation state, an engine version and a pipeline hash, emit the output and a hash of it — reproducible across the browser, desktop and server targets. Nobody offers this, because JavaScript plus WebGL across driver stacks is not reproducible. One Rust core serving three targets makes it approachable.

| **Source of variance** | **Mitigation** |
|----|----|
| Shader compilation differences | Pin the pipeline; precompile and hash the pipeline descriptor |
| Floating-point precision | Explicit rounding at every LUT-chain stage (§18) |
| Texture filtering | Nearest sampling wherever exactness matters over smoothness |
| Driver and vendor behaviour | Measure it; publish the bound rather than assuming it away |
| Compositing and colour profile | Hash the pre-display buffer, never the composited canvas |

|  |
|----|
| **CLAIM MEASURED DIVERGENCE, NOT BIT-EXACTNESS** Bit-identical output across every GPU is not achievable, and promising it would be dishonest in a market where the claim would be relied upon. A published bound, measured across all three targets on the conformance corpus, is both true and more useful. E2 already computes these hashes for the differential oracle - exposing them as a product surface is a small step from infrastructure that cannot be skipped anyway. |

## 37. Live multi-user sessions

Open source has nothing current — the canonical reference paper is from 2017 and its codebase is dormant. Commercial owns this outright, and it matters for tumour boards, teleradiology and teaching.

- A **CRDT over the presentation-state struct and the annotation set**. Both are already serialisable structs on the Rust side (§17.4), so this is a sync layer over data that exists rather than a new subsystem.

- **Presence is ephemeral and deliberately not in the CRDT** — cursors, viewport follow and participant lists should not survive a reconnect or accumulate history.

- **Transport, authentication and session lifecycle sit outside the core.** ocelli-viewport exposes state deltas and applies them, and nothing more. Anything else couples the engine to a deployment model.
