# F-050 review, pass 1

**Reviewed**: uncommitted F-050 work on `sprint/s17` at parent `825d53b54163afcc735422a465a539ffee434b37`. The pass covered `snapshot.rs`, `view.rs` decode, `presentation.rs` encode and decode, the wasm session exports, `packages/core/src/snapshot.ts`, the budget file and the size sentences in the LLD.

**Author / reviewer**: The same session wrote the implementation and this pass. The operator directed that session to review and repair without waiting on DeepSeek. This is not an independent reviewer. The discriminating check below was executed rather than inferred.

**Result**: 0 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

AC1 and the unfitted rule. `View::decode` forces fit scale 1 when `fitted` is false and ignores the stored anchor. Replacing that force with the stored scale, which the property records set to 4, made `property_family_round_trips` fail at `crates/ocelli-viewport/tests/snapshot.rs:126`. Camera component 0 was bits `1048576000` against the reference bits `1065353216`. Restoring the force made the same test pass.

AC14. `bin/ocelli.sh wasm` had already produced `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm`. `python3 scripts/pin_and_size_check.py --with-size` printed `wasm size 2,151,196 bytes, baseline 2,151,196, ceiling 2,258,755` and exited 0. The budget names F-050, from 1,999,061, delta 152,135, and keeps the note that the number does not close gate A4. `--accept-size` was not used, so the F-039 record remains under `previous`.

AC8 and AC12. `StackViewport::restore` assigns the decoded view, presentation and CSS grid and does not call into `ocelli-render`. The render-counter test is `#[ignore]` and was not run on an adapter. It is not GPU evidence.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-viewport --test snapshot property_family_round_trips -- --exact` with unfitted fit scale left at the stored 4 | `view.rs` decode | executed now | 1 failed, component 0 bits `1048576000` against `1065353216` |
| Same command after restoring `else { 1.0 }` | restored `view.rs` | executed now | 1 passed, exit 0 |
| `python3 scripts/pin_and_size_check.py --with-size` | `ocelli_wasm_bg.wasm` at 2,151,196 bytes | executed now | exit 0, ceiling 2,258,755 |
| `python3 scripts/prose_check.py` | plans, reviews, LLD and sprint prose | executed now | exit 0, 484 files |
| `SNAPSHOT_MAGIC` is `OCS1` in `snapshot.rs` and `generated.ts` | both files | executed now by search | the hand fixture's `"OCS1"` matches the constant |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Encode stores camera inputs, display area and presentation. `Record` has no matrix field. `deny_unknown_fields` is set on the record and on the nested objects.
- Decode checks magic before version. A refusal returns before `StackViewport::restore` assigns. Viewport id is written and not applied.
- A LUT sequence does not also carry rescale or a window. `encode_lut` copies `values()` and does not clone the descriptor. Entry count 0 stays the declared count.
- Presentation shape is identity or inverse. Absent on `MONOCHROME1` is not rewritten as an explicit inverse.
- The shell imports `SNAPSHOT_MAGIC`, `SNAPSHOT_VERSION` and `SNAPSHOT_ERROR` from `generated.ts`. It does not declare a second table.
- The three wasm exports return bytes or a code. They do not hand out a view of linear memory.
- Tier A, B and C share this record. The path opens no device.
- D-26 names the measured 2,151,196 bytes. The LLD current-baseline sentences name the same number. F-039's 1,999,061 bytes stays as the previous baseline.

## Limitations and handoff

Pass 1 reviewed its own implementation. A later reader should treat the executed fit-scale failure, not this summary, as the evidence. The ignored render-counter test was not opened on an adapter. Cold start was not remeasured, and gate A4 stays open. The next action is to stage this tree, run the feature profile on that index, and record only that run.
