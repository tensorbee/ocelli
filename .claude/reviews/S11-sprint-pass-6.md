# S11 whole-sprint review, pass 6, CHANGELOG scope only

**Reviewed**: staged tree `7a9f2ca780d92257e767e62eca75437a34ee089c`.
**Delta**: `git diff f6aaac09e1aa 7a9f2ca780d9` is `CHANGELOG.md`, 5 insertions
and 4 deletions across two hunks, plus `.claude/reviews/S11-sprint-pass-5.md`,
which is my own pass-5 report being staged. No other tracked file.
**Scope**: the two fixes and the four questions. The sprint was not
re-reviewed. Everything under `crates/`, `scripts/`, `bin/`, `docs/` and `ci/`
is byte-identical to `325168bb0fee`, the tree pass 4 passed clean.
**Result**: **0 defects, 0 smells, 1 nitpick**

**This pass is CLEAN.** Record it, commit, close.

---

## The four questions, answered

### 1. Are the two fixes true, and did either introduce a new claim?

**Both true. One new clause, and it is the nitpick below rather than a
blocker.**

**D1, the deletion.** `CHANGELOG.md:262-263` now ends "so a window or level
change writes those bytes and nothing else."

This is not a new claim, it is the claim the source already makes, and it is
asserted:

- `crates/ocelli-render/src/voi.rs:70-71` carries the phrasing verbatim: "A
  window-level drag writes this struct **and nothing else**." That sentence
  went through eight F-041 review passes and four of mine.
- `crates/ocelli-render/tests/voi_shader.rs:657-665` asserts the half that
  matters, that the whole difference between two window states is the uniform:
  `assert_eq!(VoiParams { width: soft.width, ..narrow }, soft)`, under the
  comment "The two uniforms differ in `width` and in nothing else, which is
  what a drag writes."
- `size_of::<VoiParams>() == 32` is asserted at `voi_shader.rs:673` and field
  by field at `voi_params.rs:62`.

So the register now claims exactly the half `voi_shader.rs:608-616` calls "the
first half" and stops where that comment stops. The forward-looking half is
gone and lands when F-038 makes it true. This is the right repair and it is the
minimal one.

**D2, both copies.** Verified by running the command rather than by reading the
edit:

```
$ python3 scripts/guard_census.py | sed -n 2p
  ... 26 declared out of scope, 0 watched by nothing
```

- `CHANGELOG.md:203-204`, "prints the bucket watched by nothing. **It is empty
  today**, and this line said it was not until S11's sprint review ran the
  command." Both halves true. The line did say "not empty", it was raised at
  pass 1 as N4 against `CLAUDE.md` and at pass 5 as D2 here, and the command is
  what settled it.
- `CHANGELOG.md:210-212`, "**That bucket is empty today**, and the number in it
  is a ratchet that may only go down, so a refusal added without a watcher
  fails the floor and the count cannot climb back."

I checked the middle clause at the line rather than taking it, because it is the
load-bearing one. "A refusal added without a watcher fails the floor" is **true**
and the path is: `scripts/guards/census.py:814-819` appends a problem when
`uncovered_sites > ceiling`, the recorded ceiling is `0`
(`ci/guard-probe-budget.json`), so any new unwatched refusal gives `1 > 0`, and
`scripts/guard_census.py:238-241` turns a non-empty `problems` into exit 1.

The trailing clause is the nitpick.

### 2. Does bullet 3 still read as a complete sentence, and are the surviving clauses unaffected?

**Yes and yes.**

The sentence is a noun phrase with an appositive and a consequence clause,
which is the form every bullet in this section takes: "A budgeted LRU across
the encoded, decoded and GPU tiers.", "The session's one long-lived GPU
device, opened from the adapter the tier resolved on...". Removing "and
re-uploads no texture" left the `so` clause with its own complete predicate,
"writes those bytes and nothing else", so nothing dangles and no conjunction is
orphaned.

The three surviving clauses are untouched by the deletion and I re-checked each
against the code rather than assuming the diff was local:

| Clause | Verdict |
|--------|---------|
| "The shader makes no LUT decision, because the uniform carries a resolved inversion flag, one selected window pair and rescale values" | True, unchanged |
| "and carries no photometric interpretation, no window multiplicity and no lookup-table sequence from which any of them could be re-derived" | True, unchanged. `VoiParams` has eight fields and none of the three |
| "A chain driven by a Modality or VOI LUT Sequence cannot be expressed in that uniform and reports unavailable rather than substituting the values the sequence overrode" | True, unchanged. `VoiParamsError`'s two variants, returned before any field is filled |

The deletion leaves line 263 short of the file's usual wrap, and the D2 edit
leaves line 212 long. Both are wrap artefacts in a file no gate reads. Neither
is a finding and neither is worth a commit on its own.

### 3. Is the delta `CHANGELOG.md` only?

**Nearly, and the difference is mine rather than yours.** The tracked delta is
two files:

```
$ git diff --name-only f6aaac09e1aa 7a9f2ca780d9
.claude/reviews/S11-sprint-pass-5.md
CHANGELOG.md
```

`S11-sprint-pass-5.md` is my own pass-5 report, which you staged, so it is
expected and it is not a change to the sprint. Saying so precisely rather than
agreeing with "CHANGELOG.md only" because it is nearly right.

Against the tree pass 4 passed clean, `git diff --name-only 325168bb0fee
7a9f2ca780d9 -- crates/ scripts/ bin/ docs/ ci/` is **empty**. No code, script,
binary, document or budget file has moved since the tree I signed off. Nothing
could have regressed and I ran no gates beyond the census, because a nine-line
edit to a file no gate reads cannot move one.

### 4. Is this pass clean?

**Yes. Zero defects, zero smells, one nitpick that I am explicitly not blocking
on. Record it, commit, and run the close.**

---

## Nitpick

**"and the count cannot climb back" is stronger than the mechanism, and I am
not blocking on it.** `CHANGELOG.md:212`.

`python3 scripts/guard_census.py --record` will write a risen ceiling and exit
zero. `scripts/guard_census.py:186-189` and `:190-195` are both bare `print`
calls with no `return 1`, and `:196-198` then writes `ratchet["sites"] =
uncovered` unconditionally and returns 0. So the number can climb, through the
documented maintenance path, with a warning printed and a green exit.

**Three reasons this is a nitpick and not a finding**, and I want them on the
record because I have blocked five times and the difference matters:

1. **It restates a clause that is already there and is eight sprints old.** The
   same sentence says "the number in it is a ratchet that may only go down"
   three clauses earlier, which carries the identical reading. The new text
   strengthens a policy statement into an impossibility. If "cannot climb back"
   is wrong then so is "may only go down", and that has stood since S03 through
   every pass including my own four.
2. **The true clause is beside it and is the one that matters.** "A refusal
   added without a watcher fails the floor" is exact, and I verified the path.
   A reader who acts on this entry acts on that clause.
3. **`--record` is not a bypass, it is the declared control.** The budget file's
   own `note` says these values are "written by scripts/guard_census.py
   --record", and `docs/lld/guards.md` records that the S11 re-record "moved
   exactly two values... so the widening is visible in the diff rather than
   asserted in a message." The ratchet is held by the gate refusing and by the
   diff, not by the integer being immutable.

**I read this path rather than ran it**, deliberately: running `--record` would
rewrite the tracked `ci/guard-probe-budget.json` and I would have to revert it,
and the code is unambiguous without that. If you want it measured before you
decide, say so and I will plant a refusal, run it, and revert both.

**My recommendation is to leave it.** Deleting the clause would be correct and
costs nothing, but it is a fourth edit to a sentence in a file no gate reads,
and `voi.wgsl:97-104` is this repository's own record of what happens when a
comment gets corrected a fourth time on thinning grounds. If you are editing
`CHANGELOG.md` again for another reason, drop it then.

---

## On the `gpu` gate, since you asked me to push back if I disagreed

**Your reasoning is better than my nitpick and I withdraw it.** You wrote that
the register's workflow entries describe a capability a reader gains, and that
a gate which only runs on a machine with an adapter and never in CI is closer
to an internal arrangement. That is the right test, and applying it correctly
separates the `gpu` gate from the two entries I cited as precedent: "Stronger
repository workflow guards" describes refusals a contributor will actually hit
in CI, and "Sprint closure evidence bound to one Git tree" describes a rule
that gates every close. Neither is conditional on hardware nobody in CI has.

I was pattern-matching on entry class rather than asking what a reader gains,
which is the same mistake as counting names instead of `register_*` functions.
Do not add it.

---

## Verified clean

- Both fixes verified against the code and the command, not against the diff.
- `0 watched by nothing`, re-run on this tree.
- The floor-refusal path for a new unwatched refusal traced end to end,
  `census.py:814-819` to `guard_census.py:238-241`.
- Bullet 3's surviving three clauses re-checked against `VoiParams` and
  `VoiParamsError` rather than assumed unaffected.
- The "and nothing else" phrasing traced to its precedent at `voi.rs:70-71` and
  its assertion at `voi_shader.rs:657-665`, so the repair introduced no new
  claim.
- Sentence structure checked against the section's own bullet form.
- Delta confirmed two ways, and everything outside `CHANGELOG.md` and my own
  report is byte-identical to the pass-4 clean tree.

---

## Closing note on the review as a whole

Six passes, twenty findings, and **not one required a change to a `.rs` or
`.wgsl` file's logic**. `ocelli-pixel`, `ocelli-cache` and
`crates/ocelli-render/shaders/voi.wgsl` are byte-identical to what the three
stories delivered. What the review changed was prose, comments, two
documentation indexes, one ledger field and nine lines of the register.

The two findings that mattered most were both of the same kind and neither was
reachable from a single story's diff: a claim that was true when written and
was falsified by a story that landed two commits later, and a claim that no
gate reads and that had been false for eight sprints. The last two passes found
them only because `CHANGELOG.md` was read by a person, which
`scripts/prose_check.py:18` guarantees no gate will do.

S11 is sound. Close it.

**Tree state**: unchanged. `git write-tree` returns
`7a9f2ca780d92257e767e62eca75437a34ee089c`. No mutation was made. The only
working-tree addition is this report.
