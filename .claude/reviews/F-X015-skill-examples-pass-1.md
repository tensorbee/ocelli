# F-X015 executable skill examples review, pass 1

**Reviewed**: staged diff from `6eca661` in `/private/tmp/ocelli-f-x015`, 15
paths, 762 insertions and 148 deletions
**Result**: 3 defects, 1 smell, 0 nitpicks

## Defects

### D1, The marked VOI example never evaluates SIGMOID

**Where**: `.claude/skills/dicom-tooling/SKILL.md:130`

**What**: The marked block defines `voi_sigmoid`, but its only executable loop
prints `voi_linear` and `voi_linear_exact`. No assertion or output expression
calls `voi_sigmoid`.

**Why it is wrong**: The approved plan's fixture row explicitly says the
marked VOI outputs cover PS3.3 C.11.2.1.2, C.11.2.1.3.2 and C.11.2.1.3.1. The
last citation is SIGMOID. Merely compiling a function body proves syntax, not
its arithmetic or precondition. A quietly wrong SIGMOID example would remain
green, which is the precise class F-X015 exists to close.

**Evidence**: I parsed the committed `dicom-tooling-voi` example, changed the
SIGMOID exponent from `-4 * (x - c) / w` to `4 * (x - c) / w`, and executed
the mutated `Example` through the production runner. It stayed green. I then
changed only SIGMOID's `assert w > 0` to `assert w < 0`. That mutation also
stayed green. By contrast, changing LINEAR's `w - 1` to `w + 1` failed with
`stdout differs`, and corrupting the stored-value sign bit failed with exit 1.

**Required repair**: Exercise SIGMOID with independently computed values that
make both the exponent sign and width precondition observable. Add semantic
mutation evidence showing each wrong form turns the marked example red.

### D2, A symlinked canonical skill root can escape the repository and execute external code

**Where**: `scripts/skill_examples_check.py:134`

**What**: `canonical_skill_paths` resolves the supplied `skills` directory and
then treats that resolved destination as the trusted root. It checks child
paths relative to the destination. If `.claude/skills` itself is a symlink,
every external child is therefore accepted.

**Why it is wrong**: The checker is an execution boundary. Its contract is to
run explicitly marked examples from this repository's canonical skills, not
from whichever directory a root symlink names. The child-symlink test covers a
different case and does not protect the root.

**Evidence**: In a temporary directory I made `skills` a directory symlink to
an external tree containing `outside/SKILL.md` and one marked example.
`canonical_skill_paths(skills)` returned the external path, and
`check_paths(...)` executed it and returned `outside`. The existing
`test_canonical_skill_symlink_may_not_escape` remains green because its root is
a real directory and only a child escapes.

**Required repair**: Anchor the canonical root to the repository before any
resolution and refuse when that root itself is a symlink or resolves outside
the expected repository path. Add separate root-symlink and child-symlink
regressions.

### D3, Fence state does not recognise valid blockquoted fenced documentation

**Where**: `scripts/skill_examples_check.py:33`,
`scripts/skill_examples_check.py:78`

**What**: The unmarked-fence state recognises only a raw fence indented by zero
to three spaces. It does not recognise a CommonMark fenced block nested in a
block quote. Marker-shaped text inside that inert documentation is then parsed
as an outside-fence near-marker and refused.

**Why it is wrong**: The implementation handoff and LLD claim marker-shaped
text inside unmarked Markdown fences is ignored. A skill may legitimately
quote a worked marker example. The gate must distinguish that documentation
from a live column-zero declaration. Failing closed on a valid inert block is
safe from execution, but it is still the wrong parser result and makes the
document grammar narrower than the approved contract.

**Evidence**: I supplied this valid blockquoted text before a normal live
example:

```text
> ```text
> <!-- ocelli-example: id=inert interpreter=bash mode=no -->
> <!-- /ocelli-example -->
> ```
```

`parse_skill` refused line 2 with `example marker must use the exact
column-zero grammar`. A four-space list-nested fence failed the same way. The
existing inert-fence test covers only a top-level fence.

**Required repair**: Either implement the promised Markdown nesting needed to
identify inert fences, or explicitly narrow and document the supported fence
grammar and provide a representation for quoted marker examples. Add the
blockquoted case as a regression.

## Smells

### S1, The new expert example expands an already oversized triggered skill

**Where**: `.claude/skills/dicom-expert/SKILL.md:278`

The canonical skill-creator guidance says to keep a skill body under 500 lines
and use progressive disclosure as it approaches that size. `dicom-expert` was
already over that threshold and this change adds a second copy of both VOI
functions directly to the triggered body. It is now 613 lines. The independent
check is useful, but the placement spends another 37 always-loaded lines on
arithmetic already shown immediately above and again in `dicom-tooling`.
Consider keeping the concise independent assertions beside the table while
moving reusable implementation detail to one directly linked reference or
script that the checker can still execute.

## Nitpicks

None.

## Independent evidence

The exact-output implementation captures bytes. It does not use `text=True`.
An example writing `b"ok\r\n"` against declared `ok\n` failed with `stdout
differs`. An example writing `b"ok\xff\n"` failed the same way. Invalid output
was replacement-decoded only for its bounded diagnostic after comparison, so
neither CRLF nor invalid UTF-8 was accepted.

The checker parses the complete supplied path list before invoking its runner.
Its fixed subprocess argument vector is `python3 -I -B -`, code travels on
stdin, no shell is requested, each example receives a fresh temporary working
directory, and the environment contains only `HOME`, `LANG`, `LC_ALL`, `PATH`
and `TMPDIR`. Nonzero output and timeout diagnostics were bounded by the
existing adversarial suite.

The LINEAR values independently reduce to `17000 / 133` at 40 HU and
`8500 / 133` at -60 HU, which are 127.81954887218045 and
63.909774436090224. Python's three-place formatting correctly produces
127.820 and 63.910. LINEAR_EXACT produces 0, 127.5, 255 and 63.75 for the four
declared inputs. The expert skill intentionally shows the HLD's truncated
127.819 and states why the tooling output rounds to 127.820. The stored-value
example correctly extracts and sign-extends the declared 12-bit right-aligned
and left-aligned cases.

Both changed adapters match their canonical sources. The measured canonical
SHA-256 values are
`914ceb32fe34f06014e61e15f2cf99a7d99125dfe7668d3727396414510945d2`
for `dicom-expert` and
`5e02aab9c39b8225105e8b327fa085bea78aba196d1052b32770bc7209a85e3f`
for `dicom-tooling`, exactly matching the staged adapters. Both canonical
folders pass the skill-creator validator.

`bin/ocelli.sh gate skills` passed all 20 adapter hashes, three marked
examples and 21 checker tests. The CI registration check proved the named
`skills` gate is invoked. The guard harness drove
`skill-examples.changed-expected-digit` red for `stdout differs`, completed
160 refusal probes, 24 accept probes and 39 controls, and reported no open
harness defects. The census reported 647 refusals, 247 probes and zero
uncovered sites. Its generated runbook row 151 names the new probe, and the
budget records 11 `skill-examples` refusal sites. The standalone catalogue
suite passed 56 tests. Prose and staged-content checks passed.

No implementation file, generated artifact, verification record, commit,
integration or remote state was changed by this review.
