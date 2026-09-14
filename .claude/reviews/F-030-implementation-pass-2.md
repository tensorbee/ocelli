# F-030 review, pass 2

**Reviewed**: the working tree after pass 1's four remediations.
**Result**: 0 defects, 1 smell, 0 nitpicks

This pass exists because pass 1's remediations were themselves unreviewed work.
Its whole subject is the diff pass 1 produced.

## Defects

None.

## Smells

### S1, pass 1's remediation merged two doc paragraphs into one

**Where**: `crates/ocelli-pixel/src/color.rs`, `ColorTransform::pixel`.

**What**: the wildcard-arm rationale was appended directly after the existing
bounds paragraph with no blank line, so rustdoc renders the two as one
paragraph and the sentence about bounds runs into the sentence about match
exhaustiveness. Nothing is false, and the two justifications stop being
separately findable.

**Why it matters enough to record**: this is the exact shape `/microscope`
warns about under "when re-applying an edit". The insertion anchored on text
that survived its own edit, and the result compiled, passed clippy and passed
every test, so no gate in the repository would ever have reported it.

**Evidence**: read back from the file after the edit rather than assumed from
the edit succeeding.

**Remediated**: blank line restored, and the same edit records that the palette
arm's `map_or` default is unreachable because `resolve` refuses a palette space
with no palette. That state was previously defaulted silently with no
explanation, which pass 1 did not notice.

## Nitpicks

None.

## Verified clean

**Each of pass 1's four remediations landed exactly once**, checked by grepping
for the NEW text rather than for the anchor it was inserted against:

| Remediation | Occurrences |
|---|---|
| `pixel` matches `self.space` once, exhaustively | `grep -n "_ =>"` returns nothing in the file |
| "F-030 is the first stage that does" | 1, `docs/lld/pixel-pipeline.md:247` |
| "case_sigmoid_width_half" named in the corpus LLD | 1, `docs/lld/corpus.md:90` |
| "Negative zero" in `first_input_bits` | 1, `crates/ocelli-pixel/src/lut.rs:110` |
| "Public despite having no caller" | 1, `crates/ocelli-pixel/src/stored_pixel.rs:137` |

**The restructure did not change behaviour.** Five mutations were re-run
against the restructured `pixel`, not carried over from pass 1: the BT.601
constant, the partial-range `+ 16` offset, the decoder `Rgb` evidence, routing
`YbrPartial422` through `FULL_RANGE`, and replacing the interleaved indices
with the planar ones. All five red, tree green again after each revert. The
last two are new in this pass and exist because the restructure moved exactly
that code.

**The two closures are only reachable from the arms that can use them.**
`triple` is called from `Rgb` and `YbrFull`, whose lengths are `3 * pixels`.
`subsampled_triple` is called from the two 4:2:2 arms, whose length is
`2 * pixels` with `pixels` even. Both bounds were re-derived from `map_into`'s
checks rather than assumed from pass 1.

**Clippy, tests and prose** green on the remediated tree.
`bin/ocelli.sh clippy ocelli-pixel` exit 0, `cargo test -p ocelli-pixel` zero
failures, `scripts/prose_check.py` clean over 289 files.

**Nothing else in the diff moved.** Pass 1's remediations touched four files
and this pass touched one. The corpus, the manifest, the guard census and the
oracle expectation files are untouched since the green sprint-profile run.
