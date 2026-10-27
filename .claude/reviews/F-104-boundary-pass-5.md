# F-104 review, pass 5

**Reviewed**: the pass 4 repair, before the feature receipt. Repaired source hashes: `boundary.rs` `65582fdeb06e5de4a7a4f5a8fc252fc8ad12f169aecf25614b8eac088e78a7b7`, `typegen.rs` `73b2e8d84511933278e351d33ac1b610afc14475492eba0c82f091b190e8eda4`, `session.rs` `c068e4e7967325df633e6f4e90f56851f6300534598812a4cb9749b99575ac0c`, `boundary.wasm.test.ts` `f68fee716697622dbac49641650fe72d4311a5ae47bf746c2b80a6916aa4431a`. This review file is the record of the pass and is not one of those hashes.

**Author / reviewer**: The same session wrote the repair and this pass. The operator directed that session to review and repair without waiting on DeepSeek. This is not an independent reviewer. The discriminating checks below were executed rather than inferred.

**Result**: 0 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

AC9, again. The presented-event test now binds an 80 by 40 canvas and expects `FLAG_INVERT` at payload offset 12. Swapping the width and height expectations failed with `expected 80 to be 40`. Restoring them passed. A swapped `push_presented` extent can no longer hide behind a square canvas, and a missing flags word would read 0 rather than the explicit inverse.

The generator contract. `generated_path` asserts when the workspace root cannot be found, and the committed-file read asserts on an error. Neither path returns success. `cargo test -p ocelli-viewport --test typegen` passed, 2 tests. Clippy on `ocelli-viewport` passed. `panic!` is denied in this workspace, so the refusal is `assert!`.

HLD 17.1. `boundary.rs` carries `compile_error!` unless `target_endian` is little. `bytes_of` stays the single copy. A big-endian compiler was not run. The guard is the compile failure the smell asked for.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-viewport --test typegen` | repaired `typegen.rs` | executed now | 2 passed, exit 0 |
| `bin/ocelli.sh clippy ocelli-viewport` | repaired `boundary.rs` and `typegen.rs` | executed now | exit 0 |
| `bin/ocelli.sh clippy ocelli-wasm` and `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | repaired `session.rs` | executed now | both exit 0 |
| `npx vitest run --config vitest.boundary.config.ts` | rebuilt dev module and repaired test | executed now | 12 passed |
| Same vitest filter, expectations swapped to width 40 and height 80 | same module | executed now | 1 failed, `expected 80 to be 40` at payload offset 4 |
| Same filter after restoring 80 then 40 | same module | executed now | 1 passed |
| `cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown` | not an F-104 file | executed now | exit 101 in `ocelli-render/src/tier_b.rs` `needless_return`, wasm32 only, pre-existing, not edited |

## Defects

None.

## Smells

None. The three pass 4 smells are closed by the repair above. `flag` still matches the invert constants for a flip byte. Those constants are 0 and 1, which is the boolean the decoder already accepted. A second match would copy the same arms.

## Nitpicks

None. `Session::copy_state` now names `STATE_BYTES`. `drain_events` imports `EVENT_HEADER_BYTES`, `EVENT_STRIDE` and `RING_CAPACITY`.

## Verified clean

- The state layout test and the generator comparison still pass on the repaired tree
- The presented payload is generation, then width, then height, then flags. The live module wrote width 80, height 40 and `FLAG_INVERT` for an explicit inverse on an 80 by 40 device at device pixel ratio 1
- The camera readback still matches the derived fit. Element 0 is `512/600` and element 12 is `-256/3`
- No new `unsafe`, no `wasm-bindgen` outside `ocelli-wasm`, and no pixel getter was added
- The earlier floor and corpus run exited 0 on index `3d7bde55a538`. That receipt does not cover this repair

## Limitations and handoff

Pass 5 reviewed its own repair. A later reader should treat the executed swap, not the author's summary, as the evidence. A big-endian target was not compiled. The wasm32 clippy failure in `tier_b.rs` is outside F-104 and was left unchanged. The next action is to stage this repair, run the feature profile on that index, and record only that run.
