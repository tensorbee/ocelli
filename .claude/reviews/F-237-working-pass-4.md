# F-237 review, pass 4

**Reviewed**: `b0677cb8..db2cc60b` on `work/f-237-xai`, one commit (db2cc60b) at tip db2cc60b. `sprint/s46` was at f4553acf during the pass. The worktree was clean apart from untracked `.venv` and `tools/oracle/out`, which are not part of the review. Only this review file was written in the worktree, and it is uncommitted.
**Author / reviewer**: db2cc60b is by the sprint lead, a Claude session. The reviewer is a separate Claude session that wrote none of F-237 and none of `sprint/s46`'s F-232 note text. Both are Claude, so independence is per session, not per model.
**Result**: 1 defect, 0 smells, 1 nitpick

F-237 pass 4: 1 defect, 0 smells, 1 nitpick.

## Acceptance challenged

- **Pass 3 D1, the lint-policy notes. Closed as ruled.** Both probe blocks, from `Probe("<name>"` to the next `Probe(`, were cut from `git show HEAD:` and `git show sprint/s46:` of `scripts/guards/catalogue.py` and compared in Python. `lint-policy.clean-module-outside-the-member` is 1277 bytes on both sides and equal. `lint-policy.clean-cfg-attr-module-path` is 1059 bytes on both sides and equal. Integration should no longer conflict on these two notes. The text copied in carries a false attribution of its own, see D1 below. Pass 3 quoted that sentence as the correct one and did not check it.
- **Pass 3 N2, the release refusal wording. Closed for the message.** `compare_problems` called with recorded tree `b*40` against `c*40` prints "the packed build id records tree 'bbbb...' and the release revision's tree is 'cccc...'. The index the tarball was staged from is not the tagged source." No test, probe or doc quotes the old phrase. `grep` finds "write-tree is" only in `package_check.py:282`, which is about the index and is correct there. The function's own docstring still uses the old phrase, see N1.
- **The path-module count.** On the worktree, the real `python3 scripts/lint_policy_check.py` exits 0 and prints "2 `#[path]` module(s) followed". `git grep '#\[path' -- '*.rs'` gives `tools/oracle/src/produce.rs:40`, `examples/folder-viewer/src/held.rs:23` and four under `vendor/ritk-codecs-0.6.0`, which is not a workspace member and is not counted. Two in the tree plus the planted one is the expect fragment's 3, and "the planted module is the third" is true.
- **The probes go red for their declared reason.** Both are accept probes, so red means the declared fragment is missing from an accepted run. In a scratch clone of db2cc60b:
  - M1, `MODULE_PATH` put back to require `path` first (`#\[\s*path\s*=`): `clean-cfg-attr-module-path` goes HARNESS red, "nothing in the output contains '3 `#[path]` module(s) followed'", with the guard printing 2. `clean-module-outside-the-member` stays green, which is the discriminating case: only the `cfg_attr` spelling was lost.
  - M2, follow a `#[path]` candidate only when it resolves inside the member directory, which is the pre-fix `member.rglob` scope: the real checker prints 2 on the tree. Both clean probes go HARNESS red for the same missing fragment, and the sibling refusal probe `lint-policy.module-outside-the-member` goes red with "exited 0. The guard it aims at did not fire".
  - On the clean worktree both probes are green and each control is green.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff b0677cb8..db2cc60b` | worktree | now | four files: two `note=` strings in catalogue.py, one f-string in release_wasm.py, plan ruling, pass 3 review |
| block diff in Python, HEAD against `sprint/s46` | `git show` of both, sprint/s46 f4553acf | now | both blocks equal, 1277 and 1059 bytes |
| `python3 scripts/lint_policy_check.py` | worktree | now | 0, "2 `#[path]` module(s) followed" |
| `git log -m --first-parent -S'#[path = "schedule.rs"]' sprint/s46 -- examples/folder-viewer/src/held.rs` | sprint/s46 | now | one commit, 198e197c "F-216, integrate folder viewer performance" |
| `git log --diff-filter=A -- held.rs schedule.rs` | worktree | now | both added by F-216 commits (ab5e5760, ae7c9c9e) |
| `git log -S'(F-210)' sprint/s46 -- scripts/guards/catalogue.py` | sprint/s46 | now | 6d95ce73 "F-232, fix round 2 after review pass 2" |
| `guard_probe.py --profile deep --only` for each of the two probes | worktree | now | 0, accept green, control green |
| the same under M1 and M2, plus `lint-policy.module-outside-the-member` under M2 | scratch clone `mut` of db2cc60b | now | as listed in Acceptance challenged |
| `compare_problems` with a mismatched tree | worktree | now | the new wording, see N2 above |
| `python3 -m unittest scripts.tests.test_release_wasm scripts.tests.test_guard_catalogue` | worktree | now | 0, 69 tests OK |
| `python3 scripts/guard_census.py` | worktree | now | 0, 1168 refusals in 80 files, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 scripts/prose_check.py` | worktree | now | 0, 914 files |
| en-dash, em-dash and `; ` scan of added lines and the commit message | `git diff`, `git log -1` | now | none |

Environment: macOS arm64, rustc 1.97.1, Python 3.14.5. The scratch clone lived under the session scratchpad `f237-p4` and was removed with `shutil.rmtree` after the pass.

## Defects

### D1, the copied notes attribute `held.rs`'s path module to F-210, and it came from F-216

**Where**: `scripts/guards/catalogue.py:12748` (note of `lint-policy.clean-module-outside-the-member`) and `scripts/guards/catalogue.py:12793` (note of `lint-policy.clean-cfg-attr-module-path`). The same text is on `sprint/s46`.
**What**: both notes say "The tree follows two `#[path]` modules, `produce` and `held.rs` (F-210)". `held.rs` and the `schedule.rs` it names were added by F-216 commits ab5e5760 and ae7c9c9e. On `sprint/s46`'s first-parent history, the `#[path = "schedule.rs"]` line arrives in 198e197c, "F-216, integrate folder viewer performance". F-210 is the value-at-the-pointer story and did not add it.
**Why it is wrong**: a false sentence in a guard note is a defect under the review command's "Claims in prose". The count and the "third" are right. Only the story ID is wrong, and a reader following it to F-210's plan to learn why the module exists finds nothing.
**Evidence**: the three `git log` commands in Evidence. The parenthetical came in with F-232's 6d95ce73 on `sprint/s46`.
**Scope and regression**: prose only. It is on HEAD and on `sprint/s46`, and it was copied, not written, in this range. The fix is to change `(F-210)` to `(F-216)` in both notes, or delete the parenthetical, which the review command prefers. Under the pass 3 ruling the two branches must stay byte-identical, so the change has to land on `sprint/s46` and be copied here identically, or land here and be taken by the integration. Either way it is a lead decision. Regression: the Python block diff against `sprint/s46` comes back equal, and `git grep -n 'held.rs` (F-210)'` finds nothing on either branch.

## Smells

None.

## Nitpicks

- N1, `scripts/release_wasm.py:55`: the docstring of `compare_problems` still says "`tag_tree` is the git write-tree of that commit". That is the phrase pass 3 N2 corrected in the message. `main()` computes it with `rev-parse <sha>^{tree}`. "the tree of that commit" would match the message.

## Verified clean

- The range changes only two `note=` strings, one refusal message, the plan's ruling section and the committed pass 3 review. No probe's arranger, command, expect fragment or polarity changed, and no code path of `release_wasm.py` changed. The runbook check, the census and the guard catalogue unit tests are green on the tip.
- Both probe blocks are byte-identical to `sprint/s46`'s.
- The count in the notes matches the real checker: 2 in the tree, 3 with the planted module, which is the expect fragment.
- Each probe goes red for its declared reason under a mutation that removes what it watches (M1 for the `cfg_attr` probe alone, M2 for both), and stays green on the clean tree.
- Pass 3 N2's refusal now reads "the release revision's tree", and nothing quoted the old wording.
- The ruling's claims, that the lead did the D1 copy and that both blocks equal `sprint/s46`'s, hold. Its N1 and N3 reasons are the ones pass 3 gave.
- No en-dash, em-dash or prose semicolon in the added lines or the commit message. `prose_check.py` exits 0.
- The worktree is unchanged apart from this file. `git status --porcelain` shows only the two untracked paths.

## Limitations and handoff

- `gate guards-deep` as a whole was not run. Only the two probes and the sibling refusal probe were, through `--only`.
- No `/verify`, floor gate, wasm build, release script run or CI run this pass, because the range touches no code those exercise beyond one message string. Pass 3's receipts for them stand for b0677cb8 and were not re-executed.
- No oracle, GPU, publish, tag or push.
- D1 is the lead's to rule on, because the fix has to keep `sprint/s46` and this branch identical. A pass 5 must review the change.
- The acceptance rows pass 3 listed are still owed: AC4's `frontend` job on `ubuntu-latest` after integration, the feature `/verify` receipt with the corpus, AC3 at the real release and AC5 against the registry after publish.
