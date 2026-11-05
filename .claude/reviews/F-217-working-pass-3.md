# F-217 review, pass 3

**Reviewed**: `git diff 690ef19..987c16a` on branch `work/f-217-xai`,
worktree `ocelli-wt-f-217`, the whole story as it would land, with focus on
`git diff c82f7e9..1e44c21` (the remediation of passes 1 and 2) and the
merge `987c16a` of `sprint/s46` at `690ef19`. HEAD
`987c16a75503cd7edd520e337eed662f41f4badc` at the start and at the end of the
pass, working tree clean apart from untracked
`.claude/reviews/F-217-working-pass-1.md`, `-2.md`, the `tools/oracle/out`
symlink and this file, which is the only path the pass added.
**Author / reviewer**: implementation and remediation by an external coding
agent (the implementer log in the session scratchpad). The lead
changed the numpy `LICENSE.txt` waiver to `decided_by = "operator"` and wrote
the 2026-11-05 plan amendment and the `CURRENT_SPRINT.md` operator question.
Reviewed by Claude Opus 5.5 as a fresh independent reviewer that wrote no
part of the implementation, the remediation, the plan or its amendment.
**Result**: 2 defects, 0 smells, 4 nitpicks

## Acceptance challenged

Every mutation and plant ran in a `git clone` of `987c16a` under the review
scratchpad (`review-f217-3/tree`, `tree2`), never in the worktree. The guard
sandbox copies the clone's `git ls-files`, so the mutated working tree is what
each probe saw. Source was restored and compared equal after the matrix.

**Pass 2 D1, the six census sites.** Closed. Each site deleted on its own and
the floor probes, the unit file and the direct check re-run. "RED" means the
probe's guard fired, which is the healthy state.

| Mutation | unit | aimed probe | other five new probes |
|----------|------|-------------|-----------------------|
| M3 drop the waiver missing-field append | red | `waiver-decided-by` green | red |
| M4 drop the waiver unknown-scope append | green | `waiver-scope` green | red |
| M5 drop the placeholder append | red | `waiver-placeholder` green | red |
| M6 drop the `is false for` append | red | `scope-dev-only` green | red |
| M7 drop the `holds for no lockfile` append | green | `scope-nowhere` green | red |
| M8 drop the stale licence file append | red | `waiver-stale` green | red |
| M1 drop `problems.extend(scope_problems())` | green | both scope probes green | four waiver probes red |
| M2 drop `problems.extend(licence_file_waiver_problems())` | green | decided-by, scope, placeholder green | `waiver-stale` red (it is in `stale_problems`) |
| M9 dev flag ignored | red | none aimed | all red |
| M10 `_waiver_authorised` bypassed | red | none aimed | all red |
| M11 `_matching_waiver` ignores version | red | none aimed | all red |

Every site now has a probe that goes green when the site is removed, and the
sibling probes stay red, so each probe isolates its site. M4, M7, M1 and M2,
the four pass 2 found watched by nothing, are each now caught by a probe. M9
to M11 are conditions rather than append sites and are watched by the unit
file, which runs inside `gate licences` (29 tests). The direct check on the
unmutated real tree stays green under every mutation, as expected.

**Pass 2 D2, waiver record truth.** Closed. Every waived file read.
- `LICENSE-heif`, `LICENSE-jetraw`, `LICENSE-libjxs`: 0 bytes in the macOS
  `.venv` and in a hash-locked Linux replica (`uv export --locked`, then
  `uv pip install --require-hashes --python-platform x86_64-manylinux_2_28
  --target`). `imagecodecs.HEIF`, `JETRAW` and `JPEGXS` report `available =
  False`, there is no `imagecodecs/_heif`, `_jetraw` or `_jpegxs` module on
  either platform, and no matching library under `.dylibs` or `.libs`. The
  rows record `licence = "no-licence-text"`, so no licence is claimed for
  text that is absent. True.
- `LICENSE-lzf`: BSD-2-Clause text by Marc Lehmann followed by "Alternatively,
  the following files carry an additional notice that explicitly allows
  relicensing under the GPLv2". The only marker found is `GPLv2`, which is
  the row's `markers`. True.
- numpy `LICENSE.txt`, both platforms (SHA-256 `4860083c...e488` Linux,
  `649a589a...4fcf` macOS, 935 lines each, differing only in four `Files:`
  paths). It has a `GCC runtime library` section, `License:
  GPL-3.0-or-later WITH GCC-exception-3.1`, and a `libquadmath` section,
  `License: LGPL-2.1-or-later`, followed by the GPL-3.0 text whose section 13
  is the source of the AGPL marker. The markers found on each platform equal
  the row's five exactly. The reason's licence names and "present on every
  platform" are true for both recorded platforms.
- et-xmlfile `LICENCE.python` (SHA-256 `4ccdaaeb...8ab2`): the preamble says
  et_xml includes stdlib code under the Python licence, then the PSF, BeOpen,
  CNRI 1.6.1 and CWI agreements and the Zero-Clause BSD grant for docs. The
  one marker, `GNU General Public License`, is at line 246 inside CNRI clause
  7, the governing-law clause. `et_xmlfile/incremental_tree.py` opens "Code
  modified from cPython's Lib/xml/etree/ElementTree.py ... Licensed to PSF".
  The `[[python]]` row is now `MIT AND Python-2.0`, both allowlisted. Every
  claim in the row is true.

**Pass 2 D3, `decided_by` against the amended rule.** The six rows read:
lzf `operator`, numpy `operator`, heif, jetraw, libjxs and et-xmlfile `lead`,
and the five `[[exception]]` rows `operator`. lzf and numpy are texts that
grant a copyleft licence (GPLv2 as an alternative, the libgfortran and
libquadmath grants), so `operator` is what the amendment requires. et-xmlfile
is an allowlisted text (Python-2.0 plus 0BSD) that grants nothing copyleft,
so `lead` fits the amendment. `CURRENT_SPRINT.md` lists exactly the four lead
rows and numpy is correctly absent. The three placeholders are D1 below: the
amendment as written does not reach them.

**Pass 2 S1, version-bound waivers.** Closed. Replayed through the real
caller.
- lzf row at `1999.0.0`: `licence_check.py python` exits 1, `stale licence
  file imagecodecs@1999.0.0 imagecodecs/licenses/LICENSE-lzf is not in the
  lock`. `installed_problems` on macOS and on the Linux replica each report
  `copyleft marker in imagecodecs/licenses/LICENSE-lzf: GPLv2`, so the
  waiver stops applying.
- `uv.lock` imagecodecs moved to `2099.0.0`: exit 1 with the stale
  exception, four stale licence files, `has no licence row` and `is not in
  uv.lock`.
- Adjacent, installed version drift: the Linux replica's imagecodecs
  `METADATA` set to `2026.11.1` gives five problems, including all three
  empty files and the lzf marker, because no row matches that version.
- Adjacent, new text on a waived path: `GPLv2` appended to et-xmlfile's
  `LICENCE.python` is refused as an extra marker, and a new empty
  `LICENSE-jpeg` is refused.
- Adjacent, all imagecodecs rows bumped together with the lock: green. That
  is a deliberate record edit, and the installed check then re-reads the new
  wheel's text against the recorded markers, so it is not a hole.

**Nitpick fixes from pass 2.** N1 `_DIST_INFO_LICENCE_PREFIXES` is
`("licen", "copying", "notice")` with a comment that says why. N2 the three
docs now say `pins.openjph-notice-hash` appends a newline, which is what
`_alter_openjph_notice` does (`catalogue.py:358`). N5 the skill says "no
installed plugin". N3 and N4 were left, as recorded.

**Merge.** `git diff --name-only 690ef19 987c16a` equals `git diff
--name-only ed383c7 1e44c21` exactly, so the merge adds no change outside
F-217's files to the story diff, and every F-217 path has the same blob at
`1e44c21` and `987c16a`. The merge brought 21 sprint paths (F-119 code,
plans and reviews, sprint ledgers), none of which F-217 touches. `gate
backlog` is green on the merged tree and `gate test` runs the F-119
`ocelli-volume` suites.

**Whole-story ACs, executed now.**
- AC1: no `pylibjpeg-libjpeg` in `pyproject.toml` or `uv.lock`,
  `imagecodecs==2026.10.10` pinned, `uv lock --check` exit 0.
- AC2 to AC5: `gate corpus-tests` 60 plus 19 tests. AC4 mutation, one
  sample of the `ljpeg` route incremented: `.57` and `.70` red. AC5
  mutation, a second `convert_color_space` on the `jpeg8-rgb` route: `.50`
  red. Both reverted.
- AC6: `gate corpus` prints `OK: 17 syntax decode digests match` after the
  manifest digests. No reader, lock or digest file changed since pass 2.
- AC7: `deny.toml` `all-features = true`, no `exclude-dev`, sixteen allowed
  identifiers, none copyleft. `gate licences` green with cargo-deny 0.19.0.
- AC8, AC9: `gate guards-deep`, see Evidence.
- AC10, AC14: unit tests in `gate licences`.
- AC11 to AC16: floor probes in `gate guards`, all `licences.*` red for
  their reason with controls green, plus the plants above.
- AC17: local macOS `gate licences-wheels` green, and the hash-locked Linux
  replica gives zero problems under `installed_problems(..., "linux",
  "x86_64")`. The CI half is still open.
- AC18: zero `lightningcss` keys and zero MPL entries in all three locks,
  none in `node_modules`, vite 7.3.7 and plugin-react 5.2.0.
- AC19, AC20: vendor `LICENSE` SHA-256 `5ddf5177...1f98`, its bytes and the
  ritk `LICENSE-MIT` bytes are contained in `THIRD_PARTY_NOTICES`, which
  names Michael Knopke and not Tensorbee.
- AC21: `pin_and_size_check.py --require-redistribution` exit 0, probes
  `pins.openjph-notice` and `-hash` in `gate guards`.
- AC22, AC23: four project `LICENSE-MIT` copies read `Tensorbee Limited`, the
  ocelli-wasm ones are symlinks, three `NOTICE` copies are byte-identical,
  `LICENSE-APACHE` blob `d6456956...` unchanged, every `package.json`,
  `pyproject.toml` and Cargo manifest `MIT OR Apache-2.0`, `NOTICE` in both
  `files` lists and in `package_check.py` `REQUIRED`, `gate packages` green.
- AC24, AC25: `grokj2k>=1` planted in `pyproject.toml` refused with the
  message for the codec that must not be used, unplanted tree OK over 1639
  files, 13 provenance unit tests.
- AC26: `gate ci` OK, 27 floor gates invoked by CI.
- AC27: census 1030 refusals, 0 watched by nothing, runbook in sync.
- AC29: README Licence section, `corpus/README.md`, `DEVELOPER_SETUP.md`
  read against the tree. True.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate licences` | worktree at `987c16a`, cargo-deny 0.19.0 | now | 0, `OK: licence check (rust, npm, python, notices, holder)`, 29 unit tests |
| `gate licences-wheels` | worktree `.venv`, darwin arm64 | now | 0, `OK: licence check (installed)` |
| `installed_problems` on a Linux replica | `uv export --locked`, `uv pip install --require-hashes --python-platform x86_64-manylinux_2_28 --target` | now | 0 problems |
| `gate prose provenance content skills lint types packages backlog deviations ci fmt` | worktree | now | all 0. prose 820 files, provenance 1639 files plus 13 tests, packages 468 vitest plus tarball check, backlog 242 F-IDs, deviations 27, ci 27 floor gates |
| `gate corpus-tests`, `gate corpus` | canonical `corpus/data` through the symlink | now | 0, 19 plus 60 tests, `OK: 17 syntax decode digests match` |
| `gate guards` | worktree | now | 0, 390 refusal, 42 accept, 65 controls, 100.6s. The six new probes red |
| `gate guards-deep` | `CARGO_TARGET_DIR=<worktree>/target` | now | 0, 440 refusal, 57 accept, 69 controls. `licences.rust-copyleft` red, `rust-disjunction` green |
| `gate clippy`, `gate test` | same own target dir | now | 0. test: 146 result lines, 1442 passed, 0 failed, 82 ignored, F-119 `tests/pyramid.rs` among them |
| `npm run test` | worktree | now | 0, 468 tests |
| `guard_census.py`, `--check-runbook` | worktree | now | both 0, 1030 refusals, 0 watched by nothing |
| `sync_agent_skills.py --check` | worktree | now | 0, 42 Codex and 19 OpenCode adapters match |
| `pin_and_size_check.py --require-redistribution` | worktree | now | 0 |
| mutation matrix M1 to M11 | scratch clone, unit file, direct check, 8 floor `licences.*` probes | now | as tabled above, source restored and compared equal |
| AC4 and AC5 mutations | scratch clone, worktree `.venv`, canonical corpus | now | `.57`, `.70` red, then `.50` red, reverted |
| S1 plants and adjacent cases | scratch clone, macOS `.venv`, Linux replica | now | as listed above |
| N1 plant, et-xmlfile waiver `licence = "GPL-3.0-only"` | scratch clone | now | `licence_check.py npm python` exit 0, installed 0 problems |
| merge isolation | `git diff --name-only` and per-path blob compare | now | story diff paths equal F-217's paths, no F-217 blob moved |
| implementer red receipts | `grok-F-217-fix2.log` | prior receipt | consistent with M1 to M8 here, not relied on |

## Defects

### D1, the plan amendment does not cover the three empty placeholders the lead decided, while two records say it does
**Where**: `.claude/plans/F-217-design.md:198` (the amendment),
`ci/licences.toml:121-128` (the rule comment), `ci/licences.toml:139`, `:151`,
`:163` (`decided_by = "lead"`), `docs/sprints/CURRENT_SPRINT.md:72-79`
**What**: the amendment lets the lead decide "a `[[licence_file]]` waiver of
a file whose text is an allowlisted licence and grants nothing under a
copyleft licence". An empty file has no text and states no licence, which is
exactly what the rows record (`licence = "no-licence-text"`). So the
amendment does not reach `LICENSE-heif`, `LICENSE-jetraw` or
`LICENSE-libjxs`, and for them the unamended sentence still governs: "written
only with an operator decision recorded in the row". The tracked rule in
`ci/licences.toml` widens it ("A waiver of an allowlisted text that grants
nothing copyleft, **or of an empty placeholder**, records decided_by =
\"lead\""), and `CURRENT_SPRINT.md` says "F-217's plan lets the lead waive a
licence file only when its text is an allowlisted licence that grants nothing
copyleft. Four rows ... use that rule: the empty ... placeholders ... and
et-xmlfile 2.0.0's `LICENCE.python`". Three of those four do not meet the
rule it quotes.
**Why it is wrong**: microscope section 3, a governance record whose claim is
false against the approved contract. The lead's intent is not in doubt and is
not relitigated here. The text that a later reviewer will apply to the next
empty-file waiver does not say it.
**Evidence**: the three files are 0 bytes on both platforms (see D2 replay),
the rows carry `licence = "no-licence-text"`, the quoted sentences read from
the three files at `987c16a`.
**Scope and regression**: records only, no code. Smallest fix, for the lead:
extend the amendment sentence to "a file whose text is an allowlisted licence
and grants nothing under a copyleft licence, or an empty file that states no
licence" so the plan, the `ci/licences.toml` comment and `CURRENT_SPRINT.md`
say the same thing. The alternative is `decided_by = "operator"` on the three
rows once the operator answers.

### D2, the guards LLD's list of recorded constants omits the one this remediation added
**Where**: `docs/lld/guards.md:599-601`
**What**: "Recorded constants added with this gate are the `deny.toml` allow
list, `COPYLEFT_MARKERS`, `EXCEPTION_FIELDS`, the provenance `MANIFESTS` set
and `OPENJPH_LICENCE_SHA256`." The remediation added
`scripts/licence_check.py:PLACEHOLDER_LICENCE` to `CONSTANTS` and to
`ci/guard-probe-budget.json` under guard `licences` (digest
`30cb77a6c643247f`). The enumeration reads as complete and is now one short.
**Why it is wrong**: microscope section 3. A reader auditing what the census
pins for this gate would not look for the placeholder value, which is the
value that stops an empty file from claiming a grant.
**Evidence**: `ci/guard-probe-budget.json` constants keys for `licences`,
`pins` and `provenance` listed at `987c16a`, the LLD sentence read.
**Scope and regression**: docs only. Smallest fix: add `PLACEHOLDER_LICENCE`
to the sentence.

## Smells

None.

## Nitpicks

- N1. `licence_file_waiver_problems` (`scripts/licence_check.py:326`) requires
  a `licence` value but never evaluates it against the allowlist, and nothing
  ties `decided_by` to the operator rule. A lead row with `licence =
  "GPL-3.0-only"` passes every check. The rule is a human one by design, but
  `expression_allowed(licence, allow) or licence == PLACEHOLDER_LICENCE` would
  make the record self-consistent at no cost.
- N2. Two fields carry the same fact. The GPLv2 branch in
  `licence_bytes_problems` (`:686-691`) reads `spdx` and `note`, while the
  field rule reads `licence` and `reason`. The lzf row has both pairs with
  equal values. If they diverge, the gate checks one and the record shows the
  other. Five rows duplicate `reason` into `note` verbatim.
- N3. Plan AC21 still says the second mutation "changes one byte". The probe
  appends a newline and the three docs now say so. The plan is the lead's.
- N4. Pass 2 N3 (`build-only` holds for no npm lock) and N4
  (`synthetic_installed_problems` in the guarded module, rebinding `ROOT`)
  stand as written there. Pass 1 N2 to N7 were not re-reviewed.

## Verified clean

- The six new probes, run alone and in `gate guards`, are red for their
  declared fragment with controls green, and each fragment is unique to its
  site (the sibling stays red when the aimed site is removed).
- `_matching_waiver` resolves the owner for both `<pkg>/licenses/...` and
  `<dist>.dist-info/...` paths, falls back only on the same path at the same
  version, and, by reading rather than by a run, fails closed when an
  import name differs from the distribution name.
- `stale_problems` now covers every `[[licence_file]]` row, including the
  four imagecodecs paths that carry no version.
- The `PLACEHOLDER_LICENCE` constant regex is non-greedy and the census
  digest is the prefix of SHA-256 of `no-licence-text`.
- The release command, `Cargo.toml` comment, `SOURCE-POLICY.md` and
  `docs/lld/codecs.md` redistribution sentences read against
  `check_openjph_redistribution`: the flag is the only caller, it refuses on
  a missing file and on a hash mismatch, and it passes now.
- No GPU, browser, wasm boundary, `unsafe`, cast or pixel arithmetic is in
  the story, so microscope sections 1 and 5 have nothing to apply to.

## Limitations and handoff

- AC17's Linux half remains open until the `corpus-tooling` job runs on a
  pushed head. The replica here used `uv export --locked` hashes with
  `--require-hashes`, which is closer to CI than pass 2's, but it is not CI.
- `gate --sprint` was not run as one profile. Its member gates were run
  individually as listed.
- The folder viewer on port 8742 was not touched.
- Next owner: the lead for D1 (one clause in the plan, or an operator
  answer), implementation or the lead for D2 (one word). Both are prose, so a
  fresh pass need only read the changed sentences and re-run `gate prose`.
  The operator's confirmation of the four lead rows is a sprint question,
  not a review gate.
