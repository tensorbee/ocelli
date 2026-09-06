# F-005 review, pass 1

**Reviewed**: working tree on `work/f-005-claude`, base `d74ad3a`
**Result**: 0 defects, 0 smells, 3 nitpicks

Self-review by the implementing agent, so it is a first pass and not an
independent one. Every claim below was executed rather than read.

## Defects

None outstanding. Four were found and fixed inside this pass, recorded here
because a fix nobody recorded is a fix nobody can re-check.

### Fixed, F1, a false count in `docs/lld/errors.md`

"Four exports, and two of them are called exactly once" sat above a table
showing three called exactly once. Corrected to three. HLD 27.3's rule is that
a claim in prose is checkable, and this one was checkable and wrong.

### Fixed, F2, a false count in `docs/lld/build-targets.md`

"The module exports four functions, up from one" is not what
`WebAssembly.Module.exports` reports: wasm-bindgen adds
`__wbindgen_add_to_stack_pointer`, `__wbindgen_export`, two globals and
`memory`. Corrected to "four functions of ours", with the glue named.

### Fixed, F3, an assertion in `scripts/panic_probe.mjs` that could not go red

`thrown instanceof WebAssembly.RuntimeError || thrown instanceof Error ||
thrown instanceof Object` is true of almost any thrown value. That is the
defect class a green suite cannot report on. Measured what the trap actually
produces on node 24, which is `WebAssembly.RuntimeError: unreachable`, and
narrowed the assertion to that.

### Fixed, F4, a weak assertion in `scripts/tests/test_error_code_check.py`

`any("0" in p for p in problems)` also matches the string `700`, so the
code-zero case would have passed on any problem at all. Narrowed to `code 0`.

## Smells

None outstanding. Two were found and fixed.

### Fixed, S1, `describe` exported from a package root

`packages/core/src/index.ts` re-exports every name in `errors.ts`, and a bare
`describe` at the root of a published package collides with the one every test
runner already has. The three new test files in this diff already had to write
`import { describe as group }` to avoid it, which is the warning. Renamed to
`describeError`. `.claude/plans/F-005-design.md` item E writes `describe(code)`
informally and the plan's decision is the table, not the identifier, so this is
an adaptation rather than a relitigation, and it is reported.

### Fixed, S2, a `Display` impl nothing executed

`impl fmt::Display for Record` was public, correct and covered by no test.
Added `display_carries_the_numbers_and_no_sentence`, which also pins the fact
that `Display` carries no sentence, because section 23 puts the sentence on the
shell side and a second copy in Rust would be a second thing to keep in step.

## Nitpicks

### N1, `Severity::Recoverable` has no producer today

`ErrorCode` was held to variants with a live producer, and `Severity` was not.
The difference is defensible: `Severity` is a two-valued field the layout
requires, and a one-valued enum would be a field that cannot vary. Recorded so
the asymmetry is deliberate rather than accidental.

### N2, log lines share the error code space

`Record::log` takes an `ErrorCode`, so a log line's code comes from the error
registry. The design plan says the log registry is the same registry and that
log-specific codes arrive with the stories that produce them, so this is what
was asked for. It will read oddly until the first log-only code exists.

### N3, `crates/ocelli-wasm/Cargo.toml` still names F-096 in a comment

Pre-existing, in a comment this story did not touch, and the base commit
`d74ad3a` is titled "correct the boundary story's F-ID in tracked prose". A
Cargo comment is not tracked prose by `scripts/prose_check.py`'s scope, so it
was missed. Left alone rather than folded into this diff, and reported.

## Verified clean

**Arithmetic and casts.** `grep -nE "\bas\b [A-Za-z_]"` over the three new or
changed Rust files returns only `use core::fmt::Write as _`. There is no
numeric `as` cast anywhere in the diff and no `#[allow]`. Narrowing is
`u32::try_from` with `unwrap_or`, enum-to-integer is a hand-written `match` so
every number is on its own line, and the address is
`core::ptr::from_ref(&PANIC_SLOT).addr()` rather than a pointer cast.

**No `panic!`, `unwrap()` or `expect()`.** `grep -n "panic!\|unwrap()\|\.expect("`
over the diff's Rust returns only two doc-comment mentions. The three
deliberate panics this story needs arrive through a failing `assert!` on a
`core::hint::black_box` condition, so no lint is switched off.

**Would the tests fail if the code were wrong.** Six mutations, each run in its
own command, each observed red, each reverted, and the tree confirmed clean
afterwards with `git diff --stat`.

| Mutation | Went red |
|----------|----------|
| `ErrorCode::Panicked.number()` 1 to 2 | 2 tests in `ocelli-core` |
| `ci/error-codes.json` Panicked 1 to 2 | `gate errors`, exit 1 |
| `PanicSlot` fields `code` and `msg_len` swapped, offsets 8 and 12 | `gate panic`, 4 assertions, exit 1 |
| `seal()` emptied, so the magic is never written | `gate panic`, and 3 `ocelli-wasm` tests |
| one byte flipped in the hand-written `ERROR_BYTES` | 3 tests in `ocelli-core` |
| `debug_assertions` removed from the console import | size gate, 17,352 over a 17,207 ceiling |

**Expected values are not from the implementation.** `ERROR_BYTES` and
`LOG_BYTES` were written from the layout table with the working shown per
offset in the doc comment above each, before the encoder existed. That was
demonstrated rather than asserted: the encoder was stubbed to return
`[0u8; 32]` and the two vectors reported the full expected arrays as `right`
before any real encoder ran. `packages/core/src/errors.test.ts` copies the same
arrays, so the two sides are two implementations of one layout and neither
derives from the other. `scripts/panic_probe.mjs` states the panic record's
layout a third time, for the same reason.

**Claims in prose, executed.**

- "The HLD has no logging section." `grep -rn -i "structured log\|logging\|tracing\|log level" docs/hld/*.md` returns nothing, exit 1.
- "The two allow-listed `unsafe` files do not exist." `ls` reports both absent, and `gate unsafe` reports 32 files checked, 2 permitted, none found.
- "D-15 is load-bearing." Reverting the workspace entry to `thiserror = "2"` turns `gate nostd` red with `ocelli-core declares no_std and reaches a std feature: thiserror feature "std"`. Restored.
- "Base commit reproduces 14,104 bytes." A throwaway worktree at `d74ad3a` built 14,104 exactly, so the 2,284-byte delta is this story's. The worktree was removed.
- "The 528-byte record costs the module nothing." Raising `MESSAGE_CAPACITY` from 512 to 1,536 left the module byte-identical at 16,388.
- "Both new gates are in the CI floor." `scripts/ci_floor_check.py` reports all 23 floor gates invoked by CI, up from 21.

**Boundary and tier.** `gate bindgen` green, so wasm-bindgen is still confined
to `ocelli-wasm`. No pixel crosses the boundary and this story touches none. The
only new view over linear memory is in `packages/core/src/panic.ts`, which is
the second and last file granted the allowance, argued in the plan's item F and
recorded in `eslint.config.js` and in two LLD files. `packages/core/src/ring.ts`
was **not** granted one. No allocation is added to any render loop: the record
is a static, the format buffer is a stack local, and the one `Box` is the hook,
allocated once at worker start. Tiers A, B and C all answer "full and
identical", which is asserted rather than assumed by the fact that nothing in
the module branches on a tier.

**Structure.** No new trait, no new generic parameter, no `Box<dyn>` with a
statically known type, no wrapper that only forwards. `CoreStatus` is a
discriminated union rather than an interface, for the reason `AGENTS.md` gives.
The one feature flag, `panic-probe`, has a named user, `scripts/panic_probe.mjs`.

**The whole floor.** `bin/ocelli.sh gate --floor` with `OCELLI_PYTHON` pointed
at an interpreter with pydicom: ALL GREEN, 23 gates, no skips.
