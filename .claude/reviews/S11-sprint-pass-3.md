# S11 whole-sprint review, pass 3

**Reviewed**: staged tree `292052e8593c0d44fff57e8b35f42760166c7158`, on
`sprint/s11` at HEAD `1974802`. Remediation is uncommitted and staged.
**Delta reviewed**: `git diff e8f6f4e343065bbd 292052e8593c0d44`, 10 files, of
which `.claude/reviews/S11-sprint-pass-2.md` is 359 of the 418 insertions.
**Scope**: the eight pass-2 findings, the six questions asked, and a sweep for
anything the remediation created or regressed.
**Machine**: working GPU, resolves tier A.
**Result**: **1 defect, 0 smells, 3 nitpicks**

**This pass is NOT clean.** The one defect is in the sprint run state rather
than in the sprint, it is one JSON field, and it is the field that stops a stale
review from closing a sprint. Every code and prose finding from passes 1 and 2
is closed and I could not falsify any replacement sentence.

---

## Verdict on each pass-2 finding

| Pass 2 | Status |
|--------|--------|
| D1, six decoder families | **Closed.** Reverted to five, with a correct explanation |
| D2, sentence did not parse | **Closed.** It parses and is true under the reading its own clause forces. See N1 |
| S1, hard count of eighteen | **Closed.** Count gone, command named, both bad shapes recorded |
| S2, `voi` computes nothing | **Closed.** Now the same formulation `pixel-pipeline.md` uses. Last copy noted at N2 |
| N1, census bucket "not empty" | **Closed.** Every clause of the replacement verified, including the ratchet reasoning |
| N2, ULP without its magnitude | **Closed.** Reads "two `f32` ULP at 255" |
| N3, over-applied dates | **Closed.** Three reverted, three correct |
| N4, ten `cargo doc` warnings | **Closed.** Zero warnings, and no logic changed to get there |

---

## Defect

### D1, pass 2's review record is stamped with the tree that fixed it, which silences the staleness guard

**Where**: `.claude/scratch/S11-run.json`, `sprint_reviews[1]`

**What**: the record is

```json
{"pass": 2, "defects": 2, "smells": 2, "nitpicks": 4,
 "tree": "292052e8593c0d44fff57e8b35f42760166c7158"}
```

Pass 2 reviewed `e8f6f4e343065bbd55cfa3edc06db28b369e77cb`. `292052e8593c` is
this tree, the one that REMEDIATES pass 2's findings. The record asserts that a
review finding 2 defects and 2 smells was performed against a tree in which
those four findings do not exist.

**Why it is wrong**: `scripts/sprint_workflow.py:265-280` stamps
`staged_tree()` at the moment `record-sprint-review` runs, so recording after
remediating attributes the review to the wrong tree. Pass 1 was recorded
correctly, at `98c2462347bd`, which is the tree it judged. Pass 2 was recorded
late.

`/microscope` depends on this field: "The record includes `git write-tree`. **A
later remediation makes that clean pass stale and requires another pass after
verification of the new tree.**" That mechanism is now off.

**Evidence, which is the part that matters.** The staleness line that
`close-preflight` printed during pass 2 is gone:

```
# during pass 2, tree e8f6f4e34306, review stamped 98c2462347bd
  latest sprint review pass 1 reports 5 defects and 3 smells
  latest sprint review tree 98c2462347bd is stale for current tree e8f6f4e34306   <-- present

# now, tree 292052e8593c, review stamped 292052e8593c
  latest sprint review pass 2 reports 2 defects and 2 smells
  (no staleness line)                                                             <-- gone
```

So the ledger now presents a review of the previous tree as a review of this
one. Today the only visible effect is the missing line, because the counts are
non-zero and `close-preflight` refuses anyway. The shape is what matters: had
pass 2 been clean, this stamping would have let a review of a superseded tree
satisfy the check that exists to stop exactly that.

**The fix is one field**: set `sprint_reviews[1].tree` to
`e8f6f4e343065bbd55cfa3edc06db28b369e77cb`. And record pass 3 before
remediating it rather than after, which is what happened for pass 1.

**This is bookkeeping, not the sprint.** Nothing about the code, the gates or
the documentation is affected by it. If the operator would rather correct the
field and push than run a fourth pass over an unchanged tree, that is a
defensible call and I am saying so plainly rather than hiding it behind a
count.

---

## Nitpicks

- **N1, `scripts/guards/catalogue.py:7994-7996` and the rendered copy at
  `docs/runbooks/guard-verification.md:706`.** The sentence now reads "...marks
  YES in its GPU column, **and deviation D-04 is the reason such a gate exists
  at all.** That column is not in the declared-constant ratchet." It parses. Is
  it true? D-04's row is "CI runs no GPU build and no GPU test", which is the
  reason such a gate is EXCLUDED FROM THE FLOOR rather than the reason it
  exists. The reason the `gpu` gate exists is in `bin/ocelli.sh` itself: an
  `#[ignore]`d test "ran in **no profile at all**". But the charitable reading
  is the one the surrounding sentence forces and it is supported by
  `scripts/guards/census.py`, which says `gpu` "is a separate gate rather than
  part of `test` because deviation D-04 leaves CI without an adapter", and the
  GPU column itself exists to drive the floor exclusion that D-04 causes. So
  "at all" is loose and the claim lands. **I am not blocking on it.** This
  sentence has now been through three versions and `voi.wgsl:97-104` records
  what happens when a comment is corrected a fourth time on thinning grounds.
- **N2, `crates/ocelli-render/src/voi.rs:4-7`** is the last copy of the
  formulation the remediation retired: "This module is the reading. **It
  computes no LUT arithmetic and makes no LUT decision**", in the file that
  declares `VOI_WGSL`. `lib.rs` and `pixel-pipeline.md` now both say the WGSL
  does evaluate the formulas. I explicitly excused this copy in pass 2 as
  pre-existing and unchanged, and I am not reversing that, but the bar around it
  moved and one clause would align it.
- **N3, `bin/ocelli.sh:307-308`** says `cargo test -p ocelli-render --
  --ignored --list` "prints how many there are". Measured: it prints seven
  per-binary lines, `3`, `0`, `0`, `6`, `0`, `9`, `0`, so the reader sums three
  non-zero numbers rather than reading one. The information is there and the
  decision to name a command instead of a number is right. `... --list | grep -c
  ': test$'` prints 18 as a single figure if one is wanted.

---

## The six questions, answered

### 1. Is the D1 revert correct and its new clause true?

**Yes, every clause.** `CLAUDE.md:187-192` now reads "five decoder families...
**Six names, five families**, because Raw and RLE register together through
`register_native_and_rle_decoders` and `crates/ocelli-codec/tests/registry.rs`
says so. S11's sprint review changed this to six and its next pass changed it
back, which is what counting names instead of `register_*` functions costs."

- Five `register_*` functions, confirmed independently again:
  `register_htj2k_decoders`, `register_jpegls_decoders`, `register_jpeg_decoders`,
  `register_jpeg2000_decoders`, `register_native_and_rle_decoders`.
- `crates/ocelli-codec/src/native.rs:226` is the combined one, so Raw and RLE
  are one family.
- `crates/ocelli-codec/tests/registry.rs:429-441` does say so: "None of them
  registers more than one family... the five coexist... a sixth family... would
  fail".
- The history clause is accurate about what both passes did.

Thank you for verifying rather than taking my correction. That is the right
response to a reviewer who has just been wrong once.

### 2. Does D2's sentence parse and is it true, and did the render change only that?

Parses: yes. True: yes under the reading its clause forces, with the looseness
recorded at N1 and not blocking.

**Render scope: exactly one line.** `git diff --numstat e8f6f4e 292052e --
docs/runbooks/guard-verification.md` is `1 1`. The catalogue change is `4 2`,
which is the same string re-wrapped across a different number of source lines,
and it is the only non-prose file in the delta.

### 3. Did S1 and S2 introduce a new false claim, and is S2 consistent with `pixel-pipeline.md`?

**No new false claim in either, and yes.**

S1, `bin/ocelli.sh:303-312`. "NO COUNT IS WRITTEN HERE" and the command is
named. The four members it lists are still exactly right, verified by running
the gate. "THIS COMMENT DID, twice: a gloss... then a hard count of eighteen put
in the same arm that warns against exactly that" is an accurate account of both
passes. "F-038, F-040 and F-042 all add to this crate" matches BACKLOG, which
puts F-038 in S12 and F-040 and F-042 in S13, all in `ocelli-render`. The only
residue is N3.

S2, `crates/ocelli-render/src/lib.rs:16-27`. Now: "**The WGSL does evaluate the
three window formulas**, because section 18.4's uniform hands a shader
`center`, `width` and `fn_kind` and a shader given those has to evaluate
something. What it does not do is make a LUT DECISION: it is handed no input
from which it could re-select a window, recompute inversion or apply a sequence.
The Rust in `voi` computes nothing at all and only reads a resolved chain."

Checked against the code rather than against `pixel-pipeline.md`: the three
window functions are `voi.wgsl:56-85`, `VoiParams` carries no Photometric
Interpretation, no window multiplicity and no sequence, so none of the three
re-decisions is reachable, `voi.rs`'s Rust is two accessor reads, a total match
and a `u32::from(bool)`, which computes nothing.

**Consistent with `pixel-pipeline.md`: yes.** Both now say the shader evaluates
and does not decide, which is the formulation the LLD adopted. One difference
worth knowing rather than fixing: `pixel-pipeline.md:17-22` says the shader
evaluates "stage 1, all three window functions and stage 3", where `lib.rs`
names the three window formulas only. `lib.rs` is narrower, not contradictory,
and its narrower scope matches the narrower parameter list it cites.

### 4. Is N1's new sentence true, including the ratchet reasoning?

**Yes, all three clauses, and the ratchet reasoning is exactly right.**

- "it is empty today": `python3 scripts/guard_census.py` prints "0 watched by
  nothing", exit 0, on this tree.
- "a ratchet that may only decrease": `scripts/guard_census.py:186-188` refuses
  when "the uncovered ceiling RISES from {previous} to {uncovered}", and
  `scripts/guards/census.py:92` states it as a rule.
- "the census refuses a non-zero count once the sweep is recorded complete":
  `scripts/guard_census.py:190-195`, `if ratchet.get("sweep_complete") and
  uncovered > 0`, message "the sweep was recorded complete and is not".
  `ci/guard-probe-budget.json` carries `{"sites": 0, "sweep_complete": true}`,
  so that branch is live rather than hypothetical.
- "so 'run the command' is still the instruction and only the answer moved":
  follows from the two above.

### 5. Any new cross-story contradiction, and did anything regress?

**None, and nothing regressed.**

- **No logic changed.** `git diff e8f6f4e 292052e` over `*.rs`, `*.wgsl` and
  `*.toml`, with comments and blanks filtered, is **empty**. `probe.rs`'s 52
  changed lines are all intra-doc links becoming code spans.
- **N4's repair cost nothing.** `cargo doc -p ocelli-render --no-deps` emits
  **zero** warnings, down from ten, with no item made public and no link
  deleted that carried meaning.
- **The stale count is gone from everywhere it could be.** `grep -rn
  "eighteen"` across `bin/`, `docs/lld/`, `docs/sprints/`, `.claude/WORKFLOW.md`
  and `scripts/` leaves only `bin/ocelli.sh:311`'s own history note, which is
  about the count rather than a count, and unrelated pre-existing uses in four
  other files.
- The three `gpu`-set descriptions, `bin/ocelli.sh:303-306`,
  `docs/lld/gpu-ownership.md:302-305` and `.claude/WORKFLOW.md`, agree with each
  other and with the gate.
- `docs/lld` dates are now right: 2026-09-15 on exactly `gpu-ownership`,
  `feature-availability` and `pixel-pipeline`, which are the three F-041 edited
  on that date, and `cache`, `guards` and `tier-resolution` are back to
  2026-09-14 with content byte-identical to HEAD.
- **The remediation touched only the files the findings named.** Ten files in
  the delta, every one of them a pass-2 finding site or the pass-2 report.

### 6. Is this pass clean?

**No, by one defect, and I want to be precise about what that means.**

The sprint itself is in good shape. Across three passes the code has needed no
change at all: every finding in all three has been documentation, ledger or
comment, and the arithmetic, the gates, the device lifecycle and the cache have
survived independent attack including a mutation that took five GPU tests red.
Passes 1 and 2 found 5 and 2 defects. This one finds 1, and it is in a JSON
field rather than in the sprint.

If the operator corrects `sprint_reviews[1].tree` to
`e8f6f4e343065bbd55cfa3edc06db28b369e77cb` and records pass 3 at the current
tree, the run state is consistent and nothing else in this pass blocks. That is
a state change to an ignored file that does not move `git write-tree`, so it
does not invalidate the recorded 31-gate verification. Whether that warrants a
fourth pass over an otherwise unchanged tree is a scheduling call and it is the
operator's, not mine. What I will not do is report zero defects over a ledger
record I have shown to be false.

---

## Verified clean

- **Gates run on this tree**, exit codes read from the command: `prose`,
  `guards`, `ci`, `fmt`, `clippy` ALL GREEN, 5 gates. `gpu` ALL GREEN, 18
  ignored tests. `cargo test -p ocelli-render --doc` green.
- **The recorded verification is real and matches.**
  `.claude/verify-ledger.json` holds `292052e8593c0d44fff57e8b35f42760166c7158`
  at `profile=sprint`, 31 gates including `oracle`, `corpus=pass`.
  `.claude/scratch/S11-run.json` carries the matching third verification row.
- **F-041's review records remain correct** after pass 1's D5: 8 records,
  passes 1 to 8, no duplicate. F-031 and F-037 likewise.
- **The `gpu` set is still four members and still eighteen tests**, measured:
  3 in `--lib`, 6 in `tests/device.rs`, 9 in `tests/voi_shader.rs`, and
  `--ignored --list | grep -c ': test$'` gives 18.
- **`ci` still parses the rewritten `gpu` arm**, reporting agreement between the
  runner's exclusion list and `NOT_IN_FLOOR` on corpus, gpu, guards-deep,
  oracle, quirk-mutations.
- **All 18 LLD index rows still match their files**, re-verified with my own
  wrap-tolerant parser after this delta: zero mismatches.
- **`CLAUDE.md`'s other claims re-checked** after the two edits: `ls crates | wc
  -l` is 13 and the "eleven shared crates" line still holds, the corpus and
  deviation lines still name commands rather than numbers, and the three S11
  entries from pass 2 are unchanged and still true.

## Not done in this pass

`gate --sprint` was not re-run end to end. The delta changes no logic and the
ledger records a passing 31-gate run against this exact tree, so I re-ran the
gates the edits could plausibly break plus `gpu`. `record-sprint-review` was not
run, as instructed.

**Tree state**: unchanged. `git write-tree` returns
`292052e8593c0d44fff57e8b35f42760166c7158`. The only working-tree addition is
this report.
