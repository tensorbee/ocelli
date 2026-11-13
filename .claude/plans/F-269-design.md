# F-269, the vendored ritk-codecs JPEG-LS comments audited against ITU-T T.87

**Status**: approved
**Epic ref**: E4.14
**Sprint**: S47
**Estimate**: 1w

Filed at the S46 close from the recovered close-backlog draft, original item
5, the sprint-close audit story F-243's lead ruling names. The estimate is
inherited and is provisional, lead-suggested and not capacity-approved. This
draft claims no implementation, no review cleanliness and no completion.
Refined in the S47 design round, on this worktree at
`16c782d17f673b1af14fb09e906e3f29f16fdb02`, branch `work/f-269-glm`.

## Normative source, transcribed

ITU-T T.87 (06/1998) is the audit's standard. The clauses the audit checks
comments against, as verified by F-243 working pass 8 at its tip and re-checked
from the official ITU PDF in the S47 preparation packet
(`DESIGN-INPUT.md`, PDF SHA-256 `4ca6f9816fb024b954fc811c8c6821814b45beb1690dff32ed1e8aef609a8778`):

- **A.2.1 step 2** prescribes the initial state `N = 1`, `B = C = 0`,
  `A = max(2, floor((RANGE + 32) / 64))`. RANGE is set at step 1, `MAXVAL + 1`
  for lossless and `floor((MAXVAL + 2·NEAR) / (2·NEAR + 1)) + 1` for NEAR > 0.
- **A.5.1, code segment A.10** selects k as the smallest k with
  `N·2^k >= A`, by a shift comparison. That segment states no division, so a
  "to avoid division by zero" reason does not match it.
- **A.3.1, code segment A.1** defines `D1 = Rd - Rb`, `D2 = Rb - Rc`,
  `D3 = Rc - Ra` and names them the local gradients. The standard gives no
  orientation adjectives, which makes the old names doubtful without proving
  them false.
- **C.2.4.1.1.1** gives the default thresholds in two branches, with
  `BASIC = (3, 7, 21)` from Table C.3 and the clamping function of Figure C.3
  over integers, `CLAMP(i, j) = j` when `i > MAXVAL` or `i < j`, otherwise
  `i`, with MAXVAL ambient and not an argument.
  For `MAXVAL >= 128`, Figure C.4:
  `FACTOR = floor((min(MAXVAL, 4095) + 128) / 256)`,
  `T1 = CLAMP(FACTOR * (BASIC_T1 - 2) + 2 + 3*NEAR, NEAR + 1)`,
  `T2 = CLAMP(FACTOR * (BASIC_T2 - 3) + 3 + 5*NEAR, T1)`,
  `T3 = CLAMP(FACTOR * (BASIC_T3 - 4) + 4 + 7*NEAR, T2)`.
  For `MAXVAL < 128`, Figure C.5: `FACTOR = floor(256 / (MAXVAL + 1))`,
  `T1 = CLAMP(max(2, floor(BASIC_T1 / FACTOR) + 3*NEAR), NEAR + 1)`,
  `T2 = CLAMP(max(3, floor(BASIC_T2 / FACTOR) + 5*NEAR), T1)`,
  `T3 = CLAMP(max(4, floor(BASIC_T3 / FACTOR) + 7*NEAR), T2)`.
  Independent arithmetic control: at `MAXVAL = 127`, `NEAR = 0`, the low
  branch gives `FACTOR = 2` and thresholds `(2, 3, 10)`. Generalising the
  high branch down instead gives `(2, 3, 4)`, which is the wrong
  implementation the two-branch transcription rejects. This is a source and
  arithmetic inspection control, not an executed codec test.
- **A.4.5, code segment A.9** maps every coded error into
  `[-floor(RANGE/2), ceil(RANGE/2) - 1]` before coding.

The audit uses T.87 only and reads no CharLS source, as F-243's own reviews
did. `docs/hld/DEVIATIONS.md` D-20 records the ritk-codecs vendoring and its
pin record, which the story refreshes through the existing mechanism.
`docs/hld/18-codec-registry.md` section 21 fixes the `Decoder` trait, which
the story does not touch, because the audit changes no decode.

## What the specification does not cover

Comment wording. T.87 says nothing about comments, so every correction is
against the code below the comment and the clause the comment names. A
doubtful comment is kept doubtful rather than rewritten as certain, on pass
8's own rating. The standard also says nothing about which file records a
patch, which is why the write set below names the existing record files
explicitly.

## Problem and evidence

F-243 working pass 8, "Audit backlog, not blocking", lists four false or
doubtful upstream comments on lines F-243 did not write or rewrite. Quoted
with this tree's positions, read at `31323a79` for the source files and
re-verified against this tree's staged snapshot:

| # | Comment | Here | F-243 tip | Why it is wrong or doubtful |
|---|---------|------|-----------|-----------------------------|
| 1 | "context model for JPEG-LS lossless decoding" | `context/mod.rs:1` | `:1` | The module also serves near-lossless, since `CodingParams::new` takes NEAR, and the encoder |
| 2 | "`n` ... initialized to 1 to avoid division by zero" | `context/mod.rs:15` | `:15` | T.87 A.2.1 step 2 initialises N to 1, and A.10 has no division, so the reason does not match the code |
| 3 | "DICOM data is ≥ 8-bit so the high branch is the production path" | `context/mod.rs:296` | `:296` | `decoder.rs:82` accepts P from 2, and Bits Stored below 8 gives MAXVAL below 128, which takes the low branch |
| 4 | D1, D2 and D3 named vertical, horizontal and diagonal | `scan.rs:164` to `:166` | `:164` to `:166` | A.3.1 names them only local gradients. Doubtful rather than plainly false, since "vertical gradient" is sometimes used for the response to a vertical edge |

All paths are under `vendor/ritk-codecs-0.6.0/src/jpeg_ls/`. The named
comments have the same positions at this staged source snapshot and F-243 tip
`fd11a482`. This records source content, not commit ancestry for a tree
object. The story re-locates each comment by content at its own tip. The
primary source is the preserved official T.87 transcription at
`/private/tmp/ocelli-s46-codex/f269-current-clause-inventory/t87.txt`,
lines 2194 to 2245, whose text extraction drops floor glyphs, so the
transcription above normalises the floors explicitly rather than copying
the extraction's missing glyphs.

### The cap on k

Pass 8's verdict proves and measures that the decoder's `k < qbpp` cap never
binds at the F-243 tip, and shows that the proof rests on `in_coded_range`,
which refuses before the A update. With that refusal disabled the cap bound
about two million times on damaged streams. Pass 8 records that the dependence
"is not written anywhere", worth a line if the cap is ever discussed again.
The story writes that line beside the cap and beside `in_coded_range`. In this
tree `in_coded_range` is defined at `scan.rs:302` to `:304` and called at
`:230` and `:272`, matching pass 8's citations.

**Corrected, caller-specific statement the plan settles.** There are two
shared k helpers, `RunInterruptionContext::compute_k` at `context/mod.rs:116`
and the free function `compute_k(a, n, qbpp)` at `context/mod.rs:214`, and
they take their premises from different callers:

- **Decoder** (`scan.rs:227` and `:259`): k is computed before any update.
  The `k < qbpp` bound never binds because the state premise holds, which
  rests on `in_coded_range` (`scan.rs:230` and `:272`) refusing a decoded
  error outside `[-floor(RANGE/2), ceil(RANGE/2) - 1]` **before** the A and
  N updates at `:233` and `:282`. Without that refusal the cap bound about two
  million times on damaged streams, measured by pass 8 and not re-proven
  here. The line states this dependence and does not claim a fresh universal
  proof.
- **Encoder** (`encoder.rs:213` and `:249`): there is no refusal. The
  encoder's own premise is `modulo_reduce` (`context/mod.rs:365`), applied at
  `encoder.rs:218` and `:256` **before** the updates at `:233` and `:265`,
  so every error entering the state update is already in the canonical range
  and the same bound holds by the same A.4.5 step. `in_coded_range` is not
  shared by the encoder and no shared refusal exists. This is a source
  inspection record with the initial and reset conditions stated, kept
  distinct from pass 8's damaged-stream **decoder** sweep, which measured
  decode only. No fresh measurement or universal proof for either side.

Both helpers share the remaining premises: initial `A` and `N` from T.87
A.2.1 step 2 via `CodingParams::new` (`context/mod.rs:345` to `:356`,
`a_init = max(2, floor((RANGE + 32) / 64))`, `N = 1`), `RANGE` and `qbpp =
ceil(log2 RANGE)` from the same constructor, and `RESET = 64`
(`context/mod.rs:48`). The new line states the caller each premise belongs to
rather than a single shared story. Historical pass 8 proof and sweep are not
rerun by this story.

## Scope, API and compatibility

In scope: the four comments above, the rest of the factual comments under
`vendor/ritk-codecs-0.6.0/src/jpeg_ls/`, the vendored pins record refreshed
through the pins gate, and the caller-specific cap record above.

Non-goals: any decoder or encoder behavior change, CharLS, corpus rows,
tolerance changes, the `Decoder` trait, `PACKAGE-INVENTORY.sha256` and its
root digest, and any file outside the write set below.

**Exact write set, future, at the story tree:**

| File | Change |
|------|--------|
| `vendor/ritk-codecs-0.6.0/src/jpeg_ls/**` | Comment text only, no byte of logic |
| `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md` | New F-269 section beside F-239's and F-243's |
| `scripts/pin_and_size_check.py` | `RITK_PROVENANCE_SHA256` refreshed, and `RITK_PATCHED_HASHES` entries for every `jpeg_ls` file whose comments change |
| `docs/lld/codecs.md` | One sentence if the k-cap line is quoted there |
| `docs/hld/DEVIATIONS.md` | **Conditional.** D-20's description only, in the same comment and pin commit, and only if the actual patched-file count changes |

`RITK_INVENTORY_SHA256` and `PACKAGE-INVENTORY.sha256` stay immutable: they
describe the published archive, not the patched checkout. A `jpeg_ls` file
that today is unpatched and receives a comment edit enters
`RITK_PATCHED_HASHES` for the first time and is described in the new
`PATCH-PROVENANCE.md` section. Files already recorded there keep their
mechanism and get new digests. The same commit updates D-20's **description
truthfully** when and only when the actual patched-file count changes, as a
reviewed description refresh of the existing deviation. This is a
conditional future write path declared here, not a new policy and not a new
deviation, and no D-20 text changes at design approval.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `crates/ocelli-codec` JPEG-LS decode, `.80` and `.81` | The vendored module, unchanged bytes of logic | Unchanged API and behavior | The same decode, corrected comments |
| The vendored encoder, called only by `crates/ocelli-codec` test `the_vendored_encoder_follows_the_same_run_rule` (`tests/jpegls.rs:1408` to `:1434`, calling `ritk_codecs::jpeg_ls::encoder::encode_grayscale_jpeg_ls` directly) and the vendored encoder's own tests | `encoder.rs`, unchanged bytes of logic | No Ocelli public encode API exists, `ocelli-codec`'s `src/jpegls.rs` is decode-only | The same output, corrected comments |
| The pins gate, `bin/ocelli.sh gate pins` | `PATCH-PROVENANCE.md` and `scripts/pin_and_size_check.py` | One coherent pin update, original published archive inventory unchanged | The gate passes on the refreshed record |
| Future readers of the k cap | The new caller-specific line beside `compute_k` and `in_coded_range` | None | The stated decoder and encoder premises |
| Future comment auditors | The recorded AC2 audit list | None | Which comments were checked, against which clause |

## State transitions and invariants

Not applicable. Comments and a pin record change no runtime state. The
invariant that does span files is provenance coherence: the provenance hash,
the patched-file hashes and the vendor bytes must agree at the story tree,
which is what the pins gate checks.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | The four pass 8 comments are corrected or removed, false and doubtful kept distinct, comment 4 using A.3.1's local-gradient naming and dropping the orientation adjectives without claiming T.87 proves the old names false, D1 to D3 arithmetic unchanged | Diff review against pass 8's list | A doubtful comment rewritten as certain, or a "the standard disproves vertical" claim | implementation, reviewer |
| AC2 | Every other factual comment under the `jpeg_ls` directory is checked against the code below it and the clause it names, and the checked list is recorded, including `context/tests.rs`, `bitstream.rs`, `parser.rs`, `reconstruction.rs`, `sample_limits.rs`, `mod.rs` and `encoder/validation.rs` | The recorded audit list | Only the four known ones touched, leaving a false comment standing | implementation, reviewer |
| AC3 | The cap-on-k record sits beside both shared k helpers and beside `in_coded_range`, stating the decoder's `in_coded_range` premise and the encoder's `modulo_reduce` premise, with initial A, N, RANGE, qbpp and RESET 64 attributed to the callers that supply them | Diff review of `context/mod.rs`, `scan.rs` and `encoder.rs` | A single shared-refusal line, or a line that names no refusal at all, or an encoder line claiming `in_coded_range`, or an encoder line presenting the decoder sweep as its own measurement | implementation |
| AC4 | The pins gate passes with the refreshed provenance hash and patched-file digests | The gate, at the story tree | A pin record that no longer matches the tree, or an unrecorded byte | implementation |
| AC4b | D-20's description states the actual patched-file count after the audit. If and only if the count changed, the same comment and pin commit updates it as a reviewed description refresh | Diff review of `docs/hld/DEVIATIONS.md` against the final `RITK_PATCHED_HASHES` | A stale five-file count left contradicting the record, or an unconditioned rewrite of the deviation's policy text | implementation, reviewer |
| AC5 | Decode unchanged. The JPEG-LS suites and corpus rows produce identical results on base and tip | The existing `crates/ocelli-codec` JPEG-LS suites at the story tree, base and tip pair | A behavior change smuggled into a comment story | implementation |
| AC6 | Semantic negative control, distinct from AC5. Restoring comment 1's wording and reading it against the NEAR-taking constructor and the encoder's use shows the defect the audit fixes, by inspection against the clause, not by a runtime red | Recorded in the review | An audit whose only evidence is that decode still passes, which cannot see comment correctness | reviewer |

Unrun ownership. No gate or test ran for this filing or its refinement. AC4,
AC4b and AC5 run at the story tree under this story as their owner. AC6 is an
inspection record owned by the reviewer, not a test execution.

## Approach

Read each comment, then the code under it, then the clause it names, from
T.87's text and in that order. Correct the false ones, keep or mark the
doubtful ones per the review's rating, and record the audit list. Comment 4 is
settled by the lead: A.3.1's local-gradient naming replaces the orientation
adjectives, the arithmetic `D1 = Rd - Rb`, `D2 = Rb - Rc`, `D3 = Rc - Ra`
stays byte-identical, and no claim that T.87 disproves the old terms. Then
refresh the pins record for the comment-only change and write the
caller-specific cap line.

Rejected: rewriting the comments to name no clause at all, which removes the
checkable claim, and fixing decode behavior found on the way, which is a
separate finding for its own record. Also rejected: claiming a fresh cap
proof or sweep, which pass 8 owns and this story does not rerun.

## Dependencies, write ownership and resources

F-243 is done on this base (`16c782d1`, F-238's commit, carries it), so this
story no longer waits on any merge. The pin refresh coordinates with the
reviewed F-239 vendor union so the vendored pins move once, with no new
formal dependency declared. Writes only the five paths in the write set
above, the fifth conditional as stated. The original
`PACKAGE-INVENTORY.sha256` remains immutable and continues to describe the
published archive, not the patched checkout. No exclusive resource.

## Verification and evidence reuse

The story's instrument is reading, not building. AC4's gate run and AC5's
identical-results run happen at the tree. Evidence invalidates on any later
F-243 or vendor change, which re-runs the audit list.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: n/a. Comments only
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three tier rows are not applicable. The story touches no rendering or
compute, and the versioned-source and provenance coherence that replaces
runtime recovery here is owned by the pins gate, not by any caller.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Decode unchanged across the audit | The existing `crates/ocelli-codec` JPEG-LS suites, AC5 |

No new test exists. A comment correction cannot turn a runtime check red and
green, which is why AC6 is an inspection record against the clause rather
than a fixture, and AC5 is the unchanged-decode pair, distinct from AC6.

## Parity surface covered

None.

## Deviations

None new. D-20 already records the vendoring and its pin record.

## LLD impact

`docs/lld/codecs.md`, one sentence if the k-cap line is quoted there. Its
F-243 section already summarises the patch and cites
`PATCH-PROVENANCE.md`, so no structure changes. `docs/hld/DEVIATIONS.md`
D-20's prose describes the F-243 patch in five files and one published test.
If the audit's edits change a `jpeg_ls` file that is not already in
`RITK_PATCHED_HASHES`, that count becomes stale. The write set and the
acceptance inventory declare the conditional path: AC4b updates D-20's
description in the same comment and pin commit when and only when the actual
patched-file count changes, as a reviewed description refresh, never an edit
made silently or left to contradict the record. No D-20 edit happens now or
at design approval.

## User-facing documentation impact

No change.

## Open questions

1. ~~Whether comment 4 is corrected to the local-gradient naming or kept with
   a doubt note.~~ Settled by the lead in the S47 design round: local-gradient
   naming, orientation adjectives dropped, no claim that T.87 proves the old
   terms false. Affects AC1. Recorded from the continuation instruction.
2. ~~The exact pin-record shape for a comments-only change.~~ Settled: the
   existing `PATCH-PROVENANCE.md` plus a new F-269 section, refreshed
   `RITK_PROVENANCE_SHA256`, and `RITK_PATCHED_HASHES` entries for every
   changed `jpeg_ls` file. `PACKAGE-INVENTORY.sha256` is never rewritten.
   Affects AC4. Recorded from the continuation instruction.
3. Which `jpeg_ls` files beyond the four comment sites the AC2 audit finds
   needing comment edits, which sets the final `RITK_PATCHED_HASHES` delta.
   Affects AC4, AC4b and the conditional D-20 description refresh. Owner: the
   implementation stage. Consequence of deferral: the pin update lands as
   one coherent commit regardless, and the delta is read off the actual
   diff before the hash is computed.


## F269 Amendment A1 (append-only to approved F269 plan)

**Scope:** exactly one extra owned file: `scripts/tests/test_pin_and_size_check.py`. Extend its hard-coded expected patched-file list 10→11 with literal `"src/jpeg_ls/context/tests.rs"` and add F269 to that file's explanatory comment. Retain the full independent expected list, per-file byte-mutation loop, and equality assertion. Never derive the list from the product map or skip the new path. No gate, budget, tolerance, immutable-inventory, or Rust runtime changes.

**Regression:** `RitkVendor.test_a_planted_byte_in_each_patched_source_file_is_refused` must reject a planted byte in the new path with exactly, no terminal punctuation: `ritk-codecs src/jpeg_ls/context/tests.rs has undeclared changes beyond its recorded patch`

**Ordering:** old-list actual floor RED is preserved at `/private/tmp/ocelli-s46-codex/f269-feature-floor.json/log`. Targeted GREEN plus meaningful mutation controls and fresh root review come BEFORE the complete new-feature floor plus corpus. The 28 prior greens are not a new profile. CURRENT stays S46, no shared completion or integration.

**Preserved record:** both author and root pass 4 missed this old consumer. Keep the failed floor receipt. No source edit before Root reviews. No checks run yet. Owner: GLM.

Approval: adopted by root independent amendment review pass 1 on 2026-11-09.
Only the declared test-consumer extension is authorized.
