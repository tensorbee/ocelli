# F-037 review, pass 3

**Reviewed**: the staged working tree, `git write-tree` =
`5bdfabb559bf092155efc99b237895aa44423370`, against base `3b00890`, with the
pass-2 remediation delta read as
`git diff 62ebde95 5bdfabb5`. Independent reviewer, not the author.
**Result**: 2 defects, 1 smell, 5 nitpicks

**This pass is NOT clean.** Two defects and one smell block completion. Both
defects are in prose rather than in behaviour, and both are single-sentence
fixes.

All mutations were reverted. The tree hash at the end of this pass is the same
as at the start.

**Pass line**: F-037, pass 3, 2 defects, 1 smell, 5 nitpicks. Counts are falling
(10/3/4, then 4/1/5, now 2/1/5) and the findings change each pass, so the loop
is converging rather than circling.

## Defects

### D1, a FIFTH site of the two-devices claim, and it is the one the LLD reads from

**Where**: `docs/lld/tier-resolution.md:360-366`, the section headed "## The
device is transient".

**What**:

> The first probe device that opens is measured and dropped before `resolve`
> returns. It never becomes a `GpuContext`, so HLD section 31's one-device
> invariant is untouched: **there is never a moment when two devices exist.**

Unqualified, in the LLD's own voice, with no subject restricting it to the
probe. It is the LLD copy of the `probe.rs` module header that this remediation
just corrected, and it was not corrected with it.

**Why it is wrong**: `GpuContext::recover` creates the moment. The same staged
tree now says so in three other places, two of which this remediation wrote:
`crates/ocelli-render/src/probe.rs:13-19`, `crates/ocelli-render/src/gpu.rs:261`
and `docs/lld/gpu-ownership.md:241-247`. `docs/lld/tier-resolution.md` is
modified by F-037 and its header now reads "**F-IDs that contributed:** F-004,
F-037", so the story has claimed the file. The sentence is also load bearing in
a way the others are not: it is the one an `## The device is transient` section
heading invites a reader to take as the file's position.

**Evidence**: this is the answer to the coordinator's question 3, and the answer
is yes. The full inventory, by grep rather than by list:
```
$ grep -rn "never a moment" --include="*.rs" --include="*.md" . | grep -v node_modules
crates/ocelli-render/src/probe.rs:9      scoped, "on this path"            OK
crates/ocelli-render/src/probe.rs:539    quotes the header, about the probe OK
crates/ocelli-render/src/probe.rs:192    scoped, "on this path"            OK  (via resolve_adapter)
docs/lld/gpu-ownership.md:237            scoped, "On the resolution path"   OK
docs/lld/gpu-ownership.md:206            NOT scoped, see S1
docs/lld/tier-resolution.md:364          NOT scoped                         D1
.claude/plans/F-037-design.md:132        approved plan, see N2
.claude/plans/F-004-design.md:435        approved plan, see N2
$ grep -rn "one-device invariant" --include="*.rs" --include="*.md" . | grep -v node_modules
.claude/plans/F-037-design.md:128, .claude/plans/F-004-design.md:435,
docs/lld/tier-resolution.md:363
```
No sixth code site exists. `gpu.rs:157`, `gpu.rs:163`, `gpu.rs:396`,
`gpu-ownership.md:56-62`, `tests/device.rs:223` and
`ocelli-compute/tests/ui/no_owned_device_out_of_context.rs:4` all say "a second
device only arrives from a second `request_device`" or "a clone is not a second
device", which are different claims and all true.

### D2, "cannot be" is false, and it is the claim the deleted test was traded for

**Where**:
- `crates/ocelli-render/src/probe.rs:226-246`, the new comment above `detect`'s
  tier C short circuit: "NO TEST CATCHES THE DELETION OF THESE THREE LINES, and
  that is **a property of the design rather than a gap somebody can close
  cheaply** ... Asserting this needs **either** a timing bound, which this
  project refuses because a permanently flaky gate is a gate that gets disabled,
  **or** an injectable instance factory, which is a seam with one production
  caller."
- `crates/ocelli-render/src/probe.rs:1180-1194`, `a_cpu_override_yields_no_
  adapter`'s doc: "**The short circuit itself is asserted by NOTHING, and cannot
  be.**"

**What**: there is a third option, it needs no new seam and no timing bound, and
it kills the mutation. `resolve_adapter` already takes `clock: &mut dyn FnMut()
-> u64` from the caller. With the short circuit present the clock is never
invoked on the tier C path. With it deleted, `measure_candidates` reaches
`measure` and `run` calls it. Counting invocations is an assertion on a
parameter the function already has.

**Why it is wrong**: the sentence is the justification for deleting a test
rather than fixing it, and it is stated as exhaustive ("either ... or"). Under
microscope section 3 a declared residue is exactly the kind of claim that has to
be executed before it is accepted, because it closes a finding by argument.

**Evidence**: a temporary probe added to `probe::tests`, run and reverted.
```rust
let mut ticks = 0_u32;
let mut clock = || { ticks += 1; 0_u64 };
let resolved = super::resolve_adapter(TierRequest::Requested(Tier::Cpu), &mut clock);
assert!(resolved.is_none());
assert_eq!(ticks, 0, "the tier C override spent startup cost anyway");
```
```
unmutated:  test review_probe_cpu_override_spends_no_clock ... ok
with detect's three lines deleted:
            assertion `left == right` failed: the tier C override spent startup cost anyway
            test result: FAILED. 0 passed; 1 failed
```
It is not a timing bound: the clock returns a constant `0` and the assertion is
on a call count, which is `0` deterministically on every machine when the short
circuit is present. It is not flaky. It needs an adapter to have its killing
power, because on a machine with none `measure_candidates` returns `NoAdapter`
without calling the clock either, so its home is `#[ignore]` and the `gpu` gate,
which is the profile this story created for precisely this shape.

**This is the answer to the coordinator's question 2.** Deleting the test was
the right call for the test that was written. Concluding that no test is
possible was not. The correct residue sentence is narrower: the RETURNED
`Resolution` is provably identical with or without the short circuit, so no
assertion on the return value can catch it, and what is observable is the
caller's clock.

**What IS true, and I verified it, is the part the comment gets right.**
`caps::classify` step 1 at `caps.rs:737-759` returns early for
`TierRequest::Requested(Tier::Cpu)` with `adapters_seen: 0`, `fill_rate: None`
and `device_created: false` hardcoded, reading only `signals.bands` and
`signals.simd` from the argument, and `TierSignals::unprobed()` at
`caps.rs:565-571` sets those two to `SimdSupport::build_target()` and
`FillRateBands::RECORDED`, which is exactly what `detect` builds on the long
path. So the `Resolution` is byte-identical either way. The comment's
description of the mechanism is correct. Only its conclusion about testability
is not.

## Smells

### S1, `gpu-ownership.md:203-207` still carries the unscoped conjunct, four paragraphs from the one that was scoped

`docs/lld/gpu-ownership.md:205-207`, in "What this story deliberately does not
do": "F-004's probe device is transient: created, measured on and dropped inside
`resolve`, so it never becomes a `GpuContext` and **there is never a moment when
two devices exist.**"

It is grammatically bound to "F-004's probe device", which is why this is a
smell and not D1's twin. But `:237` in the same file was just rewritten to open
with "**On the resolution path**", and a reader who accepts that fix reaches the
identical unqualified clause thirty lines earlier with no such prefix. One file
should not scope the same claim in two different ways. The cheapest fix is the
same four words.

## Nitpicks

### N1, "F-037 has landed" while `BACKLOG.md` says `in-progress`

`docs/lld/tier-resolution.md:460` and `:467`. Raised in passes 1 and 2, still
not addressed, still not blocking. It becomes true at `/complete-feature`.

### N2, the approved design plans carry the now-false unqualified claim

`.claude/plans/F-037-design.md:128` and `:132`, and
`.claude/plans/F-004-design.md:435`. **Do not edit these.** An approved plan is
the record of what was approved, and rewriting it after the fact destroys the
evidence that the implementation departed from it. The right home for the
correction is the AS_BUILT entry's note, which is where `/complete-feature`
records what the plan predicted and what was built. Flagging it so the next
grep for this sentence does not read the plans as live claims.

### N3, `scripts/ci_floor_check.py:463-465` is still short-wrapped

The coordinator asked. It is unchanged:
```
463 # F-037. It runs every `#[ignore]`d test in `ocelli-render`, which are the ones
464 # needing a real adapter. Deviation D-04
465 # is that CI has no GPU, so nothing in CI may run it, exactly as for `oracle`.
```
Cosmetic. No formatter reaches Python comments.

### N4, "0.96s" is a wall clock in a comment

`probe.rs:235`, "moves the lib target from 0.01s to 0.96s". Re-measured this
pass: 0.01s unmutated, 0.99s mutated. The figure is attributed to the second
pass and was 0.96s then, so it is not false, but a wall clock written into a
comment drifts with the machine. "under a second" would not.

### N5, `a_cpu_override_yields_no_adapter` is worth keeping, barely

**The coordinator's question 4.** Its doc now says "**It kills no mutation.**"
and that is accurate, which I verified with a mutation the doc does not name:
making `classify`'s step 1 return tier A `Caps` for a `Cpu` override leaves it
green, because the adapter is `None` from the short circuit and every route
after that returns `None` regardless. So its only content is that the tier C
path returns `None` end to end through the public API, in the floor, for nothing.

Keep it, because the disclosure removes the "counted as coverage forever" risk
the microscope's section 2 is about, and because the clock-count test of D2
would subsume it anyway: that test asserts `resolved.is_none()` too. If D2 is
taken, fold the two into one and this nitpick disappears.

## Verified clean

Everything below was executed on the pass-3 tree.

**Pass-1 and pass-2 findings, re-verified.**

| Finding | State |
|---------|-------|
| p1 D1, "identical product" | FIXED, four sites, arithmetic correct |
| p1 D2, rebuild test proved no rebuild | FIXED, mutation red again this pass |
| p1 D3 / p2 D1, `Recovered` caps claim | **NOW FIXED in `gpu-ownership.md:271-281`.** The replacement is correct: `open` copies the resolved `Caps`, immutable on the retained adapter, so a rebuild reports the same four values by construction. `Caps` does have exactly four fields |
| p1 D4 / p2 D2, "never two devices" | FIXED at `probe.rs:9`, `probe.rs:192` and `gpu-ownership.md:237`, including the module-header site I did not flag. NOT fixed at `tier-resolution.md:364`, which is D1, and unscoped at `gpu-ownership.md:206`, which is S1 |
| p1 D5, wildcard claim | FIXED. The wildcard mutation is still green, and both `caps.rs` and `tier-resolution.md` now say a runtime test cannot catch it and the compiler can |
| p1 D6, spliced doc comment | FIXED |
| p1 D7, unreached `opens_a_device` guard | recorded rather than closed, correctly. Deleting the `if` still leaves the whole suite green, re-verified |
| p1 D8, LUT-shader coverage claim | FIXED at five sites |
| p1 D9, past tense about unwritten stories | FIXED |
| p1 D10, stale `CURRENT_SPRINT.md` quotation | FIXED |
| p1 S1 `guards.md`, S2 threads, S3 enumeration | FIXED |
| p2 D3, `WORKFLOW.md`'s "wave-planning rule below" | FIXED. It now cites `docs/sprints/CURRENT_SPRINT.md` and quotes it accurately against `:183-189` |
| p2 D4, the test's headline over-claim | FIXED by rewriting the doc down to what the test asserts |
| p2 S1, the test kills no mutation | disclosed rather than fixed. See N5 and D2 |
| p2 N2 hang, N3 six devices, N4 acceptance criterion | FIXED. "Every ignored test here opens a device" is correct: eight ignored tests, all eight create one on a machine with an adapter |
| p2 N1 landed, N5 wrapping | not addressed, see N1 and N3 |

**Every claim the pass-2 remediation added, executed.**

| Claim | Checked |
|-------|---------|
| `classify` short-circuits a tier C override and hardcodes `adapters_seen: 0`, `fill_rate: None`, `device_created: false` | `caps.rs:737-759`, exactly those three literals in the early return |
| "the RESULT is identical with or without this block" | true, and stronger than stated: `TierSignals::unprobed()` supplies the same `bands` and `simd` the long path does, so the whole `Resolution` is identical |
| "deleting this leaves the whole suite green" | re-run: 76, 1, 1, 0 passed, 0 failed, and `gate gpu` ALL GREEN with 8 passed |
| "moves the lib target from 0.01s to 0.96s" | 0.01s against 0.99s this pass. See N4 |
| `COMPLETION_TIMEOUT_NANOS` is what a hang would be bounded by | `probe.rs:102-110` says exactly that, for the submission-completion loop |
| `CURRENT_SPRINT.md` says workers contend for the adapter and produce timeouts reading like rendering failures | verbatim at `CURRENT_SPRINT.md:183-189` |
| A7.2 names lavapipe as F-X002's acceptance criterion | `A7-tier-c.md:178-181`, and the word is now "criterion" in both |
| recovery needs an `Option` or a `destroy()` to release first, and `destroy()` converts a recoverable loss into a teardown | sound, and it matches `recover`'s documented `Refused` guarantee at `gpu.rs:247-249` |
| "the old device is lost in every non-test path" | `inject_loss` is `#[cfg(test)]` and `pub(crate)`, so yes |

**Regression battery, nine mutations, each reverted.**

| Mutation | Result |
|----------|--------|
| `recovers_from(Destroyed) = true` | RED, 2 floor tests plus `a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal` |
| `opens_a_device(Cpu) = true` | RED |
| `record_loss` without its `is_none` guard | RED, floor, no adapter |
| `supports_compute` negated | RED on the `gpu` gate |
| `recover` clears the flag and keeps the dead device | RED |
| `recover` accepts a live device | RED |
| loss callback not registered | RED, three device tests |
| transposing `edge`/`passes` | 2 compile errors |
| wildcard arm in `recovers_from` | green, which is correct and is now documented as the compiler's job |

**Nothing was broken.** `fmt`, `clippy`, `test`, `device`, `ci`, `guards`,
`guards-deep`, `prose`, `backlog`, `deviations`, `skills`, `unsafe`, `wasm` and
`gpu` all exit 0, read from the command itself. The `gpu` gate runs 8 tests
serialised. `cargo check -p ocelli-render --target wasm32-unknown-unknown --lib`
exits 0. The `wasm` gate still reports 16,455 against the recorded 16,388, which
`CURRENT_SPRINT.md` explains. No em-dash, no prose semicolon, no `as` cast, no
`unwrap`, `expect` or `panic!` anywhere in the pass-2 to pass-3 delta.
