# F-019, Multiframe and enhanced SOP class handling

**Status**: approved
**Epic ref**: E3.4
**Sprint**: S08
**Estimate**: 4w

## Normative source, transcribed

### DICOM PS3.3 enhanced multiframe functional groups

| Tag | Name |
|-----|------|
| `(0028,0008)` | Number of Frames, IS. Absent means one |
| `(5200,9229)` | Shared Functional Groups Sequence |
| `(5200,9230)` | Per-frame Functional Groups Sequence |

The lookup order is per-frame first, then shared. An attribute present in both
sources is malformed evidence even though the per-frame value is the effective
value. Dimension Index Sequence defines frame ordering, and frames are not
required to be stored in spatial order.

Useful nested groups include Plane Position Sequence, Plane Orientation
Sequence, Pixel Measures Sequence, Frame VOI LUT Sequence, Pixel Value
Transformation Sequence, Frame Content Sequence, and Real World Value Mapping
Sequence.

### DICOM PS3.5 encapsulated pixel data

```text
(7FE0,0010) OB, undefined length
  (FFFE,E000) Basic Offset Table
  (FFFE,E000) fragment 1
  (FFFE,E000) fragment 2
  ...
  (FFFE,E0DD) Sequence Delimitation
```

One frame is not one fragment. A frame may span several fragments, and a
fragment does not span two frames. An empty Basic Offset Table is legal.
Extended Offset Table `(7FE0,0001)` and Extended Offset Table Lengths
`(7FE0,0002)` provide 64-bit frame boundaries.

### `docs/hld/20-errors-and-panics.md`, section 23

> Error codes are stable and versioned. The shell switches on the code. The
> message is for humans and may change.

### `docs/hld/22-testing-and-tolerance.md`, section 25

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

## What the specification does not cover

The HLD names multiframe support but gives no Rust signatures for frame count,
frame selection, source provenance, or offset tables. The active sprint also
excludes F-020's interpretation of per-frame geometry, gantry tilt, and spacing
calibration. This story therefore preserves functional-group content and its
source without interpreting those geometry rules.

The metadata model is source-neutral. It accepts the repository's existing
lossless `MetadataSet` rather than introducing a second DICOM parser. Frame
indices are zero-based in Rust while the error identifies the declared frame
count. No API silently clamps an out-of-range frame.

## Approach

1. Add a multiframe projection module to `ocelli-dicom` over `MetadataSet`.
   Parse Number of Frames as one positive IS value, defaulting to one only when
   absent. Refuse empty, zero, multiple, malformed, or target-width-overflowing
   declarations.
2. Represent `TopLevel`, `Shared`, and `PerFrame` provenance explicitly. A
   resolved value includes the winning source and records a duplicate-source
   conflict rather than erasing it.
3. Validate that Per-frame Functional Groups has exactly the declared number
   of items when present and that Shared Functional Groups has at most one.
4. Expose checked frame selection. Resolve a requested functional-group value
   per-frame first, shared second, and top-level only for attributes whose
   module permits that source.
5. Add an encapsulated-frame index type that validates Basic Offset Table or
   Extended Offset Table offsets against ordered fragment boundaries. It
   returns borrowed fragment ranges and does not concatenate or decode them.
6. For an empty Basic Offset Table, accept a single frame over all fragments.
   For multiple frames, accept one fragment per frame only when the counts
   prove that mapping. Otherwise report unavailable boundary evidence instead
   of guessing.
7. Preserve Dimension Index and Frame Content metadata without sorting. F-020
   owns spatial grouping and calibration.
8. Mutate frame count, precedence, offset base, and final frame bound. Run the
   named tests red and revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Metadata projection and frame indexing happen
  before rendering
- unsafe: none
- Tier A (WebGPU): n/a. Frame selection is decode-worker metadata work
- Tier B (WebGL2): n/a. The same metadata contract is used
- Tier C (CPU): n/a. The same metadata contract is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | The synthetic enhanced multiframe row exposes the declared frame count and preserves distinct shared and per-frame sources, citing PS3.3 functional-group rules | `crates/ocelli-dicom/tests/multiframe.rs` |
| fixture | A controlled metadata fixture resolves per-frame before shared and reports duplicate-source evidence | `crates/ocelli-dicom/tests/multiframe.rs` |
| fixture | Basic and Extended Offset Tables select hand-computed fragment ranges, including one frame spanning multiple fragments, citing PS3.5 Annex A | `crates/ocelli-dicom/tests/frame_index.rs` |
| unit | Missing Number of Frames defaults to one while empty, zero, malformed, multiple, and inconsistent counts are refused | module tests |
| unit | Out-of-range frame selection, truncated offset arrays, non-monotonic offsets, and offsets outside fragment bounds are refused | module tests |
| conformance | The manifest-backed multiframe row is projected without patient data entering source or logs | existing ignored corpus through `bin/ocelli.sh corpus` |
| mutation | Precedence, offset base, count, and final-bound mutations make named tests fail | feature review evidence |
| cross-target | The same types compile for native and wasm | `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh wasm` |

## Parity surface covered

Appendix B has no separate multiframe count row. This story supplies the frame
selection and provenance foundation consumed by stack and volume parity work.

## Deviations

None.

## LLD impact

- Update `docs/lld/dicom-ingest.md` with enhanced multiframe provenance,
  selection, offsets, and the explicit F-020 boundary.
- Update `docs/lld/README.md`.

## Write set

- `crates/ocelli-dicom/src/lib.rs`
- new multiframe source files under `crates/ocelli-dicom/src/`
- `crates/ocelli-dicom/tests/multiframe.rs`
- `crates/ocelli-dicom/tests/frame_index.rs`
- `docs/lld/dicom-ingest.md`
- `docs/lld/README.md`
- shared sprint ledgers during completion

## Open questions

None.
