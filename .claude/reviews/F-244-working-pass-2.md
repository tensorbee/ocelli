# F-244 review, pass 2

**Reviewed**: staged tree `62351b45e9c21b8890c7b1eb11a10ff7fd133232` on `work/f-244-xai`, HEAD `025f3736282c528f061d5c91ab3e5975af7f5670`. The five staged paths are the plan, pass 1 report, `frame_index.rs`, its permanent tests and `dicom-ingest.md`. No unstaged tracked change was present. This report is a later review artifact, outside that source tree.
**Author / reviewer**: repair by Codex lead `/root`. Independent review by Codex participant `/root/glm_codec_merge`, which authored no F-244 production code or permanent test. The prior Claude implementation and pass 1 Codex observations are historical evidence. New review tests and mutations were confined to an archive of the staged tree under `/private/tmp/ocelli-f244-review-p2`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- AC24's split start prefixes and end markers were challenged with every even partition of two independently chosen marker recipes. Expected groups come from the known recipe boundary, not from the index's output. The 1024 pairs cover JPEG and JPEG 2000, a padded first frame and an unpadded second frame, through direct and parsed populated-table and empty-table controls.
- The real caller was challenged with synthetic JPEG Lossless SV1 and JPEG 2000 frames divided entirely into two-byte Fragment Values. There are 1986 and 436 Fragments per frame respectively. Both populated and empty table versions must decode two frames, each equal to 12288 bytes decoded independently by imagecodecs from the original synthetic source frame.
- AC25 was challenged through `from_parsed`, including zero, 3, the largest `u32` count and `usize::MAX`, followed by a valid call on the same parsed object. Populated Basic and Extended table count mismatches were also checked. No large allocation was needed.
- Adjacent refusal cases cover an incomplete later JPEG prefix, an incorrect JPEG 2000 SIZ prefix, nonzero padding after a divided end marker, and extra bytes after an apparent end. Both direct and parsed callers refuse these cases.
- The allocation error route was exercised by forcing the scratch reservation to `usize::MAX`, which produces a capacity-overflow error before allocating. The new public error must be `AllocationFailed`. Impossible counts must still be refused before that reservation.

The [PS3.5 2026d A.4 fragmentation rules](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html), [8.2 fragmentation requirement](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_8.2.html) and [6.2 OB padding rule](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html) were opened during this pass. Even Fragment Values may be as short as two bytes, and the required OB pad is a single null byte. The bounded marker algorithm is the approved project policy, not an algorithm supplied by those sections.

## Evidence

All Rust commands below ran natively with `CARGO_BUILD_JOBS=2` and `CARGO_TARGET_DIR=/private/tmp/ocelli-f244-review-target`. Each actual subprocess return code, source SHA-256, timestamp and full log is preserved in `/private/tmp/ocelli-f244-review-p2-evidence/runs.json`. The archived implementation's SHA-256 is `8a4339cff294f976c947d2418fccd6d7dc7177a578eacdf3d13177922d4a7b82` before and after the mutations.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test --locked -p ocelli-dicom --test frame_index` | Exact staged archive, original permanent tests | now | 0, 18 pass |
| `cargo test --locked -p ocelli-dicom --test part10` | Same archive, original permanent tests | now | 0, 46 pass |
| `cargo test --locked -p ocelli-dicom --test review_boundaries review_ -- --nocapture` | Three independently added scratch tests | now | 0, 3 pass. The 1024 even-cut pairs pass. One separate injection-only test is intentionally ignored here, and the copied 18 permanent tests are filtered out |
| `generate_two_byte_controls.py` with the repository venv | Only synthetic `syntax/jpeg_lossless_p14_sv1.dcm` and `syntax/j2k_lossless.dcm` are read | now | 0. Populated-table reconstructions decode through imagecodecs to the original source samples |
| `cargo test --locked -p ocelli-dicom --test corpus review_two_byte_real_stream_controls -- --nocapture` | Four temporary populated/empty control files, two frames each | now | 0, all eight decoded frames equal their independent expected bytes |
| Nonzero-pad mutation, then `review_partial_prefix_and_nonzero_end_pad_are_refused` | Scratch only, replace the zero-pad suffix check with an any-byte suffix check | now | 101, semantic assertion failure. It accepts a divided end marker followed by a nonzero byte |
| Forced reservation capacity overflow, then injection-only `review_forced_reservation_failure_is_typed -- --ignored` | Scratch only, reserve `usize::MAX` instead of the bounded count | now | 0, the controlled error returns `AllocationFailed`, without a panic |
| `review_impossible_count_and_refusal_recovery_through_parsed` under the forced reservation | Same injected source | now | 0, impossible counts preempt reservation, and the subsequent valid equal-count call works |
| Remove the early impossible-count check, then the same independent count test | Scratch only | now | 101, semantic assertion failure. An impossible count reaches `AllocationFailed` instead of `BoundaryUnavailable` |
| Restored permanent index and Part 10 selections | Scratch implementation restored byte for byte | now | 0 and 0, 18 and 46 pass |
| Restored independent matrix and real-stream controls | Same restored source | now | 0 and 0, 3 and 1 tests pass again |
| Staged-index comparison with `62351b45` and `git diff --exit-code` | Actual source worktree, before this report | now | each 0, source and index remained unchanged |

No pipeline status was used for any recorded command. A test compiler failure would have failed the mutation harness rather than being counted as a successful semantic mutation.

## Pass 1 findings replayed

- **D1, closed.** The bounded logical prefix is read across Fragment Values, and the rolling suffix retains bytes across a two-byte boundary. The exhaustive small recipes and real two-byte-fragment controls both exercise the repaired parsed caller. Populated controls agree, and malformed next prefixes still refuse.
- **D2, closed.** Counts larger than the actual Fragment count return the typed structural refusal before the marker reservation. The changed reservation is fallible, and the explicit per-frame count guard keeps every range push within the reserved count. Controlled capacity overflow proves the new error mapping without creating real host memory pressure. The remaining copies on this route are proportional to supplied Fragment references, not an unchecked declared count.

## Defects

None found in the bounded repair review.

## Smells

None established.

## Nitpicks

None.

## Verified clean

- The changed source, plan amendments, AC24 and AC25, and LLD describe the same logical prefix, rolling three-byte suffix, exact-count rule and fallible marker reservation.
- Prefix reads stop at the three or four expected logical bytes. Suffix scanning reads at most the last three bytes of each Fragment and retains only three stack bytes. It does not join or allocate an encoded frame to inspect markers.
- Both production consumers still use the shared `from_parsed` provider. The repair adds no second frame-splitting implementation, pixel arithmetic, tolerance change, dependency, gate change or unsafe code.
- Existing populated-table, equal-count, Extended-table and Part 10 tests remain green. Public count refusals preserve the ability to make a subsequent valid call.
- All controlled mutations were in the archive and were restored. The real source worktree was unchanged before the review artifact was written.

## Limitations and handoff

- This pass independently exercised JPEG Lossless SV1 and JPEG 2000 native decoding. It did not repeat all nineteen corpus rows, the collector mutation, vendor pins or the source-provenance sweep. Those remain prior evidence or the lead's separately recorded acceptance work.
- The small recipe matrix validates indexing and grouping only. Its artificial marker recipes are not claimed as complete compressed images. The real-stream controls supply the separate decoded-sample evidence.
- The approved conservative refusal of an ambiguous marker-like header payload remains the policy. No general JPEG or JPEG 2000 header parser was added or claimed.
- No browser, GPU, oracle, wasm build, cross-target gate, full workspace suite, floor or sprint profile was run. Native byte agreement does not close AC22's oracle observation, and a clean repair review does not establish feature completion or sprint readiness.
- Next owner: the lead integrates the repaired source and completes its remaining acceptance inventory. No implementation repair, commit, push or history update was made by this reviewer.
