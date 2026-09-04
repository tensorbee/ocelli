# F-002 implementation review, pass 1

**Scope**: the working diff for F-002 (E1.2), wasm-pack build pipeline with a
hard size budget gate.
**Result**: 2 defects, 0 smells, 0 nitpicks. Both remediated.

## Defects

### D1. The strengthened isolation check required a target it does not need, and would have failed CI

`ci/check-bindgen-isolation.sh`'s new wasm32 pass gated itself on
`rustup target list --installed`, on the stated reasoning that the check could
not run without the target present.

**That reasoning is wrong.** `cargo tree` resolves cfg and does not compile, so
it filters the dependency graph for any target rustc knows. Verified directly
against an uninstalled triple:

```
$ rustup target list --installed
aarch64-apple-darwin
wasm32-unknown-unknown
$ cargo tree -p ocelli-core -e normal --target x86_64-unknown-linux-gnu
ocelli-core v0.1.0 (...)
└── glam v0.30.10
    └── libm v0.2.16
```

The consequence was not cosmetic. The CI `guards` job runs
`ci/check-bindgen-isolation.sh` and its toolchain step installs no extra
target, so the guard would have failed that job on every push for a reason
that does not exist.

**Remediation.** The rustup requirement is gone. The triple is still validated
against `rustc --print target-list`, and that check is kept for a real reason
rather than the removed one: `cargo tree --target <typo>` errors, the error
goes to `/dev/null`, and `grep -q` finds nothing in an empty stream, so a
misspelled triple would report a clean pass over zero crates. Observed red by
mutating the triple to `wasm32-unknown-unknwn`.

### D2. A false claim in a doc comment: `/release` does not update the version literal

`crates/ocelli-wasm/src/lib.rs` justified asserting a literal `"0.1.0"` by
saying `/release` updates it with the version bump.

`docs/RELEASE.md` line 57 says the opposite, explicitly: "The version bump
itself lands earlier, through its own F-ID, so `/release` never edits a
version." `.claude/commands/release.md` line 11 repeats it. A claim in prose
that is false is a defect, not a nitpick, because the next reader acts on it.

**Remediation.** The comment now says what is true, that a version bump has to
update this literal alongside `[workspace.package].version` and
`packages/core/src/index.ts`'s `VERSION`, and that no separate guard asserts
the three agree because this test already does.

**And the replacement claim was proved rather than asserted.** Bumping
`[workspace.package].version` to `0.2.0` turns the test red with
`left: "0.2.0"  right: "0.1.0"`, reverted after.

## What was checked and found clean

- Every `as` cast: there are none in this diff.
- Arithmetic: none. This story computes no pixel and no coordinate, which is
  what the plan's test table says, so HLD 27.2 R3 does not apply.
- `unsafe`: none added. `gate unsafe` green.
- Would the tests fail if the code were wrong: yes, both proved by mutation.
  `ocelli_version()` returning `CRATE_NAME` gives
  `left: "ocelli-wasm"  right: "0.1.0"`.
- The size gate: proved red by lowering the recorded baseline to 10,000 and
  observing the ceiling breach, reverted after.
