# F-X015 executable skill examples review, pass 2

**Reviewed**: staged diff from `6eca661` in `/private/tmp/ocelli-f-x015`, 16
paths, 1164 insertions and 147 deletions
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, Minimal list indentation still prevents return to the live grammar

**Where**: `scripts/skill_examples_check.py:90`

**What**: `_fence_start` checks the zero-to-three-space top-level form before
it checks whether the line is continuation content of the active list item.
For a bullet written as `- item`, the continuation indent is two spaces. A
fence indented by two or three spaces therefore becomes a `top` fence rather
than a `list` fence. Dedenting out of an unclosed list-contained fence does not
end that state, so the following exact column-zero declaration is swallowed.

**Why it is wrong**: The corrected plan and LLD both promise that a valid
list-nested unmarked fence is inert and that leaving its list container returns
immediately to the live column-zero grammar. Two spaces is the minimal valid
continuation indentation for this ordinary bullet shape. The unit regression
uses four spaces, which avoids the precedence error because four spaces cannot
match the top-level fence regex.

**Evidence**: I parsed an unclosed inert fence followed by a live assertion:

~~~~text
- item
  ```text
  inert
<!-- ocelli-example: id=live interpreter=python3 mode=assert -->
```python
assert True
```
<!-- /ocelli-example -->
~~~~

The parser reported `example end marker has no start`. The same reproduction
failed with three spaces and passed with four. A closed two-space list fence
followed by the same live declaration passed because its closing fence happened
to terminate the incorrectly classified top-level state.

**Required repair**: When a list continuation is active, classify its fence
relative to that container before applying the top-level rule. Add two-space
and three-space list regressions that leave an unclosed fence by dedenting to a
live column-zero marker.

### D2, A non-fence backtick line can hide a marked example

**Where**: `scripts/skill_examples_check.py:31`,
`scripts/skill_examples_check.py:90`

**What**: `FENCE_START` accepts any characters after an opening backtick run.
Markdown does not permit a backtick in a backtick fence's info string. The
checker therefore treats text that is not a Markdown fence as an inert fence
and ignores exact declarations inside it.

**Why it is wrong**: The checker is meant to ignore marker-shaped text only
inside unmarked Markdown fences. Over-recognising an invalid opener creates a
false green. An exact marked example can be present in ordinary document text
and never execute.

**Evidence**: I placed an `assert False` example after an opening line made of
four backticks followed by `text` and a trailing backtick. I closed the region
with four backticks, then put one valid passing example after it.
`parse_all` returned only the passing id and `check_paths` returned green. The
initial line contains a backtick in its info text, so it is not a valid
backtick-fenced Markdown opener.

**Required repair**: Reject a backtick opening fence when its info string
contains a backtick. Keep the tilde-fence rule separate because its info string
does not have that restriction. Add a regression where the hidden example
fails and another visible example prevents the empty-selection refusal from
masking the skip.

### D3, The independent expert example does not execute either boundary row

**Where**: `.claude/skills/dicom-expert/SKILL.md:276`

**What**: The compact `Fraction` check assigns `(F(0), F(0))` for row one and
`(F(255), F(255))` for row three, then compares those same literals with zero
and 255 in the expected tuple. It contains neither the inputs -160 and 240 nor
either boundary predicate. Only the two interior rows perform arithmetic.

**Why it is wrong**: The prose immediately above the block says to check the
corrected lower boundary independently from the tooling example. The plan's
pass-1 correction says all four rows use compact exact rational arithmetic.
The marked block can currently pass without establishing that -160 lies on
both lower boundaries or that 240 lies above the applicable upper boundaries.
The pass-1 duplication smell is gone, but the replacement removed the
independent executable evidence for the row named by the example id.

**Evidence**: Lines 284 and 286 introduce the clamped outputs as constants,
and lines 289 and 290 repeat those constants. There is no occurrence of -160
or 240 in the marked code. Mutating an interior denominator from 399 to 400
correctly drove the block red, which confirms that only the interior
arithmetic is live.

**Required repair**: Keep the compact rational form, but derive the boundary
rows from their inputs and the PS3.3 predicates. At minimum, execute exact
checks that -160 satisfies both lower-bound comparisons and 240 satisfies the
declared upper-bound comparisons before assigning the clamped values.

## Smells

None. Pass-1 S1's duplicated VOI implementations were replaced by a compact
`Fraction` block. The canonical expert skill remains 596 lines against the
skill-creator guideline to stay under 500, but that size debt predates this
story and the new implementation copy that made it a pass-1 finding is gone.

## Nitpicks

None.

## Pass-1 disposition and independent evidence

Pass-1 D1 is fixed. The tooling example now evaluates SIGMOID at -60 as
`255 / (1 + e)`, prints `68.580`, and checks the zero-width refusal. Reversing
the exponent sign drove it red with `stdout differs`. Reversing `w > 0` to
`w < 0` drove it red with exit 1. Both mutations are also named catalogue
probes and the guard harness observed both for their declared reason.

Pass-1 D2 is fixed for the requested boundaries. An external directory
symlinked directly at the canonical skills root was refused as `canonical
skills root is a symlink`. A child skill directory symlinked outside a real
canonical root was refused as `canonical skill symlink escapes`. The separate
noncanonical, missing-root and symlinked-parent branches also have unit cases.

Pass-1 D3 is fixed for blockquotes, explicitly closed list fences and a
blockquoted container ending by dedent. An exact declaration after each parsed
as live. D1 above is the remaining valid list-continuation case. D2 is a
separate over-recognition of a line that is not a Markdown fence.

Exact output remains a byte comparison. Actual `b"ok\r\n"` and
`b"ok\xff\n"` output against declared `ok\n` both failed with `stdout differs`.
Invalid UTF-8 is replacement-decoded only after mismatch for the bounded
diagnostic. A malformed later file prevented the runner from seeing an earlier
valid example, confirming parse-all-before-execute ordering. The fixed
subprocess vector remains `python3 -I -B -` with code on stdin, no shell, a
fresh temporary directory, the five-key environment and a five-second timeout.

The stored-value example's three expected values are correct, and mutating its
sign threshold from `1 << (bits_stored - 1)` to `1 << bits_stored` drove it
red. The expert interior fractions reduce to `17000 / 133` and `8500 / 133`.
Mutating their denominator drove the expert block red. The tooling LINEAR,
LINEAR_EXACT and SIGMOID values agree with the transcribed PS3.3 formulas.

`bin/ocelli.sh gate skills` passed all 20 adapter hashes, three marked examples
and 30 checker tests. `bin/ocelli.sh gate ci` proved all 25 floor gates and the
complete `skills` arm are reached. The standalone catalogue suite passed 56
tests. `bin/ocelli.sh gate guards` completed 162 refusal probes, 24 accept
probes and 39 controls with zero open harness defects. The census reported 651
refusals in 62 files, 249 probes and zero sites watched by nothing.

The canonical hashes are
`e9b4df021bb1a1f2d861f97119cba6ebb9f13119e7742d5cf7cac51b7279848a`
for `dicom-expert` and
`e113a108dc24489399e676cc3e6d17cef5b4d0ce5bf513fda6c2ec07b1a17fd0`
for `dicom-tooling`. They match the generated adapters, and both canonical
folders pass the skill-creator validator. The generated budget records 15
`skill-examples` refusal sites. The generated runbook carries all three
mutation rows, and `guard_census.py --check` passed. Prose, staged-content and
source-provenance gates passed.

No implementation file, generated artifact, verification record, commit,
integration or remote state was changed by this review.
