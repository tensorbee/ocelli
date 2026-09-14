# S10 sprint review, pass 1

**Reviewed**: the whole sprint diff, `git diff main...HEAD`, 33 files.
**Result**: 6 defects, 0 smells, 0 nitpicks

Every finding is a false claim in tracked prose or a data file changed in a way
the diff cannot be reviewed through. None is arithmetic: the arithmetic was
reviewed three times at the feature level and eleven mutations were observed
red. That is the expected shape for a sprint of one story, and it is also the
defect class `/microscope` calls the highest-volume one in a documentation-heavy
workflow.

## Defects

### D1, `tools/oracle/metadata-truth.json` was mechanically reformatted

**Where**: the whole file.

**What**: the three new entries were added with a Python `json.dumps(indent=2)`
round trip, which exploded every compact one-line object in a hand-maintained
file. The diff was **637 lines changed for three added entries**, with 529
deletions, none of which changed a value.

**Why it is wrong**: the file's own header says it is hand-authored, and its
layout puts one `{ "scope": ..., "value": ... }` per line so a reviewer can read
a row at a glance. More seriously, a reviewer cannot find three real additions
inside 529 deletions, so the change becomes unreviewable in exactly the file
that holds the oracle's hand-computed truth. A green gate does not help: the
values were identical, so every check passed.

**Evidence**: `git diff main --stat` showed `637 ++++----`. After the repair the
same command shows **37 insertions and zero deletions**, and
`git diff main -- tools/oracle/metadata-truth.json | grep -c "^-[^-]"` returns
`0`.

**Remediated**: the file was restored from `main` and the three entries inserted
textually in the file's own style. `bin/ocelli.sh gate oracle` re-run green on
the restored file, so the repair is verified rather than assumed.
`tools/oracle/compare-expectations.json` was checked for the same defect and is
clean: nine insertions, no deletions.

### D2, `CLAUDE.md` claimed the pixel pipeline is stages 1 to 3

**Where**: `CLAUDE.md`, the "Current state" list.

**What**: "**The pixel pipeline**, stages 1 to 3 of DICOM PS3.3 C.11 ... Palette
and ICC, stage 4, are not implemented." Both halves were false the moment F-030
landed.

**Why it is wrong**: `CLAUDE.md` is loaded into every session as the statement
of what exists. It carries a paragraph of its own explaining that this exact
sentence class went stale through S08 and S09 and survived four clean per-story
reviews. Letting it go stale again in the sprint that completes the chain would
be the same failure with a shorter interval.

**Remediated**: the line now says all four stages, names ICC as the part that is
still not implemented, and records what it previously said so the next reader
can see the correction rather than a confident new claim.

### D3, `docs/lld/README.md` described the pixel pipeline as modality and VOI only

**Where**: `docs/lld/README.md`, the index row for `pixel-pipeline.md`.

**What**: the row read "Image-plane evidence, stored-value extraction, modality
and VOI mapping | F-018".

**Why it is wrong**: **this was already stale before S10.** F-029 added the
presentation stage and did not update this index row, so the row omitted one
completed stage before F-030 omitted a second. `/complete-feature` step 9 item 4
asks whether the file is referenced from its area README, and the answer was yes
while the description of it was wrong.

**Remediated**: the row now names all four stages and credits F-018, F-029 and
F-030.

### D4, `CURRENT_SPRINT.md` said the multi-component JPEG-LS row is owed

**Where**: `docs/sprints/CURRENT_SPRINT.md`, "What is carried in".

**What**: the bullet said the row is owed and left its placement to the design
plan. The operator answered that question in the design round and the row landed
in this sprint.

**Remediated**: the bullet records that F-030 added it, names the row, and keeps
the distinction the row does not close, which is that multi-component
**decoding** is still unmeasured and is a codec story.

### D5, `CURRENT_SPRINT.md` said there is no palette case in the corpus

**Where**: the corpus coverage table and the two paragraphs under it, plus the
section heading.

**What**: the table row read `| **PALETTE COLOR** | **none** |`, the heading
said the corpus covers three of four halves "and not the fourth", and the
paragraphs planned the weaker route of an unmanifested synthetic fixture.

**Why it is wrong**: all three are now false, and the last is wrong in an
interesting way. The section reasoned that a manifest-backed row was
unavailable, when the only obstacle was that adding one meant regenerating the
whole layer. Leaving the paragraph would preserve a conclusion whose premise the
sprint removed.

**Remediated**: the table names both rows, the heading says F-030 added the
fourth, and the paragraphs record that the weaker route was planned and turned
out to be unnecessary, plus the one thing deliberately left out of the corpus,
the 8-bits-per-entry LUT Data packing, and why.

### D6, two stale counts in `CURRENT_SPRINT.md`

**Where**: the status table and "Dependency order".

**What**: the story table said `pending`, and the closing line said "Fourteen of
M2's fifteen stories are done and F-030 is the fifteenth".

**Evidence**: counted rather than assumed. `allocation.json` has fifteen
stories with `milestone` `M2`, F-016 through F-030, and all fifteen read `done`
in `BACKLOG.md`.

**Remediated**: the status cell reads `done`, and the closing line states all
fifteen with the method used to count them, so the next reader re-derives it
instead of trusting a number.

## Smells

None.

## Nitpicks

None.

## Verified clean

**Every file in the diff was checked for the D1 defect**, by counting deletions
per file. Only six files delete anything at all, and each deletion is a line the
change deliberately replaces: `ci/guard-probe-budget.json` one recorded count,
`BACKLOG.md` one status cell, `docs/runbooks/guard-verification.md` one
generated table row, and the three LLD files whose stale prose was replaced.
`CURRENT_SPRINT.md`'s 110 insertions and 107 deletions are from commit
`b97a810`, the sprint-open rewrite, and not from this story. Verified with
`git log main..HEAD -- docs/sprints/CURRENT_SPRINT.md`.

**No patient data and no DICOM is tracked.** The three new corpus cases are
under ignored `corpus/data` and only their manifest rows are committed. The one
new binary in the diff, `jpegls_corpus_rgb8.jls`, is a bare codestream and not a
DICOM file: it has no preamble and no `DICM` at byte 128, and
`bin/ocelli.sh gate content` passes over the staged tree.

**The guard census moved for a stated reason.** `corpus-synth` went from five
refusal sites to six, the sixth being `--case`'s unknown-name refusal. The
catalogue entry names it, explains why it is covered by a unit test rather than
a sandbox probe, and the budget was re-recorded in the same change, which is
exactly what the census asked for.

**A guard was loosened, and the loosening was tested.** `quirk_check.py` now
follows reachability from `generate` transitively instead of reading direct
calls. That accepts strictly more than before, so the check that matters is
whether it still rejects: dropping `case_sigmoid_width_half` from
`case_callables` turns it red. Verified by mutation.

**Ledgers agree.** `scripts/backlog_check.py` green at 45 done,
`scripts/sync_agent_skills.py --check` green at 20 adapters,
`scripts/deviation_check.py` green at 23 deviations with every citation
resolving, `scripts/prose_check.py` green over 291 files.

**Provenance.** The F-030 commit carries
`Ocelli-Verify: profile=sprint gates=all corpus=pass tree=455b2eacd03a` and
`Ocelli-Generated-By: claude-opus-5`, both written by the pre-commit hook from
the verify ledger rather than by hand.
