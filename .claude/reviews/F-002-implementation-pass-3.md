# F-002 implementation review, pass 3

**Scope**: pass 2's remediation, which was unreviewed work until this pass,
and a final read of the whole F-002 diff.
**Result**: 0 defects, 0 smells, 0 nitpicks. **Clean.**

## Pass 2's remediation, re-read

The `wasm-opt` comment now cites `rustc --print cfg --target
wasm32-unknown-unknown | grep target_feature` as the source of the flag list,
tells the reader to re-run it if the toolchain moves, and states plainly that
only `--enable-bulk-memory` is required today. Each of those three is
checkable and each was checked. The flag list itself is unchanged, and the
artefact reproduced at 14,104 bytes after the edit, so the recorded baseline
still describes the build that produced it.

Nothing new introduced. The change is comment text only.

## Final read of the whole diff

Eleven files. Re-read in full against the plan's approach steps.

| Plan step | Landed | Evidence |
|-----------|--------|----------|
| 1, declare the dependency target-gated in ocelli-wasm only | yes | `gate bindgen` green, both target passes |
| 2, one real export | yes | `ocelli_version()`, present in the generated `.d.ts` |
| 3, take the skip out of the gate | yes | `gate --floor` reports ALL GREEN over 17 gates with no SKIPPED line |
| 4, close the target-blindness in the isolation check | yes | proved red, and the host pass proved blind to the same mutation |
| 5, record the baseline | yes | 14,104 bytes in `ci/wasm-size-budget.json` |
| 6, prove the gate goes red | yes | four mutations, all reverted, all recorded |

The two decisions taken in the design round both landed as written:
`wasm-bindgen = "=0.2.127"` is an exact pin and is in `EXACT_PINNED`, and the
export is `ocelli_version()` rather than a bare `start` shim.

## The four mutations, collected

| Mutation | Expected | Observed |
|----------|----------|----------|
| `ocelli_version()` returns `CRATE_NAME` | test red | `left: "ocelli-wasm"  right: "0.1.0"` |
| `[workspace.package].version` to `0.2.0` | test red | `left: "0.2.0"  right: "0.1.0"` |
| Recorded baseline lowered to 10,000 | size gate red | over the 10,500 byte ceiling |
| `wasm-bindgen` added to `ocelli-core` under a wasm32 target gate | isolation red | `FAIL: ocelli-core reaches wasm-bindgen under wasm32-unknown-unknown`, while the host pass reported zero hits |

All four reverted, and the floor is green after the reverts.

## One trap worth recording for the next story

Reverting a mutation with `mv file.bak file` restores the backup's **older
mtime**, and cargo then reuses the build from the mutated source. It showed up
here as a false red on reverted code, which is the harmless direction. The
same mechanism can produce a false **green** when the mutation is applied to a
file whose backup is newer. `touch` the file after any revert before believing
the result.
