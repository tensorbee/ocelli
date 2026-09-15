# S11 whole-sprint review, pass 2

**Reviewed**: staged tree `e8f6f4e343065bbd55cfa3edc06db28b369e77cb`, on
`sprint/s11` at HEAD `1974802`. The remediation is uncommitted and staged, 15
files, 537 insertions and 38 deletions, of which `.claude/reviews/S11-sprint-pass-1.md`
is 452 insertions.
**Scope**: the remediation for pass 1's 5 defects, 3 smells and 6 nitpicks, plus
the four questions the coordinator asked, plus a re-sweep for contradictions the
remediation itself could have created.
**Machine**: working GPU, resolves tier A.
**Result**: **2 defects, 2 smells, 4 nitpicks**

**This pass is NOT clean.** Do not push. One of the two defects is a regression
I caused: the remediation acted on a pass-1 nitpick that was wrong, and made a
correct line incorrect. I own that and it is written up as such.

---

## Verdict on each pass-1 finding

| Pass 1 | Status | Note |
|--------|--------|------|
| D1, `gpu` arm's stale set | **Closed**, with a new smell | The four members and the count are correct today. See S1 |
| D2, `oracle` named as the one YES gate | **Not closed** | The gate name is gone and the sentence no longer parses. See D2 |
| D3, `pixel-pipeline.md` ownership | **Closed** | Rewritten correctly. Attacked clause by clause below |
| D4, `CLAUDE.md` "What exists" | **Closed**, with a new defect elsewhere in the same edit | The three new entries are true. See D1 |
| D5, duplicate pass-8 record | **Closed** | F-041 now holds 8 records, passes 1 to 8 |
| S1, tier B at two confidences | **Closed** | F-037's row no longer says "full" and now agrees with F-041's |
| S2, `voi` missing from the crate doc | **Closed**, with a new smell | See S2 |
| S3, LLD index rows | **Closed** | All 18 rows verified against their files independently |
| N1, gpu-ownership set list | Closed | Names all four members |
| N2, `Last updated` dates | Closed for the three that needed it, over-applied to three that did not. See N3 |
| N3, `pollster` comment | Closed | Reattached inside `[dev-dependencies]`, above `pollster` |
| N4, `CLAUDE.md` census bucket | **Not fixed** | Still false. See N1 |
| N5, "five decoder families" | **Fixed in the wrong direction** | See D1 |
| N6, `gpu` row cites E6.1 only | Closed | Now `(E6.1, E6.5, D-04)` |

---

## Defects

### D1, `CLAUDE.md` now says six decoder families, and the codebase says five. This is my fault

**Where**: `CLAUDE.md:183-185`

**What**: the line now reads

> **The codec registry and its decoder families.** Raw, RLE, JPEG, JPEG 2000,
> JPEG-LS and HTJ2K, **which is six and which this line called five until S11's
> sprint review counted them.**

**Why it is wrong**: this repository defines a decoder family as a registration
function, and there are exactly five. `crates/ocelli-codec/src/native.rs:226`
registers **Raw and RLE together** as one family, so the six formats listed are
covered by five families. The crate's own test says so in the assertion the
line's last sentence points at:

`crates/ocelli-codec/tests/registry.rs:429-441`

> Each family's own test asserts that its registration is atomic... **None of
> them registers more than one family**, so the property that matters at the
> crate level is asserted here: **the five coexist**, they partition the known
> catalogue, and no two claim the same UID... a **sixth** family claiming a UID
> an existing one owns would fail at run time.

So `CLAUDE.md` now contradicts the file it cites two lines later, and the new
clause attributes the change to a sprint review that got it wrong. It did. Pass
1's N5 counted the enumeration rather than the families, did not open
`crates/ocelli-codec/`, and should not have been raised. The original text was
correct and is now incorrect, which is a strictly worse state than before the
remediation.

**Evidence**:

```
$ grep -rn "pub fn register_" crates/ocelli-codec/src/*.rs
crates/ocelli-codec/src/htj2k.rs:92:   pub fn register_htj2k_decoders
crates/ocelli-codec/src/jpegls.rs:69:  pub fn register_jpegls_decoders
crates/ocelli-codec/src/jpeg.rs:82:    pub fn register_jpeg_decoders
crates/ocelli-codec/src/jpeg2000.rs:52:pub fn register_jpeg2000_decoders
crates/ocelli-codec/src/native.rs:226:  pub fn register_native_and_rle_decoders
```

Five, and `native_and_rle` is one. `.claude/reviews/S09-sprint-pass-2.md:37`
records the S09 review closing "no test registered the five decoder families
together", which is where the number came from and it was measured then.

The right repair is to restore "five decoder families" and delete the new
clause. If the count is thought worth defending at all, the pattern this file
already uses for the corpus rows applies: name the command rather than the
number.

---

### D2, the sentence that was to stop naming a gate no longer parses

**Where**: `scripts/guards/catalogue.py:7995`, and the rendered copy at
`docs/runbooks/guard-verification.md:706`

**What**: the repair substituted text in place and produced

> The non-floor rule exempts a gate `bin/ocelli.sh` marks YES in its GPU column,
> **which deviation D-04 is the reason for it**, and that column is not in the
> declared-constant ratchet.

The old text was "...in its GPU column, **which is `oracle` and deviation
D-04's reason for it**, and that column...". Removing the gate name took the
verb with it. The result has a relative pronoun with no predicate and a
resumptive "it" with no antecedent, and it cannot be read as either true or
false.

**Why it is wrong**: the coordinator's report says "The sentence no longer names
a gate", which is true, and the review question was whether every remediation
sentence is **true**. A sentence that does not parse is not true. This string is
not a comment. It is emitted verbatim by `python3 scripts/guard_census.py`, by
`bin/ocelli.sh gate guards`, and into the runbook, so the broken text is in the
output of two gates that both report OK.

**Evidence**:

```
$ python3 scripts/guard_census.py | grep -o "GPU column, which[^,]*,"
GPU column, which deviation D-04 is the reason for it,

$ grep -o "GPU column, which[^,]*," docs/runbooks/guard-verification.md
GPU column, which deviation D-04 is the reason for it,
```

No gate caught it: `guards`, `prose` and `ci` are all green on this tree. The
`prose` gate checks voice rules, not grammar, so nothing here was ever going to.

Everything else about the D2 repair is right, and this is a wording fix rather
than a re-do. Something like "...marks YES in its GPU column, deviation D-04
being the reason, and that column is not in the declared-constant ratchet"
carries the same content. The other half of the limit, "Marking
`guards-deep|YES|` would therefore exempt it without this check noticing", is
untouched and still true.

---

## Smells

### S1, the arm that warns against a gloss beside a growing set now carries a hard count

**Where**: `bin/ocelli.sh:303-308`

The repaired comment reads "What the set is today, with F-031, F-037 and F-041
all landed: the device lifecycle, the tier C short-circuit test, the fill-rate
instrument noted below, and F-041's LUT-shader comparison against
`ocelli-pixel`. **Eighteen tests.**"

Every word of that is true today, measured. It is also a number in prose beside
a set that three scheduled stories will grow: F-038 in S12, F-040 in S13 and
F-042 in S13 all add to `ocelli-render`, and any `#[ignore]`d test they add
joins this set by the attribute alone. The comment's own next clause says "THIS
COMMENT DID [need an edit], which is the gloss-beside-a-growing-set shape
recurring in the same arm that warns about it", and then hands the next author a
count, which is a more brittle gloss than the list pass 1 objected to, not a
less brittle one.

`CLAUDE.md` already states the rule this violates twice, for the corpus rows and
for the shared-crate count: replace the number with the command that prints it.
`cargo test -p ocelli-render -- --ignored --list` is that command, and it costs
no adapter.

### S2, the crate doc restates, in the crate that owns the shader, the formulation `pixel-pipeline.md` just declared false

**Where**: `crates/ocelli-render/src/lib.rs:16-23`, against
`docs/lld/pixel-pipeline.md:17-22`

The new paragraph says the `voi` module "holds HLD section 18.4's uniform,
[`voi::VoiParams`], and **the WGSL that reads it**, [`voi::VOI_WGSL`]", and then
that "**nothing in `voi` computes a LUT value or makes a LUT decision**".

In the same staged commit, `pixel-pipeline.md` retires exactly that formulation:

> That sentence read "no shader reimplements modality or VOI arithmetic" until
> F-041, **which made it false**: `crates/ocelli-render/shaders/voi.wgsl`
> evaluates stage 1, all three window functions and stage 3.

Both readings of the lib.rs sentence are available and they disagree. Under the
narrow one, the Rust in `voi.rs` computes nothing, which is true and is what
`voi.rs:1-12` has always said. Under the one a reader takes from the sentence
before it, `VOI_WGSL` is in `voi` and it computes three window functions. The
remediation corrected the formulation in the LLD and planted a near copy of it
in the crate doc, 250 lines apart and in the same edit.

It is a smell rather than a defect because the narrow reading is defensible and
because `voi.rs`'s own module header carried the same shape through eight review
passes. The fix is one clause: say that `voi`'s Rust computes nothing and that
the WGSL it carries evaluates the formulas from parameters it cannot re-decide,
which is what `pixel-pipeline.md` now says and what the shader's own header says.

---

## Nitpicks

- **N1** `CLAUDE.md:162-163`, carried unfixed from pass 1's N4:
  "`python3 scripts/guard_census.py` prints the bucket watched by nothing, **and
  it is not empty**". The census prints "0 watched by nothing", measured on this
  tree and at base commit `571cc01`. Note the pairing: of pass 1's two
  `CLAUDE.md` nitpicks, the remediation applied the one that was wrong (N5, now
  D1) and left the one that was right. The phrase wraps across two lines, which
  is presumably why a grep for it missed.
- **N2** `CLAUDE.md:217` says the shader agrees with `LutChain::map_into` "to
  two `f32` ULP" and drops the magnitude. `docs/sprints/AS_BUILT.md` and
  `docs/lld/pixel-pipeline.md` both write it as "two `f32` ULP **at 255**, or
  one at 256", which is the form that is checkable. 0.000030517578 is 2 ULP at
  255 and roughly 8 million ULP at 1e-5, so the qualifier is doing real work.
- **N3** Three `Last updated` dates moved to 2026-09-15 without a content change
  in this remediation or on that date: `docs/lld/guards.md:4` and
  `docs/lld/tier-resolution.md:4`, whose last substantive edit is F-037 at
  `d33ed44`, 2026-09-14 21:14, and `docs/lld/cache.md:4`, whose content is
  F-031's from `27c3a76` on 2026-09-14. The three that pass 1 flagged
  (`gpu-ownership`, `feature-availability`, `pixel-pipeline`) needed the move and
  got it.
- **N4** A pass-1 miss rather than a remediation issue. `cargo doc -p
  ocelli-render --no-deps` emits **10** `rustdoc::private_intra_doc_links`
  warnings, all from F-037's additions in `crates/ocelli-render/src/probe.rs`
  (`Edge` linking to `run`, `measure`, `fragments`, `RUN_PLAN`, and
  `resolve_adapter` linking to `detect`). The links are in the S11 diff, 21 of
  them. No gate runs `cargo doc`, so nothing in `--sprint` sees this. Cosmetic,
  and recorded so it is not found a third time.

---

## The four questions, answered

### 1. Is every remediation sentence true?

No. D1 and D2 above. Everything else checked out, and the two paragraphs the
coordinator flagged for attack were attacked clause by clause.

**`pixel-pipeline.md`'s rewritten ownership paragraph, clause by clause.**

| Clause | Verdict |
|--------|---------|
| "`voi.wgsl` evaluates stage 1, all three window functions and stage 3" | True. `voi.wgsl:122` stage 1, `:56-85` the three functions, `:131` stage 3 |
| "HLD section 18.4's uniform hands a shader `slope`, `intercept`, `center`, `width` and `fn_kind`" | True, read from `docs/hld/15-lut-chain.md` section 18.4, which gives the struct and no formulas |
| "a shader given those has to evaluate something" | True, and it is the reasoning the S11 design round recorded at `3b00890` |
| "receives a resolved `invert` flag and no Photometric Interpretation" | True. `VoiParams` has eight fields and no PI, and the WGSL references only `voi.*` |
| "one selected window pair and no multiplicity" | True. `VoiTransform::window()` returns the pair `VoiTransform::new` selected |
| "rescale values and no sequence" | True. `ModalityTransform::rescale()` returns `None` for a sequence |
| "a width this crate already validated" | True on the `from_chain` path, which is every caller. `voi.rs:75-102` documents at length that a hand-assembled `VoiParams` bypasses it, and that hazard is unchanged and already reviewed |
| "`VoiParams::from_chain` refuses outright when a LUT Sequence makes the uniform inexpressible" | True, two variants, no substitution |

I also swept the repository for a second copy of the retired sentence, because a
correction applied in one place and not another is this project's standing
failure. There is none: `grep -rn "No shader\|no shader\|shader reimplement"`
over all tracked `.md`, `.rs` and `.toml` outside `docs/hld/` and
`.claude/reviews/` returns only the corrected paragraph and
`pixel-pipeline.md:452`'s separate and still-true colour-stage line.

**`CLAUDE.md`'s three new entries.**

| Claim | Verdict |
|-------|---------|
| Long-lived device "opened from the adapter the tier resolved on" | True, `ResolvedAdapter::open` |
| "rebuilds an `Unknown` loss and refuses a deliberate `destroy`" | True, `caps::recovers_from`, a total match with no wildcard arm |
| "`ocelli-render` is still the only crate that may create a device" | True, `gate device` green on this tree |
| Budgeted LRU is "section 20's `Budgeted` and `Lru<K, V>`" with section 8's tiers "as a discriminant and a pressure signal" | True, `CacheTier` and `Pressure` |
| "It holds no value type... are S12 and S13 stories" | True. BACKLOG puts F-032, F-033 and F-036 in S12 and F-040 in S13 |
| Shader "agreeing with `LutChain::map_into` on a real adapter to two `f32` ULP" | True, with the qualifier dropped. See N2 |
| "Nothing renders a frame yet, so decision D7 still holds" | True, and consistent with F-041's AS_BUILT entry |
| "F-038 and F-040 are what make a frame out of these three" | Consistent with `AS_BUILT`. `CURRENT_SPRINT.md:30-32` adds F-039, which is where the frame goes rather than how it is made, so this is not a contradiction |

### 2. Is the `oracle.md` row repair correct, and was any other row damaged?

**Yes and no damage.** I compared all 18 index rows against their files with my
own parser rather than trusting the re-check, using a lookahead that tolerates a
header wrapping across lines. **Zero mismatches.** `oracle.md`'s row now reads
`F-010, F-012, F-013, F-014, F-X006, F-X007, F-X008, F-X009, F-X012, F-X013,
F-026`, which is byte for byte what `docs/lld/oracle.md:3-4` carries across its
two lines, trailing `F-026` and all. That trailing out-of-sort `F-026` is in the
source file and predates S11, so the row is correct as a sync and the file is
the thing that is unsorted. Worth knowing, not worth fixing here.

The `Area` column is untouched on every row, and the eleven F-ID changes are all
additions of F-IDs that really do appear in the corresponding file's header. No
row lost an F-ID.

### 3. Did `--render-runbook` change only what D2 required?

**Yes, exactly one line.** `git diff --cached --numstat` reports `1 1` for
`docs/runbooks/guard-verification.md`. I then re-rendered from the current
catalogue into a copy and diffed: byte identical, so the runbook and the
catalogue are in sync and the render is idempotent at this tree. The copy was
restored and `git diff --stat` is empty.

### 4. Did the remediation introduce a new cross-story contradiction?

**Two.** D1 sets `CLAUDE.md` against `crates/ocelli-codec/tests/registry.rs`,
and S2 sets `crates/ocelli-render/src/lib.rs` against
`docs/lld/pixel-pipeline.md`. Both were created by this remediation and neither
existed at `98c2462347bd`.

Checked and found clean: `bin/ocelli.sh:303-306` and
`docs/lld/gpu-ownership.md:302-305` now name the same four members of the `gpu`
set, F-037's and F-041's tier B rows in `AS_BUILT.md` now describe the same
absence of evidence in the same terms, `scripts/guards/census.py`'s `DELEGATED`
entry for `gpu` is untouched and still true, and the new `CLAUDE.md` device
bullet does not contradict the older device-sharing-contract bullet two lines
above it.

---

## Verified clean

- **The remediation touched no logic.** `git diff --cached` over `*.rs`,
  `*.wgsl` and `*.toml`, with comment and blank lines filtered out, is empty.
  The only non-prose file changed is `scripts/guards/catalogue.py`, and that is
  one string literal.
- **Gates run on this tree**, exit codes read from the command: `prose`,
  `content`, `guards`, `backlog`, `deviations`, `ci` ALL GREEN, 6 gates.
  `fmt`, `clippy` ALL GREEN. `gpu` ALL GREEN.
- **The "Eighteen tests" count verified by execution**: `bin/ocelli.sh gate gpu`
  runs 3 in `--lib`, 6 in `tests/device.rs` and 9 in `tests/voi_shader.rs`.
  Eighteen.
- **The recorded verification is real.** `.claude/verify-ledger.json` holds
  `e8f6f4e343065bbd55cfa3edc06db28b369e77cb` with `profile=sprint`, 31 gates
  including `oracle` and `gpu`, and `corpus=pass`.
  `.claude/scratch/S11-run.json` carries the matching verification row.
- **D5 confirmed closed by reading the file**: F-031 10 records passes 1 to 10,
  F-037 6 records passes 1 to 6, F-041 **8** records passes 1 to 8, no
  duplicates in any of the three.
- **Intra-doc links in the new crate-doc paragraph resolve.** `cargo doc -p
  ocelli-render --no-deps` emits no warning for `voi::VoiParams` or
  `voi::VOI_WGSL`. The 10 warnings it does emit are N4 and predate this edit.
- **`Cargo.toml`'s dev-dependency block is correct after the move**:
  `ocelli-core`, `pollster`, `proptest`, `trybuild`, each under its own comment,
  and the `pollster` rationale is now directly above `pollster`.
- **The `ci` gate still reads the repaired `gpu` arm**, reporting "the runner's
  --floor exclusion list and NOT_IN_FLOOR agree on corpus, gpu, guards-deep,
  oracle, quirk-mutations", so the shell reader survived the comment rewrite.
- **`close-preflight`** reports only the four expected things: the sprint review
  is pass 1 with findings, its tree is stale against `e8f6f4e34306`, the working
  tree is not clean and the staged tree is not the HEAD tree. All four are
  consequences of the remediation being uncommitted and this pass being
  unrecorded.

## Not done in this pass

`bin/ocelli.sh gate --sprint` was not re-run end to end. The remediation changes
no logic, and the ledger records a passing 31-gate sprint verification against
this exact tree, so the gates I re-ran were the ones the edits could plausibly
break. `record-sprint-review` was not run, as instructed.

**Tree state**: unchanged. `git write-tree` returns
`e8f6f4e343065bbd55cfa3edc06db28b369e77cb`. The only working-tree addition is
this report. The runbook copy made for question 3 was restored and verified
identical.

## Recommendation

Two defects and two smells, all four cheap and all four documentation. D1 is a
one-line revert plus a deleted clause. D2 is a reworded sentence in
`scripts/guards/catalogue.py` followed by `--render-runbook`. S1 is a number
replaced by a command. S2 is one clause. None needs a gate re-run beyond
`prose`, `guards` and `ci`, and none touches code. Then pass 3.
