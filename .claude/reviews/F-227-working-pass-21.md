# F-227 review, pass 21

**Reviewed**: `git diff ca504aa0..c01fd6ed` on branch `work/f-227-xai`, tip `c01fd6ed`, tracked tree clean (untracked `.venv` and `tools/oracle/out` only). One commit, the viewer half's pass 20 remediation, 10 files, 84 insertions, 46 deletions. "Operator decision on the server memory budget, 2026-11-07" and "Lead rulings after review pass 20" in `.claude/plans/F-227-design.md` are read as the contract and not reviewed.
**Author / reviewer**: the remediation is by a Claude implementer, per the commit. This pass is by an independent Claude reviewer who wrote none of the reviewed material, none of the earlier remediation and none of the rulings.
**Result**: 0 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **Pass 20 findings, replayed through the real callers.** D1 closed: the bench runner itself, `node tools/bench/run.mjs viewer.resident_memory --no-build --out <scratch>`, wrote `rss_bound_bytes` 528,194,192, `series_pool_term_bytes` 8,388,608 and `thresholds_met` true at `held.budget` 268,435,456 and a DX file of 17,400,538 bytes. D2 closed: a fresh HEAD release viewer, one round over 16 files of 512 square, `GET /held` `workspace` 0 then 2,097,152 one second later, and still 2,097,152 after a 32 square round, which is what the new sentence says (the permit keeps its scratch, reported as `workspace`). S1 closed: the new test is green on HEAD and red under the grow-only mutant, and the same sequence over HTTP on the release binary answered `200 ... "skipped":0,"counted":16384` for the 32 square series after the 512 square one. N1 closed: "the run includes three series window rounds" in cache.md, benchmarks.md and the runner comment, with no claim about the peak. N2 closed: README and use.html both carry the two-at-once sentence after the free list sentences, and "there" in README now reads "on those threads". N3 closed: the `pixel-pipeline.md` line and `auto.mjs:45` are rewrapped. N4 closed: the round test requires exactly 2, red under a limit of 1 in 3 of 3. N5 closed: `series_pool_term_bytes` reads `SERVER_PERMITS`.
- **The bound by hand.** `268,435,456 + 4 * 17,400,538 + 2 * (2 * 17,400,538 + 5 * 8,700,000) + 2 * 4,194,304 + 25,165,824` = 268,435,456 + 69,602,152 + 156,602,152 + 8,388,608 + 25,165,824 = 528,194,192. F-216's formula with 32 MiB and no pool term gives the same total. `serverRssBound(268435456, 17400538, 8700000)` printed 528,194,192 and `serverRssBound(0, 17400516, 8700000)` printed 259,758,560.
- **Every current statement of the bound or its constant.** `viewer_work.mjs:116` and `:127`, `viewer_work_test.mjs:58-63`, `docs/lld/cache.md:202-205`, `docs/lld/benchmarks.md:684-688`, all at 24 MiB. `held.rs:475` and the six `ci/bench-baseline.json` mentions of 528,194,192 are true at the runner's workload. README and use.html state no total and no constant, only the `P * 4 MiB` term and the grey term.
- **Surviving figures.** `git grep` for `32 MiB`, `32 * MIB`, `33,554,432`, `536,582,800` and `268,147,168` outside `.claude/reviews` and `.claude/plans/F-216-design.md` finds only the 2 by 2 grid cap of 32 MiB (`page.html:1441`, `preview.rs:538`, F-207 and F-208 plans, AS_BUILT), a 32 MiB stalled reply test in `serve.rs:6055`, unrelated `33554432` pixel constants in `ocelli-pixel` and `ocelli-volume` tests, the F-227 plan's pass 19 ruling and operator decision (history and contract), and the three new sentences that say the constant was lowered from 32 MiB. `.claude/plans/F-216-design.md` and `ci/bench-baseline.json` are unchanged in the range (`git diff --quiet` exit 0).
- **Challenge, the exact round test's reliability.** Exactly 2 needs two rounds to overlap. Green in 15 isolated runs and in 6 runs of the whole lib suite under parallel load, and red under limits of 1, 3 and 64 in 3 of 3 each.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree `c01fd6ed` | now | 0. lib 98, preview 116, value 24, measure 16, the rest green |
| `cargo test --lib` five more times, and the two new tests 15 times | worktree | now | all green |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --all --check` | worktree | now | 0 and 0 |
| `node --test tools/bench/tests/viewer_work_test.mjs` | worktree | now | 15 pass |
| `scripts/prose_check.py`, `scripts/guard_census.py` | worktree | now | clean over 991 files, 0 watched by nothing |
| `node examples/folder-viewer/tests/browser/auto.mjs`, two runs | worktree, release | now | 0 and 0. restart `200, 410, 200`, 4 MiB `410, 200`, changed `2 marked, 200`, 1 MiB `Auto window, this image`. No viewer left |
| `run.mjs viewer.resident_memory --no-build`, record in scratch | worktree runner, release | now | 0. peak 450,117,632, `rss_bound_bytes` 528,194,192, pool term 8,388,608, `thresholds_met` true |
| HTTP probe, 512 square round, 32 square round, 512 again | release, `OCELLI_PORT=0` | now | 200 `counted` 1,048,576, 200 `counted` 16,384, 200. `workspace` 2,097,152 kept |
| NOSHRINK (`scratch.len() < count`) | scratch, `--lib` | now | new S1 test red, `400 ... X-Ocelli-Error: window` |
| `SERIES_ROUNDS` = 1, 3, 64 | scratch, `--lib`, 3 runs each | now | red x3 each, `1`, `3` and `4 rounds ran at once` |
| JS: `32 * MIB`, `SERVER_PERMITS = 3`, `SERIES_POOL_BYTES = 3 * MIB` | scratch runner | now | 1 fail each in `viewer_work_test.mjs` |
| JS: `2 * SERIES_POOL_BYTES` in place of `permits *` | scratch runner | now | green, expected since the test uses the default `permits`. Not a regression of the range |
| Implementer's "red under a grow-only scratch", "red with one round and with no limit", "red at 32 MiB" | commit message | prior receipt | consistent with the above |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1** `examples/folder-viewer/src/serve.rs:5865` and `:5901`. The test is still named `at_most_p_...` while it now requires exactly `P`, and under a limit of 1 it reports `1 rounds ran at once, the limit is 2`, which names a limit that was not exceeded. "expected exactly {p}" would read true on both sides.
- **N2** Uneven wraps in new prose. `serve.rs:1577` runs to 93 columns among lines of about 80, `series_window.rs:17` ends short at "The pool moves", and `docs/lld/benchmarks.md:688` leaves "The" alone at the end of a line.
- **N3** `tools/bench/tests/viewer_work_test.mjs:56-63` and `docs/lld/cache.md:203`. The hand value uses F-216's named 17,400,516 byte radiograph, so 259,758,560 plus B is 528,194,016, while 528,194,192 is the bound at the runner's DX file of 17,400,538 bytes. Both are correct for their inputs. cache.md says "528,194,192 bytes at its named budget" without naming which `S_max`, so a reader checking it against F-216's 17,400,516 is 176 bytes off. Carried from F-216's baseline, outside this range's change.
- **N4** `serverRssBound`'s `permits` parameter is not exercised with a value other than the default, so substituting `2` for `permits` in the pool term is green. Low value, since the runner never passes `permits`.
- **Carried**: pass 15 N1 and the earlier library nitpicks, unchanged.

## Verified clean

- All of pass 20's D1, D2, S1 and N1 to N5 closed, as under Acceptance challenged, through the runner, the HTTP route, the page under auto.mjs and the bench runner.
- Every current statement of the bound computes 528,194,192 at 256 MiB with the 24 MiB constant. F-216's plan and the baseline are untouched.
- The new S1 test's claim that stored values start at 1 and so all count was checked against `post_series` (`1 + first + k + at % 300`) and the pool's stored zero exclusion (`ocelli-pixel/src/auto_window.rs:273`).
- The runtime code of the viewer is unchanged in the range. Only doc comments changed in `serve.rs` and `series_window.rs`, plus two tests, so the window values auto.mjs reports are those of pass 20.
- README and use.html. The moved sentence reads in place for a user: it follows the free list sentences, names the two-at-once limit, the 4 MiB per window outside `OCELLI_CACHE_MIB`, and that a third waits, and neither page states a total or constant that could be stale. "Every preview decodes on those threads" (README) and "there" (use.html) both follow the two threads sentence directly.
- No `as` cast, `unsafe` or `#[allow]` added. No pixel arithmetic changed. No em-dash, en-dash or prose semicolon in the changed prose (`prose_check.py` clean).

## Limitations and handoff

- No GPU, wasm or oracle job, by instruction. Timings are informational only.
- The three rounds per resident memory run are pass 20's probe receipt. This pass did not count rounds in its own runner run, and the viewer's runtime code is unchanged since.
- `scripts/guard_probe.py` was not run. The page was exercised only through auto.mjs, not clicked by hand.
- Scratch work lived under `scratchpad/f227-p21`, including the bench record, and is deleted with Python `shutil` after this file is written. Every viewer this pass started was stopped by its driver. Viewers of other worktrees, a viewer from an earlier session's `f224-impl` scratch, and PID 98891 were not touched.
- Next owner: the lead. This is a clean bounded pass of the range and is not sprint verification.
