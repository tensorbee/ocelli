# F-229 review, pass 13

**Reviewed**: `git diff b5917e66..c6041538` on `work/f-229-xai`, HEAD `c60415380310317567e312306a790a6f96aacf90` (the fix round after pass 12, with its self-review notes). Index tree `17da4e5de35d25bfda288c0662d1dd4403779604`. No unstaged tracked changes. One untracked path, `tools/oracle/node_modules`, was there before this pass and is not reviewed material.
**Author / reviewer**: GLM in opencode wrote c6041538 and the three self-review notes. This pass is an independent Claude reviewer. I wrote none of the reviewed material and none of the self-review notes. I did not write pass 12 either, and one finding below goes back to a remedy pass 12 suggested.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 12 replay.** I checked each of D1 to D6, S1, N1 and the `$notes` sentence by running the corrected sentence against the tree, not by reading the diff. Seven are closed. D6 is not. The restored number came from pass 12's own suggested remedy ("restores ninety"), and the history shows ninety was itself a later edit (D1 below).
- **What the viewer opens.** I derived this on my own, without the README. Every one of the 100 manifest rows went through `preview_first_frame`, grouped by transfer syntax. Two syntaxes have only a colour corpus row, `.50` and `.111`, so I also built a monochrome frame for each. For `.50` I used a PIL baseline JPEG, 16 by 8, and it opens. For `.111` I put the same bytes under that UID and it is refused. So the README's eighteen syntaxes are confirmed one by one, not only through the corpus. This rules out a README that is right only for the corpus rows that happen to exist.
- **The new viewer test as a guard.** I dropped each of the six `register_*` calls from `codecs()` in turn. Each drop replaced the call with `let _ = register_x_decoders;`, so an unused import could not make the build fail and look like a guard.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Scratch copy (rsync of the worktree minus target, node_modules, .git) in the session scratchpad `f229-p13`, own `CARGO_TARGET_DIR`. `cargo test -p ocelli-folder-viewer --test preview` | HEAD c6041538 | now | 80 passed |
| Same copy, each of the six registrations replaced in turn, full `--test preview` per mutation, file restored with `cp` and `cmp` after | HEAD, mutated | now | Six for six red. native_and_rle: "the RLE Lossless frame was refused". jpeg: "the JPEG lossless frame". jpeg2000: "the JPEG 2000 frame". jpegls: "the JPEG-LS frame". htj2k: "the HTJ2K frame". Each 79 passed and 1 failed. jpegxl: 78 passed, 2 failed (the new test and `a_jpegxl_lossless_frame_draws`) |
| Throwaway probe over every manifest row through `preview_first_frame` | `corpus/manifest.tsv`, 100 rows | now | All 19 known UIDs are present. Every monochrome row opens except `synthetic/ct_signed_12in16_left.dcm` (category `legacy-nonconforming`, "an image attribute is missing or not usable"). Every colour row is refused "monochrome frames only". `.1.2.1.99` and `.1.2.2` open |
| Throwaway test: a PIL baseline monochrome JPEG wrapped by `encapsulated_frame` under `.50`, `.51` and `.111` | `uv run --with pillow`, 16 by 8, quality 95 | now | `.50` opens (16, 8, 128 samples). `.111` is refused "the frame could not be decoded". `.51` is refused too. That is not a claim of this diff, see Limitations |
| `python3` read of `jpegls_corpus_reference_u16le.raw` | tracked fixture | now | 12288 bytes, 6144 samples, first 0, last 65535, min 0, max 65535 |
| Read of `monochrome_12bit_desc` (`tests/jpeg.rs`), `corpus_desc` (`tests/jpegls.rs`), and the JPEG 2000 `u12` case | tracked tests | now | 1 by 4 at 12 bits, 64 by 96 at 16 bits, 64 by 64 repeating 0, 2047, 2048, 4095. These match the viewer test's descriptors and the claims its comments make about the codec suite |
| `git show "<rev>:corpus/manifest.tsv"` and `unsupported.json` at 2ffabe37 (F-011), b878c9c8 (F-X007), 2c80a7a7 (F-X012) | history | now | 91 rows and 2 unsupported, then 91 and 2, then 92 and 2. `git show 2ffabe37:docs/lld/oracle.md` says "eighty-nine rendered rows" at lines 246, 329 and 645 |
| `git log -G 'stack frames are the same'` on `docs/lld/oracle.md` | history | now | Born in b878c9c8 as "eighty-nine". Changed to "ninety" in 2c80a7a7. Changed to "ninety-three" in cf75181d. Changed back to "ninety" in c6041538 |
| `cargo check -p ocelli-codec --target wasm32-unknown-unknown` in the copy | HEAD | now | exit 0, with warnings |
| `cargo fmt --all --check`, `cargo clippy -p ocelli-folder-viewer --tests -- -D warnings` in the copy | HEAD | now | both exit 0 |
| `python3 scripts/prose_check.py` | HEAD worktree | now | exit 0 |
| `diff -rq` worktree against the copy after the probes were removed | HEAD | now | no difference |

The scratch copy and its target directory were deleted with Python `shutil.rmtree` and confirmed absent.

## Defects

### D1, oracle.md still says F-011 started against ninety stack frames, and it started against eighty-nine
**Where**: `docs/lld/oracle.md:1143-1145`
**What**: "`renderParamsSha256` not moving is mechanical evidence that the ninety stack frames are the same artefact F-011 started against".
**Why it is wrong**: F-011's own commit, 2ffabe37, had 91 manifest rows and 2 `unsupported.json` entries, which leaves 89 stack frames. The LLD at that commit says "eighty-nine rendered rows" three times. The sentence was born in b878c9c8 (F-X007) as "eighty-nine". F-X012, 2c80a7a7, added `synthetic/ct_sigmoid_width_half.dcm` and moved the number to ninety. That was the first time this historical sentence went false. Pass 12 D6 was right that ninety-three is false, but its suggested remedy ("restores ninety here") was wrong as well, and this round followed it. Self-review cycle 1 checked only `git show 2c80a7a7`, which is the commit that made the sentence false.
**Evidence**: the history rows in the Evidence table.
**Scope and regression**: present on HEAD. A reader of the F-X007 section gets a false fact about what F-011 compared. The smallest fix drops the count ("the stack frames are the same artefact F-011 started against"), so the sentence cannot go stale when the next row lands. Writing "eighty-nine" would also be true, but it is a number in prose and this file keeps having to correct those. This is the third pass to touch this sentence (pass 10 N1, pass 12 D6, and now this one), so it is worth telling the operator.

## Smells

None.

## Nitpicks

- **N1**: `examples/folder-viewer/tests/preview.rs`, the RLE comment in `every_registration_in_codecs_decodes_a_monochrome_frame` cites "PS3.5 G.2 and G.3.1" for "a 64 byte header naming one segment at offset 64". As I remember it, the header layout is G.5 (RLE Header Format), G.2 is byte segments, and G.3 is the algorithm with its literal run rule. `crates/ocelli-codec/tests/rle.rs:74` cites "G.2 through G.5" for the same shape. I have no local copy of PS3.5 Annex G, so this is unconfirmed. The bytes themselves are right: 1 segment, offset 64, run byte 3 copying four literals, one zero pad, 70 bytes in total. The expected greys 0, 85, 170 and 255 are exact for full range over 10 to 40 under any rounding, because 255 over 3 is 85.

## Verified clean

- **Pass 12 D1 (README).** Closed. "It opens the monochrome frames of eighteen of the nineteen known transfer syntaxes" holds UID by UID (Acceptance challenged). The deflated route is `parse_part10` with `flate2::read::DeflateDecoder` (`ocelli-dicom/src/parse.rs:298`), and `native_decoder` maps `DeflatedExplicitVrLittleEndian` to `RawDecoder::explicit_vr_little_endian()`. `.111` is refused for every frame, monochrome at decode and colour at the sample gate.
- **Pass 12 D2 (More mark).** Closed. The index mark is `partial` and the span is `status part`, following the pairing on the page's other 14 entries. The span's naming of `.110` and `.112` as decoding and `.111` as not matches `registry.rs`.
- **Pass 12 D3 (use.html).** Closed. `registry.rs` asserts `KNOWN_SYNTAX_COUNT` 19, `AVAILABLE_SYNTAX_COUNT` 17, `UNAVAILABLE_SYNTAX_COUNT` 2 and `unavailable == [DEFLATED, JPEGXL_RECOMPRESSION]`, which is exactly the sentence's bounded claim.
- **Pass 12 D4 (codecs.md bounds).** Closed. `probe` in `jpegxl_icc.rs` calls `<ImageHeader as Bundle<()>>::parse` and then tests `ec_info`, so the refusal comes after the growth and before `try_init`. Both bounds are in the `jpegxl_icc.rs` module documentation (lines 44 to 56). No stale copy of "never filled" or "recorded on the constants" is left anywhere in the tree outside reviews.
- **Pass 12 D5 (2,260,992).** Closed. The wording now matches `jpegxl_icc.rs` lines 39 to 41.
- **Pass 12 S1 (tier line).** Closed. 15 of the 15 feature sections have a `p.tiers` line. "Tiers A, B and C decode on the CPU and do not select different registry behaviour" matches `docs/lld/codecs.md` "Worker and tier boundary". `ocelli-codec` has no `wgpu`.
- **Pass 12 N1.** Closed. `jpegxl.rs:241` refuses `num_loaded_keyframes() != 1` with `FrameMismatch`.
- **`unsupported.json` `$notes[3]`.** Closed. `docs/spikes/GATES.md` lines 24 and 25 record A1 (`Fail`) and A2 (`Pure Rust`) as answered, and the wasm32 check of `ocelli-codec` succeeds. The only code that reads the file's hash is `run.mjs:1642`, which writes `unsupportedSha256` into a new run. Nothing compares a committed hash, so the edit breaks no gate.
- **The new viewer test.** It goes red on each of the six dropped registrations, each with its own named reason, which is what the request asked for. Its comments about the codec suite are true: the JPEG lossless 1 by 4 at 0, 2047, 2048, 4095 cited to PS3.5 Annex F by `tests/jpeg.rs`, the JPEG 2000 64 by 64 repeating boundaries, the JPEG-LS and HTJ2K byte-for-byte ramp, and the JPEG XL file shared with `a_jpegxl_lossless_frame_draws`. The comment says a dropped call leaves the UIDs "known but unavailable" and the file refused `PreviewError::Decode`, and the observed message is that variant's display text. The `encapsulated_frame` refactor keeps `jpegxl_frame` at 64 by 96, and all 80 tests are green.
- **Other sentences the round added.** I found no other new sentence. The status span lost "in `ocelli-codec`", which removes a claim and adds none. No em-dash, en-dash or prose semicolon is in the added prose, and this file has none either.

## Limitations and handoff

- No oracle, GPU or browser job. The 2026-11-06 oracle run's `unsupportedSha256` no longer matches the edited `unsupported.json`. Nothing consumes that field as a freshness check, but a later citation of that run should not describe it as matching the current file.
- `.51` refuses a baseline (SOF0) stream in the viewer. The corpus `.51` row opens, and no sentence in this diff claims a SOF0 stream under `.51`, so I did not count this or investigate it further.
- `synthetic/ct_signed_12in16_left.dcm` is monochrome and refused for a nonconforming attribute. That is a property of the file, not of its syntax, and it existed before this round.
- I did not run the codec suite, the registry suite or the wasm size gate. This round did not touch them.
- This pass is not clean. One defect blocks. The next owner is the implementer, for D1. After that, a fresh independent pass is needed over the one changed line.

## Self-review catch figure (for the implementer comparison)

Notes read: `.claude/scratch/F-229-self-review-1.md` to `-3.md`, the notes for this fix round.

| Finding | In reach of the self-review? | Basis |
|---------|------------------------------|-------|
| D1 | yes | Cycle 1 did go to git history for this exact sentence, but it stopped at 2c80a7a7. `git log -G` on the sentence, or the manifest at F-011's commit, gives eighty-nine in one command. The remedy came from pass 12, which reduces but does not remove the miss |
| N1 | yes | The implementer wrote the comment, and `tests/rle.rs:74` cites G.5 for the same shape |

Blocking findings: 1 of 1 in reach. With the nitpick: 2 of 2. For scale, the self-review recorded three findings in cycle 1 and none in cycles 2 and 3. All of the pass 12 replays it claimed were confirmed here, and so were its six registration drops.
