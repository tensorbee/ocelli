# F-237 review, pass 5

**Reviewed**: `db2cc60b..f92d01c7` on `work/f-237-xai`, one commit (f92d01c7) at tip f92d01c7, and `sprint/s46`'s commit 1a52c504 (parent f4553acf), read with `git show` only. The worktree was clean apart from untracked `.venv` and `tools/oracle/out`, which are not part of the review. Only this review file was written, and it is uncommitted.
**Author / reviewer**: both commits are by the sprint lead, a Claude session. The reviewer is a separate Claude session that wrote none of F-237, none of `sprint/s46` and neither commit. Both are Claude, so independence is per session, not per model.
**Result**: 0 defects, 0 smells, 1 nitpick

F-237 pass 5: 0 defects, 0 smells, 1 nitpick.

## Acceptance challenged

- **Pass 4 D1, the "(F-210)" attribution. Closed.** Both notes on both refs now read "The tree follows two `#[path]` modules, `produce` and `held.rs`, so the planted module is the third." `git grep -n 'F-210' -- scripts/guards/catalogue.py` exits 1 with no output on f92d01c7 and on `sprint/s46` (1a52c504). The only F-216 hit in the file is line 14066, an unrelated `spec=` of the folder viewer subjects, so the deletion did not swap one attribution for another.
- **Pass 4 N1, the `compare_problems` docstring. Closed.** `scripts/release_wasm.py:57` says "`tag_tree` is the tree of that commit", which matches `main()`'s `git rev-parse <sha>^{tree}` at lines 235 to 237. The one remaining `write-tree` in the release path is `scripts/tests/test_release_wasm.py:254`, where the producer side records the staged index's tree, so it is correct there.
- **Byte identity of the two probe blocks.** Each block was cut from `Probe("<name>"` to the next `Probe(` in Python from `git show` of db2cc60b, f4553acf, f92d01c7 and `sprint/s46`. On f92d01c7 and `sprint/s46`, `lint-policy.clean-module-outside-the-member` is 1281 bytes on both and equal, and `lint-policy.clean-cfg-attr-module-path` is 1063 bytes on both and equal. Each block is exactly 8 bytes shorter than at db2cc60b and f4553acf (1289 and 1071), which is the length of " (F-210)". The cut includes the 12 bytes of indentation before the next `Probe(`, which is why pass 4's figures were 1277 and 1059 for the same blocks. Each name appears once per file, so the cut cannot pick up a duplicate.
- **The sentence is still grammatical and true.** The real `python3 scripts/lint_policy_check.py` exits 0 and prints "2 `#[path]` module(s) followed". Outside `vendor/`, `git grep '#\[path' -- '*.rs'` gives `tools/oracle/src/produce.rs:39` and `examples/folder-viewer/src/held.rs:22`, so 2 in the tree plus the planted module is the expect fragment's 3. See N1 for how the note names them.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff db2cc60b..f92d01c7` | worktree | now | four files: two `note=` strings in catalogue.py, one docstring line in release_wasm.py, the plan's pass 4 ruling, the committed pass 4 review |
| `git show 1a52c504`, `git diff --name-status f4553acf 1a52c504` | sprint/s46 | now | `M scripts/guards/catalogue.py` only, two removed and two added lines, each pair differing by " (F-210)" alone |
| block cut and compare in Python | `git show` of db2cc60b, f4553acf, f92d01c7, sprint/s46 | now | equal on f92d01c7 and sprint/s46, 1281 and 1063 bytes, 8 fewer than before |
| `git grep -n 'F-210' -- scripts/guards/catalogue.py` | f92d01c7 and sprint/s46 | now | exit 1 on both, no hit |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0, "2 `#[path]` module(s) followed" |
| `guard_probe.py --profile deep --only lint-policy.clean-module-outside-the-member` | worktree | now | 0, accept green, control green |
| `guard_probe.py --profile deep --only lint-policy.clean-cfg-attr-module-path` | worktree | now | 0, accept green, control green |
| `python3 -m unittest scripts.tests.test_release_wasm scripts.tests.test_guard_catalogue` | worktree | now | 0, 69 tests OK |
| `python3 scripts/guard_census.py` | worktree | now | 0, 1168 refusals in 80 files, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 scripts/prose_check.py` | worktree | now | 0, 915 files |
| en-dash, em-dash and `; ` scan of added lines and both commit messages | `git diff`, `git log -1` | now | one hit, the pass 4 review's Evidence row quoting `; ` as a code literal, which is not prose |
| pass 4's M1 and M2 mutations | scratch clone of db2cc60b | prior receipt, pass 4 | not re-run, see Limitations |

Environment: macOS arm64, Python 3.14.5. No scratch copy was made this pass.

## Defects

None.

## Smells

None.

## Nitpicks

- N1, `scripts/guards/catalogue.py:12747` and `:12792`, the same text on `sprint/s46` at 12509 and 12554: the note names the two followed modules by the files that carry the attribute, and mixes the forms, `produce` as a module name and `held.rs` as a file name. The modules `#[path]` actually brings in are `current` (`produce_current.rs`) and `schedule` (`schedule.rs`). The count and "the third" are right, and a reader can find both sites, so this does not block. If it is ever changed, it has to land identically on both branches under the pass 3 ruling.

## Verified clean

- f92d01c7 changes only the two `note=` strings, one docstring line, the plan's ruling section and the committed pass 4 review. No probe's arranger, command, expect fragment, polarity, `needs` or `profile` changed, and no executable line of `release_wasm.py` changed.
- 1a52c504 changes nothing on `sprint/s46` except the same two strings in `scripts/guards/catalogue.py`.
- Both probe blocks are byte-identical across the two refs, so integration should not conflict on them.
- No "(F-210)" remains in the catalogue on either ref, and no new attribution was introduced.
- The plan's pass 4 ruling describes what the two commits do. The lead's claim that `held.rs` came with F-216 is pass 4's finding, which that pass established from `git log`.
- Both probes are green on the tip, and the unit tests, census, runbook check and prose check are green.
- No en-dash, em-dash or prose semicolon in the added lines or either commit message.
- `git status --porcelain` shows only the two untracked paths plus this file.

## Limitations and handoff

- The mutations were not re-run. The range touches prose only, and no arranger, command or expect changed, so pass 4's M1 and M2 receipts on db2cc60b still describe what the probes detect.
- `gate guards-deep` as a whole, `/verify`, the floor gate, a wasm build, a release script run and CI were not run this pass. Only the two probes were run, through `--only`.
- No oracle, GPU, publish, tag or push.
- The owed acceptance rows are unchanged: AC4's `frontend` job on `ubuntu-latest` after integration, the feature `/verify` receipt with the corpus, AC3 at the real release and AC5 against the registry after publish. This pass is a clean bounded review of the two commits. It is not completed acceptance or sprint readiness.
