# F-217 review, pass 5

**Reviewed**: `fdf182d` (the remediation of pass 4 D1) on branch
`work/f-217-xai`, worktree `ocelli-wt-f-217`, read against `634704f`.
HEAD `fdf182de264d308f798a49b7b6bb36375513c41a`, tree
`e04c4a9f2271ef337ea5c28deca88a285139c1eb`, both the same at the start and
the end of the pass. Working tree clean apart from untracked
`.claude/reviews/F-217-working-pass-1.md` to `-4.md`, the `tools/oracle/out`
symlink and this file, which is the only path the pass added.
**Author / reviewer**: remediation by the lead (commit `fdf182d`). Reviewed
by Claude Opus 5.5 as a fresh independent reviewer that wrote no part of the
implementation, the remediation, the plan or its amendment.
**Result**: 0 defects, 0 smells, 0 nitpicks new. Pass 4 N1 and N2 stand as
written there.

## Acceptance challenged

Every plant and mutation ran in a `git clone` of `fdf182d` under
`review-f217-5/tree` in the session scratchpad, through the real caller
`python3 scripts/licence_check.py python`. `ci/licences.toml` and
`scripts/licence_check.py` were restored and compared equal after each case,
and the clone's `git status` was empty at the end.

**Pass 4 D1, a lead waiver recording a copyleft OR alternative.** Closed.

| Plant | Expected | Observed |
|-------|----------|----------|
| baseline | accepted | exit 0 |
| et-xmlfile `"MIT OR GPL-3.0-only"`, `lead` | refused | exit 1, `is not an allowlisted expression a lead may waive` |
| same, `operator` | accepted | exit 0 |
| et-xmlfile `"GPL-3.0-only"`, `lead` / `operator` | refused / accepted | exit 1 / exit 0 |
| lzf `"BSD-2-Clause OR GPL-2.0-only"`, `operator` | accepted | exit 0 |
| lzf `"BSD-2-Clause OR GPL-2.0-only"`, flipped to `lead` | refused | exit 1, lead message |
| lzf `"BSD-2-Clause"`, flipped to `lead` | accepted, the human rule | exit 0 |
| et-xmlfile `"GPL-3.0-or-later WITH GCC-exception-3.1"`, `lead` / `operator` | refused / accepted | exit 1 / exit 0 |
| et-xmlfile `"Apache-2.0 WITH LLVM-exception"`, `lead` | accepted, allowlisted as a compound | exit 0 |
| et-xmlfile `"MIT OR Apache-2.0 WITH LLVM-exception"`, `lead` | accepted | exit 0 |
| et-xmlfile `"MIT WITH LLVM-exception"`, `lead` | refused, the compound is not listed | exit 1 |
| et-xmlfile `"Apache-2.0 with LLVM-exception"` (lower-case operator), `lead` | refused | exit 1 |
| et-xmlfile `"MIT+"`, `"GPL-2.0+"`, `lead` | refused, `+` compounds are not listed | exit 1, exit 1 |
| et-xmlfile `"GPL-2.0+"`, `operator` | accepted | exit 0 |
| et-xmlfile `"(MIT OR Apache-2.0) AND BSD-3-Clause"`, `lead` | accepted | exit 0 |
| et-xmlfile `"(MIT OR Apache-2.0) AND GPL-3.0-only"`, `lead` | refused | exit 1 |
| et-xmlfile `"MIT OR Apache-2.0 OR LGPL-2.1-or-later"`, `lead` | refused | exit 1 |
| et-xmlfile `"MIT OR Apache-2.0"`, `lead` | accepted | exit 0 |
| et-xmlfile `"MIT or Apache-2.0"`, `lead` | refused | exit 1 |
| et-xmlfile `"python-2.0"`, `lead` | accepted, ids are case-insensitive | exit 0 |
| et-xmlfile `"MIT OR"`, `lead` / `operator` | refused / refused unparseable | exit 1 lead message / exit 1 `is not a parseable SPDX expression` |
| et-xmlfile `"GPL-3.0-only"`, `decided_by = "Operator"` | refused | exit 1, lead message |

**The dependency check is untouched.** `expression_allowed` still calls
`satisfiable` and is still the reader at `scripts/licence_check.py:655` and
`:688`. The commit changes only the waiver branch. `deny.toml` and `ci/` are
unchanged since `634704f`. For r-efi's expression
`MIT OR Apache-2.0 OR LGPL-2.1-or-later`, `expression_allowed` returns
`True` and `every_term_allowed` returns `False`, and `licence_check.py rust`
exits 0, so the dependency is still admitted while the same text would be
refused as a lead waiver.

**Prose at the changed sites tells the truth.** The docstring of
`licence_file_waiver_problems` (`scripts/licence_check.py:343-349`) now says
every term under AND and under OR. The refusal comment (`:378-379`, "Only an
operator may name a licence outside the allowlist") and the probe note
(`scripts/guards/catalogue.py:7139-7141`) now agree with the OR plant's
refusal. A search of `scripts`, `docs`, `ci`, `.claude/plans` and
`README.md` for "OR needs", "AND needs every" and similar finds no stale
statement of the old rule. The plan amendment
(`.claude/plans/F-217-design.md:198`) and the `ci/licences.toml:121-128`
comment already said the stricter rule and are unchanged.

**Mutations.** Unmutated, `scripts/tests/test_licence_check.py` runs 31
tests, OK.
- M1, the waiver call changed back to `satisfiable(...)`: 1 failure, and the
  OR plants for et-xmlfile and lzf under `lead` return to exit 0. This is
  the lead's reported mutation, reproduced.
- M2, the `or` branch of `every_term_allowed` given satisfiable semantics:
  1 failure.
- M3, the leaf test replaced by `return True`: 1 failure.

The guard probe `licences.waiver-lead-copyleft` plants a single
`GPL-3.0-only`, which both semantics refuse, so it watches the append site
and the unit test is what watches the OR semantics. The unit test runs in
`gate licences`.

**Census.** The commit adds no refusal site. `guard_census.py` 1032
refusals in 77 files, 0 watched by nothing, the same as pass 4, and the
`licences` budget stays 75.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| waiver plants (table above) | scratch clone of `fdf182d`, `licence_check.py python` | now | as tabled |
| M1 to M3 and the unit file | scratch clone, `python3 -m unittest scripts/tests/test_licence_check.py` | now | 31 OK unmutated, 1 failure under each mutation, source restored and compared equal |
| `licence_check.py rust`, r-efi expression through both functions | scratch clone | now | 0, `True` and `False` |
| `gate licences` | worktree `fdf182d` | now | 0, `OK: licence check (rust, npm, python, notices, holder)`, 31 unit tests |
| `gate licences-wheels` | worktree `.venv`, darwin arm64 | now | 0, `OK: licence check (installed)` |
| `gate guards` | worktree | now | 0, 392 refusal probes, 36 guards red, 42 accept, 65 controls |
| `gate guards-deep` | `CARGO_TARGET_DIR=<worktree>/target` | now | 0, 442 refusal probes, 39 guards red, 57 accept, 69 controls |
| `gate corpus-tests`, `gate corpus` | canonical `corpus/data` through the symlink | now | 0, `OK: 17 syntax decode digests match` |
| `gate ci prose provenance content skills lint types packages backlog deviations fmt` | worktree | now | all 0. ci 27 floor gates, prose 820 files, packages 2 at 0.1.0, backlog 205 planned F-IDs, deviations 27 |
| `gate clippy`, `gate test` | own target dir | now | 0. test 146 result lines, 1442 passed, 0 failed, 82 ignored |
| `npm run test` | worktree | now | 0, 57 files, 468 passed |
| `guard_census.py`, `--check-runbook` | worktree | now | both 0, 1032 refusals in 77 files, 0 watched by nothing |
| `sync_agent_skills.py --check` | worktree | now | 0, 42 Codex and 19 OpenCode adapters match |
| `pin_and_size_check.py --require-redistribution` | worktree | now | 0 |
| lead's mutation receipt | lead report | prior receipt | consistent with M1 here, not relied on |

Logs are under `review-f217-5/logs` in the session scratchpad.

## Defects

None.

## Smells

None.

## Nitpicks

None new. Pass 4 N1 (the placeholder on a row with neither `empty` nor
`markers` is refused only by `gate licences-wheels`) and pass 4 N2 stand as
written there.

## Verified clean

- Pass 4 D1 is closed through the real caller, for both triggers and for
  every compound shape the allowlist can name: OR, AND, nested groups,
  `WITH` and `+`.
- Operator rows are unchanged in behaviour: any parseable expression, a
  `WITH` and a `+` compound included, is accepted, and an unparseable value
  is still refused with the parse message.
- The dependency admission rule is still `satisfiable`, and the r-efi
  three-way OR is still admitted.
- The docstring, the refusal comment and the probe note agree with the
  observed behaviour, and no other prose states the old rule.
- The new unit assertions go red under three independent mutations of the
  new function and its call site.
- The full gate set listed in the brief, run one gate at a time, exits 0.
- No GPU, browser, wasm boundary, `unsafe`, cast or pixel arithmetic is in
  the diff, so microscope sections 1 and 5 have nothing to apply to.

## Limitations and handoff

- A lead row with an allowlisted `licence` over a text whose marker is a
  real copyleft grant (lzf flipped to `lead` with `licence = "BSD-2-Clause"`)
  still passes. As pass 4 said, that is a human rule, listed for the operator
  in `CURRENT_SPRINT.md`.
- AC17's Linux half remains open until the `corpus-tooling` CI job runs.
- `gate --sprint` was not run as one profile. Its member gates in the brief
  were run one at a time as listed, all exit 0.
- This is a clean bounded review of `fdf182d`. It is not a verification
  receipt and does not complete the story. Next owner: the lead, for
  `/verify` and `/complete-feature`.
