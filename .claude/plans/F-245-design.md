# F-245, One showcase viewer for local, DICOMweb and future user-facing workflows

**Status**: approved
**Epic ref**: E38.40
**Sprint**: S47
**Estimate**: 4w

Scheduling and one-viewer scope were authorized by the operator on 2026-11-08.
The estimate is a provisional lead engineering estimate, not measured capacity
or a calendar promise. This draft is intake for the normal pre-implementation
design round, not authorization to skip its API and numerical review.

## Normative source, transcribed

`docs/hld/05-rendering.md`, section7:

```text
**Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.
```

`docs/hld/23-performance-rules.md`, section26:

> No allocation in the render loop.
> One queue.submit() per frame.
> Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.
> Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.
> Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.

`docs/hld/B-parity-surface.md`, AppendixB, relevant capability families:

> | Viewport types | 12 | STACK, ORTHOGRAPHIC, PERSPECTIVE, VOLUME_3D, PLANAR, VIDEO, WHOLE_SLIDE, ECG and \_NEXT variants |
> | Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
> | Blend modes | 5 | Composite, MIP, MinIP, average, labelmap edge projection |
> | Segmentation representations | 3 | Labelmap, contour, surface |
> | Adapters | 4 | SR TID 1500, SEG, RTSTRUCT, parametric map |

D-11 pins actual parity to cornerstone3D 5.8.2 rather than the nonexistent
version in the imported appendix. D-07 and F-X005 permit honest CPU volume
unavailability. User-facing delivery is in the single showcase viewer.

`docs/hld/14-the-boundary-in-code.md`, section17, exact control, bulk
and 48-byte event layout. The current safe wrappers remain authoritative for
implementation, and quoted unsafe is not permission for another unsafe site.

```rust
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
// ocelli-wasm/src/lib.rs
#[wasm_bindgen]
impl Session {
    /// Apply a packed command buffer. Returns the number of commands applied.
    #[wasm_bindgen(js_name = applyCommands)]
    pub fn apply_commands(&mut self, ptr: *const u8, len: usize) -> u32 {
        let buf = unsafe { core::slice::from_raw_parts(ptr, len) };
        self.inner.apply(buf)
    }
}
```

```rust
#[wasm_bindgen]
impl Session {
    /// Reserve `len` bytes and return a pointer into linear memory.
    pub fn alloc(&mut self, len: usize) -> *mut u8 { /* ... */ }
    /// Hand ownership back; `ptr` must be the value returned by `alloc`.
    pub fn commit_frame(&mut self, ptr: *mut u8, len: usize, meta: &FrameMeta) { /* ... */ }
}
// packages/core/src/bulk.ts -- the ONLY correct order
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

```rust
// ocelli-wasm/src/ring.rs -- single producer (Rust), single consumer (JS)
#[repr(C)]
pub struct RingHeader {
    pub head: u32, // producer writes
    pub tail: u32, // consumer writes
    pub cap: u32, // power of two
    pub dropped: u32, // overflow count; nonzero means JS is not draining
}
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

## What the specification does not cover

The HLD specifies the library boundary and capability families, not the
complete control placement, browser fixture inventory or final module split.
This draft selects the scheduled scope below. Detailed signatures, new command
codes and any new numerical rule must be transcribed from the applicable HLD
and DICOM/ICC standards before implementation, never inferred from a demo.
No tolerance or new deviation is approved by this planning change.

## Problem and reproducer

At source tree `4a3c0132b8040f5393b038366b7202404d5a2bd9`, open the
folder viewer and attempt this story's first acceptance action below.
The existing library capability is absent from, incomplete in, or split across
the user-facing applications. The More audit identifies the exact gap.
A single application and control vocabulary replace split viewer demonstrations. Existing entry points may launch it, but no user-facing feature is accepted only in a separate demo.

## Scope, API and compatibility

### Consumer and provider map

This design adds a remote retrieval path to the existing canonical viewer page `examples/folder-viewer/src/page.html`. The page remains the ONE ordinary HTML product page with its native backend plus the same backend reached through the primary npm package. The existing local S46 image, marks and conformance behavior remain untouched. The scope of F245 is exactly remote QIDO search and selected WADO-RS instance fetch, validation and staging. There is no remote decode and no future display work here, because F202 owns display. The Part10 native parser may perform its existing bounded dataset transfer adaptation to read UIDs from the opaque WADO body. No frame codec decode, no VOI and no remote image rendering is added and no native remote renderer is created. There is no second accepted React viewer and no native upstream proxy.

The consumer is the browser side. The existing browser `DicomwebClient` owns all upstream concerns: fetch, authentication injection, status handling, content-type handling, abort and legacy behavior. Legacy options and legacy method returns are unchanged. The existing `Promise<void>` method signatures stay. `postWadoInstance` is unchanged. The new surface is a bounded delivery branch plus a new provider class that composes on top of the same client behavior.

The provider is the new `DicomwebProvider` in the viewer layer. It exposes QIDO envelopes and staged instance metadata to the page. The page is the only consumer of staged data. The native viewer HANDLER owns the reply JSON serialization cap, not the browser page and not ocelli-dicom.

All new APIs in this document are PROPOSED. They are not source available. The historical reproducer source is 4a3c0132. Source was inspected at 47356703. Later 265f247e and 1f2809c0 are DOCS-ONLY changes and leave provider bytes unchanged. 265f247e is not a current source inspection and no new inspection or runtime evidence is invented. No implementation commands are run as part of this design and no approval is claimed. Lead approval is still required.

### Bounded delivery API

The chosen delivery semantics are precise. The new bounded delivery branch selects on the presence of EITHER `maxResponseBytes` OR `receive`. When either is present BOTH are required, `receive` must be callable and `maxResponseBytes` must be a positive safe integer. The branch refuses wasm, sink and any mixed unknown fields and never falls back to the legacy path. The browser client retains ownership of the upstream fetch and keeps its own safe errors and its own cancellation as the primary error. It awaits the receiver callback. Byte counting uses `Response.body` independently of any `Content-Length` value. Only a transfer that EXCEEDS the cap is cancelled and refused before final assembly. A body whose total ends EXACTLY at the cap is accepted. There is no arrayBuffer bounded path. Accepted chunks sum to at most the cap and the final assembled buffer is at most the cap. Incoming browser, network and crypto buffers and RSS are not proved bounded.

```text
export type DicomwebBoundedReceive = (raw: Uint8Array, kind: DicomwebResponseKind) => Promise<void>;
export interface DicomwebBoundedDeliveryOptions { readonly studiesServiceUrl: string; readonly wadoUriUrl: string; readonly fetch: typeof fetch; readonly cache?: DicomwebCacheMode; readonly maxResponseBytes: number; readonly receive: DicomwebBoundedReceive; readonly wasm?: never; readonly sink?: never; }
```

### QIDO semantics and result API

QIDO search requires an actual leaf UID, validated as UI and VM1. Returned parent UIDs are optional. When present they are validated and must match the captured scope. Missing parents are allowed under the captured hierarchy. Requests include fields 0020000D, 0020000E and 00080018 by level as in PS3.18 2026d10.6.3.3.1 through .3. Captured scope stays separate and is never falsely parsed from row payloads. Upstream DICOM JSON is never parsed in browser JavaScript. All projection and validation runs in the native decoder. An empty envelope still carries level, hash, source nonce and generation. The Rust side parses DICOM metadata. The browser uses DOM `textContent` for LABEL only.

The proposed endpoint `POST /dicweb/qido` carries raw DICOM JSON. Headers are `X-Qido-Level` as an enum, `X-Qido-Parent-Study` and `X-Qido-Parent-Series` as valid UIDs of at most 64 characters at needed levels, `X-Qido-Source-Nonce` as 32 lowercase hex and `X-Qido-Generation` as a canonical safe decimal no greater than 9007199254740991. All header names are prefixed `X-Qido-`. The proposed `POST /dicweb/instance` carries raw multipart. Headers are `X-Wado-Study-Instance-UID`, `X-Wado-Series-Instance-UID` and `X-Wado-SOP-Instance-UID` each at most 64 characters, `X-Wado-Source-Nonce` as 32 lowercase hex, `X-Wado-Generation` as the same safe decimal and a validated `Content-Type` boundary of at most 70 characters. There is no JSON byte wrapper, no base64 and no decoy parts. The native response carries the actual tuple and hash and the association nonce only. For WADO the actual FULL parsed dataset UID triple must equal the selection and when exactly 1 part is required a second part is refused BEFORE it is parsed. `ParsedDicom` returns objects, never part bytes. The browser keeps the SAME complete response and validated representation type and boundary and verifies the native SHA against its own pending body. The boundary is never inferred from the payload.

```text
export interface QidoStudyRow { readonly studyUid: string; }
export interface QidoSeriesRow { readonly seriesUid: string; readonly returnedStudyUid?: string; }
export interface QidoInstanceRow { readonly instanceUid: string; readonly returnedStudyUid?: string; readonly returnedSeriesUid?: string; }
export interface QidoStudyCapturedScope {}
export interface QidoSeriesCapturedScope { readonly studyUid: string; }
export interface QidoInstanceCapturedScope { readonly studyUid: string; readonly seriesUid: string; }
export interface QidoEnvelopeCommon { readonly sourceNonce: string; readonly generation: number; readonly rawBodySha256: string; }
export type QidoStudyEnvelope = QidoEnvelopeCommon & { readonly level: 'study'; readonly capturedScope: QidoStudyCapturedScope; readonly rows: readonly QidoStudyRow[]; };
export type QidoSeriesEnvelope = QidoEnvelopeCommon & { readonly level: 'series'; readonly capturedScope: QidoSeriesCapturedScope; readonly rows: readonly QidoSeriesRow[]; };
export type QidoInstanceEnvelope = QidoEnvelopeCommon & { readonly level: 'instance'; readonly capturedScope: QidoInstanceCapturedScope; readonly rows: readonly QidoInstanceRow[]; };
export type QidoEnvelope = QidoStudyEnvelope | QidoSeriesEnvelope | QidoInstanceEnvelope;
```

### Viewer provider ownership and staged state

The provider owns one private staged whole buffer and an independent frozen metadata receipt including representation. `takeStagedInstance` performs a synchronous source and receipt check, then a structured clone with transfer or detach, and returns an `OwnedEncodedInstance` on success. Stale, missing or already taken state returns null. There is no fallback copy and no release wrapper. A new intent, a source change or a cancel releases the prior staged raw body only. A transfer failure is a closed refusal and a refetch recovers.

```text
export type InstanceIdentity = Readonly<{ studyUid: string; seriesUid: string; instanceUid: string }>;
export type StagedMetadata = Readonly<{ identity: InstanceIdentity; rawBodySha256: string; sourceNonce: string; generation: number; byteLength: number; representation: Readonly<{ field: 'wado-rs-instances'; boundary: string }> }>;
export type OwnedEncodedInstance = { body: ArrayBuffer; metadata: StagedMetadata };
export type DicomwebProviderOptions = { studiesServiceUrl: string; wadoUriUrl: string; fetch: typeof fetch };
export declare class DicomwebProvider {
  constructor(opts: DicomwebProviderOptions);
  fetchSelectedInstance(identity: InstanceIdentity): Promise<StagedMetadata>;
  readStagedInfo(): StagedMetadata | null;
  takeStagedInstance(expected: StagedMetadata): OwnedEncodedInstance | null;
  releaseStaged(): void;
  cancelPending(): void;
  dispose(): void;
  searchStudies(options?: QidoSearchOptions): Promise<QidoStudyEnvelope>;
  searchSeries(studyUid: string, options?: QidoSearchOptions): Promise<QidoSeriesEnvelope>;
  searchInstances(studyUid: string, seriesUid: string, options?: QidoSearchOptions): Promise<QidoInstanceEnvelope>;
}
```

### Native bounded parsing

The core gains one new borrowed bounded method. It receives explicit limits and the route retains the `Request` body vector and the charge. The legacy owned consume covering all four source kinds and `parse_part10` are unchanged. SourceError::SizeLimit and SourceError::UnsupportedRepresentation plus ParseError::SizeLimit are PROPOSED new variants TO ADD in crates/ocelli-dicom/src/dicomweb.rs and crates/ocelli-dicom/src/parse.rs. SourceBatch::Instances is parsed objects including dataset values, not merely metadata objects and never original part bytes. No numeric ring behavior changes. The new method accepts only `QidoJson` and `WadoRsInstances`. It bounds encoded length, bounds inflated output before extending reads, bounds part count and bounds row visitor growth. The reply serialization cap belongs to the viewer. Proposed viewer limits are query 2 MiB, instance 32 MiB, decoded 128 MiB, 20000 rows, 1 part and 16 MiB reply. No physical heap or RSS proof is offered. Parser object and capacity peaks are unmeasured and must not be claimed bounded by the payload limit.

```text
pub struct SourceResponseLimits { pub encoded_body_max: usize, pub decoded_max: usize, pub max_parts: usize, pub max_rows: usize }
pub struct ParseLimits { pub decoded_dataset_max: usize }
pub fn consume_borrowed(&self, raw: &[u8], kind: &SourceResponseKind, limits: &SourceResponseLimits) -> Result<SourceBatch, SourceError>
pub fn parse_part10_with_limits(bytes: &[u8], limits: &ParseLimits) -> Result<ParsedDicom, ParseError>
```

### Concurrency and lifecycle contract

Each operation creates its own new client with an immutable copied context. The one canonical page keeps ONE persistent provider across tab switches and calls `cancelPending` instead of reconstructing pools. At most 2 operations are LIVE including canceled nonsettled ones counted through `finally`. The latest metadata queue holds 1 entry with no fetched payload and the staged buffer holds 1. Under 5C the conservative ceiling is the accepted raw payload only, where C is the per operation cap, not total process or browser or native object counts. A replaced queue entry settles Aborted and the freed slot rechecks the current context. Generation advances exactly once per user intent BEFORE capture. Internal old abort and cleanup does not increment again. Every await and every success, error and finally commit is gated by the original source and generation. An old finally touches only its own refs and never clears newer state.

Remote requested, pending and staged state is fully separate from the ACTIVE DISPLAYED local state. Discovery or valid staging never invokes `readFolder` or `resetFolderState` or `resetChecks`, never clears marks, cache or reports and never relabels local pixels. The existing local `readFolder` entry immediately resets local state per F215 and is unchanged. The fixed product status text is that the remote instance is fetched and validated, the remote image display is not available yet and the local image is unchanged. No story ID appears in product copy.

Upstream injected `authfetch` is used ONLY upstream. The native same origin POST uses global `fetch` with whitelisted metadata and no auth forwarding.

## Replacement error mapping (closes prior ambiguity)

```json
{
  "native_http_to_shell": {
    "400": {
      "malformed_metadata": "InvalidRequest",
      "unsupported_request": "InvalidRequest"
    },
    "413": "AllocationRefused",
    "415": "ContentType",
    "422": "InvalidValue",
    "503": {
      "code": "HttpStatus",
      "status": 503
    }
  },
  "native_wire_body": "Exactly {\"code\":\"<known string>\"} only. The 503 body is client-side mapping only; the actual native wire body carries the code string alone and the status number is taken from the HTTP status line. DICOM body parse failures and actual UID validation failures are native 422 InvalidValue, not 400. No SourceError variant name is a JS code and no fake HTTP status exists for client unreachability, digest mismatch or stale generation.",
  "client_only": [
    "malformed_native_200_or_unknown_fields_or_digest_mismatch_maps_to_CommitRefused_client_detected",
    "unreachable_maps_to_Network_with_no_HTTP_response",
    "stale_generation_or_abort_discards_via_Aborted_with_no_native_409_and_no_stale_UI_error",
    "upstream_codes_preserved_and_no_error_body_read_and_no_automatic_retry",
    "awaited_callback_safe_error_not_remapped"
  ],
  "constructor_and_options": {
    "invalid_constructor": "InvalidOptions",
    "unknown_keys": "UnknownOption"
  },
  "unchanged": [
    "HttpStatus",
    "Network",
    "Aborted",
    "ContentType",
    "InvalidOptions",
    "UnknownOption",
    "ring_registry_private_numbers",
    "native_error_envelope_fixed_closed_json_code_string_only",
    "native_parsed_uid_expected_tuple_mismatch_can_422_InvalidValue"
  ],
  "prohibited": [
    "no SourceError variant names forwarded as JS codes",
    "no invented native codes 1 through 13"
  ]
}
```

### Runtime decoder requirements for projected JSON (selected wire schema is the TypeScript union, so these are runtime obligations)

- Strict `level` discriminator: exactly one of `study`, `series`, `instance`. Unknown levels refused before row inspection.
- Exact allowed keys only. ROOT keys are exactly `level`, `rows`, `capturedScope`, `sourceNonce`, `generation`, `rawBodySha256`. No other root keys are allowed. Row keys per level are exact: study rows only `studyUid`. Series rows `seriesUid` plus optional `returnedStudyUid`. Instance rows `instanceUid` plus optional `returnedStudyUid` and `returnedSeriesUid`. Unknown extra keys refused.
- The root `level` at RUNTIME must equal the captured operation level, not merely any known level string.
- `capturedScope` at RUNTIME must be non-null, non-array, an object type. Study scope has ZERO own enumerable keys and no extra level nested inside. Series scope keys exactly `studyUid`. Instance scope keys exactly `studyUid` and `seriesUid`.
- Series scope runtime keys exactly `studyUid`. Instance scope runtime keys exactly `studyUid` and `seriesUid`.
- Optional returned parents are validated separately from scope identity and never merged into captured scope.
- The TypeScript empty interface alone is not proof. A decoder enforces the zero-key runtime check.
- No upstream DICOM JSON is parsed in browser JavaScript.

### New typed variants

`SourceError::SizeLimit` and `SourceError::UnsupportedRepresentation` in crates/ocelli-dicom/src/dicomweb.rs and `ParseError::SizeLimit` in crates/ocelli-dicom/src/parse.rs are PROPOSED new variants TO ADD, not existing variants. `SourceBatch::Instances` carries parsed objects including dataset values, not original part bytes. Legacy consume behavior is preserved and numeric ring code is unchanged.

### AC9 synthetic fixture selection

The same-UID same-length differing-digest case uses a synthetic fixture pair of equal length to be created at implementation, with its differing SHA-256 digests then verified with an independent tool and recorded. No fixture pair or digests exist now and no universal SHA-256 injectivity is claimed. The test asserts refusal on digest mismatch. No SHA injectivity claim is made. Other equal-length differing pairs are not asserted.

## Native, npm, assets and CSP

### Native backend and startup

The folder viewer keeps one ordinary HTML page at examples/folder-viewer/src/page.html. The same native backend serves the native path via cargo run -p ocelli-folder-viewer and the npm path via the root npm dev command. The core build is a prerequisite of the native build. A NEW launcher bin/folder-viewer-dev.mjs builds the core, then runs cargo build -p ocelli-folder-viewer --message-format=json and selects the exact native bin compiler-artifact executable from the build output rather than any hardcoded target path. The launcher spawns the backend as a direct child process, forwards INT and TERM, waits for exit, and performs owned cleanup of the child on shutdown. The root package.json dev script changes accordingly. The old React demo remains reachable only through the dev:boundary-demo script with its unchanged bin/example-dev.sh. There is no second showcase page and no code fallback renderer.

### Core export and dist

A NEW flat core module packages/core/src/dicomweb-viewer.ts is exported from packages/core/src/index.ts. Generated tsc dist output is not tracked in git. LIBRARY_FILES lists the basename dicomweb-viewer without extension or URL, plus the closure test entry. The existing import map entry @ocelli/core=/lib/ocelli-core/index.js is unchanged. The main canonical HTML gains the new source chooser and discovery wiring. The single-page and F202 remote-display boundary are unchanged.

### Configuration and policy

Startup reads exactly two full URL environment values, OCELLI_VIEWER_STUDIES_SERVICE_URL and OCELLI_VIEWER_WADO_URI_URL. When both are absent the viewer runs local only. A partial or malformed pair produces a generic startup refusal with no echo of values. The proposed viewer policy is HTTP and HTTPS only, any host, no user info, no query or fragment, path allowed. This is a viewer-scoped policy, not a global client policy. examples/folder-viewer/Cargo.toml explicitly declares the existing workspace dependencies serde_json (used by the selected viewer config and projection for JSON serialization) and uriparse 0.6.4. There is no conditional if-needed dependency choice. There is no auth product, no persistence, and no runtime URL entry UI.

The viewer issues one GET to /dicweb/config and receives JSON full URLs. There is no HTML data island and no injection of configuration into the page. The private serve context remains in the EXISTING examples/folder-viewer/src/serve.rs, holding the viewer state, the validated config, and the owned main policy, while public serve_on and handle_connection_with and the ViewerConfig cache_bytes field are preserved for existing callers. A NEW examples/folder-viewer/src/dicomweb.rs is a parsing and projection helper only and holds no serve context. Existing entrypoints default to the local context and startup serves the configured context. The EXISTING examples/folder-viewer/src/csp.rs gains a NEW configured_main_policy(page: &str, origins: &[String]) -> String, built once per context. The main page and /index.html receive the configured policy. Use, More, and default paths keep the unchanged self-only cached page_policy. Hash literals are produced automatically. Only the origin actually fetched for the studies service is allowed to connect. The Wado URI URL is required by the existing options flow but no canonical URI retrieval claim is made. An unused origin is not allowed. There are no wildcards, no eval, no wasm-unsafe-eval, and no other directive changes. Upstream CORS remains an external service concern, and the native config response cannot grant it. Rust does not fetch HTTP itself, source caps and projection roles stay correct, and the core JSON reply serialization limit belongs to the viewer layer, not to ocelli-dicom.

## Named write map

The exact actual existing writes are packages/core/src/dicomweb.ts, packages/core/src/options.ts, packages/core/src/index.ts, packages/core/src/dicomweb.test.ts, packages/core/src/options.test.ts, crates/ocelli-dicom/src/dicomweb.rs, crates/ocelli-dicom/src/parse.rs, crates/ocelli-dicom/src/lib.rs, crates/ocelli-dicom/tests/dicomweb.rs, crates/ocelli-dicom/tests/part10.rs, examples/folder-viewer/src/page.html, examples/folder-viewer/src/serve.rs, examples/folder-viewer/src/csp.rs, examples/folder-viewer/src/lib.rs, examples/folder-viewer/Cargo.toml, package.json, bin/ocelli.sh, tools/bench/package.json, README.md, examples/folder-viewer/src/use.html, examples/folder-viewer/src/more.html, docs/lld/dicomweb.md, docs/lld/dicom-ingest.md, docs/lld/public-api.md, docs/lld/feature-availability.md, docs/lld/build-targets.md, docs/lld/benchmarks.md, docs/lld/README.md. The NEW files are packages/core/src/dicomweb-viewer.ts, packages/core/src/dicomweb-viewer.test.ts, examples/folder-viewer/src/dicomweb.rs, examples/folder-viewer/tests/dicomweb.rs, examples/folder-viewer/tests/browser/dicomweb.mjs, bin/folder-viewer-dev.mjs, tools/bench/tests/folder_viewer_dev_test.mjs. eslint.config.js changes are exact globals conditional only, added where the actual new browser or Node context requires them, with no rule disabled and no allowlist widened. scripts/guards/catalogue.py and ci/guard-probe-budget.json change conditionally only for actual new repository refusals or permanent watchers, never a guessed count. There are no vendor, pin, WASM export, or pixel math files, and no packages/ocelli-core, src/index.js, tests/parse.rs, or bare ambiguous paths.

The NEW browser module is registered in bin/ocelli.sh under viewer-browser while all current 10 modules are preserved. The new launcher Node unit is registered in BOTH the bin bench gate and the tools/bench/package.json list. Vitest picks up *.test.ts automatically, and Rust crate tests are named. F266 owns config, worker, and probe plus shared guard providers until release, and F272 shared writers release after prepare. There are no concurrent writers, and the complete three way integration happens at root. F245 dependencies F207, F228, F231, and F021 are DONE and verified in the backlog. Current status is still S46, so an isolated S47 implementation would require the normal claim flow, a start user exception, AND a root approved design. Design approval does not claim acceptance or writer release. The original draft status is superseded by the dated approval below.

## Acceptance criteria

| AC | Observable | Wrong implementation rejected | Permanent test | Stage |
| --- | --- | --- | --- | --- |
| AC1 | Same HTML and assets served native and npm, including a custom CARGO_TARGET_DIR, with owned INT and TERM cleanup controlled by the launcher | A hardcoded target path or an unhandled signal orphaning the backend fails | Launcher unit plus actual browser and launcher controls | Native and npm run stage |
| AC2 | Real source chooser and QIDO requests run on the same page | A label-only canvas or a second UI shell fails | Browser suite asserts live chooser requests | Browser stage |
| AC3 | Local F215 marks, SR docs, and F225 checks plus cache are preserved through remote pending, refusal, and valid stages in an actual browser | Rust unit proof alone or a UI that drops local state on pending fails | Browser workflow suite | Browser stage |
| AC4 | Actual same-kind reversed reply including EMPTY and a valid missing parent response, immutable captured scope, source Local A then Remote B then Local A, old catch and finally unable to clear a newer request, finite queue of 2 live and 1 metadata rows, take once, stale and DTO alias handling. A non cancellable test fetch drops the signal only at a controlled endpoint but still uses the actual network and native parser, catching the generation guard mutant that an abort would mask. One old EMPTY row and one old Series row without the optional parent catch the latest mutable context substitution, since a present wrong parent would be masked by a Rust refusal | Substituted wrong parent, abort masked mutants, or stale clears pass nothing | dicomweb-viewer.test.ts and browser dicomweb.mjs | Unit and browser stage |
| AC5 | All existing S46 browser suites plus the new workflow run with unchanged old expectations | A changed old expectation or skipped suite fails | CI browser run | Browser stage |
| AC6 | Discovery and fetch are tier independent, remote display stays unavailable in all tiers until F202, and existing local render is unchanged | A GPU fallback or a claim about unrunnable hardware fails | Tier matrix suite | Browser stage |
| AC7 | The More matrix and docs show the actual partial status and the new source UI cannot relabel old local pixels | Relabeled legacy pixels or overstated matrix rows fail | More matrix snapshot | Browser and docs stage |
| AC8 | Existing RELEASE registered viewer.open, first_image, window_step, scroll_rate, and resident_memory are recorded before and after when the path changed, using the exact acquisition procedure appended under F245 performance acquisition procedure, with the same instrument and host qualifications and unchanged bands. A new remote axis requires instrument evidence BEFORE any numeric performance claim. A 5 C payload is not an RSS proof and actual parser and browser heap peaks stay unmeasured | A floor or structural gate accepted as timing, or an uninstrumented numeric claim, fails | Bench gate registration and release run | Release performance stage |
| AC9 | Actual QIDO study, series, and instance typed rows with include field and leaf versus optional parents, then WADO with a full actual UID tuple plus boundary, hash, and same owned body. Wrong UID with the same length, same UID with a synthetic payload pair of the same length to be created and recorded at implementation with independent-tool verified differing SHA-256 digests and required mismatch refusal, not any collision or universal hash injectivity claim, a 2 part response refused BEFORE the second parse, cap enforced before growth, bad VR, bad VM, missing UID, and refusal then valid retry are all covered. Fixtures are wholly synthetic with standard and independent tool gold, with no clinical files, corpus writes, or prod decoys | A length-only or digest-weak check passes nothing | Rust tests/dicomweb.rs and part10.rs, core unit tests | Unit and fixture stage |

## Evidence stages and future verification

Evidence is staged as unit tests, named Rust crate tests, the new browser workflow suite, the full S46 browser regression, launcher and bench gate registration, and release performance where the measured path changed. Future verification comprises focused tests with meaningful semantic mutants, actual new native, npm, and CSP browser positives and negatives plus all S46 browser suites, release performance where changed, and the normal feature floor plus corpus run on the FINAL STAGED source after independent review. No sprint profile appears before all S46 items are completed or carried. There is no source now, no commit, and no Done or runtime evidence claims. Technical choices here are selected for source review, and no question block claims a nothing to do outcome. External observations remain owed, and no compliance or host acceptance is fabricated.

## Boundary and tier

All three tiers are required. F245 adds no pixel, geometry or render arithmetic, so the tier rows describe the same answer in every tier for the added path. The added F245 path is tier-independent: QIDO search, WADO-RS instance fetch, bounded Part10 UID reading and staging run identically in every tier on the existing native parser, and the existing local render path is unchanged. Remote display is unavailable in all three tiers until F202, so no tier row answers differently about the same path. Tier A (WebGPU): the added path is available identically, remote display n/a until F202. Tier B (WebGL2): the added path is available identically, remote display n/a until F202. Tier C (CPU): the added path is available identically, existing local CPU render unchanged. No GPU fallback is introduced and no claim is made about unrunnable hardware. Boundary bullets for F245: wasm-bindgen is not touched, no pixels are moved across the WASM boundary by this parse and staging only path, the path adds no render-loop allocations, and no unsafe is added.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Wholly synthetic QIDO envelopes and Part10 bodies with standard and independent tool gold, no clinical files | crates/ocelli-dicom/tests/dicomweb.rs, crates/ocelli-dicom/tests/part10.rs, packages/core/src/dicomweb.test.ts |
| unit | Provider lifecycle, generation and source guards, stale and take once, nested receipt immutability mutations | packages/core/src/dicomweb-viewer.test.ts |
| browser | Live chooser QIDO, local state preservation across remote pending and refusal and valid stages, tier independence, More matrix snapshot | examples/folder-viewer/tests/browser/dicomweb.mjs, existing S46 suites |
| launcher | Owned INT and TERM cleanup, compiler artifact binary selection | tools/bench/tests/folder_viewer_dev_test.mjs |
| guard | Catalogue and probe budget changes only for actual new refusals or watchers | scripts/guards/catalogue.py, ci/guard-probe-budget.json |
| performance acquisition | The appended F245 performance acquisition procedure, before and after, no numeric observation reported here | bin/ocelli.sh bench release run records |

No fixture row is tied to pixel or geometry arithmetic because F245 adds none.

## Parity surface covered

None of the docs/hld/B-parity-surface.md rows close here. The HLD parity table has no DICOMweb specific row. The viewport types, tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation representations, core events and adapters categories are pixel, render or tooling surfaces and none of them is advanced by QIDO search and staged WADO-RS fetch. Source discovery and fetch is a partial move only on the Loaders and metadata APIs surface. F202 remains the sole owner of remote rendering and closes nothing of that surface in this design. No new parity row is introduced.

## Deviations

No new deviation is named by this design. The existing register entries are retained unchanged, including the D11 5.8.2 reference pin and the D07/F-X005 honest CPU volume unavailability. No new register entry and no rendering claim is added.

## LLD impact

The existing LLD list already names the seven files /complete-feature step 9 updates, and their concrete impacts are:

docs/lld/dicomweb.md, the QIDO envelope shapes, the staged receipt fields including all UID strings, hash, source nonce, generation, length, type and boundary, the new provider methods and the nested Readonly identity type. docs/lld/dicom-ingest.md, the bounded consume_borrowed and parse_part10_with_limits signatures, the SourceError and ParseError variants to add, and the per route limits. docs/lld/public-api.md, the new dicomweb-viewer flat core module export and the unchanged legacy DicomwebClient surface. docs/lld/feature-availability.md, the partial DICOMweb sources status and the F202 remote display boundary. docs/lld/build-targets.md, the new launcher, the new browser module registration under viewer-browser, the new viewer tests and the conditional eslint.config.js and guard entries. docs/lld/benchmarks.md, the appended acquisition procedure reference and the existing shared viewer_work and five runner duties, with no new timing axis. docs/lld/README.md, the index entries for the changed and new files. Each duty follows the existing contributor, date and index process for those documents. No new feature is claimed in any LLD edit.

## User-facing documentation impact

README.md sections updated before review: Start the example viewer is rewritten because the canonical npm dev path becomes the native same page viewer, and the legacy React Vite npm dev demo is explicitly preserved under the dev:boundary-demo entry with its own wording, no longer presented as canonical. Start the local series viewer gains the DICOMweb remote source paragraph. The Build from source material lives in examples/folder-viewer/src/use.html chapter 4 and the DICOMweb section is use.html section 5.6, and those are reused as is. The embedding CSP section gains the configured main policy origin note. examples/folder-viewer/src/use.html section 5.6 DICOMweb gains the DicomwebProvider description beside the existing DicomwebClient options table. examples/folder-viewer/src/more.html keeps the existing More DICOMweb sources row and the existing roadmap delivery cell, with the row text moved to the actual partial status per AC7 and no completion claim. These are the actual destinations. No new parity surface and no inherited render expansion appears in any of them.

## S1 amendment, immutable staged receipt

The staged receipt becomes deeply immutable. The InstanceIdentity type is changed to Readonly as shown in the replacement above. At runtime the provider copies and deep freezes the nested input identity, the representation object and the outer receipt before storing or returning them. No shared mutable caller reference is retained. The provider methods and the deep full receipt matching in takeStagedInstance, comparing all UID strings, hash, source, generation, length, type and boundary against the private stored originals, are preserved. Readonly alone is not a runtime guarantee, the freeze and copy are.

Input identity is snapshotted synchronously before any await, and every returned metadata object and receipt is a detached deep frozen snapshot. The provider never freezes or mutates caller owned objects. Tests cover mutation of the nested input identity, mutation of the returned DTO identity and representation before and after the awaited completion, stale and take once behavior under mutation attempts, and state before and after transfer. A bypassed TypeScript cast mutation of a returned snapshot cannot alter private state, and a malformed supplied expected snapshot is refused rather than partially matched. No current runtime bug is claimed, no implementation exists, and this is the only type addition permitted, with no entire API rewrite.

## F245 performance acquisition procedure

This procedure is the concrete method AC8 links to. It is not an observation and no F245 measurement exists now.

Prerequisites. The host is the available ARM Apple M4 Max, 16 CPU, 48 GiB, Darwin 27 machine, with Node 24.13.0, Playwright 1.62.1 and Chromium 151.0.7922.34 exactly as recorded in the existing host class and instrument. Both the before and after runs use this one host and the same unchanged instrument and fixture identities. Both runs are controlled: no other jobs, no GLM local compilation, no other browser and no other server are permitted on the host for the full duration of each run, and the actual OS conditions including load and free memory are recorded before and during each run and kept in the raw record. A recorded reading does not by itself prove an otherwise idle host or the absence of interference for the full runtime. If that controlled condition cannot be established and recorded, the run is reported as not established idle, never as idle. No hardware capability claim is made from this procedure.

Source capture. The before source is the literal commit 1f2809c020c147b73219eada4e55c3556aa3e2d2, which contains no F245 providers. The after source does not exist during design. The after run therefore uses the final reviewed F245 staged tree, and the record captures the staged tree reference, the HEAD it was staged from, and the build input. Full hashes are recorded for before and after: source tree, selected native binary, instrument versions, fixture inputs and every captured input. The selected binary is the release native binary chosen from the Cargo compiler artifact by the existing ensureViewerBinary selection, never a hardcoded target path.

Separation. The before and after runs use separate real worktrees, separate output directories and separate targets, so no artifact of one run can feed the other. The synthetic workload and fixture identities are unchanged between runs.

Command. For each of the five registered subjects viewer.open, viewer.first_image, viewer.window_step, viewer.scroll_rate and viewer.resident_memory, the run is bin/ocelli.sh bench followed by the five subject IDs and --compare --out pointing at a fresh private directory. The Node on PATH is the exact Node 24.13.0. No --accept, no tolerance edit and no baseline edit is used. The existing comparison is two sided and an incompatible host or instrument remains incomparable and is reported as such, never a guess. The absolute F216 budgets, the zero blank frame restart checks, the existing recorded bands and the existing F227 informational detail are kept unchanged. No new timing axis is added.

Scope of the measurement. Only the existing canonical local path is measured. No remote rendering performance and no remote memory guarantee is claimed anywhere. The tables in docs/lld/benchmarks.md and the baseline JSON are historical observations from other stories and are not F245 measurements. No observation is reported in this design.

Acquisition path record.

| Metric | Runner | Subject and samples | Timing source | Output |
| --- | --- | --- | --- | --- |
| viewer.open | tools/bench/src/runners/viewer_open.mjs -> measureOpen/openOnce | CT 512x512 mono16, one series, 1693 slices | controller Date.now before setInputFiles to awaited series-status match | median of 9 ms, PET 267x128x128 signed16 no-window informational 9 retained, no new threshold |
| viewer.first_image | tools/bench/src/runners/viewer_first_image.mjs -> measureFirstImage/openFolder/openPage hooks | same CT plus one DX 3000x2900 mono16 | page performance.now, MutationObserver series-status __listAt to queued microtask after actual putImageData view __firstDrawAt | median of 9 each, recorded CT ms, restart same-port and next-draw control unchanged, PET informational median of 9 unchanged |
| viewer.window_step | tools/bench/src/runners/viewer_window_step.mjs -> measureWindow/windowSteps/openPage timedRequest/timedDraw | CT and DX same as above, 9 steps each, C40..48 W400 LINEAR | page performance.now, requestGrey call to drawGrey return | median of 9 CT ms, DX detail, assertWindowSample, canvas check and held preview body 0 unchanged |
| viewer.scroll_rate | tools/bench/src/runners/viewer_scroll_rate.mjs -> measureScroll/scrollOnce | CT 1693 | ArrowDown each rAF for 3 s plus 300 ms settle, page performance.now, changed DRAWN index per callback over elapsed seconds | median of 5 fps, per-run keys >= 120, zero blanks and faults, 55 fps absolute, debug companion zero blanks not a timing-profile substitution |
| viewer.resident_memory | tools/bench/src/runners/viewer_resident_memory.mjs -> measureResident/rssKb/readRetainedHeap | one owned RELEASE server run open, first, scroll, window and PET opened 3 times | native ps -o rss= -p pid every requested 20 ms, not exact cadence, max sample times 1024 bytes | GET /held plus existing GC page heap plus actual sized canvases, sampled peak is a lower observation, not a true peak or all-process collector, existing formulas unchanged |

All 5 wrappers live under tools/bench/src/, existing collectors stay unmodified, and tools/bench/run.mjs writes a fresh --out/run.json plus the existing comparison output files, capturing actual direct exit and timestamps and all input, source, tool and binary hashes separately where not built-in. Bands are 6, 25, 25, 25 and 12 percent for open, first image, window step, scroll rate and RSS, the existing absolute F216 limits remain, no new instrument, threshold, baseline or acceptance is introduced, and the initial helper Cargo selection is verified correct.

## Lead design approval, 2026-11-10

Independent root pass 2 approves the selected design with no open defects or smells. The original draft narrative above records its intake history. This approval sets the plan status and authorizes the normal isolated S47 claim and implementation under the operator's existing work-order exception while S46 remains open. Dependencies and acceptance obligations remain in force. No runtime observation, canonical implementation integration, publication or sprint completion is claimed. The final GLM candidate is 5a6029540259310feab3d5b8b417a21a12d0c7595fd3c419e0a80b3e7efe8abb. Root changed only approval metadata and the two recorded editorial typos when adopting it.

## Lead write-map amendment, 2026-11-10

The named write set also includes `Cargo.lock`, solely to add `serde_json` and `uriparse` to the `ocelli-folder-viewer` package dependency array. These are the two workspace dependencies already selected in the approved Cargo.toml design above. The existing locked versions are serde_json 1.0.151 and uriparse 0.6.4. No new package, version, checksum or unrelated lock entry is authorized by this amendment. The actual lock diff must be checked after resolution.

Independent bookkeeping review by `/root/glm_codec_merge` found 0 defects and 0 smells against the unchanged plan, manifests and lock. This records the existing dependency choice in the generated lock graph. It does not approve the unimplemented viewer consumer or complete F-245.
