# S11 whole-sprint review, pass 5, CHANGELOG scope only

**Reviewed**: staged tree `f6aaac09e1aad6309ab39952a481ccdfd4f8a3f9`.
**Delta**: `git diff e6cad653ae61 f6aaac09e1aa` is `CHANGELOG.md`, 22 insertions,
0 deletions, and no other tracked file. Confirmed.
**Scope**: the three new `## Unreleased` bullets and the five questions asked.
The sprint itself was not re-reviewed. Every file under `crates/`, `scripts/`,
`bin/` and `docs/` is byte-identical to `325168bb0fee`, the tree pass 4 passed
clean.
**Result**: **2 defects, 0 smells, 1 nitpick**

**This pass is NOT clean.** One defect is in the new text and blocks. The other
is pre-existing, is two lines above the new bullets, and is described separately
so you can decide it on its own.

---

## Defects

### D1, bullet 3 claims a window change re-uploads no texture, and the repository says that claim belongs to F-038

**Where**: `CHANGELOG.md:255-257`

**What**: "The LUT chain's WGSL shader, evaluating DICOM PS3.3 C.11's first
three stages from a thirty-two-byte uniform, **so a window or level change
writes those bytes and re-uploads no texture.**"

The first half is true and asserted. The second half is not, and this project
has already had the argument about it and decided the other way.

**Why it is wrong.** `crates/ocelli-render/tests/voi_shader.rs:608-616`, on the
test that exists to carry exactly this claim:

> **What this asserts is the first half only.** Two windows over the same input
> give different outputs, and the whole difference between the two runs is a
> `VoiParams`... **It does not assert that no texture is created, and an earlier
> version of this comment said it did.** `on_gpu` is a test harness that
> rebuilds its module, pipeline and buffers on every call, so there is nothing
> here to measure a re-upload against. **The claim that a drag re-uploads
> nothing belongs to F-038's render graph, which is the first code with a frame
> to not re-upload during.**

So a reviewer forced this sentence out of a test comment during F-041 and
assigned it to a story in S12, and it has reappeared in the user-facing
register, where it is stronger rather than weaker: a CHANGELOG bullet reads as
delivered capability.

**There is also no path in which it could be measured.** The only
`create_texture` in production code is `crates/ocelli-render/src/probe.rs:716`,
the fill-rate probe's render target, which carries no image data. There is no
texture upload path, no render graph and no frame. `CURRENT_SPRINT.md:30-32`
says so: "Nothing renders a corpus frame end to end at the close of this
sprint." `AS_BUILT.md`'s own F-041 entry says "**Nothing renders a frame yet**".
A reader of the register would take this bullet as an interactive window/level
path that avoids re-uploads. There is no interactive path at all.

**Evidence**:

```
$ grep -rn "create_texture" crates --include="*.rs" | grep -v tests
crates/ocelli-render/src/probe.rs:716:    let texture = device.create_texture(&wgpu::TextureDescriptor {
```

That is the fill-rate benchmark. Nothing else in the workspace creates one.

**The repair is a deletion, not a rewrite.** "...from a thirty-two-byte uniform,
so a window or level change writes those bytes and nothing else" is true and
asserted by
`changing_the_window_changes_the_output_through_thirty_two_bytes` plus
`voi_params.rs`'s field-by-field layout test. Dropping "and re-uploads no
texture" is the whole fix, and the claim lands in the register when F-038 makes
it true.

---

### D2, two lines above the new bullets the register says the census bucket is not empty, and it is empty. **Not S11's**

**Where**: `CHANGELOG.md:203` and `CHANGELOG.md:210`

**What**: the guard-harness entry says, twice:

- "`python3 scripts/guard_census.py` prints the bucket watched by nothing **and
  it is not empty**."
- "It sorts every discovered refusal into four buckets, and the fourth is the
  refusals watched by nothing. **That bucket is not empty**, and the number in
  it is a ratchet that may only go down."

**Evidence**:

```
$ python3 scripts/guard_census.py | sed -n 2p
  503 belong to an entry carrying one of this harness's 452 probe(s), 329 to an
  entry naming a standing test that opens the file, 26 declared out of scope,
  0 watched by nothing
```

**This is the same claim I raised at pass 1 as N4 against `CLAUDE.md`**, which
you corrected at pass 3 to "it is empty today", with the ratchet reasoning I
verified. The `CHANGELOG.md` copies were never in scope of any pass and were
never touched. They survive for precisely the reason you flagged in your
message: `scripts/prose_check.py:18` puts `CHANGELOG.md` out of scope entirely,
so nothing reads it.

**I am raising this even though it predates S11 and is outside the sprint
diff**, because you asked for a human-equivalent read of a section nothing
checks, and reading it and not reporting a false sentence in it would defeat the
point of asking. It is also a two-word fix in a file D1 requires you to open
anyway.

**It is your call whether it blocks.** If you would rather carry it, say so and
I will not raise it again. If you fix it, the correction `CLAUDE.md` already
carries is the model: say it is empty today, say the line said otherwise, and
keep the instruction to run the command because the bucket is a ratchet.

---

## Nitpick

- **The `gpu` gate is arguably a missing entry**, and I put it at nitpick
  because it is a scope judgement rather than an error. Your framing, that these
  bullets describe crate-level capability, is defensible and consistently
  applied. But the register is not purely crate-level: "Stronger repository
  workflow guards" and "Sprint closure evidence bound to one Git tree" are both
  workflow entries, and F-037 added a gate that changes what
  `bin/ocelli.sh gate --list` prints and what `--sprint` runs, along with the
  finding behind it, that an `#[ignore]`d test previously ran in no profile at
  all. That is the same class as the two entries above it. Not a defect, and I
  would not block on it.

---

## The five questions, answered

### 1. Is each of the three bullets true?

Clause by clause, against the code rather than against `AS_BUILT.md`. Everything
below is true except the one clause at D1.

**Bullet 1, the cache.**

| Clause | Verdict |
|--------|---------|
| "across the encoded, decoded and GPU tiers" | True. `CacheTier::{Encoded, Decoded, Gpu}`, `tier.rs:19-29`, each citing HLD section 8 |
| "`Budgeted::bytes` reports what an entry costs the resource it is budgeted against rather than what it was made from" | True, and it is the trait's own documented contract verbatim, `lru.rs:24-28`. Note it is a requirement on an implementer rather than an observed behaviour, because the crate ships no value type. The register's wording matches the source exactly, so it is not overstating |
| "insertion reports evictions, a replaced value and a refused entry as three distinct outcomes" | True. `Admission { evicted, displaced, refused }`, `lru.rs:51/53/58`, deviation D-24 |
| "so a caller can surface three different events" | True as stated intent, and it is D-24's recorded reason, F-032 surfacing them as JS events |
| "the budget is asserted in bytes against hand-computed entry sizes" | True. `tests/budget.rs`. I recomputed all four in pass 1: 524,288, and 262,144 allocated against 153,600 source with a 108,544 difference, and 314,572,800 against 256 MiB |

**Bullet 2, the device.** Every clause true, and this is the bullet I attacked
hardest because it makes the most specific claims.

| Clause | Verdict |
|--------|---------|
| "opened from the adapter the tier resolved on" | True. `ResolvedAdapter::open` |
| "with the adapter's own limits rather than the WebGPU defaults" | True, measured at the line. `probe.rs:343`, `required_limits: self.adapter.limits()`, and `required_features: wgpu::Features::empty()` |
| "so a downlevel adapter can open one at all" | True as a statement about the mechanism. It does **not** claim tier B has been exercised, and it should not, because it has not been. Read as mechanism it is correct and it matches `tier-resolution.md`'s "A downlevel GL adapter does not meet the WebGPU defaults" |
| "Device loss is an observed state rather than a later call failing" | True. `DeviceState`, written by the callback `GpuContext::new` registers |
| "an unspecified loss is rebuilt on the same adapter **without re-resolving the tier**" | True, and I checked the second half specifically. `recover`'s body calls `adapter.open()` on the `ResolvedAdapter` it is handed and takes `*rebuilt.caps()`. There is no call to `resolve`, `classify` or `detect` anywhere in it, which is what HLD section 7's "resolves once at startup" requires |
| "a deliberate destroy is refused rather than rebuilt behind the caller" | True. `if !recovers_from(loss.reason) { return Err(DeviceError::Unrecoverable(...)) }`, and `recovers_from` is a total match, `Unknown` yes and `Destroyed` no |
| "A session that resolves to the CPU tier holds no device by construction" | True. `probe.rs:207`, `if !crate::caps::opens_a_device(&resolution.caps)` returns before any device exists, and `caps.rs:1072` asserts `!opens_a_device` for `Tier::Cpu` |

**Bullet 3, the shader.**

| Clause | Verdict |
|--------|---------|
| "evaluating DICOM PS3.3 C.11's first three stages" | True. `voi.wgsl:120-134` is stages 1, 2 and 3 in order |
| "from a thirty-two-byte uniform" | True, asserted twice, `voi_params.rs:62` and `voi_shader.rs:673`, plus the eight field offsets |
| "so a window or level change writes those bytes" | True, and asserted by `changing_the_window_changes_the_output_through_thirty_two_bytes` |
| "**and re-uploads no texture**" | **False as a claim about this sprint.** D1 |
| "The shader makes no LUT decision" | True, and it is enforced by omission rather than by discipline |
| "the uniform carries a resolved inversion flag, one selected window pair and rescale values" | True. `invert` from `LutChain::inverts`, the pair from `VoiTransform::window`, `slope` and `intercept` from `ModalityTransform::rescale` |
| "carries no photometric interpretation, no window multiplicity and no lookup-table sequence" | True. `VoiParams` has exactly eight fields and none of the three, and the WGSL references only `voi.*` |
| "A chain driven by a Modality or VOI LUT Sequence cannot be expressed in that uniform and reports unavailable rather than substituting the values the sequence overrode" | True. `VoiParamsError::ModalitySequenceNotExpressible` and `::VoiSequenceNotExpressible`, returned by `from_chain` before any field is filled |

### 2. Is anything user-visible in S11 still missing?

**Nothing that I would call a gap, and one judgement call**, which is the `gpu`
gate at the nitpick above.

I also considered and would **not** add:

- The new public accessors on `ocelli-pixel`, `ModalityTransform::rescale`,
  `VoiTransform::window`, `LutChain::modality` and `LutChain::voi`. They are
  literally new public API, but the register describes capability rather than
  signatures, and bullet 3's "the uniform carries... rescale values" is the
  capability they exist for. Adding them would be out of step with every entry
  above.
- The `Edge` and `Passes` newtypes exported from `ocelli-render`. Internal
  workload dimensions with no consumer outside the crate.
- Deviation D-24. The register does not carry deviations anywhere, and
  `docs/hld/DEVIATIONS.md` is the place for them.

### 3. Is the register consistent with the entries around it?

**Yes on tone and level of detail.** The three bullets are capability
paragraphs, one per delivered thing, in the same voice as "An explicit runtime
codec registry with exact Transfer Syntax UID capability states..." and "The
presentation stage of the LUT chain, completing DICOM PS3.3 C.11's first three
stages...". They name a small number of specific mechanisms and do not narrate
the story that built them, which is what `AS_BUILT.md` is for and which these
correctly avoid. Naming `Budgeted::bytes` is in step, because the neighbouring
entries name `ls crates | wc -l`, `ci/check-bindgen-isolation.sh` and
`bin/ocelli.sh gate guards`.

**One thing is out of step and it is not the new bullets.** The guard-harness
entry above them is roughly 20 lines of self-correcting narrative, twice the
length of anything else in the section, and it is the entry carrying D2. That is
an observation about the neighbour rather than a request to change it.

The three bullets are also correctly ordered to match the section's existing
shape, general capability first and the constraint or refusal second, which is
the pattern the codec and presentation-stage entries use.

### 4. Is the delta CHANGELOG-only, and did anything regress?

**Yes and no.**

```
$ git diff --name-only e6cad653ae61 f6aaac09e1aa | grep -v '^CHANGELOG.md$'
(empty)

$ git diff --stat e6cad653ae61 f6aaac09e1aa
 CHANGELOG.md | 22 ++++++++++++++++++++++
 1 file changed, 22 insertions(+)
```

Against the tree pass 4 passed clean, `git diff --name-only 325168bb0fee
f6aaac09e1aa -- crates/ scripts/ bin/ docs/` is **empty**. Every crate, script,
binary and document is byte-identical, so no code, gate, guard or document could
have regressed. I ran no gates beyond confirming that, because a 22-line
addition to a file no gate reads cannot move one, and because you asked for this
to stay narrow.

**And the thing worth stating plainly about the `prose` gate**: your note is
right and `scripts/prose_check.py:18` says it in the file itself, "**`CHANGELOG.md`
is not in scope at all.** Not 'its released sections are...'". So `gate --sprint`
being green over these 22 lines is not evidence about them, which is why D1 and
D2 were both found by reading rather than by running. `/close-sprint`'s item 8
has no mechanism behind it either: `sprint_workflow.py:472-475` opens the
CHANGELOG only to check that a release heading exists, never to check what the
`## Unreleased` section says.

### 5. Is this pass clean?

**No.**

D1 blocks and is one clause to delete. D2 is real, is false, and is not S11's,
and whether it blocks is yours to decide, but since D1 has you in this file
anyway the marginal cost of fixing it is two words in two lines.

**Nothing else about S11 has changed or needs to change.** The code is still the
tree I passed clean at pass 4, the four earlier passes stand, and if you fix D1
alone the section is true.

---

## Verified clean

- The delta is `CHANGELOG.md` and nothing else, confirmed two ways.
- Twenty of the twenty-one clauses across the three bullets are true, each
  checked against the code rather than against the prose that describes it.
- Bullet 2's six specific mechanisms were each located at their line:
  `required_limits: self.adapter.limits()`, the absence of any tier
  re-resolution inside `recover`, the `Unrecoverable` return for `Destroyed`,
  and the `opens_a_device` short circuit that makes a tier C session
  deviceless before a device could exist.
- The uniform is 32 bytes, asserted in two files, and the eight fields carry no
  photometric interpretation, no multiplicity and no sequence.
- The register's voice, length and structure match its neighbours.

**Tree state**: unchanged. `git write-tree` returns
`f6aaac09e1aad6309ab39952a481ccdfd4f8a3f9`. No mutation was needed for this
pass and none was made. The only working-tree addition is this report.
