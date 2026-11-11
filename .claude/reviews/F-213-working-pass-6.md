# F-213 review, working pass 6

**Reviewed**: `work/f-213-xai`, range `88d7287d..cc7a3fa7`, HEAD `cc7a3fa7954d752e7cad3b87a45048f5b360dc0a`. `88d7287d` adds only `.claude/reviews/F-213-working-pass-5.md` on top of `966d319c`, so the code under review is `966d319c..cc7a3fa7`. No unstaged tracked paths when the pass began or ended. Untracked `.venv` and `tools/oracle/out` are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range, remediating `.claude/reviews/F-213-working-pass-5.md` (D1 the LLD sentence naming `per_frame_replies`, S1 the one-parse test driving only the direct branch, N2 the README rewrap). This pass is an independent Claude reviewer launched by the sprint lead. It did not write or edit any reviewed code.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- Pass 5 S1, the pooled branch. `file_route` now builds one `inspect` closure over `inspect_post` and calls it inside `slot.call` for a pooled body and in place otherwise. The new `a_pooled_file_post_parses_and_builds_its_planes_once` posts a 12-frame per-frame file over TCP to `serve_on` with `Viewer::server`, and `held.rs:1231` `schedules()` is true there, so `read_body` takes `read_pooled_body`, which always returns `Some(permit)`. Pass 5's M2, a second `inspect_json` in the pooled closure only, now turns that test red and leaves the direct test green. Its direct twin M1 turns the direct test red and leaves the pooled test green. M4, the extra call inside the shared closure, turns both red. Closed.
- The cross-thread counter. T1 keys the map by UID and the current thread id, which is the old thread-local behaviour. The pooled test then goes red and the direct test stays green, so the pooled test observes work done on a decode worker and not on the test thread.
- Keying by SOP Instance UID cannot hide work. Counts are only added under a key, and the key comes from the same parsed object for both `project_metadata` (`optional_text` of 0008,0018) and `plane_tokens` (the projected `SOP_INSTANCE`), so a repeated parse of the same bytes lands under the same key. X1, a scratch test calling `inspect_post` twice on one 6-frame file with one UID and asserting `(2, 2)`, passes, so two posts sharing a UID add rather than mask. U1, the fixture's 0008,0018 element dropped, reads `(0, 0)` and turns `per_frame_replies_build_the_shared_parts_once` and both post tests red, so a file the counter cannot key fails rather than passing silently. The only UIDs read by `inspect_work` are `2.25.213004`, `2.25.2130012`, `2.25.213101` and `2.25.213102`, each used by one test. The default `2.25.213000` of `per_frame_file` is never read. The odd-length UID is padded with one NUL, as PS3.5 6.2 requires for UI, and both counter sites agree on the trimmed value, which the `(1, 1)` result shows.
- Release builds. The release binary is stripped, 126 symbols and none of the crate's, so `nm` proves nothing either way. Instead `compile_error!` was planted in the `INSPECT_WORK` initialiser, `note_inspect_work` and `inspect_work`. `cargo build --release -p ocelli-folder-viewer` exited 0 and `cargo test --no-run` exited 101 with all three errors, so the static, the map and both functions are absent from the release build. The two call sites are statement-level `#[cfg(test)]` and would fail to resolve in release if they were compiled. A release build of HEAD gave no warning.
- The replies against `966d319c`. Release binaries built from `git archive` copies of `966d319c` and `cc7a3fa7`, each with its own `CARGO_TARGET_DIR`, answered pass 5's `f213p5-bytes.mjs` on nine synthetic files: `le`, `rle`, `late`, `late-rle`, `spacing`, `mid-le`, `mid-rle`, the windowless `n1` and `big500_pf`. The two dump trees, 1087 lines, are identical, every one of the nine also equals pass 5's own `966d319c` dump file byte for byte, id equals body for every frame of every file, and `/held` `kept` reads the same bytes after each file on both binaries, ending at 681163.
- The changed LLD sentence, `docs/lld/cache.md:193` to `:201`, executed against the tree. `inspect_post` calls `parse_part10` once, on a decode worker for `Viewer::server` (shown by M2 and T1) and in place for `Viewer::new` (shown by M1). `InspectParts::new` is what calls `project_metadata` and `plane_tokens` (`preview.rs` lines 116 and 126 of that function), measured once each per post on both branches. `kept_replies` calls only `frame_fields` per frame. `per_frame_replies` is no longer named. "every transfer syntax keeps the replies the same way" holds for explicit LE and RLE in the dumps above. Pass 5 D1 closed.
- Pass 5 N2. The README diff is a rewrap only. The text with all runs of spaces and newlines collapsed is identical before and after, and lines 249, 277, 281 and 291, previously 94 to 125 characters, are now all under 80. Closed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at cc7a3fa7 | now | 0. lib 90, preview 107, every suite ok, both one-parse tests ok |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build, synthetic folders | now | 0, ALL GREEN. `tiles, lists held back` reads `WW 400  WL 40` at 128/0 on tiles 2 to 4 |
| `f213p6-mut.py` R0, M2, M1, M4, T1, U1, X1 | scratch copy `f213-p6` of cc7a3fa7, own target `f213-p6-target` | now | listed below |
| `f213p6-cfg.py` | same copy | now | release exit 0, test build exit 101 with the three planted errors |
| `f213p5-bytes.mjs` with `DUMP`, both release binaries | nine synthetic files from passes 3 to 5 | now, plus pass 5's `966d319c` dumps as a prior receipt | identical to each other and to the prior dumps |
| `git diff 88d7287d..cc7a3fa7` on README with whitespace collapsed | worktree | now | word-identical |

Rust mutations, each reverted after its run with the file hash checked equal, then touched:

- R0 control: exit 0.
- M2, a second `crate::preview::inspect_json` in the `slot.call` closure only: exit 101, `a_pooled_file_post_parses_and_builds_its_planes_once` FAILED, the direct test ok.
- M1, the same in the `else` branch only: exit 101, `one_file_post_parses_and_builds_its_planes_once` FAILED, the pooled test ok.
- M4, the same inside the shared `inspect` closure: exit 101, both FAILED.
- T1, the counter keyed per thread: exit 101, the pooled test FAILED, the direct test ok.
- U1, the fixture without 0008,0018: exit 101, three FAILED as above.
- X1, two inspections of one UID asserted to add to `(2, 2)`: exit 0, the scratch test ok.

## Defects

None.

## Smells

None.

## Nitpicks

- N1. `a_pooled_file_post_parses_and_builds_its_planes_once` (`serve.rs:2862`) spells out the listener, `Viewer::server`, the `serve_on` thread and the raw `POST /file` by hand, which is what the same test module's `grey_server` (`serve.rs:4523`) and `post_bytes` (`serve.rs:4162`) already do. Behaviour is the same, so this does not block.
- N2. Carried from pass 5 and not touched by this range: its N1 (the full-range tile label has no test), N3 (a tile shows its previous picture under a bare `Image N` for one round trip) and N4 (pass 4's N3 to N5). Not re-executed in this pass.

## Verified clean

- Pass 5 D1, S1 and N2 closed, S1 by four mutations that each go red on the branch they touch and stay green on the other.
- The counter and its map are test-only, proved by a release build that ignores planted `compile_error!`s. Keying by UID was challenged by a shared-UID case and a missing-UID case, and neither hides work.
- Scan and kept replies byte-identical to `966d319c` for nine synthetic files, plain and RLE, single-frame and 500 frames.
- The changed LLD sentence executed clause by clause. `use.html` and `more.html` say nothing about inspection counting or the decode worker, so nothing there went stale. The README change is whitespace only.
- No new `as` cast and no pixel arithmetic changed. The gate's greys are unchanged from pass 5.

## Limitations and handoff

- Synthetic files only, no patient data. No JPEG-LS, JPEG 2000 or HTJ2K file was posted.
- A bodyless `POST /file` to the process server takes the `fill_body` path with no slot and so runs in place. By code trace only, not executed. It fails to parse and returns 400, and the LLD's "on a decode worker for the process server" is read as being about posts with a file.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- The scratch copies `f213-p6` and `f213-p6-base` were checked equal to their commits over the 73 tracked files under `examples/folder-viewer`, `crates/ocelli-pixel` and `docs/lld`, then deleted with `shutil.rmtree` along with both target directories. Scripts, logs and dumps of this pass are under the scratchpad with the `f213p6-` prefix.
- This is a clean bounded pass on the changed material. It is not completed acceptance or sprint readiness.
