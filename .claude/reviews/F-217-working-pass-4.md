# F-217 review, pass 4

**Reviewed**: `git diff 987c16a..634704f -- . ':!docs/sprints'` plus the
sprint-doc merge, on branch `work/f-217-xai`, worktree `ocelli-wt-f-217`.
That is `fe60d8a` (the remediation of pass 3 N1 and N2, D2, AC21 wording and
the lead's widened amendment) and the merge `634704f` of `sprint/s46` at
`e47b9ba`. HEAD `634704f12e54b3f429d66bcc3c65ad2dc26ddd8d` at the start and
at the end of the pass. Working tree clean apart from untracked
`.claude/reviews/F-217-working-pass-1.md` to `-3.md`, the `tools/oracle/out`
symlink and this file, which is the only path the pass added.
**Author / reviewer**: remediation by an external coding agent (report
`grok-F-217-fix3.log` in the session scratchpad). The amendment widening and
the `CURRENT_SPRINT.md` sentence are the lead's. Reviewed by Claude Opus 5.5
as a fresh independent reviewer that wrote no part of the implementation,
the remediation, the plan or its amendment.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

Every plant and mutation ran in a `git clone` of `634704f` under
`review-f217-4/tree`, never in the worktree. The guard sandbox copies the
clone's tracked files, so the mutated working tree is what each probe saw.
`scripts/licence_check.py` and `ci/licences.toml` were restored and compared
equal after each case.

**Pass 3 D1, the amendment and the empty placeholders.** Closed. The plan
(`.claude/plans/F-217-design.md:198`) now reads "a file whose text is an
allowlisted licence and grants nothing under a copyleft licence, or of an
empty file that states no licence". The `ci/licences.toml:121-128` comment
says "or of an empty placeholder", and `docs/sprints/CURRENT_SPRINT.md:72-80`
says "or when it is an empty file that states no licence". The three say the
same thing. Against every `decided_by`: heif, jetraw, libjxs are `empty =
true`, `licence = "no-licence-text"`, `lead`. et-xmlfile is `Python-2.0`, an
allowlisted text whose one GPL marker is in a governing-law clause, `lead`.
lzf (a GPLv2 relicensing alternative) and numpy (GCC runtime and libquadmath
grants) are `operator`. The five `[[exception]]` rows are `operator`.
`CURRENT_SPRINT.md` lists exactly the four lead rows. The merge `634704f`
adds only `docs/sprints/CURRENT_SPRINT.md` (`git diff --name-only fe60d8a
634704f`), and its text is `e47b9ba`'s.

**Pass 3 D2.** Closed. `docs/lld/guards.md:599-601` names
`PLACEHOLDER_LICENCE`.

**Plan AC21 wording.** Closed. The second mutation now "appends a newline",
which is what `_alter_openjph_notice` does.

**Pass 3 N1, lead licence value.** Implemented in
`licence_file_waiver_problems` (`scripts/licence_check.py:363-380`).
Replayed through the real caller `licence_check.py python`:

| Plant | Expected | Observed |
|-------|----------|----------|
| et-xmlfile `licence = "GPL-3.0-only"`, `lead` | refused | exit 1, `is not an allowlisted expression a lead may waive` |
| same, `decided_by = "operator"` | accepted | exit 0 |
| et-xmlfile `"MIT AND GPL-3.0-only"`, `lead` | refused | exit 1, lead message |
| et-xmlfile `"MIT OR GPL-3.0-only"`, `lead` | refused (D1) | **exit 0** |
| lzf `"BSD-2-Clause OR GPL-2.0-only"`, flipped to `lead` | refused (D1) | **exit 0** |
| et-xmlfile `"SEE LICENSE IN FILE"`, `lead` | refused | exit 1, lead message |
| lzf `"SEE LICENSE IN FILE"`, `operator` | refused | exit 1, `is not a parseable SPDX expression` |
| lzf `"MIT or GPL-2.0-only"` (lower-case operator), `operator` | refused, SPDX D.2 operators are case-sensitive | exit 1, parseable message |
| et-xmlfile `decided_by = "Operator"` with GPL | refused, only the exact `operator` is privileged | exit 1, lead message |
| et-xmlfile `"python-2.0"`, `lead` | accepted, identifiers are case-insensitive | exit 0 |
| et-xmlfile `licence = ""` | refused | exit 1, `is missing licence` |
| et-xmlfile `"no-licence-text"` (markers, not empty) | refused | exit 1, `is valid only on an empty file` |
| heif with `empty = true` removed, placeholder kept | refused | floor exit 0, `installed` exit 1 `empty licence file imagecodecs/licenses/LICENSE-heif` (N1 below) |

**Pass 3 N2, GPLv2 branch.** Closed. `licence_bytes_problems`
(`scripts/licence_check.py:712-721`) reads `licence` and `reason`. No
reader of a `[[licence_file]]` row's `spdx` or `note` remains (the one
`spdx` reader at `:669` is the `[[python]]` table). Reverting the two reads
to `spdx` and `note` turns `test_empty_marker_and_library_rules` and
`test_a_waiver_applies_only_at_its_own_version` red. Replacing the condition
with `False` turns the first red. The lzf `reason` contains "not taken", so
the real row passes.

**The two new probes isolate their sites.** Unmutated clone: both red for
their fragment, controls green.
- Lead append replaced by `pass`: `licences.waiver-lead-copyleft` exits 0
  and the harness fails ("a guard nobody has watched fail"),
  `waiver-operator-expression` and `waiver-placeholder` stay red, unit file
  1 failure.
- Operator append replaced by `pass`: `waiver-operator-expression` goes
  green, the other two stay red, unit file 1 failure.

**Census and runbook.** `ci/guard-probe-budget.json` `licences` 73 to 75 for
the two new append sites. `guard_census.py` 1032 refusals in 77 files, 0
watched by nothing, `--check-runbook` exit 0. The runbook diff is the
re-render's renumbering plus the two new rows (`guard-verification.md:243`
for the lead probe).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `gate licences` | worktree `634704f` | now | 0, `OK: licence check (rust, npm, python, notices, holder)`, 31 unit tests |
| `gate licences-wheels` | worktree `.venv`, darwin arm64 | now | 0 |
| `gate guards` | worktree | now | 0, 392 refusal, 42 accept, 65 controls, both new probes red |
| `gate guards-deep` | `CARGO_TARGET_DIR=<worktree>/target` | now | 0, 442 refusal, 57 accept, 69 controls |
| `gate corpus-tests`, `gate corpus` | canonical `corpus/data` through the symlink | now | 0, 19 plus 60 tests, `OK: 17 syntax decode digests match` |
| `gate ci prose provenance content skills lint types packages backlog deviations fmt` | worktree | now | all 0. ci 27 floor gates, prose 820 files, provenance 13 tests, packages 2 at 0.1.0, backlog 242 F-IDs, deviations 27 |
| `gate clippy`, `gate test` | own target dir | now | 0. test 146 result lines, 1442 passed, 0 failed, 82 ignored |
| `npm run test` | worktree | now | 0, 468 passed |
| `guard_census.py`, `--check-runbook` | worktree | now | both 0, 1032 refusals, 0 watched by nothing |
| `sync_agent_skills.py --check` | worktree | now | 0, 42 Codex and 19 OpenCode adapters match |
| `pin_and_size_check.py --require-redistribution` | worktree | now | 0 |
| waiver plants (table above) | scratch clone, `licence_check.py python`, `installed --python <worktree>/.venv/bin/python` | now | as tabled |
| site deletions and GPLv2 read reversion | scratch clone, probes alone, unit file | now | as listed, source restored and compared equal |
| implementer receipts | `grok-F-217-fix3.log` | prior receipt | consistent with the deletions here, not relied on |

## Defects

### D1, a lead waiver may record a copyleft licence as an OR alternative
**Where**: `scripts/licence_check.py:374-380` (the `expression_allowed`
test for a non-operator row), the docstring at `:329-331` ("OR needs one"),
the comment at `:364-365`, `scripts/tests/test_licence_check.py:417-418`
(asserts `"MIT OR GPL-3.0-only"` accepted for a lead row), and the probe
note at `scripts/guards/catalogue.py:7140-7141`.
**Trigger**: et-xmlfile row `licence = "MIT OR GPL-3.0-only"`, `decided_by =
"lead"`, or the lzf row recorded honestly as `licence = "BSD-2-Clause OR
GPL-2.0-only"` with `decided_by = "lead"`.
**Expected**: refused. **Actual**: `licence_check.py python` exit 0 for both.
**Why it is wrong**: the amendment (`.claude/plans/F-217-design.md:198`)
decides who may waive by what the text grants: "Any waiver of text that
grants a copyleft licence still needs `decided_by = "operator"`." A row whose
own licence value is `X OR GPL-3.0-only` records that the text grants the
GPL as an alternative. That is the lzf case exactly, and the lead and pass 3
put lzf under `operator` for that reason. `expression_allowed` is the
`deny.toml` admission rule, where choosing one disjunct is correct (plan
fact 12, `r-efi`), but admission answers "may we use it", not "may the lead
decide it". The code's own comment says "Only an operator may name a
licence outside the allowlist", and the probe note says "GPL-3.0-only is on
no allowlist, so the lead may not record it", and the OR plant has the lead
naming exactly that. The unit test asserting acceptance encodes the
implementation rather than the amendment (HLD 27.2 R2).
**Evidence**: plants P4 and P4b above, exit 0.
**Scope and regression**: no current row is affected, every lead row is a
single allowlisted id or the placeholder. Smallest fix: for a non-operator
row, require every leaf compound of `parse_expression(licence)` on the
allowlist (a walk over `or` and `and` nodes calling `_compound`), keep the
same message, flip `test_licence_check.py:417-418` to assert the lead
refusal, and say "every term, under AND or OR" in the docstring. The
existing probe still watches the append site. If the lead instead decides
that OR with one allowlisted term is acceptable for a lead row, the
amendment, the comment at `:364` and the probe note must say so, because
today all three say the opposite.

## Smells

None.

## Nitpicks

- N1. The placeholder on a row with neither `empty` nor `markers` is not
  refused at the floor. `licence_file_waiver_problems` skips non-waiving
  rows (`scripts/licence_check.py:340-341`), so removing `empty = true` from
  the heif row passes `gate licences`. `gate licences-wheels` refuses it as
  an empty licence file, so the outcome is safe, but the `ci/licences.toml`
  comment "accepted only together with empty = true" is enforced by the
  installed half only. Moving the placeholder test above the `continue`
  would close it. Pre-existing at `987c16a`.
- N2. Pass 3 N4 (the two pass 2 nitpicks) and pass 1 N2 to N7 stand as
  written there. The `[[licence_file]]` `spdx` and `note` fields are now
  read by nothing, so they are record-only duplicates of `licence` and
  `reason`.

## Verified clean

- The amendment, the `ci/licences.toml` rule comment and the
  `CURRENT_SPRINT.md` question agree, and every `decided_by` fits them.
- Lead and non-operator rows: a single copyleft id, an AND containing one,
  an unparseable value and a case-variant `Operator` decider are refused.
- Operator rows: a copyleft id and a `WITH` compound are accepted, an
  unparseable value and a lower-case operator are refused, the placeholder
  on an empty file is accepted.
- The GPLv2 branch reads `licence` and `reason`, and the new unit cases go
  red when it reads `spdx` and `note` again.
- Each new probe goes green when its own append is deleted, while its
  sibling and `waiver-placeholder` stay red.
- Budget, census and runbook agree at 1032 refusals with nothing unwatched.
- The merge carries one sprint path, no F-217 path.
- No GPU, browser, wasm boundary, `unsafe`, cast or pixel arithmetic is in
  the diff, so microscope sections 1 and 5 have nothing to apply to.

## Limitations and handoff

- A lead row with an allowlisted `licence` over a text whose marker is a
  real copyleft grant (lzf flipped to `lead` with `licence = "BSD-2-Clause"`)
  passes, and cannot be refused mechanically, because et-xmlfile shows a
  marker that grants nothing. That stays a human rule, listed for the
  operator in `CURRENT_SPRINT.md`.
- AC17's Linux half remains open until the `corpus-tooling` CI job runs.
- `gate --sprint` was not run as one profile. Its member gates were run
  individually as listed, all exit 0.
- Next owner: implementation for D1 (or the lead, if the OR reading is
  ruled acceptable, in which case the three texts change). A fresh pass need
  only replay the OR plants, the unit file and `gate licences guards`, and
  re-check the census if a site moves.
