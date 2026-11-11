# F-217 review, pass 1

**Reviewed**: `git diff ed383c7..HEAD` on branch `work/f-217-grok`, whose
suffix names the coding agent and not a read-blocked project, HEAD
`c82f7e9` (merge of sprint/s46 `ed383c7` into `02c0804`), worktree
`ocelli-wt-f-217`, clean before this file was written. This file is the only
path the pass added.
**Author / reviewer**: implementation by an external coding agent (commit
`02c0804`). Reviewed by Claude Opus 5.5 as an independent reviewer that wrote
no part of the implementation or the plan. The approved plan
`.claude/plans/F-217-design.md` is the contract.
**Result**: 4 defects, 3 smells, 7 nitpicks

## Acceptance challenged

- **AC4 and AC5 mutations, re-run now** in a scratch copy (`git archive HEAD`),
  because the implementer's handoff says they were not repeated after
  compaction. `ljpeg` route with `decoded.flat[0] += 1`: red on both `.57` and
  `.70`. A second `convert_color_space("YBR_FULL", "RGB")` on `.50`: red,
  `1.0 not less than or equal to 0.03`. An offset of 25 on `.51`: red at
  `0.00659 > 0.005`. Source restored and `cmp` equal to the worktree file.
- **AC6, independently recomputed.** A scratch venv built outside the
  repository from `ed383c7`'s `uv.lock` (pydicom 3.0.2 with
  `pylibjpeg-libjpeg` 2.4.0) decoded all 17 `SYNTAX_CASES` rows through the
  pre-change `ds.pixel_array` path. Same canonical digest form as
  `decode_array_sha256`. 15 of 17 rows are byte-identical to
  `corpus/decode-digests.tsv`, including the four JPEG lossless and three
  JPEG-LS rows. Only `.50` and `.51` differ. New against old: `.50` max 3 in
  4347 of 18432 samples, `.51` max 1 in 339 of 6144, which is the lead's
  figure. Against the uncompressed reference, old and new are both 4 of 255
  (0.015686, bound 0.03) and 2 of 4095 (0.000488, bound 0.005). My own
  `imagecodecs` calls, not the route module, reproduce the TSV digests for
  `.50`, `.51`, `.57`, `.70` and pydicom with `decoding_plugin="pyjpegls"`
  reproduces the three JPEG-LS digests. `corpus_check.py --decode-digests`
  refuses a one-hex-digit change and a deleted row.
- **AC28, every licence reader, re-run now** in a git-initialised scratch copy
  with one plant at a time, each reset before the next. Red and naming the
  item: openjph-core `GPL-3.0-only`, workspace `AGPL-3.0-only`, an oracle-lock
  entry `GPL-3.0-or-later`, a bench-lock entry `LGPL-2.1-only`, root lock
  `MIT AND MPL-2.0`, a reinstated `lightningcss` MPL-2.0 entry, a removed
  `license` field, `caniuse-lite` added to `@ocelli/core` dependencies, an
  exception without `decided_by`, a new `uv.lock` package without a row, a row
  changed to GPL, a row version drift, a stale Python exception, one byte in
  each notice body, `Tensorbee` in the third-party file, each holder copy
  reverted, a filled-in Apache appendix, an `MIT`-only `package.json`,
  `pyproject.toml` and member `Cargo.toml`. Accepted: `MIT OR GPL-3.0-only`.
  Unmodified scratch copy: `OK: licence check (rust, npm, python, notices,
  holder)`.
- **AC16 and AC17, real wheels.** The ten locked manylinux x86_64 wheels (one
  per package in `uv.lock`, so the same files CI's `uv sync --locked` gets)
  were installed with `uv pip install --target` into scratch and passed to
  `installed_problems(site, "linux", "x86_64")`: no problem. This is a local
  replica and does not replace the `corpus-tooling` run AC17 names.
- **Allowlist against the plan.** Decision 1 is the sweep's fourteen
  (scratch `deny.toml` from the sweep: MIT, Apache-2.0, Apache-2.0 WITH
  LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0,
  Unicode-DFS-2016, BSL-1.0, CC0-1.0, Unlicense, 0BSD, MIT-0) plus
  BlueOak-1.0.0 and Python-2.0, sixteen. The tree has fifteen. Every one of
  the fifteen is permissive: `BSL-1.0` is the Boost licence (the Business
  Source licence is `BUSL-1.1`), `Python-2.0` is the PSF licence,
  `Unicode-3.0` and `BlueOak-1.0.0` are permissive, and `Apache-2.0 WITH
  LLVM-exception` only adds permissions. See S2.
- **Composition.** `npm_runtime_closure()` returns the twelve names the
  sweep derived independently for `@ocelli/core`. The census refuses a
  `deny.toml` allowlist widened by `GPL-3.0-only`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate licences` | worktree HEAD c82f7e9, cargo-deny 0.19.0 | now | 0, `OK: licence check (rust, npm, python, notices, holder)`, 17 tests OK |
| `gate licences-wheels` | worktree `.venv`, darwin arm64 | now | 0, `OK: licence check (installed)` |
| `gate prose`, `provenance`, `content`, `skills`, `ci`, `backlog`, `deviations` | worktree | now | all 0. prose 813 files, provenance 1631 files plus 13 tests |
| `gate lint`, `types`, `packages` | worktree node_modules, vite 7.3.7 | now | all 0, packages verifies `NOTICE` in both tarballs |
| `gate corpus-tests`, `gate corpus` | canonical `corpus/data` via the worktree symlink | now | 0, 19 plus 60 tests, `95 verified`, `OK: 17 syntax decode digests match` |
| `gate guards` | worktree | now | 0, 384 refusal probes, 42 accept, 65 controls, 98.4s |
| `guard_probe.py --profile deep --only licences.rust-copyleft` and `rust-disjunction` | worktree, `CARGO_TARGET_DIR` canonical | now | red and green as declared |
| `npm run test` | worktree | now | 0, 57 files, 468 tests |
| `pin_and_size_check.py --require-redistribution` | worktree | now | 0 |
| `guard_census.py --check-runbook` | worktree | now | 0 |
| `uvx --from uv==0.11.19 uv lock --check` | worktree | now | 0, 11 packages |
| OpenJPH LICENSE fetched from tag 0.26.3 | `raw.githubusercontent.com/aous72/OpenJPH/0.26.3/LICENSE` | now | SHA-256 `5ddf5177...1f98`, equal to the vendor file and the notices body |
| `vite build` with `worker.format` removed | scratch copy, vite 7.3.7 | now | fails, `Invalid value "iife" for option "worker.format"`. With it, the worker is an ES chunk using `import(` |
| `npm audit --package-lock-only` | ed383c7 and HEAD root locks | now | base 15 (9 moderate, 6 high), head 14 (9 moderate, 5 high). The one removed is the vite 8 `launch-editor` advisory. None is new |
| base `guard_probe.py --profile floor` | ed383c7 scratch | now | 0, 89.9s on this machine against a recorded 33.0, so the new 103.5 is a measurement and not inflation |
| AC4, AC5, AC6, AC28 mutations and digests | scratch copies listed above | now | see the section above |
| AC4, AC5, AC28 | implementer handoff | prior receipt, not relied on | superseded by the runs above |

## Defects

### D1, three claims the diff made false and left in place
**Where**: `docs/lld/codecs.md:281-293`, `.claude/commands/release.md:33-37`,
`Cargo.toml:119-121`
**What**: The codecs LLD heads the section "The redistribution condition,
which is open", says the archive carries no notice, that
`--require-redistribution` "refuses while that material is absent" and that
`pins.openjph-notice` "fires today, and it stops firing once a notice is
placed". `/release` step 5 says "**It fails today**". The `Cargo.toml`
comment says the package "carries NO LICENCE TEXT" and the condition "is open
today".
**Why it is wrong**: `vendor/openjph-core-0.1.0/LICENSE` is present,
`--require-redistribution` exits 0, and the rewritten probe now refuses only
when the file is deleted or altered. The microscope rule on prose: a change in
the diff must not leave a claim stale.
**Evidence**: `python3 scripts/pin_and_size_check.py
--require-redistribution` printed `OK: wasm-bindgen, wgpu pinned exactly`,
exit 0. The implementer's handoff names the codecs LLD lines and leaves them.
**Scope and regression**: present on HEAD. The plan's write set omits all
three files, so this is also a plan gap. Smallest fix: rewrite the three
passages to the present state in the same words `docs/SOURCE-POLICY.md` now
uses, leaving the D-22 row itself to `/complete-feature` as the plan says.
`docs/SOURCE-POLICY.md:176-177` ("are still a production distribution risk")
reads as present tense too and is worth the same edit.

### D2, the wheel scan's walkers are watched by nothing, and the LLD says they are
**Where**: `scripts/guards/catalogue.py:7047` onward, probes
`licences.wheel-marker`, `licences.wheel-library` and
`licences.wheel-platform` in the entry at line 6991,
`docs/lld/guards.md:596-597`, `scripts/licence_check.py:572-602`
**What**: The three level-1 probes call `licence_bytes_problems` and
`bundle_list_problems` on literal lists. No probe or unit test calls
`installed_problems` over a synthetic site-packages, so `_licence_files` and
`_shared_libraries` have no failing test. The LLD sentence "Three level-1
probes call the wheel checks on a synthetic tree" is false.
**Why it is wrong**: AC16 names "level-1 probes over a synthetic
`site-packages` built in a temporary directory". Microscope section 4.
**Evidence**: in a scratch copy, both walkers were made to return an empty
list. `test_licence_check.py` stayed `OK`, and `installed_problems` on the
real Linux wheels returned `[]`, so `gate licences-wheels` would be green on
both hosts with the scan disabled.
**Scope and regression**: present on HEAD. Smallest regression: one probe or
unit test that builds a temporary site-packages with a dist-info, a
`licenses/` file carrying a marker and a `<pkg>.libs/` library, calls
`installed_problems`, and expects both refusals.

### D3, licence texts outside `licenses/` are never read
**Where**: `scripts/licence_check.py:590-602` (`_licence_files`)
**What**: Only `*.dist-info/licenses/` and `<package>/licenses/` are scanned.
`openpyxl-3.1.5.dist-info/LICENCE.rst`, `et_xmlfile-2.0.0.dist-info/LICENCE.rst`
and `LICENCE.python` sit at the dist-info root, the pre-PEP 639 layout, on
both macOS and Linux.
**Why it is wrong**: AC16, "a copyleft marker in any bundled licence text
... is refused". Two of the ten locked distributions ship their only licence
text where the scan does not look. Decision 4 names only `licenses/`, so the
plan text and AC16 disagree and the AC is the acceptance contract.
**Evidence**: appending `GNU General Public License version 3` to
`openpyxl-3.1.5.dist-info/LICENCE.rst` in a copy of the worktree's
site-packages left `installed_problems` empty. The same text in
`pydicom-3.0.2.dist-info/licenses/LICENSE` was refused.
**Scope and regression**: present on HEAD. Smallest fix: also scan dist-info
root files whose names start with `LICEN`, `COPYING` or `NOTICE`, with a
synthetic-tree regression per D2.

### D4, the dicom-tooling skill misstates when `ds.pixel_array` raises
**Where**: `.claude/skills/dicom-tooling/SKILL.md:35-38`
**What**: "Without a decoder handler, `ds.pixel_array` raises on a compressed
transfer syntax that has no declared route." The four DCMTK JPEG rows have
declared routes and `ds.pixel_array` raises on all of them, because
`imagecodecs` is not a pydicom plugin and no installed plugin handles JPEG
processes 1, 2 and 4 or 14 any more.
**Why it is wrong**: "declared route" is a test-module concept that pydicom
never consults. A developer following the skill on any JPEG corpus file gets
`RuntimeError: Unable to decompress ... because all plugins are missing`.
**Evidence**: worktree `.venv`: `jpeg_baseline_rgb8.dcm`,
`jpeg_extended_12.dcm` and `jpeg_lossless_p14.dcm` raise, `jpegls_lossless.dcm`
and `j2k_lossy.dcm` decode.
**Scope and regression**: present on HEAD, and the regenerated adapter only
carries the hash. Smallest fix: state that pydicom has no plugin for the four
JPEG process syntaxes after F-217 and name `read_back` in
`scripts/tests/test_corpus_synth.py` or the `imagecodecs` call as the way to
decode them.

## Smells

### S1, a `[[licence_file]]` row waives copyleft text with no recorded decision
**Where**: `scripts/licence_check.py:526-549`, `ci/licences.toml`
`[[licence_file]]` rows
**What**: `markers` and `empty = true` waive a refusal, and the row carries
no `reason`, `scope`, `decided` or `decided_by`. `exception_problems` never
sees these rows.
**Why it is a hazard**: AC16 says the waiver is "an exception" naming the
file, and AC13 requires every exception field. A later row can waive a GPL
text without an operator decision and the gate stays green.
**Evidence**: a bare row with `markers = ["GNU General Public License"]` for
a planted GPL line in pydicom's licence file gave no problem.
**Smallest fix**: require `EXCEPTION_FIELDS` on rows that set `markers` or
`empty`, or move those waivers into `[[exception]]` with a `file`, and add a
refusal probe.

### S2, the allowlist is not the approved one and nothing records the change
**Where**: `deny.toml:13-29`
**What**: fifteen identifiers where decision 1 approves sixteen. The missing
one is `Unicode-DFS-2016`.
**Why it is a hazard**: the narrowing is harmless today, cargo-deny and every
lockfile pass, but the normative plan and the recorded constant now disagree,
and older `unicode-ident` releases declare `Unicode-DFS-2016`, so a lock
movement would fail with the plan saying it should pass. The implementer's
report says the fourteen could not be recovered. They were in the sweep's
scratch `deny.toml`.
**Smallest fix**: add `Unicode-DFS-2016` and re-record the census, or have
the lead record the narrowing as a plan amendment.

### S3, the `caniuse-lite` exception's scope is false for one of the locks it covers
**Where**: `ci/licences.toml` first `[[exception]]`
**What**: one row keyed on name and version covers both lockfiles. In
`package-lock.json` the entry is `dev: true`. In
`tools/oracle/package-lock.json` it carries no `dev` flag, so it is a
production dependency of the oracle tool, and the row says `dev-only`.
**Why it is a hazard**: `scope` is never checked against the lock, so the
field the operator approved reads as verified and is not.
**Smallest fix**: check `dev-only` against the entry's `dev` flag in every
lock the row matches, and ask the operator whether the oracle half is `tool`.

## Nitpicks

- N1. `holder_problems` tests the substring `license.workspace = true`, so
  `license = "MIT" # was license.workspace = true` in a member manifest passes
  (measured). Parse with `tomllib`.
- N2. A `rust` row in `ci/licences.toml` is never passed to cargo-deny, so it
  can only refuse and never waive (measured on openjph-core). Fail-closed, but
  the LLD says exceptions live in that file without that caveat.
- N3. The plan's prefix rule refuses `horoscope` (measured). A scoped npm name
  is matched on its final segment only, and `[project.optional-dependencies]`
  is not extracted. `uv.lock` and the prose rule cover the last two today.
- N4. `examples/viewer-react/vite.config.ts` is outside the plan's write set.
  The change is necessary (the iife error reproduces without it) and low risk:
  the page constructs the worker with `type: "module"`, and no tool, script or
  workflow consumes the built viewer. Record it in the plan's write set.
- N5. AC6 asks for before and after tables and the `.50`/`.51` maximum
  difference against the old decode in the handoff. The handoff has neither.
  This pass measured them (3 and 1). Record them at `/complete-feature`.
- N6. `test_pep508_name_stops_at_a_marker` builds its expectation as
  `[...][:1] + ["liberty"]`. Write the two-element list.
- N7. `docs/SOURCE-POLICY.md:187-193` reflowed into one long line and a
  one-clause line. Cosmetic.

## Verified clean

- AC1: `pylibjpeg-libjpeg` absent from every tracked file except the backlog
  note and a negative test assertion. `uv lock --check` 0. Locked versions
  differ from the base only by the swap.
- AC2, AC3: the route table and `DECODED_WITH` test pass and name
  `libjpeg_turbo 3.2.0/8.0`. No other tracked Python file decodes a
  compressed syntax through pydicom's plugin order.
- AC4 to AC6: see "Acceptance challenged". All re-run now.
- AC7 to AC10: `include-dev = true`, `all-features = true`, deep probes red
  and green, pin unit tests in the gate. AC14's Annex D cases are in the
  tests and agree with the precedence text.
- AC11 to AC13, AC15: every refusal driven in scratch and in the probes.
- AC18: no `lightningcss` key in any of the three locks or in
  `node_modules`. vite 7.3.7, `@vitejs/plugin-react` 5.2.0, vitest 4.1.11.
- AC19 to AC22: notice bodies byte-equal to the fetched upstream file and to
  the vendored ritk `LICENSE-MIT`, which the diff does not touch.
  `LICENSE-APACHE` blob `d6456956...` unchanged. Every `LICENSE-MIT` copy says
  `Tensorbee Limited`, `crates/ocelli-wasm/LICENSE-MIT` is still the symlink.
  SPDX unchanged everywhere.
- AC23: `gate packages` green with `NOTICE` required.
- AC24, AC25: the six grammars, the `lib` prefix and the probes. The agent
  trailer in a tracked file is refused without a nearby negation and accepted
  with one, identically on the base and on HEAD, and commit messages are
  never read by the provenance check, so the agent trailer stays allowed. The
  read-blocked JPEG 2000 codec is not a dependency anywhere.
- AC26, AC27: CI installs cargo-deny before `gate licences` and before
  `gate guards-deep` in the same job, `licences-wheels` runs after `uv sync
  --locked`, census and runbook clean, budget moves explained.
- AC29: README Licence section, corpus README, developer setup and source
  policy rows read sentence by sentence. Faults are in D1 and D4.
- npm audit: nothing new. The `adm-zip` high advisory reaches the published
  `@ocelli/core` runtime closure through `dcmjs` and predates this story.

## Limitations and handoff

- AC17's Linux half is still open until the pushed `corpus-tooling` job runs.
  The local replica on the real wheels is green.
- `gate guards-deep` was not run as a whole. Its two new probes were run
  singly. `gate --sprint` was not run.
- No GPU or browser path was exercised, and none is touched.
- Next owner: implementation remediates D1 to D4 and S1 to S3, then a fresh
  independent pass. D-22 and the Q6 follow-up stay with `/complete-feature`.
