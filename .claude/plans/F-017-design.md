# F-017, Metadata model and provider registry

**Status**: approved
**Epic ref**: E3.2
**Sprint**: S07
**Estimate**: 4w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a full stop because `scripts/prose_check.py` covers this plan. No
other word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/03-architecture-and-crates.md`, sections 3 and 4

> The shell is TypeScript on the main thread: DOM and pointer events, the SVG
> annotation layer, tool interaction state, framework bindings, and DICOMweb
> fetch and authentication. The core is Rust running in workers. Between them
> sits a deliberately narrow boundary carrying commands down, bulk bytes down,
> and events up.

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-dicom | Parsing, transfer-syntax dispatch, metadata model and providers | yes | yes |
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |

### `docs/hld/08-validation-architecture.md`, section 11

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.

> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic,
> and the same corpus a regulatory submission would want to see.

### `docs/hld/12-workspace-and-build.md`, Part II and sections 15.1 to 15.3

> This part is prescriptive. Where it gives a formula, a layout or a signature,
> that is the intended implementation and a deviation should be raised rather
> than improvised. It exists because the dangerous defect in medical imaging
> is not the crash - it is the pixel that is quietly wrong, and quietly wrong
> code is produced by reasonable people making locally reasonable choices.

> ocelli-dicom/ \# parse, metadata, providers

> ocelli-wasm/ \# \*\* the only wasm-bindgen crate \*\*

> **DICOM-RS FEATURES** Disable default features. The defaults are rayon and
> simd, and the gdcm feature is native-only. On wasm you want:
> default-features = false, then jpeg, rle, deflate and openjp2 selected
> explicitly. Note also that the inventory-based transfer-syntax plugin
> registry does not work on wasm at all - which is why section 21 specifies an
> explicit runtime registry instead.

### `docs/hld/20-errors-and-panics.md`, section 23

> thiserror in the core crates. The boundary maps everything to a stable
> numeric code and a message. The important part is what happens when that is
> not enough.

> Error codes are stable and versioned. The shell switches on the code. The
> message is for humans and may change.

### `docs/hld/22-testing-and-tolerance.md`, section 25

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Unit and property | LUT arithmetic; geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
| Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

### `docs/hld/24-agent-code-standards.md`, sections 27.2 and 27.3

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |
| R6 | Provenance trailer on every commit | Cheap now; a retrofit across sixty thousand lines is not, and a device pathway may require it |

> - Every as cast and every rounding decision.
>
> - That a new test would actually fail if the code were wrong. Mutate one
>   constant, re-run, confirm it goes red.

## What the specification does not cover

The HLD assigns the metadata model and providers to `ocelli-dicom`, but it does
not specify their Rust signatures, the set of projected attributes, provider
precedence, registration semantics, or the representation of absent and empty
values. The active sprint supplies the missing acceptance contract: absence,
padding, multiplicity, signed values, and the identity of the provider that
answered must remain observable.

This plan chooses a lossless `MetadataSet` projection over the F-016 parsed object.
It represents a missing tag as a lookup outcome, a present zero-length element
as an empty value, and a present primitive element as its declared VR plus an
ordered value list. Text components retain the parser's stored spelling and
legal trailing pad. A trimmed view is computed when requested rather than
replacing the stored spelling. Signed integer variants remain signed.

`MetadataSet` and its element constructors are public within `ocelli-dicom` so
F-021 can construct the same representation from QIDO-RS DICOM JSON without
editing this story's files. F-021 owns JSON parsing. F-017 owns the value and
collection invariants shared by Part 10 and DICOM JSON. The shared value type
therefore also represents nested sequences, DICOM JSON Person Name component
objects, `BulkDataURI`, and validated `InlineBinary`. Those carriers do not
collapse into strings. Encapsulated Pixel Data fragments remain outside the
metadata projection because F-023 and later decoder stories own that path.

The provider registry is an ordered collection of named provider functions. A
lookup returns both the value and the stable provider identifier that supplied
it. It has no implicit default provider and no cross-provider merge. The first
provider that returns a present result wins. A provider that observes a present
empty value has answered, so lookup does not continue and turn emptiness into a
fallback value. Duplicate provider identifiers and duplicate registration of
the same function are refused.

The HLD Appendix B has no `Covered by` column despite the design workflow's
wording. Its metadata source count is context, not an acceptance list. This
story therefore claims the metadata model and provider boundary, not numerical
parity with every symbol in the reference metadata package.

## Approach

1. Add `metadata.rs` in `ocelli-dicom`. Define the lossless public metadata
   set, element, and value types plus conversion from a
   `dicom_core::value::PrimitiveValue`. Preserve
   the element VR, explicit empty state, ordered multiplicity, text padding,
   signed integer widths, unsigned integer widths, floating values, tags, and
   byte or word payloads. Recursively project ordinary sequence items into
   nested `MetadataSet` values. Add explicit constructors for the Person Name,
   `BulkDataURI`, and `InlineBinary` carriers that F-021 obtains from DICOM
   JSON. Pixel-fragment values are reported as unsupported rather than
   flattened.
2. Add `provider.rs`. Define stable `ProviderId`, a request containing the
   parsed object and requested tag, a function-pointer provider signature, the
   ordered registry, and a lookup result containing provider identity and
   projected value. Function pointers avoid a one-implementer trait and avoid
   `Box<dyn>`.
3. Supply two production providers today. The data-set provider reads the
   parsed main data set. The file-meta provider exposes File Meta Information
   through the same result type. They are distinct sources with explicit
   precedence, not test-only implementers.
4. Keep provider registration caller-owned and deterministic. Registration
   order is the only precedence rule. Lookup never guesses a provider, tag, VR,
   or default value.
5. Extend the existing F-016 hand-encoded Part 10 test helpers with synthetic,
   non-identifying attributes. Exercise absent, explicit empty, padded UI and
   space-padded text, ordered DS multiplicity, and negative SS and SL values.
6. Record a controlled mutation that changes present-empty into missing or
   reverses provider precedence. The corresponding test must fail for that
   reason.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none, metadata lookup is outside the render loop
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Missing differs from present empty, first answering provider wins, present empty stops fallback, and provider identity is returned | `crates/ocelli-dicom/src/provider.rs` |
| fixture | PS3.5 value representation and padding rules retain padded UI and text spelling, ordered multi-valued DS text, signed SS and SL values, and a nested sequence from hand-encoded non-identifying data elements | `crates/ocelli-dicom/tests/metadata.rs` |
| property | Projection preserves primitive multiplicity and signedness for generated bounded primitive values | `crates/ocelli-dicom/tests/metadata.rs` |
| cross-target | Native and wasm32 compile and run the same metadata and provider API without `wasm-bindgen` | `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh wasm` |
| mutation | Treating empty as missing or reversing provider order makes the named boundary test fail | recorded in the clean feature review |

No pixel or geometry arithmetic is introduced, so the pixel fixture rule does
not apply.

## Parity surface covered

Appendix B records 6,395 source lines for the reference metadata package, but
contains no row or `Covered by` mapping for E3.2. F-017 covers the metadata
model and provider lookup contract from the backlog and crate table. It does
not claim the reference package's full line-by-line surface.

## Deviations

Existing D-02 supplies the `ocelli-dicom` crate name. Existing D-18 supplies
the direct dicom-rs component dependency shape. No new deviation is planned.

## LLD impact

- Extend `docs/lld/dicom-ingest.md` with the lossless metadata representation,
  provider ordering, present-empty semantics, provider identity, and refusal
  boundaries.
- Update `docs/lld/README.md` only if its F-ID index requires F-017 to be added.

## Write set

**Created**

- `crates/ocelli-dicom/src/metadata.rs`
- `crates/ocelli-dicom/src/provider.rs`
- `crates/ocelli-dicom/tests/metadata.rs`

**Modified**

- `crates/ocelli-dicom/Cargo.toml`, only if property-test support is not
  already available from the workspace
- `crates/ocelli-dicom/src/lib.rs`
- `docs/lld/dicom-ingest.md`
- `docs/lld/README.md`, only if its story index requires it
- `.claude/plans/F-017-design.md`
- `docs/sprints/CURRENT_SPRINT.md`
- `docs/sprints/BACKLOG.md`
- `docs/sprints/SPRINT_TRACKER.md`
- `docs/sprints/AS_BUILT.md`
- `CHANGELOG.md`

F-021 owns new DICOMweb source files. F-022 owns new NIfTI files. Neither may
edit `metadata.rs` or `provider.rs` in its implementation wave. Sprint ledger
files remain integrator-only when a parallel worker prepares a feature.

## Open questions

None.
