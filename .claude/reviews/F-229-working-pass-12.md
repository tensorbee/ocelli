# F-229 review, pass 12

**Reviewed**: `git diff 04442962..HEAD` on `work/f-229-xai`, HEAD `2f1fd11b7c2e831923ee3d1ceb10908d98f874fc` (cf75181d, the implementation round, and 2f1fd11b, its self-review round). Index tree `66fcbb10c8d78efbd23ec4e7c0a8f78f72f65768`. No unstaged tracked changes. One untracked path, `tools/oracle/node_modules`, was present before this pass and is not reviewed material.
**Author / reviewer**: GLM in opencode wrote both commits. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of the self-review notes.
**Result**: 6 defects, 1 smell, 1 nitpick

## Acceptance challenged

The lead amendments for the viewer and documentation half, at the end of `.claude/plans/F-229-design.md`.

- **Measured numbers.** 19 known, 17 available, 2 unavailable (Deflated and `.111`), six registration functions. Read from `crates/ocelli-codec/tests/registry.rs` lines 430 to 434 and `KNOWN_TRANSFER_SYNTAXES`, and the registry suite run now. Every count in the touched documents agrees with those numbers. The wrong behaviour this rejects is a document still saying sixteen or fifteen.
- **The viewer registers JPEG XL.** I derived the expectation independently from the amendment: drop the registration and the monochrome draw must go red. It does. I then asked a question the amendment does not ask. Which syntaxes does the viewer actually open? The answer comes from running `preview_first_frame` on all 24 `syntax/` corpus rows. That is how D1 was found. Deflated opens, through the parser, with no registered decoder.
- **README, use.html section 8 and more.html say the same measured facts.** The counts agree. The README sentence about what the viewer opens does not (D1). The more.html status mark disagrees with the page's own mark definitions (D2). One use.html evidence sentence overclaims (D3).
- **No `.111` decode path described.** I grepped the whole tree's user documents, LLDs, the skill, `unsupported.json` and DEVIATIONS for `.111`, `Recompression` and `reconstruct`. Every hit names the refusal or the route that is not built.
- **Adapters regenerated.** `sync_agent_skills.py --check` passes, and the canonical hash equals the `.agents` adapter's recorded SHA-256.
- **AC20's wasm half.** Both builds are within budget and `ci/wasm-size-budget.json` is byte-identical before and after, and across the range.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Scratch copy (rsync of the worktree minus target, node_modules, .git, corpus/data), `CARGO_TARGET_DIR=target/f229-pass12-mut`, `cargo test -p ocelli-folder-viewer --test preview jpegxl` with `register_jpegxl_decoders` removed from `codecs()` | HEAD sources, mutated in the copy | now | exit 101. `a_jpegxl_lossless_frame_draws` FAILED, "the frame could not be decoded". The colour test stays green, which its comment predicts because the refusal comes before decode |
| Same copy, mutation reverted, `cmp` against the worktree file, full `--test preview` | HEAD | now | identical. 79 passed |
| Same copy, Samples per Pixel gate removed in `frame_parts` | HEAD, mutated | now | `a_jpegxl_colour_frame_refuses_not_monochrome` FAILED with "a three-sample MONOCHROME2 frame was refused as the frame could not be decoded". 78 passed |
| Same copy, `photometric()` maps `RGB` to `Monochrome2` | HEAD, mutated | now | `a_single_sample_colour_photometric_refuses_not_monochrome` FAILED. 78 passed |
| Same copy, `photometric()` maps `RGB` to `Rgb` | HEAD, mutated | now | 79 passed. `SampleLayout::new(1, Rgb)` still refuses `NotMonochrome`, so the outcome stays guarded. Not a finding, see Verified clean |
| Same copy, a throwaway probe test calling `preview_first_frame` on each of the 24 files in `corpus/data/syntax/` | corpus manifest SHA-256 `5207ff99...` | now | 18 open at 96 by 64, including `deflated_explicit_vr_le.dcm`. 6 refuse "monochrome frames only", all six colour rows |
| `cargo test -p ocelli-codec --test registry` in the same copy | HEAD | now | 18 passed |
| `bin/ocelli.sh gate wasm`, with `shasum -a 256` of the budget file before and after | HEAD worktree | now | exit 0. wasm 2,179,571 bytes against baseline 2,151,196 and ceiling 2,258,755. decode wasm 16,455 against baseline 16,455 and ceiling 17,277. Budget hash unchanged, `git diff 04442962..HEAD -- ci/wasm-size-budget.json` empty |
| `python3 scripts/sync_agent_skills.py --check` | HEAD | now | exit 0, "42 Codex and 19 OpenCode adapter files match their 21 canonical sources" |
| `uv run --frozen python -B -m unittest scripts.tests.test_corpus_synth -k jpegxl -k lossless_syntaxes -k lossy_syntaxes` | HEAD, corpus present | now | 4 tests OK |
| Raw item walk of the five JPEG XL corpus files | `corpus/data/syntax/jpegxl_*.dcm` | now | `jpegxl_lossless` and `jpegxl_recompression_rgb8` start with the 12-byte container signature, the other three with `FF 0A`. One BOT item plus one fragment each |
| `tools/oracle/out/run.json` and the 102 sidecars | run of 2026-11-06 20:25, `manifestSha256` equals the current manifest | prior run, not reproduced (no browser job) | 100 rows, 93 stack, 7 unsupported, readBack 93, 91 magnified and 2 fitted down, 86 VOI from file and 7 none, all 7 colour, 16 low-information stack rows |
| `cargo tree -p ocelli-wasm`, `-p ocelli-native`, decode-worker wasm32 tree | HEAD | now | no `ocelli-codec` and no `jxl-*` in any. `alloc-stdlib` is in `ocelli-codec`'s tree |
| Cited dependency lines in `~/.cargo/registry` | `jxl-oxide` 0.12.6, `jxl-jbr` 0.2.1 | now | `aux_box.rs:72` is the unbounded `DecompressorWriter::new(Vec::new())`, `lib.rs:59` the same for `jbrd`, and `reconstruct.rs:457` is `expect("is_last not found")` in the `0xc4` arm |
| RSS figures 1,095,647,232, 270,811,136 and 2,260,992. Wasm cost 940,335 and 75 bytes. Numerator 12 and 13 | module docs and the LLD | prior receipts, not reproduced | consistent with the source docs in number. D5 is about the condition one of them was measured under |

Scratch copy and `target/f229-pass12-mut` were deleted with Python `shutil.rmtree` after the pass.

## Defects

### D1, README says the viewer opens seventeen syntaxes, and it opens eighteen
**Where**: `README.md:181-184`
**What**: "The preview registers all six decoder families of `ocelli-codec`, so it opens the monochrome frames of the seventeen of the nineteen known transfer syntaxes that have a registered decoder". A reader takes this to mean the viewer opens monochrome frames for those seventeen syntaxes and not the other two. In fact the viewer also opens Deflated Explicit VR Little Endian. `parse_part10` inflates the data set, and `preview.rs` lines 1328 and 1346 route `DispatchPath::DeflatedExplicitVrLittleEndian` to `RawDecoder::explicit_vr_little_endian()`, never touching the registry. Only `.111` is refused for every frame.
**Why it is wrong**: the lead amendment asks README to name "what the viewer opens". The microscope rule makes a false behaviour sentence in README a defect.
**Evidence**: the probe over the 24 `syntax/` rows. `deflated_explicit_vr_le.dcm => ok 96x64`.
**Scope and regression**: present on HEAD. Affects README readers deciding whether a deflated study will open. The smallest fix states what the viewer opens without tying it to registration, for example monochrome frames of every known syntax except JPEG XL JPEG Recompression. A regression would be one viewer test that previews a deflated synthetic frame, if the lead wants the sentence guarded.

### D2, more.html marks JPEG XL "implemented" while naming a syntax that is not built
**Where**: `examples/folder-viewer/src/more.html:138` (index mark) and `:393` (status span)
**What**: the mark is `implemented` and the span is `status done`. The section itself says JPEG XL JPEG Recompression has no decoder because of jxl-oxide issue 512. `docs/lld/codecs.md` says "A released dependency that fixes the defect turns the syntax on in its own story."
**Why it is wrong**: the page defines its marks at line 131. "**Partial** means part of it exists and the rest is named." Two of three JPEG XL syntaxes decode, and the third is named as a later story. That is the page's definition of Partial, the same shape as Interpolation at line 177 ("In-plane nearest and linear are built. Blending two stack slices is not."). Implemented overstates it.
**Evidence**: read of lines 131, 138, 177 and 391-395, against the `.111` refusal asserted in `registry.rs`.
**Scope and regression**: present on HEAD. Affects anyone reading the More page as a capability list. The fix is `partial`, with a status line naming `.110` and `.112` as built and `.111` as not.

### D3, use.html says registry.rs asserts claims it does not
**Where**: `examples/folder-viewer/src/use.html:588`, last sentence
**What**: before this diff the sentence was "`crates/ocelli-codec/tests/registry.rs` asserts both halves", meaning the fifteen available and Deflated unavailable. It is now "asserts all of this", and "this" now includes "`jxl-jbr` 0.2.1 panics in the DHT branch of JPEG reconstruction" and that Deflated "is handled by `parse_part10`". `registry.rs` asserts the partition and that `.111` has no decoder. The panic and the `parse_part10` route appear only in comments there and are asserted nowhere in that file.
**Why it is wrong**: a sentence naming a test as the evidence for a claim the test does not execute is a false claim about evidence (microscope section 3).
**Evidence**: read of `registry.rs` lines 425 to 499. The only `.111` assertions are `KnownUnavailable` and the error string. Deflated is asserted only as `KnownUnavailable`.
**Scope and regression**: present on HEAD. The smallest fix restores the bounded form, for example "asserts the nineteen, the seventeen and which two are unavailable".

### D4, codecs.md says the extra channel vector is never filled, and the pre-check's own parse fills it
**Where**: `docs/lld/codecs.md:425-429`
**What**: "Two bounds sit outside the tracker by measurement and are recorded on the constants: ... the extra channel vector the header parser grows for a header that declares them ... Extra channels refuse in the pre-check, so the vector is never filled."
**Why it is wrong**: the pre-check refuses extra channels after `<ImageHeader as Bundle<()>>::parse` returns (`jpegxl_icc.rs`, `probe`). That parse is what grows the vector. The module documentation of `crates/ocelli-codec/src/jpegxl_icc.rs` records this. "`jxl-image` 0.13.0 grows the extra channel vector and each channel name while it parses ... a 540 byte header declaring 4096 channels measured a 262144 byte block from it before the dependency refused". `extra_channel_header_refuses_before_the_reparse` is named for exactly this ordering. The refusal prevents the `try_init` reparse, not the fill. Second part: "recorded on the constants" is also false. Neither `ALLOC_LIMIT_NUMERATOR`, `FIRST_FEED_BYTES` nor `FIRST_PROBE_BYTES` documents either bound. They are recorded in the two module docs.
**Evidence**: read of `jpegxl.rs` 46-63 and `jpegxl_icc.rs` module docs and `probe`.
**Scope and regression**: present on HEAD. This is an allocation-bound claim in the LLD that a later reviewer of D-21 would rely on. The fix replaces the last sentence with what the module doc says (the parse grows it, bounded by the format's limits, and the refusal stops the reparse) and says "recorded in the module documentation".

### D5, codecs.md attributes the 2,260,992 byte measurement to the wrong probe shape
**Where**: `docs/lld/codecs.md:344-346`
**What**: "a probe that parsed the header and stopped at the ICC flag peaked at 2,260,992 bytes".
**Why it is wrong**: the source of the number, `jpegxl_icc.rs` module documentation, says the measured probe "parsed the header and then read the encoded ICC size behind the flag", and then says "The probe here does the parse and stops at the flag." The LLD moves the shipped probe's shape onto the measured probe. The number is true. The condition it was measured under is not the one stated.
**Evidence**: side-by-side read of the two passages.
**Scope and regression**: present on HEAD. It is low impact, but the measurement claim is false. The fix copies the module doc's wording.

### D6, oracle.md now says F-011 started against ninety-three stack frames
**Where**: `docs/lld/oracle.md:1143-1145`
**What**: "`renderParamsSha256` not moving is mechanical evidence that the ninety-three stack frames are the same artefact F-011 started against".
**Why it is wrong**: this is a historical sentence in the F-X007 section. It was written in 2c80a7a7 (F-X012, 2026-09-06) when there were ninety stack frames. Rows were added after F-011. For example, `syntax/jpegls_lossless_rgb8.dcm` first appears in b5457b6c (F-030, 2026-09-14). So F-011 did not start against ninety-three. The other count edits in this file are present-tense corpus claims and are correct against the run (lines 281, 385, 761, 1106 and 1153). This one changed a true historical number into a false one. Pass 10's N1 listed this line among the stale counts, which is probably why it was edited.
**Evidence**: `git log -S` on the sentence and on the row, and the sidecar count from `tools/oracle/out`.
**Scope and regression**: present on HEAD. The fix restores "ninety" here, or drops the count ("the stack frames").

## Smells

### S1, the JPEG XL entry is the only one of fifteen More features without a tier line
**Where**: `examples/folder-viewer/src/more.html:391-395`
**What**: the page lede at line 131 says "A tier line says what tiers A (WebGPU), B (WebGL2) and C (CPU) do." Fourteen of the fifteen `section.feature` blocks carry a `p.tiers`, and the non-rendering ones say so (Documents: "do not draw, so tiers do not apply"). The new block has none.
**Mechanism**: CLAUDE.md says "not applicable" is a legitimate answer and an omitted row is not. The next entry modelled on this one inherits the omission, and the lede's promise becomes false for the page. The answer is real and short. Decode is CPU on every tier and `docs/lld/codecs.md` says the tiers select no different registry behaviour.

## Nitpicks

- **N1**: `docs/lld/codecs.md:387`, "More than one keyframe". `jpegxl.rs` refuses `num_loaded_keyframes() != 1`, so the row is "Other than one keyframe".

## Pre-existing, not counted, for the lead

- `tools/oracle/unsupported.json` `$notes[3]`, the sentence this diff edited, still says A1 and A2 "ask about Ocelli's own wasm codec build, which does not exist yet". A1 and A2 are resolved (`docs/spikes/`), and `ocelli-codec` builds for wasm32. The lead amendment scoped this file to counts, so this pass does not count it. A one-clause deletion fixes it.
- Self-review cycle 2 named a gap in the viewer tests. Removing any other `register_*` line from `codecs()` leaves all 79 viewer tests green. I did not re-run it. It predates this round.

## Verified clean

- `registry.rs` counts 19, 17 and 2, and the unavailable pair is in order. Every count sentence in README, use.html, more.html, codecs.md, the skill and `unsupported.json` matches.
- use.html section 8. The code block's six imports and calls exist. The table has 19 UIDs and the `.111` row is "None, unavailable".
- more.html paragraph. Six functions in the stated grouping. `jxl-oxide` 0.12.6 is pinned exactly with default features off, and `jxl-render` 0.12.4 too. cornerstone3D has no decoder for the three JPEG XL syntaxes, per `unsupported.json` entries and run.json's 7 unsupported rows. The round-trip clause matches `test_lossless_syntaxes_round_trip_exactly` and the worst-pixel test with its XYB non-zero lower bound.
- codecs.md. The `.111` section, including that no Ocelli path calls `jxl-jbr`. The pre-check paragraph's five container refusals and the codestream-only hand-off. Every refusal-table row was read against `validate_descriptor`, `validate_header`, `decode_limited`, `stored_integer` and `refuse_trailing`. The quantiser formula and its rounding, which is half away from zero on a non-negative operand and correct. `FrameDesc::new` accepts only 1, 8, 16 and 32. Interleaved and `Rgb` or `Preserved` match the trait methods. Neither shipped module reaches `ocelli-codec`. `alloc-stdlib` is in the closure.
- corpus.md. The five rows and their sources match `encode_jpegxl_cases` and `SYNTAX_REFERENCE`. Distance is 1.0. The container framing was observed on the files. The tests named exist and pass. `BUILT_WITH` pins imagecodecs 2026.10.10 and libjxl 0.12.0.
- pixel-pipeline.md and frame-open.md. `Xyb` maps to `ColourSelection::Colour` and expects three samples. `ColorTransform::resolve` refuses it `CodecOwnedColorTransform` under `Preserved` and accepts it under `Rgb`.
- The skill table's library column matches `ocelli-codec`'s dependencies and D-18, D-20, D-22 and D-28. The Table 8.2.15-1 ranges match the plan's transcription of PS3.5. 13 times 65,535 over 4,095 is 208.05.
- oracle.md present-tense counts. 93, 91 and 2, 86 and 7, the 16 low-information rows, and the syntax/ breakdown of 15, 3 and 6 out of 24 with tags read from the manifest. readBack is 93. Against a run whose manifest hash is current.
- The viewer test fixture claims. 87 bytes. A 64 by 96 ramp with 0 first and 4095 last. Full range maps those to 0 and 255.
- The third viewer test. It goes red when RGB is silently treated as greyscale, which is the dangerous widening. A widening that keeps RGB as RGB is still refused by `SampleLayout::new`, so the behaviour is guarded either way. Its comment is true of the current tree.
- No em-dash or en-dash was introduced in the reviewed prose. This file uses neither and has no prose semicolon.

## Limitations and handoff

- No oracle, GPU or browser job. The oracle counts rest on the 2026-11-06 run, citable because its manifest hash is current. The RSS, wasm-cost and numerator figures are prior receipts, not reproduced.
- I did not run the whole codec suite or `malformed_jpegxl_never_panics`. Both are outside the documentation half.
- This pass is not clean. Six defects and one smell block. The next owner is the implementer for D1 to D6 and S1. After that, a fresh independent pass over the changed lines.

## Self-review catch figure (for the implementer comparison)

Notes read: `.claude/scratch/F-229-self-review-1.md` to `-3.md`.

| Finding | In reach of the self-review? | Basis |
|---------|------------------------------|-------|
| D1 | yes | Cycles 1 to 3 read `preview.rs` and mutated its gates, and executed the README counts, but never asked what the viewer opens |
| D2 | yes | Cycle 3 checked this exact mark and concluded it "match[es] the page's conventions" |
| D3 | yes | Cycle 1 executed the 19, 17 and 2 claim but not the sentence naming its evidence |
| D4 | yes | Cycles 1 and 3 say the allocation section was read against `jpegxl.rs` and `jpegxl_icc.rs`, whose module doc contradicts it |
| D5 | yes | Cycle 1 checked the 2,260,992 figure against the same module doc and missed the condition beside it |
| D6 | partly | Cycle 3 says "Every oracle.md sentence now states these", but the edit followed pass 10 N1's list, which named this line |
| S1 | yes | Same mark check as D2 |
| N1 | yes | Cycle 1 read every refusal-table row against the code |

Blocking findings: 6 of 7 in clear reach (D1, D2, D3, D4, D5, S1), and 7 of 7 if D6 counts even though a reviewer's list steered the edit. Including the nitpick: 7 of 8, or 8 of 8 with D6. For scale, cycle 1 labelled D1 to D9 (its header says 8 defects and 2 nitpicks, its labels say 6 defects and 3 nitpicks), cycle 2 found D10, and cycle 3 found none.
