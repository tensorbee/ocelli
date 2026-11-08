# F-244 review, pass 1

**Reviewed**: `3da2ea76..025f3736282c528f061d5c91ab3e5975af7f5670` on `work/f-244-xai`. The source worktree had no tracked diff. The untracked `.venv` and `tools/oracle/out` symlinks were excluded. Runtime probes and mutations used an archive of that exact tip at `/private/tmp/ocelli-f244-review-p1`, with a separate target directory.
**Author / reviewer**: implementation attributed to a Claude implementer by the lead's handoff. Independent review by Codex participant `/root/f244_review`, which authored none of the reviewed material. Added review-only tests are outside the worktree and are not implementation repairs.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- AC8, AC9 and the claimed marker proof, challenged with permitted two-byte Fragments that divide a start or end marker across Fragments. This includes real JPEG Lossless SV1 and JPEG 2000 codestreams, each with populated-table and empty-table controls.
- AC10, challenged with an impossible declared frame count before boundary scanning. The public checked constructor must refuse an impossible structure without an infallible reservation based on that count.
- AC1 to AC6 and the compressed icon through the Part 10 suite, with the exact collector patch removed as a mutation.
- AC12's exclusion of populated Basic and Extended tables, with that exclusion disabled as a mutation.
- AC14 and AC15 through the existing nineteen-row empty-table corpus test. This is native decode evidence, separate from AC22's unperformed oracle run.
- AC16: the plan's encapsulation and Extended Offset Table transcriptions were checked against the published PS3.5 and PS3.3 2026d pages. Their fragment, padding, table-origin and one-Fragment EOT rules agree. The plan's additional marker algorithm has the factual limitation in D1.

## Evidence

All commands below were executed during this pass. Prior implementation receipts were context only. Rust commands used `CARGO_BUILD_JOBS=2` and `CARGO_TARGET_DIR=/private/tmp/ocelli-f244-review-target` in the archived tip. Logs, review-only Rust tests and the synthetic control generator are retained under `/private/tmp/ocelli-f244-review-evidence`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-dicom --test part10` | Archived tip, original tests | now | 0, 46 pass |
| `cargo test -p ocelli-dicom --test frame_index -- --skip review_` | Archived tip, original tests | now | 0, 15 pass |
| `cargo test -p ocelli-dicom --test frame_index review_ -- --nocapture` | Three independently added hand-byte and impossible-count tests | now | 101, all three fail semantically, D1 and D2 |
| `cargo test -p ocelli-dicom --test corpus review_real_codestream_fragment_controls -- --nocapture` | Synthetic `.70` and `.90` source rows, re-encapsulated into six temporary controls | now | 101, every populated-table control decodes two 12,288-byte frames, every empty-table twin refuses at the index |
| `generate_controls.py` with the repository venv | Same three populated-table controls through imagecodecs | now | 0, both frames equal the original synthetic row's samples in every case |
| `cargo test -p ocelli-dicom --test corpus manifest_backed_empty_table_rows_decode_as_their_sources -- --ignored --nocapture` | Archived manifest and shared corpus read only | now | 0, existing nineteen-row test passes |
| Collector mutation, remove the added ItemEnd assignment, then `part10 empty_table` | Scratch vendor only | now | 101, all four selected tests fail, including the icon |
| Marker mutation, disable the later-start guard, then `frame_index marker_rule_refuses_unproven_boundaries` | Scratch index only | now | 101, false-boundary rejection fails |
| EOT mutation, disable populated-BOT exclusion, then `frame_index from_parsed_prefers_the_extended_offset_table` | Scratch index only | now | 101, expected structural refusal fails |
| Restored `part10` and original `frame_index` selections | All source mutations restored | now | 0 and 0, 46 and 15 pass again |
| `python3 scripts/pin_and_size_check.py` | Actual tip | now | 0 |
| `python3 -m unittest scripts.tests.test_pin_and_size_check` | Actual tip | now | 0, 24 tests pass |
| `python3 scripts/source_provenance_check.py` | Actual tracked tip | now | 0, 1,874 files |
| `python3 scripts/unsafe_allowlist_check.py` | Actual tracked tip | now | 0, vendored dicom-object count remains zero |
| Compare cached `.crate` with vendor collector | Archive SHA-256 `0504b87987f0e7624cf7ee063fe7ffdb438c66c092620817fb9bb0f130686b85` | now | Exact one-line insertion confirmed |
| `git diff --exit-code` | Actual worktree before report | now | 0 |

## Defects

### D1, markers divided by a legal Fragment boundary are refused

**Where**: `crates/ocelli-dicom/src/frame_index.rs:85`, `:426` and `:432`, and the marker algorithm in `.claude/plans/F-244-design.md` and `docs/lld/dicom-ingest.md`.

**What**: the scanner recognizes marker bytes only within one Fragment. A later frame whose first Fragment contains only its two-byte SOI or SOC is refused because the prefix check also requires the following marker bytes in that same Fragment. An EOI divided so that one Fragment ends with FF and the next contains D9 and its Item pad is also invisible to `ends_frame`.

**Why it is wrong**: [PS3.5 A.4](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html) permits even Fragment lengths starting at two bytes. JPEG and JPEG 2000 permit one frame to span Fragments. [PS3.5 8.2](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_8.2.html) requires support for such fragmentation. There is no requirement that codec marker bytes remain in one Fragment. The approved algorithm's prefix restriction therefore excludes conforming inputs, and the implementation follows that restriction faithfully.

**Evidence**: only the permitted synthetic rows `syntax/jpeg_lossless_p14_sv1.dcm` and `syntax/j2k_lossless.dcm` were used. Each temporary file declares two frames, with identical encoded source samples in each. Only encapsulation differs between the populated and empty table twins.

| Control | Fragment Value lengths | Populated table | Empty table |
|---------|------------------------|-----------------|-------------|
| JPEG SV1, second frame begins with SOI alone | 3972, 2, 3970 | Two frames decode | `BoundaryUnavailable { declared: 2, fragments: 3 }` |
| JPEG SV1, EOI spans Fragments | 3976, 2, 3978 | Two frames decode | Same refusal |
| JPEG 2000, second frame begins with SOC alone | 872, 2, 870 | Two frames decode | Same refusal |

The EOI case adds a legal five-byte JPEG COM segment after SOI to make the codestream length odd. Splitting immediately before its last D9 gives an even first Fragment and a two-byte final Fragment containing D9 plus its pad. Independent imagecodecs decoding of the populated-table controls proves that the added segment preserves the source samples. Ocelli's registry also decodes both frames of each populated-table control successfully.

**Scope and regression**: affects the shared index used by folder preview and the oracle producer. The current tests choose cuts away from markers and miss this class. Add specification-derived start and end cases for JPEG and JPEG 2000, including two-byte Fragments and padded final Fragments, through `from_parsed` as well as the pure constructor. Retain false-boundary controls. The smallest repair reads a bounded logical prefix or suffix across adjacent Fragment Values instead of allocating a concatenated frame merely to inspect markers. Amend the plan and LLD with the actual rule. No tolerance change is involved.

### D2, an impossible frame count panics before structural refusal

**Where**: `crates/ocelli-dicom/src/frame_index.rs:423`.

**What**: the empty-table marker path reserves `frame_count` ranges before rejecting a count larger than the number of Fragments. `from_basic(usize::MAX, &[], two_jpeg_fragments, JpegMarkers)` panics with `capacity overflow`. Both constructors are public and accept `usize` frame counts.

**Why it is wrong**: each frame needs at least one Fragment, so this count is already impossible without an allocation. The checked index's structural-refusal contract and AC10 call for a typed error, not a panic on malformed input. This path is new in F-244, the former empty-table path returned `BoundaryUnavailable` directly.

**Evidence**: review-only test `review_impossible_frame_count_returns_error_without_panicking` wraps the public call in `catch_unwind`. It fails after observing the capacity-overflow panic. No enormous allocation was attempted to reproduce the defect, and no real-host out-of-memory result is claimed.

**Scope and regression**: the public `from_basic` and `from_parsed` paths, inherited by both production consumers. Production preview reads a positive `u32` frame count from the file, so large real counts also reach a reservation proportional to the declaration rather than the available Fragments. That resource-exhaustion mechanism was traced, not forced on the host. Reject `frame_count > fragments.len()` before reserving and keep any remaining input-derived capacity fallible or bounded by supplied data. Add a large-count regression and an ordinary count-greater-than-Fragments control.

## Smells

None found in this bounded pass.

## Nitpicks

None.

## Verified clean

- The vendored source change is exactly the recorded ItemEnd assignment relative to the cached published archive. It fixes top-level and nested empty tables without changing the populated-table collector path.
- The Part 10 and index suites distinguish the collector patch, later-start refusal and EOT exclusion from the corresponding wrong implementations. Each recorded mutation causes semantic assertion failures, followed by restored green selections.
- The current nineteen-row corpus selection decodes as its source rows, including the existing multi-frame shapes. This finding does not cover D1's new fragment cuts.
- The two production consumers call `from_parsed`, so table selection and marker policy remain in one provider. No pixel arithmetic or tier-specific decoder route was added by this story.
- The vendor pin unit suite, provenance scanner and unsafe scanner pass at the reviewed tip.

## Limitations and handoff

- No remediation was written. The lead owns the rulings and implementation assignment, followed by a fresh independent review.
- No browser, GPU, oracle, wasm build, native cross-target gate, full workspace suite, licence gate or sprint profile was run. AC19, AC20, full AC21, AC22 and the unrun licence part of AC23 are not established by this pass.
- This pass inspected the added corpus generator and pin wiring but did not regenerate the shared corpus. It read no patient source, and created synthetic DICOM only under temporary storage.
- AC16's quoted DICOM rules agree with the fetched 2026d pages. That does not validate the extra algorithm the plan derives from those rules, which D1 disproves.
- Source identity and actual observations are bounded to the reviewed range. A clean gate elsewhere cannot close D1 or D2.
