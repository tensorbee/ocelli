# F-046 review, pass 2

**Reviewed**: working tree on `sprint/s15` against `1dfaa71`, remediation of pass 1 D1
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, `PresentationError::Unavailable` is unchanged

**Where**: `crates/ocelli-viewport/src/presentation.rs:59`
**What**: Pass 1 N1 is still the residual `Err(_)` arm. The plan still does not name it. This remediation did not touch it.
**Why it does not block**: An honest chain does not produce it. It exists so the match stays total without `unwrap` or `panic`. Pass 1 recorded that, and this pass does not require a change.

## Verified clean

Only `crates/ocelli-viewport/tests/presentation.rs` changed for this remediation. `set_window` still calls `replace_window(center, width, function)` at `presentation.rs:129`. `replace_window` still passes that function into `VoiTransform::new`. `SoftTissue` width is still `400.0` at `presentation.rs:35`. `git diff -- crates/ocelli-viewport/src/presentation.rs` printed nothing. The file is untracked, so that empty diff is not a blob comparison. The working line was read after the mutation was removed, and it is the pre-mutation call. `git diff 1dfaa71 --stat` is still the nine tracked paths, 93 insertions and 11 deletions. `lut.rs` and `voi.wgsl` were not edited.

**The new asserts.** `set_window_uses_the_voi_domain` reads `window()` back through `stored_window`. Width `0.5` is `InvalidWindow` for `Linear` and leaves `(0.0, 1.0, Linear)`. Width `1` stores `Linear`, then `LinearExact`. Width `0.5` then stores `LinearExact`. Width `0` for `LinearExact` is `InvalidWindow` and leaves `(40.0, 0.5, LinearExact)`. Width `0.5` stores `Sigmoid`. Width `0` for `Sigmoid` is `InvalidWindow` and leaves that `Sigmoid` window. The four plan rows are still there: `Linear` width `0` refused, `Linear` width `1` accepted, `LinearExact` width `1` accepted, `LinearExact` width `0` refused. The comment cites PS3.3 C.11.2.1.2 and C.11.2.1.3. The test does not recompute a display value and does not add a formula. It uses `assert!`, `assert_eq!` and `matches!`. It does not use `unwrap`, `expect` or `panic!`. The older helpers in this file still use `unreachable!` after an `is_ok` check. This remediation did not add those.

**Mutation.** `set_window` was changed to ignore `function` and call `replace_window` with `VoiFunction::Linear`. `bin/ocelli.sh test ocelli-viewport --test presentation set_window_uses_the_voi_domain` exited 101. The panic was `tests/presentation.rs:262`. It printed left `Some((40.0, 1.0, Linear))` and right `Some((40.0, 1.0, LinearExact))`. That is the readback after `set_window(40.0, 1.0, LinearExact)`. The forced call was restored before the suite run. Pass 1 D1 does not survive.

**Restored suite.** `bin/ocelli.sh test ocelli-viewport --test presentation -- --test-threads=1` exited 0. All 7 tests passed.
