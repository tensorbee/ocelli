# F-037 review, pass 2

**Reviewed**: the staged working tree, `git write-tree` =
`62ebde959235b945c82922b43ab1171dadbfc3ff`, against base `3b00890`. The
remediation of pass 1 treated as new unreviewed work. Independent reviewer, not
the author.
**Result**: 4 defects, 1 smell, 5 nitpicks

All mutations were reverted. The staged set and the tree hash at the end of this
pass are the same as at the start.

**Pass line**: F-037, pass 2, 4 defects, 1 smell, 5 nitpicks. Two of the four
defects are pass-1 findings reported as fixed that were not applied to the file
they were reported against.

## Defects

### D1, pass-1 D3 was fixed in `gpu.rs` and not in the LLD, and the two now contradict each other

**Where**: `docs/lld/gpu-ownership.md:254-257`.

**What**: the file still reads "`Caps` is in it because a rebuilt device can
report different limits, and a caller that assumed otherwise would size a
texture against a stale number." The remediation note says this claim was
"deleted from both `gpu.rs` and `gpu-ownership.md`". It was deleted from
`gpu.rs` only.

**Why it is wrong**: `crates/ocelli-render/src/gpu.rs:89-99` now says the
opposite, in the same staged tree: "**`caps` cannot currently differ from the
pre-loss `caps`, and an earlier version of this comment claimed it could.**" One
of the two is false and a reader has no way to tell which without reading
`ResolvedAdapter::open`. `open` copies `self.resolution.caps`, which is
immutable on the retained adapter, so `gpu.rs` is the correct one and the LLD is
the false one. The LLD is also the file `docs/lld/` is meant to be read from.

**Evidence**:
```
$ git ls-files -s docs/lld/gpu-ownership.md
100644 dedf4f92d509da4262bf610cb87bbef56a0ef8ff 0  docs/lld/gpu-ownership.md
```
`dedf4f9` is the same blob the pass-1 diff carried (`index ea1aed3..dedf4f9`),
so the file is byte-identical to the tree pass 1 reviewed and was not touched by
the remediation at all.
```
$ grep -n "rebuilt device can report different limits" docs/lld/gpu-ownership.md
256:rebuilt device can report different limits, and a caller that assumed otherwise
```
The mutation is still green, as in pass 1: `gpu.rs:276`,
`let caps = *rebuilt.caps();` to `let caps = self.caps;`, leaves
`cargo test -p ocelli-render --lib` at 75 passed and the `gpu` gate at 8 passed.
That is expected and is not itself the defect, because the two expressions are
provably equal. The defect is the sentence saying they are not.

### D2, pass-1 D4 survives in two places, and one of them now contradicts `recover`'s own comment

**Where**:
- `docs/lld/gpu-ownership.md:237`, unqualified, opening a paragraph in "The
  device lifecycle" whose own table lists `GpuContext::recover` as a step.
- `crates/ocelli-render/src/probe.rs:184-185`, in `resolve_adapter`'s doc.

**What**: both still read "there is still never a moment when two devices
exist", in bold, as a standalone claim. The remediation note says
"`gpu-ownership.md`'s unqualified claim is scoped to the probe". It is not.
`probe.rs:184` was not mentioned and was not changed either.

**Why it is wrong**: `crates/ocelli-render/src/gpu.rs:261` now says, in the same
tree, "**THIS IS THE ONE PLACE TWO DEVICES BRIEFLY COEXIST**". That comment is
correct and is the right fix. Two doc comments and one LLD section in one crate
now assert both halves of a contradiction. The `probe.rs` site is the worse of
the two, because a reader of `resolve_adapter` has no reason to open `gpu.rs`.

The module header at `probe.rs:7-12` is fine, because it is introduced by "**The
probe device is transient.**" and `gpu.rs:262` explicitly points at it as "the
module header's claim about the probe is about the probe". `resolve_adapter`'s
copy has no such anchor.

**Evidence**:
```
$ grep -n "two devices exist" crates/ocelli-render/src/probe.rs docs/lld/gpu-ownership.md
crates/ocelli-render/src/probe.rs:10:  (module header, scoped, correct)
crates/ocelli-render/src/probe.rs:185: two devices exist.** This function retains an `Adapter`, which is not a
docs/lld/gpu-ownership.md:207:  (F-004's bullet, scoped, correct)
docs/lld/gpu-ownership.md:237:**There is still never a moment when two devices exist.** `resolve_adapter`
```
The underlying fact was proved in pass 1 by instrumenting `recover` between
`gpu.rs:275` and `:276`: both devices polled `Ok(Poll)` and `self.device ==
rebuilt.device` was `false`. Not re-run, because `recover`'s new comment already
concedes it.

**On the coordinator's question 3, whether `recover` should drop the old device
first.** No. The code is right and only the prose was wrong. There is no way to
release `self.device` in place without making it an `Option` or calling
`destroy()`, and `destroy()` would convert a recoverable `Unknown` into a
deliberate teardown and leave a caller with nothing if `open()` then failed,
which is exactly the guarantee the comment at `gpu.rs:258-259` is protecting.
Section 31's stated concern is texture sharing between two devices, this
workspace has no texture type, and the old device is lost by the check above in
every non-test path. Keep the order, fix the two sentences.

### D3, `WORKFLOW.md` points at a rule that is not in `WORKFLOW.md`

**Where**: `.claude/WORKFLOW.md:132-134`, added by the remediation.

**What**: "The gate runs `--test-threads=1`, because the adapter is an exclusive
resource. **The wave-planning rule below** says two workers must not run device
tests concurrently on one machine."

**Why it is wrong**: there is no such rule below. `WORKFLOW.md`'s wave-planning
section is lines 275 to 292 and says nothing about devices, adapters or GPU
contention. The rule being cited is in `docs/sprints/CURRENT_SPRINT.md:183-189`,
"**The GPU is an exclusive resource and the wave plan must serialise it.**",
which is a per-sprint file that will be rewritten when S12 opens. `WORKFLOW.md`
is the file the process says wins, and it now forwards a reader to a rule it
does not contain.

**Evidence**:
```
$ grep -n -i "device test\|exclusive resource\|contend" .claude/WORKFLOW.md
133:The wave-planning rule below says two workers must not run device tests
$ sed -n 289,292p .claude/WORKFLOW.md
**A wave of one story runs serial**, in the canonical worktree, with no claim,
worker branch, worktree, handoff or integration step. ...
```
`bin/ocelli.sh`'s parallel comment gets this right and names
`docs/sprints/CURRENT_SPRINT.md` explicitly, quoting it near-verbatim, so the
two copies of the same justification disagree about where the rule lives.

### D4, the new test's headline claims a property the test does not probe

**Where**: `crates/ocelli-render/src/probe.rs:1135-1136`, the doc on
`a_cpu_override_yields_no_adapter`.

**What**: "**A tier C override yields no `ResolvedAdapter`, and creates no
instance, no adapter and no device on the way to saying so.**" followed by
"Deviation D-07 names the estate that overrides to tier C getting its startup
cost back, and a `resolve_adapter` that enumerated adapters before reading the
override would spend it anyway."

The test body asserts one thing, `resolved.is_none()`. Nothing observes whether
an instance was created, whether adapters were enumerated or whether a probe
device was opened.

**Why it is wrong**: microscope section 3. It is a claim about what a test
proves, stated in the same doc block that is otherwise careful about what the
test does not prove, and it is false. The second half of the paragraph names
exactly the regression the test would have to catch and does not catch.

**Evidence**: mutation, deleting `detect`'s tier C short circuit at
`probe.rs:216-218`, so a `Cpu` override creates an instance, enumerates every
adapter and opens a probe device before answering:
```
$ cargo test -p ocelli-render --lib
test probe::tests::a_cpu_override_yields_no_adapter ... ok
test result: ok. 76 passed; 0 failed; 2 ignored
```
Green. The suite's wall clock for the lib target moved from 0.01s to 0.96s,
which is the startup cost the paragraph invokes, and nothing asserts it.

## Smells

### S1, the test added to answer pass-1 D7 is killed by no mutation of the thing it names

`a_cpu_override_yields_no_adapter` is green under all three mutations of the
behaviour its name and doc invoke:

| Mutation | Result for this test |
|----------|----------------------|
| delete `resolve_adapter`'s `opens_a_device` guard | green |
| `opens_a_device(Cpu) = true` | green |
| delete `detect`'s tier C short circuit | green |

What it actually asserts is that `None.map(..)` is `None`, because the `Cpu`
override makes `detect` return no adapter and every path after that returns
`None` regardless of the predicate, the guard or the short circuit. The doc is
honest about one of those three, the guard, and that honesty is the reason this
is a smell and not a second defect. It will still be read as coverage of the
tier C path forever, and the pass-1 finding it was added for is unchanged:
`resolve_adapter`'s guard is reached by nothing.

**The residue statement itself is accurate**, which is the coordinator's
question 2, and I verified both halves:
```
$ # guard deleted from probe.rs:194-196
$ cargo test -p ocelli-render          -> 76, 1, 1, 0 passed; 0 failed
$ cargo test -p ocelli-render -- --ignored --test-threads=1 -> 2, 6 passed; 0 failed
test probe::tests::a_cpu_override_yields_no_adapter ... ok
```
Deleting the `if` leaves the whole suite green, and the new test does not kill
it, exactly as `probe.rs:168-182` says. The pointer to F-X002 is also correct:
`docs/sprints/BACKLOG.md:171` is "Three-layer GPU-less render testing: lavapipe
in CI and Chrome with SwiftShader nightly", S14, depends on F-037, and
`docs/spikes/A7-tier-c.md:154-181` is section A7.2 with the three-layer table
and the lavapipe acceptance criterion.

## Nitpicks

### N1, "F-037 has landed" while the backlog says `in-progress`

`docs/lld/tier-resolution.md:460` and `:467`. Carried from pass 1, still not
addressed, still not blocking. It becomes true at `/complete-feature`.

### N2, "a panic or a hang there fails this gate"

`bin/ocelli.sh`, the `gpu` arm. A hang hangs the gate. Carried from pass 1,
still not addressed, still not blocking. `--test-threads=1` does not change it.

### N3, "Six of these tests open a device"

`bin/ocelli.sh`'s `--test-threads=1` comment and `.claude/WORKFLOW.md:132-135`.
There are eight `#[ignore]`d tests and all eight create a device: the six in
`tests/device.rs`, plus `gpu::tests::an_unknown_loss_is_rebuilt_and_the_new_
context_is_live`, plus `probe::tests::measures_a_fill_rate_on_this_machine`
through `resolve`'s probe. Six is right for the largest set that was concurrent
inside one test binary, which is what the fix is about, so the number is
defensible and the wording is not.

### N4, "acceptance question" against "acceptance criterion"

`probe.rs:180-181` says `A7-tier-c.md` A7.2 "already names that as its
acceptance question". A7.2 line 178 says "it is F-X002's acceptance criterion
rather than a gate question", which is the opposite pairing of the same two
words. Same substance.

### N5, a short-wrapped line in the new `ci_floor_check.py` comment

"needing a real adapter. Deviation D-04" then a new comment line beginning "is
that CI has no GPU". Cosmetic, and `fmt` does not reach Python comments.

## Verified clean

Everything below was executed on the remediated tree, not read.

**The eight pass-1 defects that were remediated in the file they were reported
against.**

| Pass-1 finding | State |
|----------------|-------|
| D1, "identical product" | FIXED at all four sites. The replacement arithmetic is correct: `python3 -c "print(256*256*1, 1*1*256, 1024*1024*16, 16*16*1024)"` gives `65536 256 16777216 262144`, and 65,536/256 is 256 and 16,777,216/262,144 is 64, which is what `probe.rs:48-50` now says. The new explanation, that `fragments` and `run` receive the same transposed pair so nothing internal disagrees, is correct by inspection of `run`'s body |
| D2, the rebuild test proved no rebuild | FIXED. `assert_ne!(*context.device(), device_before)` at `gpu.rs:480`. Mutation re-run: `drop(rebuilt); *self.lost.lock() = None;` now gives `assertion left != right failed: recover cleared the loss flag and kept the dead device`, 1 failed. The comparison is sound because `wgpu::Device` is `PartialEq` by `DeviceId` (`wgpu-30.0.1/src/backend/wgpu_core.rs:752`) and the test's clone keeps the old id alive across `adapter.open()`, so the ids cannot collide |
| D3, `Recovered` caps claim | fixed in `gpu.rs`, NOT in the LLD. See D1 above |
| D4, "never two devices" | fixed in `recover`, NOT in the LLD or in `resolve_adapter`. See D2 above |
| D5, the count test's wildcard claim | FIXED in `caps.rs:1094-1101` and `docs/lld/tier-resolution.md:544`. Both now say the array names two variants so a third is untested, and that what catches a third is the compiler. That is correct |
| D6, the spliced doc comment | FIXED. `tiers_a_and_b_open_a_device_and_tier_c_does_not` and `exactly_one_loss_reason_recovers` each carry their own doc, and each doc describes its own test |
| D7, the unreached guard | honestly recorded rather than closed, which is the right answer given no machine in this repository can reach it. The residue statement is accurate, verified above. The new test is S1 |
| D8, the LUT-shader coverage claim | FIXED at all five sites. `bin/ocelli.sh gate --list` now prints `gpu YES ocelli-render's tests that need a real adapter (E6.1, D-04)`, and the arm comment says what it runs today and that F-041 joins it with no gate edit |
| D9, past tense about unwritten stories | FIXED. `bearing_on_gate_A4` now names only the S11 decision not to add the edge, and states no outcome for F-031 or F-041 |
| D10, the stale `CURRENT_SPRINT.md` quotation | FIXED. The heading is now "The size budget does NOT move this sprint, and this section said it would", the quotation is carried in its corrected form, and the reason is recorded |
| S1, `guards.md` | FIXED. F-ID line, date, the three-gate D-04 list replaced with a no-list rule naming the mechanism, and a new paragraph recording the census refusals. `gates_declared` moved 30 to 31, which matches `ci/guard-probe-budget.json` |
| S2, concurrent device tests | FIXED. `--test-threads=1`, and the `gpu` gate runs 8 tests serialised, green |
| S3, `WORKFLOW.md`'s enumeration | FIXED, and the fill-rate instrument now has its own paragraph saying it asserts nothing and what it is still worth |
| N2, 16,388 as a measurement | FIXED. Both sentences now say RECORDED, and `CURRENT_SPRINT.md` records the drift |
| N4, the first-loss guard's real reason | FIXED, and the wgpu citation is exact. `wgpu-core-30.0.1/src/device/resource.rs:252` is `pub(crate) device_lost_closure: Mutex<Option<DeviceLostClosure>>` and `:987` is `if let Some(device_lost_closure) = self.device_lost_closure.lock().take() {`. The browser half is also right: `wgpu-30.0.1/src/backend/webgpu.rs:2622` wraps the callback in `Closure::once` on `self.inner.lost().then(..)`, and a `lost` promise settles once |

**Every numeric claim the remediation added.**

| Claim | Checked |
|-------|---------|
| calibration 65,536 to 256, full pass 16,777,216 to 262,144, factors 256 and 64 | computed, correct |
| `Recovered` reports "the same four values" | `Caps` has exactly four fields: `compute`, `max_tex_3d`, `max_buffer`, `tier` |
| 16,455 measured against 16,388 recorded, drift 67 | `bin/ocelli.sh gate wasm` prints "wasm size 16,455 bytes, baseline 16,388, ceiling 17,207". 16,455 minus 16,388 is 67 |
| "inside the file's 5 per cent tolerance" | `ci/wasm-size-budget.json` `tolerance` is `0.05`, and 16,388 times 1.05 is 17,207.4, which is the printed ceiling |
| the drift "reproduces at this sprint's base commit `3b00890`" | measured in pass 1 in a worktree at that commit: 16,455 bytes, byte-identical |
| `gates_declared` 30 to 31, two values moved | `ci/guard-probe-budget.json` in `c062774` changes exactly the `NOT_IN_FLOOR` digest and that count |
| `NOT_IN_FLOOR` "named three gates while the set held four", `gpu` a fifth | the old sentence named `corpus`, `guards-deep`, `oracle`, the old set had four, the new set has five |
| F-X002 is lavapipe plus SwiftShader, S14, and A7.2 is the section | `BACKLOG.md:171` and `A7-tier-c.md:154-181` both match |

**Regression battery.** Every pass-1 mutation that went red still goes red on
the remediated tree, each reverted:

| Mutation | Result |
|----------|--------|
| `recovers_from(Destroyed) = true` | RED, 2 floor tests and `a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal` |
| `opens_a_device(Cpu) = true` | RED, `tiers_a_and_b_open_a_device_and_tier_c_does_not` |
| `record_loss` without its `is_none` guard | RED, `the_first_loss_reported_is_the_one_kept`, floor, no adapter |
| `supports_compute` negated | RED, `compute_availability_on_a_real_context_matches_the_decision` |
| transposing `edge`/`passes` at `run`'s call site | 2 compile errors, `cargo check` non-zero |
| `recover` clears the flag and keeps the dead device | RED now, was green in pass 1 |

**Nothing was broken.** `fmt`, `clippy`, `test`, `device`, `ci`, `guards`,
`guards-deep`, `prose`, `backlog`, `deviations`, `skills`, `unsafe`, `wasm` and
`gpu` all exit 0, each read from the command itself. The `ci` gate still agrees
the runner's exclusion list and `NOT_IN_FLOOR` are the same set after both were
re-commented. The trybuild `.stderr` was updated for the new line numbers, 34
and 35, and `compile_fail` passes. `cargo check -p ocelli-render --target
wasm32-unknown-unknown --lib` exits 0. No `as` cast, no `.unwrap()`, no
`.expect(`, no `panic!` in the whole staged diff including the new test. No
em-dash and no prose semicolon in any added line across `docs/`, `ci/`, `bin/`,
`scripts/`, `.claude/WORKFLOW.md` and the Rust doc comments.

**Files the remediation did not need to touch and did not.**
`crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/device.rs`,
`crates/ocelli-render/tests/compile_fail.rs`,
`docs/lld/feature-availability.md`, `crates/ocelli-render/Cargo.toml` and
`Cargo.lock` are unchanged from the tree pass 1 reviewed, which is correct.
`docs/lld/gpu-ownership.md` is also unchanged, and that one is not correct. It
is D1 and D2.
