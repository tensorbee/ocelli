# F-238 review, pass 8

**Reviewed**: `11557416..3d1c61e1` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `3d1c61e1`. One commit, "F-238, review pass 7, the probe note says HARNESS where it said red", touching `scripts/guards/catalogue.py` (the note of `runner.step6-two-named-executables` only), `.claude/plans/F-238-design.md` (the lead ruling after pass 7) and `.claude/reviews/F-238-working-pass-7.md` (added). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`.
**Author / reviewer**: the sprint lead, a Claude session, wrote the commit. Reviewer is Claude Opus 5.5 as an independent subagent of the lead, and wrote none of the reviewed material and none of passes 1 to 7.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 7 D1, replayed.** The note now reads "so under the break it exists to catch, as over a parse that fell back to the stale path, this probe reports HARNESS rather than red", which is the fix pass 7 proposed word for word. Measured through `guard_probe.py --only runner.step6-two-named-executables` in scratch clones of the tip with only `bin/ocelli.sh` replaced: the first-match `break` (`536507e9`) printed `HARNESS` with "exited 0. The guard it aims at did not fire", exit 1, and the `cargo metadata` stale path (`ffdddb00`) printed the same, exit 1. The tip printed `red`, exit 0, "1 refusal probe(s) drove 1 guard(s) red for their declared reason ... 1 control(s) green". At `guard_probe.py:602` to `:608` a refusal probe that passes prints `red` in green, and an outcome of `fail` prints `HARNESS` in red, so the note now uses the harness's own vocabulary the right way round. Closed.
- **Pass 7 N1, replayed.** "The build names two ocelli-desktop executables. A multi-target build's stream carries two, one under each target's directory" no longer claims the triple directory for the real stream, and no longer says anything about where the fixture puts them, so nothing reads as one directory. Closed, with the residue under N1 below.
- **Every sentence of the new note, executed.** A script in each scratch clone called `_write_step6_probe(named_desktops=2, stale_report=honest)` and `_run_step6_probe` with `CARGO_BUILD_TARGET` exported, then read the stub's run marker:

  | Step 6 variant | Step exit | Stderr | Marker (first named binary ran) |
  |---|---|---|---|
  | tip `3d1c61e1` | 1 | "the shipped build named 2 ocelli-desktop executables, expected exactly one" | absent, nothing ran |
  | first-match `break` (`536507e9`) | 0 | empty | present, the first named binary ran and passed |
  | stale path (`ffdddb00`) | 0 | empty | absent, the stale `release/ocelli-desktop` ran and passed |

  That confirms "a parse that takes the first match runs the first named binary, which here prints the honest report and passes", "the step must refuse naming the count and run nothing", and the HARNESS sentence in both states. The control sentence was checked against `_step6_triple_build_names_the_binary` (`catalogue.py:7804`): the same `CARGO_BUILD_TARGET=_STEP6_TRIPLE`, the same `stale_report=_HONEST_APPLE_REPORT`, `named_desktops` left at its default of 1, and the control reported green on the tip and on both older steps.
- **The multi-target claim, against real cargo.** A scratch crate with bins `ocelli-desktop` and `ocelli-server` and a lib, cargo 1.97.1, `CARGO_TARGET_DIR` in the scratchpad, `--bins --release --message-format=json-render-diagnostics`:

  | `build.target` | Build | Desktop `compiler-artifact` messages, in stream order |
  |---|---|---|
  | `["aarch64-apple-darwin","wasm32-unknown-unknown"]` | cold | `aarch64-apple-darwin/release/ocelli-desktop`, then `wasm32-unknown-unknown/release/ocelli-desktop.wasm` |
  | `["wasm32-unknown-unknown","aarch64-apple-darwin"]` | fresh, same dir | `.wasm` first, then the host binary |
  | `["wasm32-unknown-unknown","aarch64-apple-darwin"]` | cold, second dir | `.wasm` first, then the host binary |

  Two messages, one under each target's directory, as the note says. The order flips with the configured order, which is caller configuration and nothing step 6 sets, so "decided by the order cargo finishes the units and not by anything the step controls" holds, and under the third row a first-match parse would have picked the non-runnable `.wasm`. Step 6's parse, extracted verbatim from the tip, refused the third row's real 7-line stream with exit 1 and the count message.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git show --stat HEAD`, `git diff 11557416 HEAD -- scripts/guards/catalogue.py` | worktree | now | 3 files, the catalogue hunk is inside one `note=` string |
| AST of `catalogue.py` at `11557416` and `3d1c61e1`, every `Probe(... note=...)` value replaced by one constant, `ast.dump` compared | `git show` of both commits | now | 301 notes normalised on each side, dumps equal, raw dumps differ, so the note is the only change |
| `guard_probe.py --only runner.step6-two-named-executables` | scratch clone at tip | now | 0, `red` |
| same, `bin/ocelli.sh` from `536507e9` | scratch clone | now | 1, `HARNESS`, "exited 0" |
| same, `bin/ocelli.sh` from `ffdddb00` | scratch clone | now | 1, `HARNESS`, "exited 0" |
| marker script over the three variants | scratch clones | now | as in the table above |
| real two-target cargo builds, three rows | scratch crate, cargo 1.97.1 | now | as in the table above |
| tip parse, verbatim, on the real cold reversed stream | scratch | now | 1, count refusal |
| `guard_probe.py --only runner.step6-two-named-executables` | worktree HEAD | now | 0, `red`, 1 control green |
| `bin/ocelli.sh gate guards`, scratch `CARGO_TARGET_DIR` and `TMPDIR` | worktree HEAD | now | 0, `ALL GREEN 1 gate(s)`, 453 refusal probes over 41 guards, 45 accept, 113 controls, all nine `runner.step6-*` as declared |
| `guard_census.py` | worktree HEAD | now | 0, 1175 refusals in 81 files, 573 probes, 0 watched by nothing |
| `guard_census.py --check-runbook` | worktree HEAD | now | 0 |
| `python3 -m unittest scripts.tests.test_guard_catalogue` | worktree HEAD | now | 0, 57 tests |
| `prose_check.py` | worktree HEAD | now | 0, 929 files clean |
| `git grep two-named-executables` | HEAD | now | the runbook row (renders `refuses`, not the note), the fixture docstring, the probe and the limit, none restating the note |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, "A multi-target build's stream carries two" is exact only for two targets

**Where**: `scripts/guards/catalogue.py:12501` to `:12502`
**What**: a build for three targets carries three. The clause after the colon, "cargo emits one such message per requested target", states the general rule, so nothing is misled, and only two-target streams were measurable here (two targets installed). "A two-target build's stream carries two" or "carries one per target" would be exact.

### N2, the pass 6 ruling still says the probe "goes red under today's first-match `break`"

**Where**: `.claude/plans/F-238-design.md:169`
**What**: the same inversion pass 7 D1 corrected in the note, in the lead's dated ruling that asked for the probe. Under that break the probe reports HARNESS. It is a dated record of the instruction rather than a description of the tree, which is how pass 7 treated the commit message carrying the same words, so it does not block. A parenthesis in the pass 7 ruling, or nothing, are both reasonable.

## Verified clean

- Pass 7 D1 and N1 closed, each by measurement rather than by reading the new text.
- Every sentence of the new note executed: the two-message real stream and its directories, the order not controlled by the step, the refusal with the count and no binary run on the tip, the first-match parse running the first named binary and passing, the HARNESS verdict under first match and under the stale path, and the control's identical target and stale binary with one named executable.
- Nothing but the note changed behaviour, by AST comparison with all 301 probe notes normalised. The plan and the added pass 7 review are prose only.
- The pass 7 ruling's two bullets match the two edits.
- No em-dash, en-dash or prose semicolon in the added lines or the commit message. No Rust, shell, dependency, `unsafe`, `wasm-bindgen` or pixel arithmetic in the range. `README.md`, `use.html` and `more.html` are untouched and describe nothing this range changed.

## Limitations and handoff

- Only two targets were installed, so no three-target stream was measured (N1).
- The two-target case used a scratch crate, not `ocelli-native`, which is native only on this host. The step 6 refusal on a real stream was exercised through the verbatim parse, not the full step, because pass 7 already ran the full step on a two-target stream and the step's code did not change in this range.
- `gate gpu` and the oracle were not run, under this pass's no-GPU rule. AC3, AC4's Linux and Windows halves and AC1 for `x86_64-apple-darwin` stay the lead's carried items, untouched here.
- Scratch clones, the scratch crate and both target directories lived under the session scratchpad and were removed with `shutil.rmtree`. Nothing in the worktree was written except this file, which is uncommitted.
- A clean bounded review of one commit, not acceptance and not sprint readiness. Both nitpicks are the lead's call.
