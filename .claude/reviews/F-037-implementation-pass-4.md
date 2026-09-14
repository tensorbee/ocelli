# F-037 review, pass 4

**Reviewed**: the staged working tree, `git write-tree` =
`b810ae52aed6e65eff1e8fe78fa50d7c4b970692`, against base `3b00890`, with the
pass-3 remediation delta read as `git diff 5bdfabb5 b810ae52`. Independent
reviewer, not the author.
**Result**: 1 defect, 0 smells, 4 nitpicks

**This pass is NOT clean.** One defect blocks completion. It is a quantifier in
prose, in six places, that the remediation itself falsified by adding a ninth
`#[ignore]`d test that neither needs an adapter nor opens a device.

All mutations were reverted. The tree hash at the end of this pass is the same
as at the start.

**Pass line**: F-037, pass 4, 1 defect, 0 smells, 4 nitpicks. Trend across four
passes: 10/3/4, 4/1/5, 2/1/5, 1/0/4. No finding has survived a pass, and every
one of this pass's predecessors is confirmed fixed below.

## Defects

### D1, the gate's ignored-test set is described by a property that the new test breaks, in six places

**Where**:

| File | Text |
|------|------|
| `bin/ocelli.sh:87` | the `GATES` row, "ocelli-render's tests that need a real adapter (E6.1, D-04)" |
| `bin/ocelli.sh:295-296` | "Every test in `ocelli-render` that needs a real adapter, **which is** every test marked `#[ignore]` there" |
| `bin/ocelli.sh:331` | "**Every ignored test here opens a device**, and the default harness runs them on one thread per core" |
| `.claude/WORKFLOW.md:118-119` | "runs every `#[ignore]`d test in `ocelli-render`, **which are the ones needing a real adapter**" |
| `scripts/ci_floor_check.py:463-464` | "It runs every `#[ignore]`d test in `ocelli-render`, **which are the ones needing a real adapter**" |
| `scripts/guards/census.py:187` | "cargo test again, **filtered to the `#[ignore]`d tests that need a real adapter**" |

**What**: this remediation added
`probe::tests::a_cpu_override_yields_no_adapter_and_spends_no_startup_cost`. It
is `#[ignore]`d, and it is the one ignored test in the crate that does **not**
need a real adapter and does **not** open a device. Its whole subject is the
tier C short circuit, which returns before `new_instance_with_webgpu_detection`
is ever called, so no instance, no adapter and no device are created. Its own
`#[ignore]` string says so, and correctly: "the startup-cost assertion only
bites where adapters exist", not "needs a real GPU adapter" like the other
eight.

So `bin/ocelli.sh:331` is flatly false, and the other five state a set equality
or a gloss that no longer holds. The `:331` one is the worst of the six because
it is the stated justification for `--test-threads=1`, so a later reader
deciding whether that flag is still needed is reasoning from a false premise.

**Why it is wrong**: microscope section 3. It is also the exact pattern
`CLAUDE.md` warns about at length, a quantifier written in prose beside a set
that grows, and it has now recurred inside this review loop: pass 2's N3 flagged
"Six of these tests open a device", the pass-3 remediation strengthened it to
"Every ignored test here opens a device", and the pass-4 remediation added the
test that falsifies it. The durable fix is the one `bin/ocelli.sh:296-299`
already uses for test files, "this names no test file", applied to the property
as well: the gate runs what `#[ignore]` selects, and what that is today is a
list, not a property.

**Evidence**:
```
$ cargo test -p ocelli-render -- --ignored --list
gpu::tests::an_unknown_loss_is_rebuilt_and_the_new_context_is_live
probe::tests::a_cpu_override_yields_no_adapter_and_spends_no_startup_cost
probe::tests::measures_a_fill_rate_on_this_machine
a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal
a_device_opens_and_agrees_with_the_resolved_tier
compute_availability_on_a_real_context_matches_the_decision
destroying_the_device_is_observed_as_a_loss
recovering_a_live_device_is_refused_as_not_lost
the_retained_adapter_opens_a_second_live_device_after_the_first_is_gone
```
Nine, of which eight open a device. That the ninth opens none is not inference:
I simulated a machine with no adapter at all, by returning `Vec::new()` from
`enumerate` alongside deleting the short circuit, and the test still passed,
which it could not do if it required an adapter.
```
MUT Y: short circuit deleted AND enumerate returns nothing
test probe::tests::a_cpu_override_yields_no_adapter_and_spends_no_startup_cost ... ok
```

## Smells

None.

## Nitpicks

### N1, "F-037 has landed" while `BACKLOG.md` says `in-progress`

`docs/lld/tier-resolution.md:460` and `:467`. Raised in passes 1, 2 and 3.
Becomes true at `/complete-feature`. Not blocking, and I will stop raising it.

### N2, the CI floor no longer drives `resolve_adapter` at all

Moving the tier C test to `#[ignore]` removed the only floor-profile caller of
`resolve_adapter`. No detection was lost, because the floor version killed no
mutation, and the doc names
`caps::tests::tiers_a_and_b_open_a_device_and_tier_c_does_not` as the floor's
coverage of the decision, which is accurate. Recording the trade so a later pass
does not rediscover it as a gap. A second, non-ignored assertion of just
`resolved.is_none()` would restore it for nothing, if anyone wants it.

### N3, "What only this test adds is that `detect` honours it before spending anything"

`probe.rs:1201-1202`. The antecedent of "it" is `opens_a_device`, from the
previous sentence, but `detect`'s short circuit branches on
`request.requested() == Some(Tier::Cpu)` and never calls `opens_a_device`. The
two happen to agree. Loose pronoun, not a false claim.

### N4, `gpu-ownership.md:289` enumerates the device-lifecycle cases, not the gate's

"`crates/ocelli-render/tests/device.rs` and one `#[ignore]`d case in `gpu.rs`,
all run by `bin/ocelli.sh gate gpu`." Every word is true, and the section is
headed "What the tests prove" under "The device lifecycle", so it is not
claiming to enumerate the gate. It now reads one way at a glance and another on
inspection, because the gate runs two `probe.rs` cases as well.

## Answers to the five questions

**1. Is the new test's doc accurate in every sentence, especially the
no-adapter-machine claim?** Yes, every sentence, and I executed the hard one.

| Sentence | Checked |
|----------|---------|
| "`resolve_adapter` already takes `clock: &mut dyn FnMut() -> u64` from the caller" | `probe.rs:189-192`, yes |
| "The short circuit returns before any instance, adapter or device is created, so nothing times anything and the count is deterministically zero" | the `return` precedes `new_instance_with_webgpu_detection`. Baseline run: count 0, test passes |
| "Delete the short circuit and `measure` runs, `run` calls `clock` around its submission, and the count is non-zero" | MUT X, red: `assertion left == right failed: the tier C override spent startup cost anyway, so the short circuit is gone` |
| "This is not a timing bound. The closure returns a constant `0` and the assertion is on how many times it was CALLED" | the body is `\|\| { ticks += 1; 0_u64 }`. No duration is read anywhere |
| **"a machine that enumerates nothing reaches `ProbeOutcome::NoAdapter` without ever calling `clock`, so the count is zero there either way and the mutation survives"** | **MUT Y, executed.** `enumerate` stubbed to `Vec::new()` with the short circuit also deleted: test passes. The code path confirms it, `measure` is called only in `AttemptOutcome::Opened` |
| "The floor's coverage of the same DECISION is `caps::tests::tiers_a_and_b_open_a_device_and_tier_c_does_not`, which drives `opens_a_device` over all three tiers with no adapter at all" | true, three assertions, no adapter |

One refinement rather than a correction: the mutation also survives on a machine
that enumerates an adapter but opens no device, because `measure` is reached
only from the `Opened` arm. The doc's stated case is true, it is just not the
only one.

**2. Is the rewritten `detect` comment now true?** Yes, all of it.
"NO ASSERTION ON THE RETURN VALUE CAN CATCH DELETING THESE THREE LINES" holds
for both public entry points: `resolve` returns a `Resolution` that is identical
either way, because `classify` step 1 (`caps.rs:737-759`) hardcodes
`adapters_seen: 0`, `fill_rate: None` and `device_created: false` and reads only
`signals.bands` and `signals.simd`, which `TierSignals::unprobed()`
(`caps.rs:565-571`) sets to the same `FillRateBands::RECORDED` and
`SimdSupport::build_target()` the long path builds. `resolve_adapter` returns
`None` either way, because `opens_a_device` masks the `Some(adapter)` that
`detect` would otherwise carry. "Two earlier attempts at that test asserted the
return value instead and were green under this mutation" matches passes 2 and 3
exactly. The "0.96s" wall clock is gone.

**3. Did D1 and S1 introduce anything false?** No.
`tier-resolution.md:363-376` is correctly scoped with "**on the resolution
path**" and the recovery paragraph is accurate, including "found it as the fifth
copy", which is what pass 3 reported. `gpu-ownership.md:205-208` now reads "no
second device exists on that path. The recovery path is the exception and is
scoped below, under 'The device lifecycle'", and that heading does exist below,
at `:216`. The `ci_floor_check.py` rewrap is clean.

**4. Did any previously-red mutation regress?** No. Eight of eight still red,
each reverted.

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

Plus the new one: deleting `detect`'s tier C short circuit is now RED, where it
was green through three passes. The two declared residues are still green and
still correctly declared: deleting `resolve_adapter`'s `opens_a_device` guard
leaves the whole suite green, and a wildcard arm in `recovers_from` leaves it
green, both of which the code says in the places that matter.

**5. Is this pass clean?** **No.** One defect, D1. Zero smells. It is a
six-site wording change with no code consequence, and I expect pass 5 to be
clean if nothing else moves with it.

## Verified clean

**Your grep reading was right in its conclusion and wrong in its count.** You
said two hits remain, both in `probe.rs`. Sweeping ten phrasings rather than
one, the unscoped claim is gone everywhere in code and LLD, but there are more
scoped survivors than two:

```
probe.rs:9     "so on this path there is never a moment ..."          scoped
probe.rs:192   "and on this path there is still never a moment ..."   scoped
probe.rs:541   quotes the header verbatim, inside measure_candidates  correct
gpu-ownership.md:206  "no second device exists on that path"          scoped
gpu-ownership.md:238  "On the resolution path there is still never"   scoped
tier-resolution.md:364 "so on the resolution path there is never"     scoped
```
Phrasings swept: `never a moment`, `two devices exist`, `one-device invariant`,
`one device invariant`, `only one device`, `single device`, `no second device`,
`second .*request_device`, `exactly one device`, `never two`. **No sixth
unscoped site exists.** The `second request_device` family at `gpu.rs:158`,
`gpu.rs:398`, `gpu-ownership.md:57` and `gpu-ownership.md:227` is a different
and true claim. The two design plans still carry the unqualified sentence and
must not be edited, which your N2 answer already records.

**Everything else executed.** `fmt`, `clippy`, `test`, `device`, `ci`, `guards`,
`guards-deep`, `prose`, `backlog`, `deviations`, `skills`, `unsafe`, `wasm` and
`gpu` all exit 0, read from the command itself. `gate gpu` runs 9 tests
serialised, all green. `cargo check -p ocelli-render --target
wasm32-unknown-unknown --lib` exits 0. The `wasm` gate still reports 16,455
against the recorded 16,388, which `CURRENT_SPRINT.md` explains as pre-existing
at base `3b00890`. No em-dash, no prose semicolon, no `as` cast, no `unwrap`,
`expect` or `panic!` in the pass-3 to pass-4 delta. The new test's `ticks` borrow
is correctly released by the inner block before the `assert_eq!` reads it, which
is why it compiles at all.
