# F-X021 review, pass 7

**Reviewed**: the two edits after pass 6 on branch `sprint/s20`. `git rev-parse HEAD` is `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. This pass read the left-aligned assertion in `tools/oracle/src/produce.rs` and the `git push, hooks enabled` control in `scripts/guard_probe.py`. The rest of the uncommitted tree was not re-opened. Decision 5's five formulas were not re-derived.
**Author / reviewer**: independent pass. This reviewer did not write the delta.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The floor gate expected `Some("pixel-layout-refused")` for `synthetic__ct_signed_12in16_left` and observed `Some("pass")`. Decision 5 says a native frame whose high bit is not `BitsStored - 1` is unpacked with the conforming high bit, and that `StoredBits::new` still refuses the file high bit. The challenged claim is that the new assertion locks that pass, rather than a row that should stay refused.

The guard claim is that the pre-push control now supplies comparison evidence the same way the check-commit control does. It must not forge a trailer, skip the hook, or make the unverified push succeed.

## Evidence

| Check | What was read | Observation |
|---|---|---|
| `pixel_facts` and `unpack_frame` | `tools/oracle/src/produce.rs` 791-802 and 853-866 | The file high bit is kept on the facts. Both calls pass `bits_stored - 1` into `StoredBits::new`. High bit at or past the container is still `pixel-layout-refused`. |
| `StoredBits::new` | `crates/ocelli-pixel/src/stored_pixel.rs` 185-186 | `high_bit != bits_stored - 1` returns `InvalidHighBit`. File high bit 15 with 12 stored bits is that refusal. Conforming bit 11 is not. |
| Candidate bytes | `write_pair` at 1644-1658 | The reference raw is copied only into the reference subset. The candidate raw is `frame.rgba` from `render_row`. |
| Comparator reason | `compare_written` 2106-2122 and `compare_view` in `attribution.rs` 822-925 and 932-964 | Reason `pass` with disposition `compared` is `Outcome::Pass`. That branch requires the class-one predicate and the bias to pass, and the decimated and weak overlays must not demote it. A layout refusal stays disposition `failed`. |
| New assertion | `produce.rs` 2346-2360 | Disposition must be `compared` and reason must be `pass` for `synthetic__ct_signed_12in16_left`. `None` or `pixel-layout-refused` fails the test. |
| Pre-push | `.githooks/pre-push` 15 | The hook calls `check-commit` with `--require-comparison`. |
| `check-commit` | `scripts/verify_ledger.py` 1081-1134 | It reads the commit message. It does not read the ledger. Comparison evidence is a 64-hex `comparison`, a positive `comparison-views`, and `comparison-verdict=pass`. |
| Control body | `scripts/guard_probe.py` 183-197 | Hooks stay enabled. `probe-note.txt` is staged. `write_green_comparison_report()` runs after `stage_all`. `record` then takes `--comparison-report .claude/probe-comparison.json`. The commit message is `F-000, a probe` and contains no trailer. |
| Report fixture | `green_comparison_document` at `scripts/guards/catalogue.py` 1025-1035 | The same contract `greenReport` the check-commit control already uses. Two distinct `.keep` directories. Not a trailer line. |
| Hook path | `.githooks/commit-msg` 22-30 and `cmd_trailer` 1063-1077 | A hand-written `Ocelli-Verify` line is refused. The hook appends `trailer` only when that command exits 0. `trailer` prints nothing without a ledger entry for the staged tree, and it adds the comparison fields only when that entry is valid. The commit is not `--no-verify`. `Sandbox.write` does not stage, so the report stays out of `git write-tree`. |
| Refusal probe | `catalogue.py` 8687-8695, 2541-2553, and `sandbox.py` 366-372 | Unchanged. `_enable_hooks` then `_push`. `run_probe` does not call `_prepare_control`. The sandbox base commit is message `base`, made before hooks are enabled. `reset` returns to that commit. `check-commit` on it has no trailer, so the hook prints `Push refused`. A leftover ledger cannot make that head healthy, because `check-commit` does not consult the ledger. |

Prior receipts, not re-run here. Pass 6 recorded this view as pass with equal hashes and signed mean 0. After the assertion edit, `bin/ocelli.sh test ocelli-oracle applicable_monochrome_rows_reach_the_comparator -- --test-threads=1` exited 0, with 1 passed and 115 filtered out. After the control edit, `python3 scripts/guard_probe.py --only hooks.pre-push.unverified` exited 0. Its output was `red hooks.pre-push.unverified` and one control green.

The updated test locks a real comparator pass of the rendered left-aligned row. It does not hide a refusal decision 5 still wants. The control does not forge the trailer, does not skip the hook, and does not make the unverified push look healthy.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Limitations

This pass did not re-run the full floor gate or the compare produce. It did not re-run the producer filter or the guard probe. The exits above are the edit receipts. No patient pixels were read.
