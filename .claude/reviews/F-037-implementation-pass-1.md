# F-037 review, pass 1

**Reviewed**: the staged working tree against base `3b00890`, plus the separate
workflow commit `c062774`. Independent reviewer, not the author.
**Result**: 10 defects, 3 smells, 4 nitpicks

All mutations below were reverted. `git status` at the end of this pass shows
the same fifteen staged paths it showed at the start.

## Defects

### D1, the transposition's arithmetic is wrong, in four places, three of them new

**Where**:
- `crates/ocelli-render/src/probe.rs:45` (`Edge`'s doc, new in F-037)
- `crates/ocelli-render/src/probe.rs:564` (`measure_with`, reworded by F-037)
- `crates/ocelli-render/tests/ui/workload_dimensions_are_not_interchangeable.rs:7`
  (new)
- `docs/lld/tier-resolution.md:541` (new row)

**What**: every one of them states that transposing `edge` and `passes` leaves
the fragment count unchanged. `probe.rs:45` says "a transposed
256-by-256-once becomes 1-by-1-256-times with the same product".
`probe.rs:564` says "`fragments` agrees with itself because the product is
unchanged". The ui header and the LLD row say "with an identical product".

**Why it is wrong**: `fragments` is `edge * edge * passes`, which is not
symmetric in its two arguments. The calibration is `fragments(Edge(256),
Passes(1)) = 65,536`. Transposed it is `fragments(Edge(1), Passes(256)) = 256`.
The full pass is `fragments(Edge(1024), Passes(16)) = 16,777,216`, transposed
`fragments(Edge(16), Passes(1024)) = 262,144`. The numerator changes by 256x
and 64x respectively, not by nothing.

The real reason the transposition was invisible is the other half of the same
sentences, and it is correct: no test reaches `run`, because every test supplies
its own `issue` closure. The stated mechanism is wrong and it UNDERSTATES the
defect the newtypes close. A reader who believes "identical product" will
conclude the transposition was harmless arithmetic that only affected timing.
It was a 256x error in the fill rate's numerator, which is exactly the D-07
misdetection the surrounding paragraphs say they are about.

**Evidence**:
```
$ python3 -c "print(256*256*1, 1*1*256, 1024*1024*16, 16*16*1024)"
65536 256 16777216 262144
```
The claim is inherited from `3b00890:crates/ocelli-render/src/probe.rs:363`,
where it read "the fragment count is unchanged". F-037 propagated it into three
new files rather than correcting it, which is why it is in scope for this pass.

### D2, the recovery test does not prove a rebuild happened

**Where**: `crates/ocelli-render/src/gpu.rs:372-449`,
`an_unknown_loss_is_rebuilt_and_the_new_context_is_live`, and the claim in its
own doc comment at `gpu.rs:376-379`: "what is proved is everything after the
decision: the retained adapter opens a replacement, **the context becomes the
new device**, the reported capabilities come back". Restated in
`docs/lld/gpu-ownership.md:266` as "An injected `Unknown` is rebuilt".

**What**: nothing in the test asserts that `self.device` changed. The test
checks `state() == Live`, `caps` equality and that an encoder submits. A
`recover` that opens nothing, keeps the dead device and merely clears the loss
flag satisfies every assertion.

**Why it is wrong**: HLD section 22 is "rebuild the device". The one test that
claims to cover the rebuild arm passes when no rebuild occurs, which under the
microscope's section 2 is a defect and not a nitpick, because it will be counted
as coverage forever. `wgpu::Device` implements `PartialEq` in the pinned
version, measured below, so the assertion is available.

**Evidence**: mutation, `gpu.rs:244`:
```rust
// was: *self = rebuilt;
drop(rebuilt);
*self.lost.lock().unwrap_or_else(PoisonError::into_inner) = None;
```
```
$ cargo test -p ocelli-render -- --ignored
gpu::tests::an_unknown_loss_is_rebuilt_and_the_new_context_is_live ... ok
test result: ok. 2 passed; 0 failed
test result: ok. 6 passed; 0 failed   (tests/device.rs)
```
Green everywhere, floor and `gate gpu`.

### D3, `Recovered` cannot carry different limits, and two places say it is there because it can

**Where**: `crates/ocelli-render/src/gpu.rs:83`, "**It carries `caps` because a
rebuilt device can report different limits**, and a caller that assumed
otherwise would size a texture against a stale number", and
`docs/lld/gpu-ownership.md:255-258` with the same sentence.

**What**: `ResolvedAdapter::open` builds every `GpuContext` with
`self.resolution.caps` (`probe.rs:287`). `resolution` is private, immutable and
fixed at `resolve_adapter` time. `recover` therefore reads back exactly the
`Caps` the context already had, on every path, by construction. A rebuilt
device that really did report different limits would not be reflected, because
`open` never re-reads the adapter after detection.

**Why it is wrong**: the sentence claims the field exists to carry a value it
provably cannot carry, and it promises the caller protection from the stale
number it is in fact handed. The honest statement is the opposite one, and that
is worth writing down because F-038 to F-040 are told to extend this struct.

**Evidence**: mutation, `gpu.rs:241`, `let caps = *rebuilt.caps();` to
`let caps = self.caps;`:
```
$ cargo test -p ocelli-render --lib        -> 75 passed; 0 failed
$ cargo test -p ocelli-render -- --ignored -> 2 passed, 6 passed, 0 failed
```
Not merely untested. The two expressions are equal for every possible input.

### D4, "there is never a moment when two devices exist" is false, and `recover` is where

**Where**: `docs/lld/gpu-ownership.md:237`, unqualified, inside "The device
lifecycle" section whose own table lists `GpuContext::recover`. Secondary:
`crates/ocelli-render/src/probe.rs:153-154`, in `resolve_adapter`'s doc.

**What**: `GpuContext::recover` opens the replacement at `gpu.rs:240` and only
replaces the context at `gpu.rs:244`. Between those two lines the old device and
the new device are both alive, both from `request_device`, and both usable. The
code comment at `gpu.rs:238-239` states the ordering deliberately, so the code
and the LLD contradict each other in the same story's diff.

**Why it is wrong**: HLD section 31's invariant is the reason the sentence is
being asserted, so an unqualified restatement of it that the same story
falsifies is worse than no sentence. The overlap is defensible design, which is
the point: the fix is the prose, not the code.

**Evidence**: temporary instrumentation between `gpu.rs:240` and `:241`,
submitting on both devices and comparing the handles, driven by the existing
ignored recovery test:
```
TWO DEVICES ALIVE AT ONCE: old poll Ok(Poll), new poll Ok(Poll), same handle false
test result: ok. 1 passed
```
`same handle false` rules out the refcounted-clone reading. Reverted.

### D5, `exactly_one_loss_reason_recovers` cannot detect what two files say it detects

**Where**: `crates/ocelli-render/src/caps.rs:1057-1060` and
`docs/lld/tier-resolution.md:544`, both claiming the count test "is what goes
red / moves if a future wgpu adds a third reason AND somebody adds a wildcard
arm to absorb it".

**What**: the test counts over a hardcoded two-element array literal
(`caps.rs:1097-1100`). No third variant can enter that array, so the count
cannot move for that reason. A wildcard arm added today is caught by nothing.

**Why it is wrong**: the design plan was correct here and said the total match
is "asserted by the absence of a wildcard arm and checked by the human review
item rather than by a test". The doc comment and the LLD row upgrade that to a
mechanism that does not exist, which removes the human review item by claiming
it is automated.

**Evidence**: mutation, `caps.rs:154`, replacing the `Destroyed` arm with
`_ => false`:
```
$ cargo test -p ocelli-render --lib
test result: ok. 75 passed; 0 failed; 2 ignored
$ cargo clippy -p ocelli-render --all-targets   -> exit 0
```
Green. The only clippy output was an incidental `matches!` suggestion.

### D6, a doc comment landed on the wrong test, and one test lost its doc entirely

**Where**: `crates/ocelli-render/src/caps.rs:1055-1072`.

**What**: the block opens with "Exactly one of the two reasons recovers, stated
as a count. The pair above separates the two constant implementations. This
separates them by a different measure ... the count would move off one without
either assertion above changing." That paragraph describes
`exactly_one_loss_reason_recovers`. It is attached to
`tiers_a_and_b_open_a_device_and_tier_c_does_not` at `caps.rs:1074`, whose body
counts nothing. The block then restarts mid-paragraph with "**Tier C opens no
device, and the other two do.**" `exactly_one_loss_reason_recovers` at
`caps.rs:1095-1096` has no doc comment at all, and is the only test in that module
without one.

**Why it is wrong**: this is the microscope's own closing warning about
re-applied edits, landed. Two unrelated claims now read as one description of a
three-assertion tier predicate, and the claim D5 falsifies is the half that is
hardest to notice because it sits under the wrong heading.

**Evidence**: read at `caps.rs:1055-1105`. `cargo fmt --check` and clippy are
both green, because no formatter catches a misplaced doc comment.

### D7, `resolve_adapter`'s tier C guard is reached by nothing

**Where**: `crates/ocelli-render/src/probe.rs:163-165`.

**What**: deleting the `opens_a_device` short-circuit entirely leaves the whole
suite green, on the floor and on `gate gpu`. Its doc at `probe.rs:147-151`
claims the case that makes it worth having, "an adapter opened perfectly well
and D-07's combination rule demoted it anyway", and that is the one case nothing
drives.

**Why it is wrong**: microscope section 4. A guard present, authoritative and
never reached. It also diverges from the approved plan: the Tests table has a
row "A tier C `TierRequest` yields no `ResolvedAdapter`, driven through the
existing `classify` path with `TierSignals::unprobed`" in
`crates/ocelli-render/src/probe.rs`, and no such test exists. Worse, that test
as specified would NOT have killed the mutation either, because
`TierRequest::Requested(Tier::Cpu)` short-circuits in `detect` and returns
`None` for the adapter, so `adapter.map(..)` yields `None` with or without the
guard. Reaching the guard needs a `Resolution` whose tier is `Cpu` while an
adapter was chosen, which only `detect` produces today.

**Evidence**: mutation, removing `probe.rs:163-165`:
```
$ cargo test -p ocelli-render        -> 75, 1, 1, 0 passed; 0 failed; 6 ignored
$ cargo test -p ocelli-render -- --ignored -> 2 passed, 6 passed; 0 failed
```

### D8, the `gpu` gate's declared coverage names a LUT shader that does not exist

**Where**:
- `bin/ocelli.sh:87`, the `GATES` row, "the device lifecycle and the LUT shader
  on a real adapter (E6.1, E6.5)"
- `bin/ocelli.sh`, the `gates_cmd` comment, "the LUT shader agrees with
  `ocelli-pixel`"
- `.claude/WORKFLOW.md`, "runs every `#[ignore]`d test in `ocelli-render`: ...
  and the LUT-chain shader agrees with `ocelli-pixel` over the section 18.3
  fixture inputs"
- `scripts/ci_floor_check.py`, the `NOT_IN_FLOOR` comment, same clause
- `scripts/guards/census.py`, `DELEGATED["gpu"]`, "ocelli-render's device
  lifecycle and the LUT shader are asserted by their stories' tests"

**What**: all five are present tense. F-041 is `pending` in
`docs/sprints/BACKLOG.md:167`. The gate runs eight tests today and none of them
touches a shader other than `fill_rate.wgsl`.

**Why it is wrong**: `bin/ocelli.sh gate --list` is operator-facing output. An
operator reading it today is told a LUT-chain comparison against `ocelli-pixel`
is being verified, and nothing verifies it. This is the exact failure mode the
same commit message says it is fixing in two other sentences.

**Evidence**:
```
$ bin/ocelli.sh gate gpu
gpu          YES   the device lifecycle and the LUT shader on a real adapter
running 2 tests   (lib: measures_a_fill_rate_on_this_machine,
                   an_unknown_loss_is_rebuilt_and_the_new_context_is_live)
running 6 tests   (tests/device.rs)
ALL GREEN  1 gate(s)
$ grep -n '^| F-041' docs/sprints/BACKLOG.md
167:| F-041 | E6.5 | S11 | WGSL LUT-chain shader | Rust | 4w | F-029 | pending |
```

### D9, `ci/wasm-size-budget.json` reports two unwritten stories in the past tense

**Where**: `ci/wasm-size-budget.json`, `bearing_on_gate_A4`.

**What**: "S11 built the long-lived device, the LUT-chain shader and the cache,
and none of them moved this figure by a byte, because `crates/ocelli-wasm/
Cargo.toml` names `ocelli-core` and nothing else and no S11 story changed
that."

**Why it is wrong**: F-031 (the cache) and F-041 (the LUT shader) are both
`pending`, and F-037 itself is `in-progress`. The file states as measured fact
the outcome of two stories that have not been written. The claim about F-037 is
true and was verified below, which is what makes the other two a false claim
rather than a guess nobody can check.

**Evidence**:
```
$ grep -nE '^\| F-0(31|37|41)' docs/sprints/BACKLOG.md
157:| F-031 | E5.1 | S11 | ocelli-cache ... | pending |
163:| F-037 | E6.1 | S11 | ocelli-render ... | in-progress |
167:| F-041 | E6.5 | S11 | WGSL LUT-chain shader | ... | pending |
```
F-037's own half verified in a worktree at the base commit:
```
$ git worktree add /tmp/ocelli-f037-base 3b00890 --detach
$ (cd /tmp/ocelli-f037-base && bin/ocelli.sh gate wasm)
  wasm size 16,455 bytes, baseline 16,388, ceiling 17,207
$ bin/ocelli.sh gate wasm        # staged tree
  wasm size 16,455 bytes, baseline 16,388, ceiling 17,207
```
Byte-identical, so F-037 moved nothing. Worktree removed.

### D10, `CURRENT_SPRINT.md` now quotes text F-037 deleted, and the plan required it be corrected

**Where**: `docs/sprints/CURRENT_SPRINT.md:134-140`.

**What**: the heading is "## The size budget starts moving this sprint, and
that is expected" and the body reads "`ci/wasm-size-budget.json` records 16,388
bytes and its own note says the number that will bear on Appendix A gate A4
'arrives with the render path from S11, not here'". That string is exactly what
F-037 removed from `bearing_on_gate_A4`. The sprint file is now a quotation of a
sentence that no longer exists, supporting a heading the same story decided is
false.

**Why it is wrong**: the approved plan, Approach section 8, is explicit:
"`docs/sprints/CURRENT_SPRINT.md`'s size-budget section records that the
expectation was not triggered and why, under `## Carried forward from S11` if
the sprint carries anything, otherwise inline." It was not touched. The
divergence is not recorded anywhere.

**Evidence**:
```
$ grep -n "render path from" docs/sprints/CURRENT_SPRINT.md
137:number that will bear on Appendix A gate A4 "arrives with the render path from
$ git diff --cached --name-only | grep CURRENT_SPRINT
(no output)
$ grep -c "arrives with the render path from S11" ci/wasm-size-budget.json
0
```

## Smells

### S1, `docs/lld/guards.md` was named in the plan's LLD impact and was not touched

`docs/lld/guards.md:793` reads "D-04's exclusions remain a separate set
comparison: `corpus`, `guards-deep` and `oracle` do not become floor gates
through this equivalence rule." `NOT_IN_FLOOR` now has five members and `gpu`
is one of them. The sentence is not false as written, it is an enumeration that
silently stopped being the set it reads as. The plan's LLD impact list says
guards.md "gains the `gpu` gate beside `oracle` in whatever census it keeps of
what runs where", and its `F-IDs that contributed` header was not updated
either. The `guards` and `guards-deep` gates are both green, so nothing
mechanical will find this.

### S2, the `gpu` gate runs six device-opening tests concurrently

`cargo test -p ocelli-render -- --ignored` uses the default thread pool.
Observed: six tests in `tests/device.rs` each call `resolve_adapter` (one
transient probe device) and then `open` (one long-lived device), in parallel,
plus two in the lib target. That is up to eight `request_device` calls in
flight. It passed on this machine every time I ran it, roughly a dozen runs. It
is still a contradiction of the one-device framing the same story asserts in
prose, and it is the shape that produces a gate that fails once a month on a
loaded machine. `docs/sprints/CURRENT_SPRINT.md` states "The GPU is an
exclusive resource and the wave plan must serialise it" for exactly this reason
and the gate does not serialise.

### S3, `WORKFLOW.md` says "every `#[ignore]`d test" and then enumerates only the asserting ones

`.claude/WORKFLOW.md`'s new paragraph claims the gate runs every `#[ignore]`d
test and lists five behaviours. `probe::tests::measures_a_fill_rate_on_this_
machine` is in the set and asserts nothing. `bin/ocelli.sh`'s own comment
handles this honestly and at length. The `WORKFLOW.md` copy does not, and
`WORKFLOW.md` is the file the process says wins.

## Nitpicks

### N1, `tier-resolution.md` says F-037 "has landed"

`docs/lld/tier-resolution.md:460` and `:467` use "F-037 has landed" and "F-037
has since built". `docs/sprints/BACKLOG.md:163` says `in-progress`. Harmless
once the story completes, and wrong in the tree being reviewed.

### N2, "the size budget still reads 16,388 bytes"

`docs/lld/tier-resolution.md:463`. True of the recorded baseline, not of the
measurement. `bin/ocelli.sh gate wasm` reports 16,455 bytes against that
baseline on this toolchain, and it reports the same 16,455 at base commit
`3b00890`, so the 67-byte drift is not F-037's. Out of scope for this story and
worth somebody's attention, because the sentence reads as a measurement.

### N3, "a panic or a hang there fails this gate"

`bin/ocelli.sh`, the `gpu` arm comment about
`measures_a_fill_rate_on_this_machine`. A hang hangs the gate. It does not fail
it, and a hung `--sprint` is a different operator experience from a red one.

### N4, `record_loss`'s first-loss-wins guard cannot fire in production on the pinned backend

`wgpu-core-30.0.1/src/device/resource.rs:987` and `:5423` both `take()` the
closure before invoking it, so a device's loss callback fires at most once. The
`is_none` guard is therefore defence against a contract wgpu does not make
either way, and its doc at `gpu.rs:106-107` says exactly that. Correct and
worth keeping. Recording it so a later pass does not rediscover it as a hole.

## Verified clean

Everything below was executed, not read.

**Mutations that went red, as the author reported.** Each reverted immediately.

| Mutation | Result |
|----------|--------|
| `recovers_from(Destroyed) = true` | RED. `caps::tests::an_unknown_loss_recovers_and_a_destroy_does_not` and `exactly_one_loss_reason_recovers` on the floor, `a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal` on `gate gpu` |
| `opens_a_device(Cpu) = true` | RED. `tiers_a_and_b_open_a_device_and_tier_c_does_not`, floor |
| `record_loss` loses its `is_none` guard | RED. `the_first_loss_reported_is_the_one_kept`, floor, no adapter. The doc's claim that this runs in the CI floor is true |
| loss callback never registered in `GpuContext::new` | RED on `gate gpu`, three tests. Green on the floor, which is declared |
| `recover` keeps the old loss slot | RED. `an_unknown_loss_is_rebuilt_and_the_new_context_is_live` |
| `recover` no longer returns `NotLost` for a live device | RED. `recovering_a_live_device_is_refused_as_not_lost` |
| `supports_compute` negated | RED. `compute_availability_on_a_real_context_matches_the_decision`. The forwarder that nine sprint reviews could not reach is now reached, and the rewritten doc comment at `gpu.rs:288-308` is accurate |
| transposing `edge`/`passes` at `run`'s call site | `E0308`, `cargo check` exit 101. The newtypes close it |

**The recorded known hole is accurate.** Replacing `self.adapter.limits()` with
`wgpu::Limits::default()` at `probe.rs:278` left
`cargo test -p ocelli-render --test device -- --ignored` at exit 0 with 6
passed and 0 failed, reproducing the recorded figure exactly, and the whole
`gate gpu` ALL GREEN. It is recorded at the call site (`probe.rs:265-277`, the comment above `:278`) and
in `docs/lld/gpu-ownership.md:278-288`, in both cases with the reason and the
two stories that would close it. Correctly recorded and correctly placed.

**The two total matches have no wildcard arm.** `caps.rs:152-155` and
`caps.rs:178-183`, read directly. `DeviceLostReason` in
`wgpu-types-30.0.1/src/lib.rs:554-559` has exactly the two variants the plan
transcribes, and `Tier` has three.

**Only two `request_device` call sites exist in the workspace**, both in
`crates/ocelli-render/src/probe.rs`, at `:283` and `:442`.
`grep -rn "request_device" --include="*.rs" crates tools` returns no other
call, only doc-comment mentions. `bin/ocelli.sh gate device` exit 0, and
`ci/check-device-ownership.sh` is not in the diff at all, so it cannot have
been weakened.

**The probe device is dropped before the long-lived one is opened.** Traced
through `detect`, `measure_candidates` and `attempt_candidates`.
`attempt_candidates` opens at most one device per iteration and returns on the
first success, so the loop never holds two. `measure_candidates` takes the pair
by value out of `AttemptOutcome::Opened` and drops it at `probe.rs:478` before
returning. The claim holds for `resolve` and `resolve_adapter`. It does not hold
for `recover`, which is D4.

**`Adapter` outliving `Instance` is sound in the pinned wgpu.**
`wgpu-30.0.1/src/backend/wgpu_core.rs:44` is
`pub struct ContextWgpuCore(Arc<wgc::global::Global>)` and `:460` is
`CoreAdapter { context: ContextWgpuCore, id }`, so the adapter holds an owning
refcount on the global context. `detect`'s comment at `probe.rs:214-215` is
correct.

**Thread safety and deadlock.** `Arc<Mutex<Option<DeviceLoss>>>` satisfies the
`Send + 'static` bound on `set_device_lost_callback`
(`wgpu-30.0.1/src/api/device.rs:681`, read from the vendored source). Poisoning
is handled with `unwrap_or_else(PoisonError::into_inner)` at both lock sites.
`state()` takes the guard and does nothing re-entrant while holding it, and
`recover` calls `state()` as a complete statement at `gpu.rs:231` so the guard
is released before `adapter.open().await` at `:240`. No lock is held across an
await. No public API hands out the lock.

**A stale callback cannot affect the new context.** The old device's closure
holds an `Arc` to the OLD `LossSlot`. `*self = rebuilt` installs the new
context's own slot, and the old `Arc` survives only inside the dead closure,
writing where nothing reads. Verified by construction and by the
`recover`-keeps-the-old-slot mutation going red.

**wasm32 is not broken.** `cargo check -p ocelli-render --target
wasm32-unknown-unknown --lib` exit 0. `--all-targets` fails on
`getrandom`/`wait-timeout` reached through `proptest` and `trybuild`, both
dev-only, and `proptest` was already a dev-dependency at the base commit, so
that is pre-existing and not F-037's.

**Error-handling rules.** No `as` cast, no `.unwrap()`, no `.expect(`, no
`panic!` anywhere in the diff, including the tests.
`git diff 3b00890 -- crates/ | grep "^+"` filtered for each. `bin/ocelli.sh
clippy ocelli-render` exit 0 with zero warnings, and the workspace denies
`unwrap_used`, `expect_used` and `panic`. `u64::try_from(..).unwrap_or(u64::MAX)`
in the two clocks is `unwrap_or`, not `unwrap`, and is the documented shape.

**Voice rules.** No em-dash and no prose semicolon in any added line, checked
over `docs/`, `ci/`, `crates/` doc comments and the `.claude/WORKFLOW.md` half
of `c062774`. `bin/ocelli.sh gate prose` exit 0.

**Gates.** `fmt`, `clippy`, `device`, `ci`, `guards`, `guards-deep`, `prose`,
`backlog`, `deviations`, `skills`, `unsafe`, `wasm` and `gpu` all exit 0, each
read from the command itself. `guards-deep` reports 396 refusal probes driving
35 guards red and 0 known defects open. `bin/ocelli.sh gate --list` shows `gpu`
with `YES` in the GPU column beside `oracle`, and the `--floor` exclusion in
`gates_cmd` and `scripts/ci_floor_check.py`'s `NOT_IN_FLOOR` agree, which the
`ci` gate asserts for set equality.

**The trybuild case is real.** `tests/ui/workload_dimensions_are_not_
interchangeable.rs` takes its values as parameters rather than from a diverging
initialiser, which is the rule `no_owned_device_out_of_context.rs` set, and the
`.stderr` names both directions of the transposition. It runs in the floor.

**`bearing_on_gate_A4` is corrected in the direction the plan said**, and no
re-baseline was taken. `only_rebaseline_in_S03` is untouched, and the `wasm`
gate's baseline field still reads 16,388.
