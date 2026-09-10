# F-022, NIfTI volume ingest

**Status**: approved
**Epic ref**: E3.7
**Sprint**: S07
**Estimate**: 2w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its
prose semicolon to a comma because `scripts/prose_check.py` covers this plan.
No technical word, formula, signature, comparison, field, or value is changed.
The tracked HLD wins where exact source punctuation matters._

### Direct NIfTI requirements are absent

The complete tracked HLD contains no occurrence of `NIfTI`, `NIFTI`,
`nifti`, `qform`, `sform`, or `vox_offset`. There is therefore no repository
normative source to transcribe for the format version, header layout, supported
container forms, datatype set, byte order, scaling, affine selection, or
RAS-to-LPS conversion. The nearest prescriptive rule is
`docs/hld/12-workspace-and-build.md`, Part II:

> This part is prescriptive. Where it gives a formula, a layout or a
> signature, that is the intended implementation and a deviation should be
> raised rather than improvised. It exists because the dangerous defect in
> medical imaging is not the crash - it is the pixel that is quietly wrong,
> and quietly wrong code is produced by reasonable people making locally
> reasonable choices.

The missing NIfTI contract is recorded in `## Open questions`. This draft does
not present a plausible NIfTI convention as if the HLD selected it.

### `docs/hld/03-architecture-and-crates.md`, sections 3 and 4

> The shell is TypeScript on the main thread: DOM and pointer events, the SVG
> annotation layer, tool interaction state, framework bindings, and DICOMweb
> fetch and authentication. The core is Rust running in workers. Between them
> sits a deliberately narrow boundary carrying commands down, bulk bytes down,
> and events up.

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-core | Types, coordinate spaces, geometry primitives, error model. No I/O. | yes | yes |
| ocelli-dicom | Parsing, transfer-syntax dispatch, metadata model and providers | yes | yes |
| ocelli-volume | Volume assembly, geometry, reslicing | yes | yes |
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |

### `docs/hld/04-boundary-and-data-path.md`, sections 5 and 5.2

> This is the design decision everything else follows from. Get it wrong and
> the port is slower than what it replaces while being harder to debug. Three
> channels, and nothing else crosses.

> The core allocates and returns a pointer and length, JavaScript builds a
> typed-array view immediately before writing and discards it after. Views are
> never cached across a call that might allocate, because any WebAssembly
> memory growth relocates the backing buffer and detaches every outstanding
> view. This is the sharpest edge in the whole design and section 17.2 gives
> the exact pattern.

### `docs/hld/11-decision-log.md`, section 14

| **#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D2 | wasm-bindgen in one crate only | Bindings wherever convenient | Phases 2 and 3 are entry points, not rewrites |
| D3 | Pixels never cross the boundary | Decode in wasm, render in JS | The main architectural gain |
| D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |
| D11 | Chunked residency is the default path | Brick only above a size threshold | Bounded GPU memory becomes a claim, not a fallback |

### `docs/hld/13-core-types.md`, section 16

> Cornerstone represents canvas points, world points and voxel indices all as
> number\[\]. Mixing them is a silent, common and expensive bug. Rust can make
> the mistake impossible at compile time, and this is one of the clearest
> places the language actually earns its cost.

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

> *The payoff: Transform\<Canvas, World\> composes with
> Transform\<World, Index\> and will not compose with anything else. A whole
> class of tool bugs stops compiling.*

Deviation D-08 changes the marker derives in the current tree. It also means
the transcribed `Clone` and `Copy` note no longer describes why the
hand-written implementations remain. F-022 consumes the current
`Index`, `World`, and `Transform<Index, World>` contract and does not reopen
that deviation.

### `docs/hld/16-volume-representation.md`, section 19

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    pub voxel: VoxelKind, // U8 | I16 | U16 | F32
    pub data: VoxelStore, // one contiguous allocation, x fastest
    pub levels: Vec<Level>, // multiscale; level 0 is full resolution
    pub present: bitvec::BitVec, // per-slice, for progressive assembly
}
```

> - **Layout.** One contiguous allocation, x fastest then y then z. Slice n
>   starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not
>   introduce a per-slice Vec, the whole point is a single upload region.
>
> - **Progressive assembly.** Clear the 3D texture at creation and upload
>   slices as they land. present drives the loading indicator and tells the
>   oracle which frames are comparable yet.
>
> - **Bricks and a level axis, from the start.** The volume carries a
>   multiscale level axis and 128³ brick decomposition even when a series
>   fits comfortably. Phase 1 uploads every brick, section 30 makes residency
>   selective. Retrofitting a level axis into a volume model that never had
>   one is the rewrite cornerstone3D cannot afford.

The `Volume` type is still a scaffold. Backlog F-051 owns construction from a
stack in S18 and F-058 owns its multiscale hooks in S19. This plan therefore
cannot assume that F-022 may implement a reduced second volume model without
an explicit design answer.

### `docs/hld/20-errors-and-panics.md`, section 23

> thiserror in the core crates, the boundary maps everything to a stable
> numeric code and a message. The important part is what happens when that is
> not enough.

> Error codes are stable and versioned. The shell switches on the code, the
> message is for humans and may change.

> A poisoned instance surfaces to the user as a viewport-level error state,
> never as a silent blank canvas.

### `docs/hld/22-testing-and-tolerance.md`, sections 25 and 25.1

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Unit and property | LUT arithmetic, geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
| Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.
>
> - A tolerance change is a pull request with a rationale, reviewed like code.

The test taxonomy is DICOM-specific and supplies no NIfTI conformance source.
The 1e-6 mm world-coordinate bound still applies after conversion into
`World`, which the HLD defines as DICOM patient LPS millimetres.

### `docs/hld/24-agent-code-standards.md`, sections 27.2 and 27.3

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R3 | Every function doing pixel arithmetic needs a fixture test with hand-computed values, citing the DICOM section | This is the defect class that reaches patients |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

> - Every as cast and every rounding decision.
>
> - LUT and geometry arithmetic against the cited specification section - not
>   against the comment above it, which was generated by the same process as
>   the code.
>
> - That a new test would actually fail if the code were wrong. Mutate one
>   constant, re-run, confirm it goes red.

## Sprint acceptance source, transcribed separately

These are tracked delivery requirements, not missing HLD text.

### `docs/sprints/CURRENT_SPRINT.md`, what this sprint is

> F-022 adds a second medical-image input format without pretending its
> geometry conventions are DICOM conventions.

### `docs/sprints/CURRENT_SPRINT.md`, defect class

> NIfTI and DICOM do not name patient axes the same way. Treating a NIfTI
> affine as if it were already DICOM patient geometry can produce a
> well-shaped but mirrored or transposed volume. The input format and the
> coordinate conversion must be explicit and independently tested with a
> non-symmetric affine.

### `docs/sprints/CURRENT_SPRINT.md`, what done means

> **F-022** validates NIfTI headers and payload bounds, preserves the declared
> affine, and converts its geometry into an explicit internal coordinate
> contract. Truncated input, unsupported datatype or endianness, and invalid
> dimensions are refused with synthetic fixtures.

### `docs/sprints/CURRENT_SPRINT.md`, dependency order

> F-017, F-021, and F-022 all touch `ocelli-dicom`, so their designs must
> settle shared types and file ownership before concurrent implementation.

The backlog and allocation agree on E3.7, S07, Rust, 2w, dependency F-016,
and status pending. `allocation.json` contains no architecture note for F-022.

## What the specification does not cover

1. **Normative format source.** The HLD does not identify a NIfTI edition or
   specification. The official NIfTI site defines NIfTI-1.1 and NIfTI-2, but
   neither is incorporated into the repository's normative set today.
2. **Container scope.** It does not choose NIfTI-1 or NIfTI-2, single-file
   `n+1` or paired `ni1`, uncompressed input or gzip, or extension handling.
3. **Crate and output contract.** Section 4 puts parsing and metadata in
   `ocelli-dicom` and volume assembly in `ocelli-volume`. It does not place a
   non-DICOM volume parser. The active sprint says F-022 touches
   `ocelli-dicom`, while section 19's eventual `Volume` is not implemented.
4. **Supported dimensionality.** A NIfTI header can describe one through seven
   dimensions. The HLD `Volume` has exactly three. No source says whether a
   three-dimensional ingest may accept higher singleton dimensions or how it
   treats time, vector, and intent dimensions.
5. **Datatype, scaling, and units.** Section 19 names only `U8`, `I16`, `U16`,
   and `F32`. NIfTI has more datatypes and optional `scl_slope` and
   `scl_inter`. The sources do not say whether F-022 normalises payload bytes,
   applies scaling, retains scaling for a later pixel stage, or rejects a
   non-identity scale. They also do not say which spatial-unit codes may enter
   a contract expressed in millimetres.
6. **Byte order.** The sprint requires an unsupported endianness refusal but
   does not name the supported endianness. NIfTI-1 permits byte-swapped
   headers and payloads, so a little-endian-only implementation and a
   two-endian implementation both fit part of the current wording.
7. **Affine selection and retention.** NIfTI can declare both `qform` and
   `sform`, with separate codes, and they can differ. The sprint uses the
   singular phrase "the declared affine" but supplies no precedence, mismatch
   policy, fallback when both codes are zero, finite or invertible requirement,
   or definition of what preserving the unselected transform means.
8. **Coordinate conversion.** The sprint requires an explicit conversion but
   the HLD defines only DICOM LPS `World`. It does not define a NIfTI RAS space,
   the matrix multiplication order for RAS to LPS, or whether a second marker
   type is required to make an unconverted affine unrepresentable as `World`.
9. **Volume representation.** Section 19 stores origin, spacing, and direction
   rather than a full affine. It says direction is derived from DICOM
   `ImageOrientationPatient`. A general NIfTI `sform` may include shear. The
   sources do not prove that decomposition into those fields preserves every
   supported affine or say whether extending the exact section 19 layout
   requires a deviation.
10. **Failure boundary.** Section 23 requires stable numeric boundary codes,
    but F-022 has no boundary export. It does not say whether local
    `NiftiError` variants are sufficient until the worker protocol, as F-016
    does with `ParseError`, or whether codes in a reserved crate range land
    now.
11. **Conformance and parity.** Section 25 names DICOM fixtures, the DICOM
    corpus, and cornerstone3D. Appendix B has no `Covered by` column and no
    NIfTI row. No source names the independent implementation or published
    fixtures that constitute NIfTI conformance evidence.

## Approach

No NIfTI format behavior becomes approved until the questions below select
the missing normative contract. The following implementation envelope is
fixed by the existing HLD and sprint acceptance regardless of those answers.

1. Add a dedicated NIfTI module. It accepts an in-memory byte slice and never
   performs network or filesystem I/O. It does not reuse DICOM parse types,
   transfer-syntax dispatch, metadata providers, or DICOM geometry names.
2. Validate the complete selected header form before reading any field-dependent
   offset. Detect byte order from the selected specification's header rule.
   Validate magic, rank, positive spatial dimensions, the exact datatype to
   bit-width pairing, finite required geometry fields, spatial units, payload
   offset, checked voxel-count multiplication, checked byte-count
   multiplication, and the full payload range. No failed interpretation is
   retried as a common datatype, byte order, or container form.
3. Return immutable evidence of the parsed header, selected byte order,
   datatype, dimensions, spatial units, scaling declaration, exact payload
   range, both declared affine forms and codes, and the selected affine plus
   its provenance. No error retains source bytes, free-text header fields,
   file paths, or identifiers.
4. Keep source coordinates distinct until the explicit conversion step. The
   converted result is a `Transform<Index, World>`, where `World` is the HLD's
   DICOM patient LPS millimetres. Tests use its action on basis points because
   the current `Transform` intentionally does not expose its matrix.
5. Do not create the section 19 `Volume` or a reduced parallel volume type in
   F-022. Retain validated contiguous payload bytes or their exact range in x
   fastest order for the later owner. Any decision to instantiate or change
   `Volume` first updates the dependency plan for F-051 and F-058 and evaluates
   a deviation from section 19.
6. Use checked integer conversions and arithmetic. Do not use `as` casts for
   dimensions, offsets, byte counts, datatype widths, or payload bounds. Do
   not use `unsafe` for typed views or byte swapping.
7. Build synthetic non-identifying byte fixtures directly from the adopted
   format specification. Include a non-symmetric affine with translation and
   unequal axis scales. Check at least the origin and the three basis points
   by hand in millimetres, so an omitted axis flip or transposed matrix turns
   the fixture red.
8. Add controlled mutations for byte-order detection, one dimension in the
   checked payload product, and one coordinate-axis sign. Each mutation must
   make its named test fail for that reason.
9. Build the same API for native and `wasm32-unknown-unknown`. Any dependency
   selected in the design round must have defaults disabled where possible,
   must not reach `wasm-bindgen` through Ocelli source, and must not introduce
   a native-only compression or I/O route.

The recommended contract in `## Open questions` would put this module in
`ocelli-dicom` to honour the active sprint's explicit shared-file statement,
while keeping all public names NIfTI-specific. It would return validated
metadata and payload evidence rather than implementing section 19 early.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no additional crossing. Raw input bytes use the
  existing bulk channel when a later boundary story exposes this parser
- Render-loop allocation: none. Header and payload validation happen during
  ingest, outside the render loop
- unsafe: none
- Tier A (WebGPU): n/a. Parsing and coordinate conversion are tier-independent
- Tier B (WebGL2): n/a. Parsing and coordinate conversion are tier-independent
- Tier C (CPU): n/a. The same parsed contract is produced for every tier

## Tests

The exact rows depend on the selected format contract. These categories and
failure classes do not.

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | A synthetic header with a hand-computed, non-symmetric voxel-to-RAS affine converts the origin and three basis points to DICOM LPS within 1e-6 mm under the adopted NIfTI affine section | `crates/ocelli-dicom/tests/nifti.rs` |
| fixture | Synthetic payload bytes in every supported datatype and byte order produce the selected specification's declared value layout without fallback | `crates/ocelli-dicom/tests/nifti.rs` |
| unit | Truncated header, wrong magic, unsupported container or version, datatype and bit-width mismatch, unsupported byte order, invalid rank or dimension, non-finite geometry, invalid units, arithmetic overflow, offset before the allowed payload start, and a short payload are distinct refusals | `crates/ocelli-dicom/tests/nifti.rs` |
| property | For supported positive dimensions and voxel widths, the checked payload length equals the product defined by the adopted specification, while any truncation is refused | `crates/ocelli-dicom/tests/nifti.rs` |
| cross-target | Native and wasm32 compile the same parser and coordinate contract without `wasm-bindgen` or native-only I/O | `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh wasm` |
| mutation | Reversing byte order, omitting one dimension from the payload product, or removing one RAS-to-LPS sign flip makes the named fixture fail | recorded in the clean feature review |

If the approved contract applies `scl_slope` and `scl_inter`, that operation is
pixel arithmetic and gains a separate hand-computed fixture from the adopted
NIfTI specification before implementation. A DICOM section cannot honestly be
cited for NIfTI scaling. The design round must either approve that external
normative citation or defer scaling application and retain the declaration.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column despite the
design workflow's wording, and it contains no NIfTI row or E3.7 mapping. F-022
therefore claims the backlog and sprint ingest contract only. It does not
change any Appendix B count.

## Deviations

Existing D-02 supplies the `ocelli-dicom` crate name. Existing D-08 defines
the current coordinate-marker derives. No new deviation is planned under the
recommended contract because F-022 does not implement or change section 19's
`Volume`.

If the design round chooses to add a full affine field to `Volume`, create a
second volume representation, or implement section 19 without its level and
brick hooks, the plan must return to draft and assess a new deviation before
code begins.

## LLD impact

- Create `docs/lld/nifti-ingest.md` with the approved format scope, header and
  payload contract, datatype and byte-order table, affine precedence and
  provenance, RAS-to-LPS conversion, and error taxonomy.
- Update `docs/lld/README.md` to index the new area and list F-022.
- Update `docs/lld/build-targets.md` only if F-022 adds a dependency whose
  native and wasm feature shape needs recording.
- Do not fold NIfTI behavior into `docs/lld/dicom-ingest.md`. The two formats
  share a crate under the recommended placement, not a wire format or geometry
  convention.

## Write set and shared ownership

### Created by F-022

- `crates/ocelli-dicom/src/nifti.rs`
- `crates/ocelli-dicom/tests/nifti.rs`
- `docs/lld/nifti-ingest.md`

### Modified by F-022

- `crates/ocelli-dicom/src/lib.rs`, additive NIfTI module and re-exports only
- `crates/ocelli-dicom/Cargo.toml`, only for approved `ocelli-core`, `glam`, or
  parser dependencies
- `Cargo.toml` and `Cargo.lock`, only if the approved parser uses a new
  workspace dependency
- `docs/lld/README.md`
- `docs/lld/build-targets.md`, only if dependency or target behavior changes
- `.claude/plans/F-022-design.md`
- `docs/sprints/CURRENT_SPRINT.md`
- `docs/sprints/BACKLOG.md`
- `docs/sprints/SPRINT_TRACKER.md`
- `docs/sprints/AS_BUILT.md`
- `CHANGELOG.md`

### Reserved to sibling S07 stories

- F-017 owns `crates/ocelli-dicom/src/metadata.rs`,
  `crates/ocelli-dicom/src/provider.rs`, and
  `crates/ocelli-dicom/tests/metadata.rs`.
- F-021 owns its DICOMweb source implementation and tests.
- F-023 owns `ocelli-codec` registry files and codec capability tests.
- F-022 does not edit F-016's `crates/ocelli-dicom/src/parse.rs`,
  `crates/ocelli-dicom/tests/part10.rs`, or
  `crates/ocelli-dicom/tests/corpus.rs`.
- F-022 does not edit `crates/ocelli-volume` unless the operator rejects the
  recommended parser-only boundary and the design is revised first.

`crates/ocelli-dicom/src/lib.rs`, `crates/ocelli-dicom/Cargo.toml`, workspace
dependency files, `docs/lld/README.md`, and the sprint ledgers are shared merge
points. Parallel workers keep changes to them additive. Sprint ledger files
remain integrator-only when a worker prepares a handoff.

## Open questions

None.

## Design-round decision

The operator approved the recommended contract as one bundle. F-022 adopts
the official NIfTI-1.1 `nifti1.h` definition and implements a direct parser for
uncompressed single-file `n+1` input. It refuses NIfTI-2, paired `ni1`, gzip,
extensions, and big-endian input with distinct errors.

The parser accepts three spatial dimensions and requires only dimensions 4
through `dim[0]` to be one when they are declared. Undeclared higher dimension
slots are not interpreted. It accepts only `U8`, `I16`, `U16`, and `F32`,
requires `(xyzt_units & 0x07) == NIFTI_UNITS_MM`, and treats temporal unit bits
independently. It preserves `scl_slope` and `scl_inter` exactly without
applying them or adding a finiteness refusal that the source does not require.

The signed `qform_code` and `sform_code` fields are active only when greater
than zero, as Methods 2 and 3 in the official header specify. Negative codes
are invalid. The parser retains both affine declarations and codes, selects an
active `sform` before an active `qform`, and refuses an input with neither. An
unselected declaration remains raw header evidence. Finiteness and singularity
validation apply to the selected constructed affine. Differing active forms
are retained with visible selection provenance rather than refused.

For qform construction, `b*b + c*c + d*d` greater than one is invalid and is
not normalised or admitted through a tolerance. `pixdim[0]` accepts `-1` and
`1`, while `0` means `1` as the official NIfTI-1 header directs. Every other
qfac value is invalid.

`vox_offset` must be finite, exactly integral, representable as the target
range type, and at least 352. The below-352 refusal is Ocelli product scope,
since the official NIfTI-1 text treats a smaller value as 352. Alignment to 16
bytes is recommended by NIfTI-1 but is not required here. A zero extension
flag permits an offset greater than 352, and bytes after the checked payload
range are permitted. Checked length arithmetic is implemented by one private
helper instantiated as `usize` in production and `u32` in a test that proves
the 32-bit overflow refusal.

Distinct NIfTI-2 recognition uses the official `nifti2.h` header definition,
where `sizeof_hdr` is 540 and the eight-byte magic follows it at byte 4:
<https://github.com/NIFTI-Imaging/nifti_clib/blob/master/nifti2/nifti2.h>.

The selected RAS affine is converted explicitly to DICOM LPS and exposed only
as `Transform<Index, World>`. The module lives in `ocelli-dicom` and returns
validated header, affine, and payload-range evidence. It does not implement or
change the section 19 `Volume`. Synthetic fixtures cite the official NIfTI-1.1
specification and calculate expected values independently. A second parser
implementation is not required.
