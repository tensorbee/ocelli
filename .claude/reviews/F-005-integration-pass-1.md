# F-005 review, integration pass 1

**Reviewed**: `739ba11`, the integrated F-ID commit, against `d74ad3a`.
**Reviewer**: the integrator, independent of the implementing agent.
**Result**: 0 defects, 0 smells, 0 nitpicks beyond the three the author recorded.

## Defects

None.

## Smells

None.

## Verified clean

Checked by execution rather than by reading, in the order `/microscope` puts
them.

**Arithmetic and casts.** Zero `as` casts in `crates/ocelli-core/src/error.rs`
and `crates/ocelli-wasm/src/panic.rs`. The three grep hits are two `use ... as _`
imports and one comment. Zero `#[allow]` anywhere in the new Rust, and the two
hits are comments stating that none was added. Zero `unwrap` and zero `expect`.
Line 279 says the arity is written as a `match` on the length rather than
`len() as u8`, which is the discipline HLD 27.3 asks a human to check.

**Would the test fail if the code were wrong.** I re-ran the mutation myself
rather than accepting the author's report. `ci/error-codes.json` renumbered
`Panicked` from 1 to 2, `bin/ocelli.sh gate errors` exit **1** with three
specific messages naming the Rust, the registry and the missing `MESSAGES` line.
Control run after restoring the file, exit **0**, and `git diff` clean. The
guard refuses, and its refusal is specific rather than generic.

**Fixture provenance, HLD 27.2 R2 and R3.** `ERROR_BYTES` and `LOG_BYTES` are
hand-derived byte by byte, each offset carrying its own comment with the
little-endian reasoning, and each is used by both an encode test and a decode
test. They were computed from the layout, not copied from the encoder's output.

**The layout against the specification.** HLD 17.3's `Event` is
`kind: u32, viewport: u32, seq: u64, payload: [u8; 32]`, a 48-byte stride with
`payload` at offset 16 and exactly 32 bytes long. `Record::BYTES` is 32 and fits
it exactly.

**The panic strategy, re-measured independently.** On rustc 1.97.1,
`rustc --print cfg --target wasm32-unknown-unknown` reports `panic="abort"`, the
host reports `panic="unwind"`, and `-C panic=unwind` overrides the target. So
the story's premise holds and a catching boundary would not have worked.

**Boundary and posture.** `unsafe_allowlist_check.py` reports 32 files checked
and 2 permitted, and neither permitted file exists, so `unsafe` is still at zero
files. `check-bindgen-isolation.sh` green. `no_std_check.py` reports 9 crates,
and the attribute is present in exactly those 9, with `ocelli-compute` and
`ocelli-render` mentioning it only in prose per D-10. The set is unchanged from
the base commit, so `ocelli-core` did not drop `no_std` to make D-15 work.

**Gates.** `gate --floor` ALL GREEN over 23 gates, up from 21 by `errors` and
`panic`. `gate corpus` pass over 91 rows. Both read from the command's own exit
status and not from the end of a pipe.

## Carried to the sprint review

- The story's own review is a self-review. This pass is the independent one.
- The two new guards must appear in F-X009's catalogue, which is why the handoff
  lists them.
