# F-238 review, pass 7

**Reviewed**: `536507e9..11557416` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `11557416`. One commit, "F-238, review pass 6 S1, step 6 refuses two named desktop binaries", touching `bin/ocelli.sh` (step 6's parse and comment), `scripts/guards/catalogue.py` (the fixture's `named_desktops` count, the docstring, one new probe, the runner entry's `refuses` and `limit`), `ci/guard-probe-budget.json` (`runner` 6 to 7) and `docs/runbooks/guard-verification.md` (re-rendered, row renumbering plus one new row). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`.
**Author / reviewer**: GLM in opencode is the author of the commit. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write passes 1 to 6.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 6 S1, replayed.** The parse now collects every `compiler-artifact` executable named `ocelli-desktop`, exits non-zero naming the count when there are two or more, prints the one when there is one, and prints nothing when there are none, so the existing `-z` refusal still answers zero. Through step 6's own lines, extracted verbatim from the tip and run under `set -euo pipefail` against real cargo on a scratch crate with `--config 'build.target=["aarch64-apple-darwin","wasm32-unknown-unknown"]'`: exit 1, stderr "FAIL: the shipped build named 2 ocelli-desktop executables, expected exactly one", and the scratch binary's "Tier Cpu" line never printed, so nothing ran. Closed.
- **Pass 6 N1, replayed.** The `stale_report` docstring now reads "without which a build that writes the desktop binary would overwrite it", which is exact for `named_desktops=0`. Closed.
- **The parse against real cargo streams.** Verbatim parse, a scratch crate with bins `ocelli-desktop` and `ocelli-server`, a lib with an unused variable and a build script emitting `cargo:warning`, `CARGO_TARGET_DIR` in the scratchpad, cargo 1.97.1:

  | Case | Stream | Parse stdout | Parse exit |
  |---|---|---|---|
  | one artifact, cold | 6 lines, 1 desktop artifact, `fresh: false` | `.../ctarget/release/ocelli-desktop` | 0 |
  | fresh rebuild | 6 lines, 1 desktop artifact, `fresh: true` | same path | 0 |
  | two targets via `build.target` array | 10 lines, 2 desktop artifacts | nothing, stderr names 2 | 1 |
  | none (`--bin ocelli-server` only) | 5 lines, 0 desktop artifacts | nothing, so the `-z` refusal fires | 0 |
  | interleaved diagnostics, `--message-format=json` (2 `compiler-message`, 1 `build-script-executed`) with a non-JSON line before every line | desktop artifact among them | `.../release/ocelli-desktop` | 0 |

- **The new probe against the old logic and four mutants**, each applied alone to `bin/ocelli.sh` in a scratch clone at the tip and reverted:

  | Step 6 variant | `two-named-executables` | `no-named-executable` | `target-triple-over-a-stale-binary` |
  |---|---|---|---|
  | tip `11557416` | red, at its refusal | red, at its refusal | green |
  | pass 6 first-match `break` (`536507e9`) | HARNESS, exited 0 | red, at its refusal | green |
  | pass 5 metadata path (`ffdddb00`) | HARNESS, exited 0 | not run | not run |
  | count warned on stderr, first still printed and run | HARNESS | red | green |
  | threshold `> 2` | HARNESS | red | green |
  | last match, no count check | HARNESS, exit status right, refusal text absent | red | green |
  | count printed on stderr with exit 0 and nothing on stdout | red | red | green |

  The last row is a refusal that still runs nothing, reached through the `-z` branch with both messages on stderr, so the probe accepts a behaviour the ruling also accepts. It cannot tell the parse's own exit from the fall-through, which is recorded under limitations rather than as a finding.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 536507e9..11557416` | worktree | now | 4 files, `bin/ocelli.sh` changes step 6 only |
| `bash -n bin/ocelli.sh` | HEAD | now | 0 |
| `bin/ocelli.sh gate native`, scratch `CARGO_TARGET_DIR`, cold | HEAD | now | 0, `ALL GREEN 1 gate(s)`, step 6 passed |
| `bin/ocelli.sh gate guards`, scratch `CARGO_TARGET_DIR` and `TMPDIR` | HEAD | now | 0, all nine `runner.step6-*` probes as declared, `target-triple-over-a-stale-binary` green, the other eight red, 453 refusal probes, 45 accept, 113 controls |
| `guard_census.py` | HEAD | now | 0, 1175 refusals in 81 files, 573 probes, 0 watched by nothing |
| `guard_census.py --check-runbook` | HEAD | now | 0 |
| `deviation_check.py` | HEAD | now | 0, 33 deviations |
| `prose_check.py` | HEAD | now | 0, 928 files clean |
| `python3 -m unittest scripts.tests.test_guard_catalogue` | HEAD | now | 0, 57 tests |
| `guard_probe.py --only` the three step 6 probes | scratch clone, tip, two older steps, four mutants | now | as in the table above |
| step 6 extracted verbatim, real cargo two-target stream | scratch clone at tip, scratch crate | now | 1, count refusal, binary not run |
| verbatim parse on scratch crate streams | scratchpad crate, cargo 1.97.1 | now | as in the table above |
| `git status`, `git diff 11557416`, `git fsck --no-dangling`, `git ls-tree -r 11557416 \| wc -l` against `git ls-files \| wc -l`, a search for `f238` paths | worktree | now | clean, diff empty, fsck silent, 1912 and 1912, no `target/f238-step6-stub` and no `f238` path outside `.git`, `/tmp` holds none |

## Defects

### D1, the new probe's note says it goes red under the break it catches, which is the opposite of what the harness reports

**Where**: `scripts/guards/catalogue.py:12508` to `:12513`
**What**: "A parse that takes the first match runs the first named binary, which here prints the honest report and passes, so this probe goes red under the break it exists to catch as well as over a parse that fell back to the stale path." In this harness `red` is the verdict a refusal probe reports when the guard refused for its declared reason, which is the passing outcome, and `guard_probe.py:602` to `:608` prints `red` in green for exactly that. Under the first-match `break` and under the metadata-path fallback this probe reports `HARNESS` and the run fails. The sentence therefore states, in the catalogue's own vocabulary, that the probe passes over the two states it exists to reject.
**Why it is wrong**: a claim in prose that is false is a defect under the microscope's severity table. The repository has corrected this exact inversion before, recorded in the comment at `guard_probe.py:603`, and the convention elsewhere in the catalogue is "reports HARNESS" (`scripts/guards/catalogue.py:7121`, `:10419`).
**Evidence**: `guard_probe.py --only runner.step6-two-named-executables` in a scratch clone with `bin/ocelli.sh` at `536507e9` printed `HARNESS runner.step6-two-named-executables` and exit 1, and at `ffdddb00` printed `HARNESS` with "exited 0. The guard it aims at did not fire". On the tip it printed `red` and exit 0.
**Scope and regression**: the note only, the code is right. The runbook does not render notes, so nothing else carries the sentence. The commit message's "the new probe red under the pass 6 first-match break ... and green on the fix" carries the same inversion and is history, so it is recorded here rather than asked to be rewritten. Fix: "so under the break it exists to catch, as over a parse that fell back to the stale path, this probe reports HARNESS rather than red." No probe can watch a note, so the regression is this review's replay.

## Smells

None.

## Nitpicks

### N1, "which is what a multi-target build's stream carries" reads as one directory

**Where**: `scripts/guards/catalogue.py:12500` to `:12503`
**What**: "The build names two ocelli-desktop executables under the triple directory, which is what a multi-target build's stream carries". The measured two-target stream names them under two directories, `aarch64-apple-darwin/release/` and `wasm32-unknown-unknown/release/`. The fixture's docstring says the stub writes both "under the one destination directory", which is exact. "The build names two ocelli-desktop executables, as a multi-target build's stream does with one per target, here both under the triple directory" says the same without the misreading.

## Verified clean

- **Pass 6 S1 and N1 are closed**, measured through the real step and real cargo for S1 and against the fixture for N1.
- **The ruling's three branches**: exactly one is printed and run, none keeps "the shipped build named no ocelli-desktop executable" (red at that refusal on the tip and on the old step), and two or more refuse naming the count and run nothing, through the parse's own exit under `set -euo pipefail` on the assignment at `bin/ocelli.sh:11` and the pipeline into `python3`.
- **The new probe discriminates**: red on the tip, HARNESS under the first-match `break`, the metadata-path fallback, a warn-and-run parse, a raised threshold and a last-match parse, with its control green throughout.
- **The comment above the parse** (`bin/ocelli.sh:653` to `:661`) says what the step does: which message it reads, that one is run and anything else refuses and runs nothing, and that the count refusal stops the step through the parse's non-zero status. Its one claim about cargo, one message per requested target, was measured at 2 for a two-target build.
- **The fixture**: `named_desktops` 0, 1 and 2 write and name 0, 1 and 2 desktop artifacts, the second a `.wasm` that is not host-runnable, matching the real wasm32 artifact's suffix. The `refuses` text, the `limit` and the runbook row describe the probe as the code builds it, `--check-runbook` agrees, and the census and budget move together (`runner` 7, 1175 refusals, 573 probes).
- **The tree after GLM's disclosed stub directory**: worktree clean, `git diff 11557416` empty, `git fsck --no-dangling` silent, 1912 tracked paths matching the commit's tree, no `target/f238-step6-stub`, no `f238` path anywhere in the worktree outside `.git`, none under `/tmp`.
- No em-dash, en-dash or prose semicolon in the added lines or the commit message. No Rust, dependency, `unsafe`, `wasm-bindgen` or pixel arithmetic in the range. `README.md`, `use.html` and `more.html` are not touched and describe nothing this range changed.

## Limitations and handoff

- The new probe does not observe "runs nothing" directly. It relies on the refusal text and the exit status, and the stderr-only mutant above shows it cannot tell the parse's own exit from a fall-through to the `-z` refusal. Both still run nothing, so no mutant was found that runs a binary and stays red.
- The two-target case was run through step 6 against a scratch crate, not with a multi-target `ocelli-native` build, which is native only on this host.
- `gate gpu` and the oracle were not run, under this pass's no-GPU rule. AC3, AC4's Linux and Windows halves and AC1 for `x86_64-apple-darwin` remain the lead's carried items, untouched by this range.
- Scratch clones, the scratch crate and both target directories lived under the session scratchpad and were removed with `shutil.rmtree`. Nothing was written in the worktree except this file.
- Self-review: GLM's self-review should have caught 1 of 2 findings, D1, because the harness it ran printed HARNESS for the state its note calls red, and its commit message repeats the inversion.
- Next owner: the lead rules on D1, then a fresh independent pass 8 reviews that change. This is a bounded review of one commit, not acceptance and not sprint readiness.
