# F-210 review, pass 5

**Reviewed**: `git diff 354b73a HEAD` on `work/f-210-xai`, HEAD `5c0ba943bbf4b7d0c7d9ba93e2c560e3113347c5`, tree `2792e5c0f925e7fc5ddd2d8045a1ea1144a79fc0`. That is the pass 4 remediation commit `5c0ba94`. Nothing was staged or unstaged before or after the pass. The only untracked paths are the pass 1 to 4 reports and this report.
**Author / reviewer**: The remediation is by a Claude implementer agent. This review is by a separate Claude reviewer agent that wrote none of F-210, its plan, its remediations or the earlier reviews. All are the same model family, which limits independence.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **What changed.** `git diff 354b73a HEAD --stat` is two files. `held.rs` is the `claim_buffer` doc comment rewrapped, and the word sequence of the removed and added lines is identical, so it is pass 4 N1 and nothing more. The longest line is now 78 columns. `tests/value.rs` adds the `inflight` helper and `value_route_refusals_are_400_and_release_their_charge`. No source file outside `tests/` changed behaviour.
- **Pass 4 S1, the two refusal arms of `value_route`.** I traced each of the test's five requests to the arm it claims.
  - `frame 3` of single-frame slice 0, asked before anything decodes. `preview_source` is `Shared`, so `decode_for_value` runs, and `frame_parts` refuses `Frame` before `prepare_samples` and `on_samples`. That is arm 1 (`serve.rs:1317`) with no charge.
  - `outside point` (x = 200.5 on 128 columns, `synthetic.rs:11`). Frame 0 decodes and is stored, then `value_json` refuses `point`. That is arm 2 (`serve.rs:1326`) after a charged successful decode.
  - `Modality LUT`. The frame decodes, `prepare_held` hits `modality_blocked` and sets `value: Err(Window)`, then `value_json` refuses `window`. Arm 2 again, after a charged decode.
  - `short pixel data` (1 x 2, 16-bit, Pixel Data of 2 bytes against 4 expected). `/file` accepts it, `borrowed_pixels_fit` fails so the full parse runs, `frame_parts` and the `output_len` check pass, `prepare_samples` sizes the buffer and `on_samples` charges it, then `decode_frame` refuses `decode`. That is arm 1 after the charge. The probe and MR3 and MR4f below confirm the charge happened first: removing the failure release leaves exactly 8 bytes (the buffer's capacity) in flight on this case only.
  - `decoded value`, slice 1 at 0.5,0.5, a plain 200 with `source` `decoded`.
- **Pass 4 S2, the inflight bookkeeping.** After every one of the five requests the test reads `GET /held` and requires `inflight` 0, on both the direct viewer and the process server. The `inflight` helper splits on the exact key `"inflight":`, and the census JSON (`serve.rs:445`) has no other key with that prefix, since `peak_inflight` is only in the trace line format.
- **Ordering hazard pass 4 named.** Frame 3 is asked before frame 0 decodes, so the single-frame file's bytes are still held and the refusal is not F-216's 410 `file-gone` rule. The probe shows frame 3 at 400 `frame` on both viewers.
- **Test comment claims.** One frame and 128 columns for slice 0 is true (`synthetic.rs` writes no Number of Frames and `COLUMNS = 128`). "Refused after the sample buffer is charged" is true by MR3 and MR4f. "Every decode, answered or refused, leaves no bytes in flight" is what the five checks assert, and the frame 3 request does enter `decode_held`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD tree above, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-dicom` | same, `OCELLI_PORT=0` | now | 0. Every binary `ok`, folder viewer lib 72, `tests/value.rs` 24 (was 23), `tests/preview.rs` 59. The `ocelli-dicom` corpus binary has 3 ignored |
| `npm run lint` | HEAD | now | 0 |
| `bin/ocelli.sh gate guards`, `bin/ocelli.sh gate bench` | HEAD | now | 0 ALL GREEN, 0 ALL GREEN (bench floor: 155 pass, 2 skipped) |
| `python3 scripts/guard_census.py` | HEAD | now | 0, "0 watched by nothing" |
| `python3 scripts/prose_check.py`, `scripts/source_provenance_check.py` | HEAD | now | 0 "voice rules clean over 878 file(s)", 0 "clean over 1734 files" |
| Baseline, the new test plus a scratch-only probe printing status, code, source and `inflight` for every case on both viewers | scratch `git archive HEAD` under `target/f210-p5`, own target dir | now | green. Codes `frame`, `point`, `window`, `decode`, then 200 `decoded` and 200 `held`, `inflight` 0 after every one |
| MR1 arm 1 answers `gone()` | scratch `serve.rs` | now | **red**: `direct frame 3: HTTP/1.1 410 Gone`. Probe: frame 3 and short both 410 on both viewers |
| MR2 arm 2 answers `gone()` | same | now | **red**: `direct outside point: HTTP/1.1 410 Gone`. Probe: outside point and Modality LUT both 410 on both viewers |
| MR3 scheduled failure: `release_counter` removed | same | now | **red**: `server short pixel data: 8 bytes left in flight`. Only the short case leaks |
| MR3b scheduled success: `leave_inflight` after `store_frame_with` removed | same | now | **red**: `server outside point: 24576 bytes left in flight` |
| MR4 direct path: `leave_inflight` removed | same | now | **red**: `direct outside point: 24576 bytes left in flight` |
| MR4s direct success only not released (`&& stored.is_err()`) | same | now | **red**: `direct outside point: 24576 bytes left in flight` |
| MR4f direct failure only not released (`&& stored.is_ok()`) | same | now | **red**: `direct short pixel data: 8 bytes left in flight` |

Each mutation was applied by a script that asserted its anchor occurred exactly once in the saved original and wrote from that original, so no mutation stacked on another. After the last run the original was written back and `cmp`'d. The scratch copy was then deleted with `shutil.rmtree` and the worktree's `serve.rs` and `tests/value.rs` were touched. No corpus symlink was made. At the end `git status` showed only the untracked review reports, HEAD was still `5c0ba94`, `git write-tree` was still `2792e5c`, and there were no staged or unstaged changes.

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** The short Pixel Data case checks only `400`, not code `decode`. The post-charge path is established by the mutations above, not by the assertion, so a later fixture change that moved the refusal before the charge (an attribute refusal, say) would keep the test green and quietly stop covering MR3 and MR4f. Pinning `decode` narrows that but does not close it, since the `output_len` check also answers `decode` before the charge.
- Carried and not counted again: pass 4 N2 (the steal test's reused +1 cannot tell a steal from a parked buffer of the same length), pass 3 N1 and N3, pass 2 N2, and pass 1 N2 to N4.

## Verified clean

- Pass 4 S1 is closed. Both `value_route` refusal arms now go red when sent down the 410 path, on both viewers, through the real HTTP caller.
- Pass 4 S2 is closed. All three `leave_inflight` or `release_counter` sites in `decode_for_value` go red when removed, and the direct path's single release was split into success and failure and each half goes red alone. That covers the scheduled path's charged-decode failure arm, which pass 4 recorded as unreached.
- Pass 4 N1 is closed. The rewrap changes no words.
- The test's five requests reach the paths they claim, checked against `value_route`, `decode_for_value`, `decode_held`, `finish_decode`, `frame_parts`, `prepare_held` and `modality_blocked`, and confirmed by the probe's codes and `inflight` values.
- The test is not timing-dependent. On the process server the failure release runs inside the worker closure before `permit.call` returns, and every success release runs before the route returns its reply, so `GET /held` after the reply sees the settled counter.
- No code outside `tests/value.rs` changed behaviour, so `README.md`, `use.html`, `more.html` and `docs/lld/frame-open.md` cannot have gone stale through this diff.

## Limitations and handoff

- **Not exercised.** A browser (AC16) and pydicom (AC17), which are external ACs for sprint close and are still open. No oracle or GPU job was run, as instructed.
- **Bench.** `gate bench` ran its floor tests. No `viewer.resident_memory` or `viewer.open` measurement was taken, and this diff adds test code only.
- **Probe.** `probe_p5` was a scratch-only test and was deleted with the copy.
- **Next owner.** This pass is clean on the reviewed tree `2792e5c`. The story can move to `/complete-feature` once its verification receipts are recorded against this tree. AC16 and AC17 stay open for the sprint close.
