# F-007 implementation review, pass 2

**Scope**: pass 1's one change, the CI Python pin, plus a second read of the
whole F-007 diff.
**Result**: 0 defects, 0 smells, 0 nitpicks. **Clean.**

## Pass 1's change, re-read

Pass 1 added `actions/setup-python@v5` to the CI `rust` job, because `gate
native` step 4 runs `scripts/target_feature_check.py` and that job previously
relied on the runner image happening to ship a `python3`. The `guards` job
already pins it the same way, so this makes the two consistent rather than
introducing a new pattern. No behaviour change locally.

## Second read of the whole diff

Nine files.

| Plan step | Landed | Evidence |
|-----------|--------|----------|
| 1, two binaries, both stubs, both real | yes | `cargo build -p ocelli-native --bins` links, both run and print |
| 2, four-step proof in `bin/ocelli.sh native` | yes | all four steps observed green, and three of the four observed red under mutation |
| 3, a `native` gate in the floor | yes | `gate --floor` reports 18 gates green, up from 17 |
| 4, feature unification caught rather than hoped for | yes | `scripts/target_feature_check.py`, three failure categories, all three proved |
| 5, prove the gate goes red | yes | four mutations recorded in pass 1 |

## Two things checked specifically because they are easy to get wrong

**The `--all-targets` asymmetry between steps 2 and 3 is correct and is
explained where it lives.** For wasm32 the flag pulls in dev-dependencies and
`proptest` reaches `wait-timeout`, which does not compile for wasm32 and is not
meant to. It was observed failing that way before the flag was removed, so the
comment describes something that happened rather than something anticipated.
Step 3 keeps the flag because a native build runs its tests.

**The baseline keys on package name without the version.** Checked against the
alternative: keying on name and version, which the first draft did, would turn
every routine `cargo update` of a transitive like `syn` into a red gate
reporting a stale declaration. That is a gate that goes red for reasons nobody
can act on, which is how a project learns to re-baseline reflexively. The
reason is recorded in the script and in the baseline's own note.

## What is deliberately not guarded, and is recorded rather than left implicit

`ocelli-wasm` is `native: no` in section 4's table and that is **not** enforced,
unlike `ocelli-native`'s `wasm: no`. The crate compiles natively today because
`wasm-bindgen` is target-gated, and `cargo test -p ocelli-wasm` depends on that.
The cell means the crate is not shipped natively, and its native compilation is
what lets the boundary's logic be unit-tested without a browser. A
`compile_error` there would cost real coverage to enforce a claim the table is
not making. Written into `docs/lld/build-targets.md` under the heading that
says the two cells are not symmetric.
