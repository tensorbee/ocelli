# F-269, the vendored ritk-codecs JPEG-LS comments audited against ITU-T T.87

**Status**: draft
**Epic ref**: E4.14
**Sprint**: S47
**Estimate**: 1w

Filed at the S46 close from the recovered close-backlog draft, original item
5, the sprint-close audit story F-243's lead ruling names. The estimate is
inherited and is provisional, lead-suggested and not capacity-approved. This
draft claims no implementation, no review cleanliness and no completion.

## Normative source, transcribed

ITU-T T.87 is the audit's standard. The clauses the audit checks comments
against, as verified by F-243 working pass 8 at its tip: A.2.1 step 2
initialises N to 1, A.10 has no division, A.3.1 names D1 to D3 only local
gradients, and C.2.4.1.1.1 gives the default thresholds. The audit uses T.87
only and reads no CharLS source, as F-243's own reviews did not.

`docs/hld/DEVIATIONS.md` D-20 records the ritk-codecs vendoring and its pin
record, which the story refreshes. `docs/hld/18-codec-registry.md` section 21
fixes the `Decoder` trait, which the story does not touch, because the audit
changes no decode.

## What the specification does not cover

Comment wording. T.87 says nothing about comments, so every correction is
against the code below the comment and the clause the comment names. A
doubtful comment is kept doubtful rather than rewritten as certain, on pass
8's own rating.

## Problem and evidence

F-243 working pass 8, "Audit backlog, not blocking", lists four false or
doubtful upstream comments on lines F-243 did not write or rewrite, "for the
sprint-close audit story the ruling names". Quoted with this tree's
positions, read at 31323a79:

| # | Comment | Here | F-243 tip | Why it is wrong or doubtful |
|---|---------|------|-----------|-----------------------------|
| 1 | "context model for JPEG-LS lossless decoding" | `context/mod.rs:1` | `:1` | The module also serves near-lossless, since `CodingParams::new` takes NEAR, and the encoder |
| 2 | "`n` ... initialized to 1 to avoid division by zero" | `context/mod.rs:15` | `:15` | T.87 A.2.1 step 2 initialises N to 1, and A.10 has no division, so the reason does not match the code |
| 3 | "DICOM data is ≥ 8-bit so the high branch is the production path" | `context/mod.rs:296` | `:296` | `decoder.rs:82` accepts P from 2, and Bits Stored below 8 gives MAXVAL below 128, which takes the low branch |
| 4 | D1, D2 and D3 named vertical, horizontal and diagonal | `scan.rs:164` to `:166` | `:164` to `:166` | A.3.1 names them only local gradients. Doubtful rather than plainly false, since "vertical gradient" is sometimes used for the response to a vertical edge |

All paths are under `vendor/ritk-codecs-0.6.0/src/jpeg_ls/`. The named comments have the same positions at this staged source snapshot
and F-243 tip `fd11a482`. This records source content, not commit ancestry
for a tree object. The story re-locates each comment by content at its own tip.

The cap on k. Pass 8's verdict proves and measures that the decoder's
`k < qbpp` cap never binds at the F-243 tip, and shows that the proof rests on
`in_coded_range`, which refuses before the A update. With that refusal
disabled the cap bound about two million times on damaged streams. Pass 8
records that the dependence "is not written anywhere", worth a line if the
cap is ever discussed again. The story writes that line beside the cap and
beside `in_coded_range`. In this tree `in_coded_range` is defined at
`scan.rs:302` to `:304` and called at `:230` and `:272`, matching pass 8's
citations.

## Scope, API and compatibility

In scope: the four comments above, the rest of the factual comments under
`vendor/ritk-codecs-0.6.0/src/jpeg_ls/`, the vendored pins record refreshed
through the pins gate, and the one-line record that the k-cap proof rests on
`in_coded_range`.

Non-goals: any decoder behavior change, CharLS, the encoder's output, corpus
rows, tolerance changes, and any file outside the vendored `jpeg_ls` subtree
and its pin record.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `crates/ocelli-codec` JPEG-LS decode | The vendored module, unchanged | Unchanged API and behavior | The same decode, corrected comments |
| The pins gate | `PATCH-PROVENANCE.md` and the patched-file/provenance hashes in `scripts/pin_and_size_check.py` | One coherent pin update, original published archive inventory unchanged | The gate passes on the refreshed record |
| Future readers of the k cap | The new line beside the cap | None | The stated dependence on `in_coded_range` |

## State transitions and invariants

Not applicable. Comments and a pin record change no runtime state.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | The four pass 8 comments are corrected or removed, false and doubtful kept distinct, each with its clause named | Diff review against pass 8's list | A doubtful comment rewritten as certain | implementation, reviewer |
| AC2 | Every other factual comment under the `jpeg_ls` directory is checked against the code below it and the clause it names, and the checked list is recorded | The recorded audit list | Only the four known ones touched | implementation, reviewer |
| AC3 | The cap-on-k line sits beside the cap and beside `in_coded_range`, stating the dependence | Diff review | A line elsewhere that never names the refusal | implementation |
| AC4 | The pins gate passes with the refreshed provenance | The gate, at the story tree | A pin record that no longer matches the tree | implementation |
| AC5 | Decode unchanged. The JPEG-LS suites and corpus rows produce identical results on base and tip | The suites at the tree | A behavior change smuggled into a comment story | implementation |
| AC6 | Negative control. Restoring comment 1's wording and reading it against the NEAR-taking constructor shows the defect the audit fixes | Recorded in the review | An audit that cannot say what it rejected | reviewer |

Unrun ownership. No gate or test ran for this filing. AC4 and AC5 run at the
story tree after F-243 merges, and until then they are unrun with this story
as their owner.

## Approach

Read each comment, then the code under it, then the clause it names, from
T.87's text and in that order. Correct the false ones, keep or mark the
doubtful ones per the review's rating, and record the audit list. Then
refresh the pins record for the comment-only change.

Rejected: rewriting the comments to name no clause at all, which removes the
checkable claim, and fixing decode behavior found on the way, which is a
separate finding for its own record.

## Dependencies, write ownership and resources

Depends on F-243, which edits the files this story edits, so it lands after
F-243 merges and follows it if F-243 carries out of S46. The pin refresh
coordinates with the reviewed F-239 vendor union so the vendored pins move
once, with no new formal dependency declared. Writes only
`vendor/ritk-codecs-0.6.0/src/jpeg_ls/**`, its `PATCH-PROVENANCE.md` and
the patched-file/provenance hashes in `scripts/pin_and_size_check.py`. The
original `PACKAGE-INVENTORY.sha256` remains immutable and continues to describe
the published archive, not the patched checkout. No exclusive resource.

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
compute.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Decode unchanged across the audit | The existing `crates/ocelli-codec` JPEG-LS suites, AC5 |

## Parity surface covered

None.

## Deviations

None new. D-20 already records the vendoring and its pin record.

## LLD impact

`docs/lld/codecs.md`, one sentence if the k-cap line is quoted there.

## User-facing documentation impact

No change.

## Open questions

1. Whether comment 4 is corrected to the local-gradient naming or kept with a
   doubt note. Affects AC1. Owner: the design round, on pass 8's doubtful
   rating.
2. The exact pin-record shape for a comments-only change. Affects AC4. Owner:
   the design round, beside the D-20 record.
