# F-217 review, pass 2

**Reviewed**: `git diff ed383c7` on branch `work/f-217-xai`, worktree
`ocelli-wt-f-217`. Committed HEAD `c82f7e9e56652c83e2ca5c1380a01c4884b7498d`
plus the uncommitted remediation (14 modified paths), focus on `git diff
HEAD`. `git diff HEAD | sha256sum` was
`e5195898bf3ed8c00704dd2f30b6d3b110c5522d3eb2acc620610a0f85c53782` at the
start and at the end of the pass, HEAD unchanged. Untracked paths besides
this file: `.claude/reviews/F-217-working-pass-1.md` and the
`tools/oracle/out` symlink. This file is the only path the pass added.
**Author / reviewer**: implementation and remediation by an external coding
agent (report `grok-F-217-fix1.log` in the session scratchpad). Reviewed by
Claude Opus 5.5 as a fresh independent reviewer that wrote no part of the
implementation, the remediation or the plan. The approved plan
`.claude/plans/F-217-design.md` is the contract.
**Result**: 3 defects, 1 smell, 5 nitpicks

## Acceptance challenged

Every pass-1 trigger was replayed through the real caller
(`python3 scripts/licence_check.py`, `installed_problems` or the gate) in a
git-initialised scratch copy of the working tree, one plant at a time, each
reset before the next, plus at least one adjacent case.

- **D1, stale redistribution claims.** Closed. `git grep` for the stale
  phrases outside `docs/hld/`, reviews and plans finds only historical
  AS_BUILT entries and the superseded, dated 2026-09-13 SOURCE-POLICY row,
  which now has a 2026-11-05 row after it. `--require-redistribution` exits 0
  and no gate arm or workflow runs that flag, as the rewritten text says.
- **D2, wheel walkers.** Closed. Walker returning `[]`: unit tests red (2
  failures) and `licences.wheel-marker` red. `_shared_libraries` returning
  `[]`: unit test red and `licences.wheel-library` red.
- **D3, dist-info root files.** Closed. GPL text appended to
  `openpyxl-3.1.5.dist-info/LICENCE.rst` in a copy of the real site-packages
  is refused. Adjacent: a planted `COPYING` with LGPL text at a dist-info
  root, and an empty `NOTICE` there, are both refused. Removing the root scan
  turns one unit test red. A Linux replica (the ten locked versions installed
  with `uv pip install --python-platform x86_64-manylinux_2_28 --target`) is
  green and the new scan reaches four root files there, including
  `pylibjpeg_openjpeg-2.5.0.dist-info/LICENSE`.
- **D4, skill.** Closed. `ds.pixel_array` raises `RuntimeError` on all four
  DCMTK JPEG rows (`.50`, `.51`, `.57`, `.70`) in the worktree `.venv`, and
  decodes JPEG-LS and RLE. `read_back` exists at
  `scripts/tests/test_corpus_synth.py:94`.
- **S1, waiver fields.** Code closed: a bare markers row is refused and does
  not waive. The records written to satisfy it are not, see D2 and D3.
- **S2, allowlist.** Closed. `Unicode-DFS-2016` present, sixteen identifiers.
  The census refuses both a widening (`GPL-3.0-only` added) and the old
  fifteen-entry list against the new digest.
- **S3, scope.** Closed in behaviour. Original trigger, caniuse-lite back to
  `dev-only`: refused for `tools/oracle/package-lock.json`. Adjacent: `tool`
  alone refused for the root lock. Root entry stripped of `dev` refused with
  both messages. Oracle copy removed refused as "holds for no lockfile".
  Unknown token refused. caniuse-lite added to `@ocelli/core` dependencies
  refused as published runtime closure even with `dev-only, tool`. A package
  present in both the root lock and a `tools/` lock therefore needs a token
  that holds for each copy, and each token must hold somewhere, which is the
  lead's decision. The lack of watchers is D1.
- **N1, workspace key.** Closed. The comment trick, `license.workspace =
  false` and the key moved into `[package.metadata.x]` are refused, and the
  inline form `license = { workspace = true }` is accepted.
- **Question (a), et-xmlfile `LICENCE.python`.** Read in full (298 lines,
  SHA-256 `4ccdaaeb...8ab2`). It is genuinely the CPython licence stack (PSF,
  BeOpen, CNRI 1.6.1, CWI, and the 0BSD documentation grant) behind a
  preamble saying et_xml includes stdlib code under the Python licence. No
  clause grants anything under the GPL. The `GNU General Public License`
  hit is CNRI clause 7 at line 246, a governing-law carve-out for derivative
  works that incorporate GPL material, and the other GPL mentions are the
  history table and its footnote. So the waiver is substantively sound and
  `Python-2.0` is allowlisted. The record is not, see D2 and D3.
- **Question (c), the six census sites.** Measured by mutation, see D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate licences` | worktree, cargo-deny 0.19.0 | now | 0, `OK: licence check (rust, npm, python, notices, holder)`, 25 tests |
| `gate licences-wheels` | worktree `.venv`, darwin arm64 | now | 0 |
| `gate prose provenance content skills lint types packages backlog deviations ci fmt` | worktree | now | all 0. provenance 1631 files plus 13 tests, packages verifies `NOTICE` |
| `gate corpus-tests`, `gate corpus` | canonical `corpus/data` through the symlink | now | 0, 19 plus 60 tests, `OK: 17 syntax decode digests match` |
| `gate guards` | worktree | now | 0, 384 refusal, 42 accept, 65 controls, 98.8s |
| `gate guards-deep` | `CARGO_TARGET_DIR=<worktree>/target` | now | 0, 434 refusal, 57 accept, 69 controls. `licences.rust-copyleft` red, `rust-disjunction` green |
| `gate clippy`, `gate test` | same own target dir | now | 0. test: 145 result lines, 1407 passed, 0 failed, 82 ignored |
| `npm run test` | worktree | now | 0, 57 files, 468 tests |
| `guard_census.py`, `--check-runbook`, `sync_agent_skills.py --check` | worktree | now | all 0, 1028 refusals, 0 watched by nothing |
| `pin_and_size_check.py --require-redistribution` | worktree | now | 0 |
| `vite build` in `examples/viewer-react`, `--outDir` in scratch | vite 7.3.7, plugin-react 5.2.0, vitest 4.1.11 | now | built, wasm chunk emitted. 0 `lightningcss` keys in all three locks and in `node_modules`, 0 MPL entries |
| mutation matrix M1 to M9 on `scripts/licence_check.py` | scratch copy, unit file plus 12 floor licences probes plus the check | now | see D1, source restored and compared equal |
| provenance plants | scratch copy | now | `grokj2k` in `pyproject.toml`, `libgrokj2k` in `uv.lock`, `grok-sys` in `Cargo.lock`, `@x/dwv-core` in the bench lock, a `horos-reader` key in `Cargo.toml` all refused, because those blocked projects must not be used. `grokj2k` in a `.md` accepted |
| agent trailer `Ocelli-Generated-By: grok` in a tracked review file, which names the coding agent and not the codec that must not be used | `ed383c7` archive against the working tree | now | identical on both: refused bare, accepted with a negation in the same line |
| notices and holder | worktree | now | `5ddf5177...1f98` vendor LICENSE, body contained in `THIRD_PARTY_NOTICES` as is the ritk `LICENSE-MIT`, no Tensorbee in that file, four `LICENSE-MIT` read `Tensorbee Limited`, three `NOTICE` copies equal, `LICENSE-APACHE` blob `d6456956...` unchanged, every manifest `MIT OR Apache-2.0` |
| corpus manifest | `git diff ed383c7 -- corpus/manifest.tsv` | now | empty |
| CI workflow | `git diff ed383c7 -- .github/workflows/ci.yml` | now | cargo-deny 0.19.0 installed then `gate licences`, `gate provenance`, `gate licences-wheels` after `uv sync --locked` |
| implementer red receipts | `grok-F-217-fix1.log` | prior receipt | consistent with M4, M5, M6, M8 here, not relied on |

## Defects

### D1, four of the six new refusal sites are watched by nothing, and no probe watches any of them
**Where**: `scripts/licence_check.py:1225-1226` (the two `main` calls),
`:341` (unknown waiver scope), `:449` (scope holds for no lockfile), and the
probe list of the `licences` entry in `scripts/guards/catalogue.py`, which
the remediation did not extend
**What**: census moved `licences` from 65 to 71 sites and counts all 71 as
belonging to an entry carrying probes. No probe drives any of the six.
Mutations, each run against `test_licence_check.py`, the check and the 12
floor `licences.*` probes:

| Mutation | unit | check | probes |
|----------|------|-------|--------|
| M1 delete `problems.extend(scope_problems())` | green | green | green |
| M2 delete `problems.extend(licence_file_waiver_problems())` | green | green | green |
| M3 delete the unknown waiver scope refusal | green | green | green |
| M9 disable the "holds for no lockfile" refusal | green | green | green |
| M4 ignore the `dev` flag | red | green | green |
| M8 bypass `_waiver_authorised` | red | green | green |

With M1 applied, the S3 trigger (caniuse-lite back to `dev-only`) passes
`gate licences`, so the remediation's main effect can be removed without any
gate going red.
**Why it is wrong**: AC27 rejects "a guard added without a probe", and
microscope section 4. The unit tests call `scope_problems` and
`licence_file_waiver_problems` directly, so they cannot see the call sites
that put those refusals into the gate.
**Scope and regression**: present only in the remediation. Smallest fix: two
or three floor probes through `scripts/licence_check.py` in the sandbox, for
example caniuse-lite scope set to `dev-only` (expect `is false for
tools/oracle/package-lock.json`), the oracle copy removed (expect `holds for
no lockfile`), and one `[[licence_file]]` row with `decided_by` deleted and
another with `scope = "prod"` (expect `is missing decided_by` and `is not
dev-only, tool`). Going through `main` also watches M1 and M2. Re-record
census, budget and runbook.

### D2, three waiver records state things that are false
**Where**: `ci/licences.toml:171`, `:193`, `:130`, `:142`, `:154`
**What**:
- `LICENSE-lzf` carries `decided_by = "lead"`. The plan's Q3 answer reads
  "Answered by the operator ... imagecodecs' `LICENSE-lzf` ... is recorded as
  `BSD-2-Clause` ... with its GPLv2 relicensing notice noted as an
  alternative not taken". The decider is the operator.
- The et-xmlfile reason says the file "mentions the GPL only to say Python
  is not distributed under it" and "The package is used under the recorded
  MIT grant". The marker that matched is CNRI clause 7 (line 246), a
  governing-law clause, and the file's own preamble says et_xml includes
  stdlib code under the Python licence: `et_xmlfile/incremental_tree.py`
  opens "Code modified from cPython's Lib/xml/etree/ElementTree.py ...
  Licensed to PSF". Part of the package is used under `Python-2.0`, not MIT.
- The three empty-file rows record `licence = "BSD-3-Clause"`. Those files
  are the slots for third-party codecs that are not built into the wheel
  (`HEIF`, `JETRAW` and `JPEGXS` report `available = False`, no matching
  library under `.dylibs`). Upstream libheif is LGPL-3.0, so the field
  names a licence nobody determined for the thing it waives. It reads as a
  value filled in to satisfy the new field rule.
**Why it is wrong**: these rows are the record of a licence decision, which
the plan makes the operator's evidence (decision 2, "Owner of exceptions").
Microscope section 3.
**Evidence**: the file read in full, the module header, `imagecodecs`
availability, and the plan text quoted above.
**Scope and regression**: remediation only. Smallest fix: `decided_by =
"operator"` on the lzf row. Reword the et-xmlfile reason to "CPython licence
text for stdlib code in `incremental_tree.py`. The GPL mentions are the
history table, its footnote and CNRI clause 7, none a grant", and consider
`spdx = "MIT AND Python-2.0"` on the et-xmlfile `[[python]]` row, both
allowlisted. On the empty rows write what is true, such as "none, the codec
is not bundled".

### D3, the et-xmlfile waiver is a new exception without the operator decision the plan requires
**Where**: `ci/licences.toml:186-196`
**What**: a new `[[licence_file]]` waiver of the strongest marker,
`GNU General Public License`, recorded `decided_by = "lead"`. It is not in
Q7's approved list and no answer covers it.
**Why it is wrong**: the plan, "Owner of exceptions: `ci/licences.toml`,
written only with an operator decision recorded in the row", and Q7, "each
needing an operator decision recorded in its row". After S1, AC16 makes a
waiver "an exception". So "lead" is not an acceptable decider under the
approved plan, even for scope `tool`. The same applies to the empty-file and
numpy `LICENSE.txt` waivers, which no operator answer names. The numpy one
can reasonably be read as part of Q3.
**Evidence**: plan lines 198 and 383 to 395. The waiver itself is sound on
the text, see "Acceptance challenged".
**Scope and regression**: remediation only. Not a code fix. Either the lead
relays the et-xmlfile waiver (and the empty-file rows) to the operator and
records `decided_by = "operator"` with the date, or the lead amends the plan
so that marker or empty-file waivers on allowlisted texts are the lead's
decision. Either way the change must be in a tracked artefact.

## Smells

### S1, `[[licence_file]]` waivers are not bound to their version and never go stale
**Where**: `scripts/licence_check.py:786` (lookup by package and path, version
ignored), `stale_problems` at `:453` (reads `exception_rows()` only)
**What**: every waiver row's `version` was rewritten to `1999.0.0` in scratch.
`installed_problems` on the real site-packages stayed `[]`, and
`licence_file_waiver_problems` and `stale_problems` stayed `[]`.
**Why it is a hazard**: the remediation made these rows exceptions, and AC13
refuses a stale exception. The four `imagecodecs/licenses/*` paths carry no
version, so an imagecodecs upgrade would carry the lzf GPLv2 waiver onto new
text that nobody has re-read, provided the text still trips only `GPLv2`.
The dist-info paths include the version and are safe.
**Smallest fix**: match the row's `version` against the installed dist's
version in the lookup, and report a `[[licence_file]]` row whose
package and version are not in `uv.lock` as stale, with one unit test or
probe.

## Nitpicks

- N1. `_DIST_INFO_LICENCE_PREFIXES` at `scripts/licence_check.py:57` lists
  `"licenc"` beside `"licen"`, which already covers it, and the comment's
  reason for keeping it says nothing.
- N2. `docs/lld/codecs.md:294`, `docs/SOURCE-POLICY.md:191` and
  `docs/lld/guards.md:605` say `pins.openjph-notice-hash` "changes one byte".
  `_alter_openjph_notice` appends a newline. It does refuse on the hash.
- N3. `_scope_holds_for_lock` returns False for `build-only` on every lock,
  so an npm exception with that plan-listed scope can never pass. That was
  Q2's option (b) shape, so if the scope comes back it will fail in a
  confusing way.
- N4. `synthetic_installed_problems` lives in the module under test and
  rebinds the global `ROOT`. It is restored in `finally` and the probes do
  go red under M6 and M7, but a probe fixture inside the guarded file is
  harder to keep independent of it.
- N5. `.claude/skills/dicom-tooling/SKILL.md:36` "pydicom has no plugin" is
  truer as "no installed plugin". pydicom ships GDCM and Pillow decoders for
  some of those syntaxes, and neither is installed.
- Pass-1 nitpicks N2 to N7 were not re-reviewed and remain as written there.

## Verified clean

- AC1 to AC6: the manifest is unchanged against `ed383c7`, `gate corpus`
  verifies the 17 decode digests after file digests, and `gate corpus-tests`
  runs the route, version-pin, lossless and lossy tests. Pass 1's
  independent AC6 recomputation is not repeated, because no reader or lock
  changed in the remediation.
- AC7 to AC10: `gate licences` green with cargo-deny 0.19.0, deep probes red
  and green as declared under this worktree's own target dir.
- AC11 to AC13, AC15: the npm, shipped-closure and scope plants above, plus
  the floor probes in `gate guards`.
- AC16, AC17 local half: macOS gate green, Linux replica green under the
  widened scan, with the three operator-approved runtime libraries and the
  lzf row honoured.
- AC18: Vite 7.3.7 build, no lightningcss, no MPL entry in any lock.
- AC19 to AC23: notices and holder as in the Evidence table, `gate packages`
  green with `NOTICE` required.
- AC24, AC25: prefixed names refused in five manifest grammars, prose and
  agent-trailer behaviour identical on base and tree.
- AC26, AC27: CI wiring as above, census and runbook clean. AC27's probe
  half for the new sites is D1.
- AC29: the D1 and D4 text from pass 1 read sentence by sentence against the
  tree. README Licence section unchanged since pass 1 and still true.
- No GPU, browser, wasm boundary, `unsafe` or pixel arithmetic is touched,
  so microscope sections 1 and 5 have nothing to apply to.

## Limitations and handoff

- AC17's Linux half is still open until the pushed `corpus-tooling` job
  runs. The replica used pinned versions, not `uv sync --locked` hashes.
- `gate --sprint` was not run as one profile. Its member gates listed above
  were run individually.
- The folder viewer on port 8742 was not touched.
- Next owner: implementation adds the D1 probes and fixes the D2 records and
  S1. The lead resolves D3 with the operator or through a plan amendment.
  Then a fresh independent pass.
