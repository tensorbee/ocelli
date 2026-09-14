# S09 whole-sprint review, pass 2

**Reviewed**: the sprint diff after pass 1's remediation, staged tree
`cc1d48561b90`, which is the tree `gate --sprint` verified at 30 gates green
after the remediation rather than before it.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## What pass 1 raised, and what closed it

### D1, `CLAUDE.md`'s claim that no pixel, LUT or geometry code exists

Closed. The paragraph now says what is true and says why the old sentence
survived, because a correction that deletes the error without recording how it
lasted two sprints teaches nothing. The narrower claim that D7 actually asks is
stated in its place: the oracle existed before the code, and it still has no
Ocelli renderer to compare against.

Three further stale claims in the same section were corrected in the same edit.
**Two counts were removed rather than updated**, the corpus row count and the
shared-crate count, each replaced by the command that prints it. That is the
shape `CLAUDE.md` already applies to the deviation register and to the tier-C
story range, and this section had two more instances of the pattern it warns
about elsewhere in its own text.

### S1, no test registered the five decoder families together

Closed by `every_decoder_family_registers_into_one_registry_without_collision`
and `registering_a_family_twice_refuses_and_leaves_the_others_intact` in
`crates/ocelli-codec/tests/registry.rs`.

**The first asserts the exception by name rather than by count.** Deflated
Explicit VR Little Endian is unavailable by design under D-18, and the test says
so and asserts the reason, because a bare "15 of 16" would let a later story
close the gap by registering a frame decoder for a whole-data-set encoding and
still read as an improvement.

## What pass 2 found

### A false claim I wrote while remediating D1, caught before it was committed

Pass 1's evidence table listed `bin/ocelli.sh`'s "eleven shared crates" as a
discrepancy against `ls crates | wc -l` being 13, and the first `CLAUDE.md` edit
repeated that as fact. **It is not a discrepancy.** Thirteen crates minus
`ocelli-native`, which the step excludes because it is native-only, minus
`ocelli-wasm`, which the same line names separately, is exactly eleven.

The arithmetic was done only when the sentence was about to be committed. Both
the review table and `CLAUDE.md` are corrected, and the row records that it read
as a discrepancy first. **This is D1's own defect class reproduced inside D1's
remediation**, three paragraphs after writing that the file's problem was a
claim nobody executed, which is worth recording rather than quietly fixing.

## Verified clean

- **`bin/ocelli.sh gate --sprint` is green at 30 gates on the remediated tree**,
  `cc1d48561b90`, and that verification was recorded in both evidence stores
  after the remediation rather than carried over from before it. A clean review
  for one tree and a passing verification for another are not closure evidence.
- **The two new registry tests are falsifiable**, checked by probe rather than
  by reading. Each mutation was reverted and the suite re-run green:

  | Mutation | Result |
  |----------|--------|
  | `jpegls` declares a JPEG 2000 UID, so two families collide | 14 passed, 2 failed |
  | `htj2k` silently stops registering `.203` | 14 passed, 2 failed |

  The second matters as much as the first: a family that stops claiming a UID
  is as much a regression as one that claims another's, and a collision-only
  test would have caught just one direction.
- **Every claim in the corrected `CLAUDE.md` section was executed**, not read.
  The corpus row count, the crate count, the available-syntax count and the
  Deflated exception were each measured against the tree during this pass.
- **The whole sprint's arithmetic risks are each held by a mutation observed
  red**: 5 for F-029, 8 for F-020, 11 for F-028, 9 for F-027 and 2 more here,
  35 in total, each reverted with the tree re-run green afterwards.
- **No tolerance was widened, no `#[allow]` added, no gate disabled and no
  allow-list extended anywhere in the sprint.** The one guard that changed,
  `scripts/unsafe_allowlist_check.py`, was made stricter, and the one new
  refusal that fails today does so deliberately and is named in `/release`,
  `docs/SOURCE-POLICY.md` and D-22.
- **`python3 scripts/sprint_workflow.py status` reports four stories completed
  and no story pending or carried.**
