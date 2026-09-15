# S11 whole-sprint review, pass 1

**Reviewed**: `git diff main...HEAD` on `sprint/s11`, HEAD `1974802`, tree
`98c2462347bd`. 66 files, ~12,000 insertions. Commits `c062774`, `d33ed44`,
`3129501`, `27c3a76`, `f863886`, `1974802`.
**Scope**: whole-sprint, independent. The three per-story loops (F-031 ten
passes, F-037 six, F-041 eight) are treated as done and were not re-run. This
pass looked only where a single-story diff cannot see.
**Machine**: working GPU, resolves tier A, so `gate gpu` and the shader
comparison ran for real.
**Result**: **5 defects, 3 smells, 6 nitpicks**

---

## Defects

### D1, the `gpu` gate's own comment enumerates a set F-041 changed two commits later

**Where**: `bin/ocelli.sh:303-306`

**What**: the gate's comment reads

> What the set is today: the device lifecycle, that short-circuit test, and
> the fill-rate instrument noted below. F-041's LUT-shader comparison joins it
> when that story lands, and the gate needs no edit for it because this names
> no test file.

F-041 landed in this sprint, at `3129501`, two commits after the comment was
written at `c062774`. The enumeration headed "What the set is today" omits the
nine `#[ignore]`d tests in `crates/ocelli-render/tests/voi_shader.rs`, and the
sentence after it is in the future tense about work that is in the tree under
review.

**Why it is wrong**: `/microscope` section 3, a factual sentence in a doc
comment is checkable and this one is false. It is also the exact failure F-037's
own AS_BUILT entry claims to have fixed: "A quantifier written beside a growing
set goes stale the first time a member joins it for a new reason... The fix was
not to correct six sentences, it was to stop glossing the set at all"
(`docs/sprints/AS_BUILT.md`, F-037 "Notes for future sessions"). The gloss was
then re-introduced three lines below the definition, and the story that made it
stale is in the same sprint.

**Evidence**:

```
$ bin/ocelli.sh gate gpu
  src/lib             3 passed   (gpu.rs, probe.rs x2)
  tests/device.rs     6 passed
  tests/voi_shader.rs 9 passed
ALL GREEN  1 gate(s)          # 18 ignored tests, not 5
$ grep -c '#\[ignore' crates/ocelli-render/tests/voi_shader.rs
9
```

Neither the definitional sentence ("every test marked `#[ignore]` in
`ocelli-render`") nor the gate's behaviour is wrong. Only the gloss is.

---

### D2, the guard catalogue and the guard runbook still say `oracle` is the one gate marked YES

**Where**: `scripts/guards/catalogue.py:7994-7996`, and the same text in
`docs/runbooks/guard-verification.md:706`

**What**: both carry

> The non-floor rule exempts a gate `bin/ocelli.sh` marks YES in its GPU
> column, **which is `oracle`** and deviation D-04's reason for it

F-037 added a second `YES` row. The sentence identifies the set by a singular
that is now wrong.

**Why it is wrong**: F-037 corrected this same claim in three other places in
the same commit, `.claude/WORKFLOW.md` ("There are now two gates that need a
GPU"), `scripts/ci_floor_check.py` ("**No count is written here and no gate is
named here**, deliberately") and `bin/ocelli.sh`. It missed the two sites that
the census PRINTS on every run, so the stale sentence is emitted by
`bin/ocelli.sh gate guards` and by `python3 scripts/guard_census.py` while the
gate reports OK. A declared limit that names the wrong gate is the failure mode
this repository already recorded once, in the same paragraph: "It named one
until the S03 review's tenth pass, and the job had been deleted a pass
earlier... Both halves were false, in a declared limit, which this project
treats as load-bearing."

**Evidence**:

```
$ bin/ocelli.sh gate --list | awk '$2=="YES"'
gpu          YES   ocelli-render's #[ignore]d tests, the hardware tier (E6.1, D-04)
oracle       YES   the differential corpus against cornerstone3D (HLD 11, D7)

$ python3 scripts/guard_census.py | grep -o "GPU column, which is .oracle."
GPU column, which is `oracle`
```

The limit's *other* half ("Marking `guards-deep|YES|` would exempt it without
this check noticing") is still true, so the paragraph is half right, which is
why it survived.

---

### D3, `pixel-pipeline.md` still says no shader reimplements VOI arithmetic, in the file F-041 edited

**Where**: `docs/lld/pixel-pipeline.md:14-15`

**What**:

> `ocelli-pixel` owns all interpretation between decoded DICOM sample
> containers and display values. No decoder interprets Bits Stored or
> signedness. **No shader reimplements modality or VOI arithmetic.**

`crates/ocelli-render/shaders/voi.wgsl` evaluates stage 1's rescale
(`let m = stored * voi.slope + voi.intercept;`, line 122), all three of section
18.2's window functions (`voi_linear`, `voi_linear_exact`, `voi_sigmoid`, lines
56-85), and stage 3's reflection (line 131). The shader's own header says so:
"The three window functions below **ARE written out here**".

**Why it is wrong**: F-041 edited this very file, adding 91 lines under
"**F-041 built that shader and the accessors it reads through**" at line 213,
and left the Ownership section's flat denial at line 14 untouched. A reader
checking `voi.wgsl` against the LLD that documents it finds a contradiction 200
lines apart. The defensible intent, settled in the design round at `3b00890`
("the review item is whether the shader can re-decide anything rather than
whether it contains a formula"), is that the shader makes no LUT *decision* ,
but that is not what the sentence says, and "arithmetic" is the word it uses.

**Evidence**: `sed -n '13,15p' docs/lld/pixel-pipeline.md` against
`sed -n '56,85p;120,134p' crates/ocelli-render/shaders/voi.wgsl`. The shader was
mutated (`let cp = c - 0.5;` to `let cp = c;`) and five GPU tests went red, which
is independent proof the WGSL is doing the arithmetic rather than reading a
result. Reverted, tree back to `98c2462347bd`.

---

### D4, `CLAUDE.md`'s "What exists" omits everything S11 built, which is the failure `CLAUDE.md` documents

**Where**: `CLAUDE.md:166-203`

**What**: the list headed "What exists, in the order it matters:" names the
oracle, the corpus, `ocelli-core`, the codec registry, the pixel pipeline,
derived geometry, every build target, the GPU device-sharing contract and the
npm pipeline. It names none of S11's three deliverables:

- `ocelli-cache`'s budgeted LRU (HLD section 20's `Budgeted` and `Lru`, section
  8's three tiers), a new crate with 715 lines of implementation
- the session's first long-lived `wgpu::Device`, and device loss as an observed
  state with a recovery path
- the first WGSL shader, PS3.3 C.11 stages 1 to 3 on the GPU

Line 195's bullet still describes `GpuContext` as "the GPU device-sharing
contract... section 38's Phase 1 hook", which was true when it owned a device
somebody else made and understates what F-037 built.

**Why it is wrong**: `CLAUDE.md:205-216` is a standing record of precisely this:

> **This paragraph used to say "No pixel, LUT or geometry port code has been
> written", and by S09 every word of that was false.** ... **The S09 sprint
> review found it**, and it survived four clean per-story reviews because no
> story in either sprint touched this file and each review looked at its own
> diff.

The same thing happened again. `git log --oneline -3 -- CLAUDE.md` shows the
file last changed at `8691714`, "S10, sprint review remediation", and no S11
story touched it. A list headed "What exists" that omits three delivered
components is false by omission in the same way the sentence it replaced was.

**Evidence**:

```
$ git log --oneline main..HEAD -- CLAUDE.md
(nothing)
$ git log --oneline -3 -- CLAUDE.md
8691714 S10, sprint review remediation
b3cbdfa S09, sprint review remediation
266d795 S03, sprint review pass 15, ...
```

The mechanically checkable claims in the section still hold: `ls crates | wc -l`
is 13, minus two is 11, and the deviation and corpus lines correctly refuse to
carry a number.

---

### D5, `S11-run.json` records nine review passes for F-041 where eight happened

**Where**: `.claude/scratch/S11-run.json`, `features["F-041"]["reviews"]`

**What**: the array's FIRST element is
`{"pass": 8, "defects": 0, "smells": 0, "nitpicks": 0}`, followed by passes 1
through 8. Nine records, and the duplicate claims a clean pass 8 completed
before pass 1 ran.

**Why it is wrong**: `scripts/sprint_workflow.py:241` only ever appends, so this
record cannot have been produced by `record-review` in that position. Two other
ledgers say eight: `docs/sprints/SPRINT_TRACKER.md` line 90 ("eight independent
review passes converging 5/4, 3/2, 1/1, 1/0, 1/0, 0/2, 1/0, 0/0", which matches
records 1..8 exactly), and the eight files
`.claude/reviews/F-041-implementation-pass-{1..8}.md`. A run-state file that
records a pass that did not occur is the ledger equivalent of a test that
asserts nothing.

Impact is bounded today, because `status` and `close-preflight` both read
`reviews[-1]`, which is the real pass 8. It is a defect because the record is
wrong, not because it currently misleads a tool.

**Evidence**:

```
$ python3 -c "import json;d=json.load(open('.claude/scratch/S11-run.json'));
r=d['features']['F-041']['reviews'];print(len(r),[x['pass'] for x in r])"
9 [8, 1, 2, 3, 4, 5, 6, 7, 8]
```

F-031 (10 records, passes 1..10) and F-037 (6 records, passes 1..6) are correct.

---

## Smells

### S1, the sprint declares tier B at two different confidence levels

**Where**: `docs/sprints/AS_BUILT.md:3092` against `docs/sprints/AS_BUILT.md`
F-041's tier row, and `docs/lld/gpu-ownership.md:330-334`

F-037: "**Tier coverage.** A: full... **B: full**, the device is opened with the
adapter's OWN limits and never `Limits::default()`, and loss and recovery are
backend-agnostic in wgpu 30.0.1."

F-041, for the same tier and the same absence of evidence: "B: the shader uses
nothing tier B lacks, asserted by text and by a real render pipeline, and
**nothing in this repository has ever run on tier B**, so the claim is that the
shader needs nothing tier B denies rather than that it has been seen to work
there."

F-037's own residue 1, eighteen lines below its tier row, and
`docs/lld/gpu-ownership.md:330-334`, both say the line that makes tier B work is
untested and that only a downlevel adapter separates it from
`Limits::default()`. So one entry says "full" and the documents around it say
"never exercised".

`docs/sprints/CURRENT_SPRINT.md:82-86` names exactly this risk: "F-037 and F-041
are where a feature written for tier A and never tried on tier B starts to look
**finished** while being unavailable on half the declared matrix." A reader
scanning the tier rows sees `B: full` and stops. Neither story is wrong about
its own work. The sprint's answer for tier B is incoherent, and only a
whole-sprint pass sees both rows at once.

### S2, `ocelli-render`'s crate doc does not mention `voi`

**Where**: `crates/ocelli-render/src/lib.rs:5-19`, against lines 33 and 42

The doc narrates the crate story by story ("F-001 creates the crate. F-008 gives
it the device-ownership contract. F-004 resolves the tier, F-037 creates the
long-lived device... and the render graph follows in F-038") and then describes
the module split as `caps` / `probe` / `gpu`. `voi` is a fourth public module,
added by F-041 in this sprint, carrying the crate's first shader, HLD section
18.4's uniform and three of the crate's public exports. It appears in
`pub mod voi;` and `pub use voi::{VOI_WGSL, VoiParams, VoiParamsError};` and
nowhere in the prose. The next reader of this crate learns its shape from a
paragraph that stops one module short.

### S3, four LLD index rows drifted from their own files in one sprint

**Where**: `docs/lld/README.md:30-32, 37`

| File | Its own `F-IDs that contributed` line | The index row |
|------|---------------------------------------|---------------|
| `gpu-ownership.md` | F-004, F-005, F-008, **F-037, F-041**, F-X001 | F-004, F-005, F-008, F-X001 |
| `tier-resolution.md` | F-004, **F-037**, F-X001, F-X016 | F-004, F-X001, F-X016 |
| `feature-availability.md` | **F-037, F-041**, F-X001 | F-X001 |
| `guards.md` | **F-037**, F-X008, ... | F-X008, ... |

`docs/lld/README.md:18` states the rule: "If you edit an LLD file, edit its
header line and its row here in the same hand." Lines 12-17 record that the S03
review found ONE such mismatch out of eleven rows and called it out. This sprint
produced four, and F-031 got its row right, so the mechanism works when it is
used. Nothing mechanical reads these lines, which the README also says, so they
only ever drift back at a sprint review.

---

## Nitpicks

- **N1** `docs/lld/gpu-ownership.md:303-306` glosses the `gpu` gate's set as
  "the device lifecycle... and also holds the tier C short-circuit test and the
  fill-rate instrument", omitting the nine `voi_shader` tests. Weaker than D1
  because the definitional sentence beside it ("every `#[ignore]`d test in the
  crate") is correct and the list reads as partial.
- **N2** `docs/lld/gpu-ownership.md:4`, `docs/lld/feature-availability.md:4` and
  `docs/lld/pixel-pipeline.md:8` all carry `**Last updated:** 2026-09-14`.
  F-041 edited all three at `3129501`, 2026-09-15 00:55.
- **N3** `crates/ocelli-render/Cargo.toml:59-69`: the comment explaining
  pollster now sits above the `[dev-dependencies]` header and F-041 inserted
  `ocelli-core` between it and `pollster`, so it reads as a section header for a
  section whose first entry it does not describe.
- **N4** `CLAUDE.md:162-163`: "`python3 scripts/guard_census.py` prints the
  bucket watched by nothing, **and it is not empty**". The census prints
  "0 watched by nothing". Reproduced at base commit `571cc01` in a scratch
  worktree, so this predates S11 and is not this sprint's doing.
- **N5** `CLAUDE.md:183-184`: "**The codec registry and five decoder
  families.** Raw, RLE, JPEG, JPEG 2000, JPEG-LS and HTJ2K" lists six. Also
  predates S11.
- **N6** `bin/ocelli.sh:87`: the `gpu` row's provenance reads "(E6.1, D-04)".
  The set it runs is now E6.1 and E6.5. The COVERS text itself is correct.

---

## Verified clean

Everything below was executed on this tree, not read and judged.

**Gates run** (exit code read from the command): `backlog`, `deviations`,
`prose`, `content`, `ci` , ALL GREEN, 5 gates. `bindgen`, `unsafe`, `device`,
`nostd`, `pins` , ALL GREEN, 5 gates. `wasm` , GREEN. `gpu` , GREEN, 18 ignored
tests across `src/lib`, `tests/device.rs` and `tests/voi_shader.rs`.
`python3 scripts/guard_census.py` , exit 0, 858 refusals, 0 watched by nothing.
`python3 scripts/sprint_workflow.py close-preflight` , the only two complaints
are "no sprint-scope review recorded" (this pass) and the scratch worktree I
created, since removed.

**"What done means", clause by clause.**

- F-031 implements section 20's `Budgeted` and `Lru<K, V>`, `insert` returns
  `Admission` under declared deviation **D-24**, whose text I checked against
  the code: the row says refusal is "`bytes` above the whole budget or a budget
  of zero", and `would_admit` is `self.budget > 0 && bytes <= self.budget`,
  which is exactly that complement. `deviations` gate reports 24 declared,
  matching the AS_BUILT note that "the count stayed at 24".
- Three tiers with distinct pressure: `CacheTier::{Encoded, Decoded, Gpu}` and
  `Pressure`, carried by the cache and not the value.
- "No allocation in the render loop" holds as stated, because `insert` is not a
  render-loop call and `get`/`remove`/`contains_key`/`pressure`/`len`/
  `is_empty`/`would_admit` allocate nothing. `docs/lld/cache.md:144-177` records
  the measurement honestly rather than repeating the plan's false
  allocation-free claim, and F-031's AS_BUILT names that correction.
- Budget asserted in hand-computed bytes: 512x512x16-bit = 524,288, a 512-row
  texture at 300 source bytes per row padded to 512 = 262,144 allocated against
  153,600 source, difference 108,544, 512x512x600x16-bit = 314,572,800 against
  256 MiB = 268,435,456. All four arithmetic claims recomputed and correct.
- F-037's device: `gate device` green, `ci/check-device-ownership.sh` unweakened,
  `grep -rn "submit(" crates --include="*.rs"` finds four call sites and all
  four are tests or the transient fill-rate probe, so "no second
  `queue.submit()` per frame" holds (vacuously, there is no frame yet).
- F-041 adds no arithmetic `ocelli-pixel` does not own **in the sense the design
  round settled**: HLD 18.4 specifies only the uniform, not the formulas, so a
  shader given `fn_kind` must evaluate something. I diffed
  `shaders/voi.wgsl:56-85` against `crates/ocelli-pixel/src/lut.rs:546-588`
  term by term. `c - 0.5` and `w - 1` in LINEAR and neither in LINEAR_EXACT,
  lower `<=` and upper `>` in both, `-4 * (x - c) / w` in SIGMOID with no clamp,
  stage 3 as `ymin + ymax - d` and not `ymax - d`. The shader receives a resolved
  `invert` and no Photometric Interpretation, so the F-029 double-inversion
  defect is unreachable by construction, and a LUT Sequence yields
  `VoiParamsError` rather than substituted rescale values.
- All three declare all three tier rows. F-031 n/a/n/a/n/a with a reason,
  F-037 and F-041 as quoted under S1.
- `wasm-bindgen` confined: `gate bindgen` green, and D-12's wasm32 carve-out is
  unchanged by this sprint.
- Pixels do not cross the boundary: the only readback is
  `tests/voi_shader.rs` mapping an `f32` storage buffer on the host natively,
  which the test module header addresses against D3 explicitly.

**The sprint's four named defect classes, each checked rather than assumed.**

1. *A second copy of the LUT arithmetic in WGSL.* Present as transcription and
   argued for in the design round and in the shader's header. The two decisions
   that could diverge, inversion and window selection, are resolved on the CPU
   and the shader is given no input from which to re-make either. D3 above is a
   documentation contradiction about this, not a second copy.
2. *Tier B written for and never tried.* Confirmed never tried. F-037 and F-041
   describe that fact with different confidence, which is S1.
3. *Tier C omitted rather than answered.* Answered by all three. F-037 made it a
   mechanism (`caps::opens_a_device` false, `resolve_adapter` returns nothing),
   F-041 answers "n/a as a shader, full as arithmetic" and reimplements nothing,
   F-031 n/a with a reason.
4. *Device loss treated as unreachable.* Not: the `Destroyed` path is proved end
   to end on a real adapter and the `Unknown` rebuild through a `pub(crate)`
   `#[cfg(test)]` seam, with the gap stated in three places rather than glossed.
   `recovers_from` is a total match with no wildcard arm.
5. *`Budgeted::bytes` reporting a source size.* Not: the trait doc, the LLD and
   the fixture all state and test the allocated-versus-source distinction, and
   `Entry::bytes` is read once at admission with three unit tests holding each
   site that gives bytes back.

**AS_BUILT factual claims, executed.**

- F-041's "Measured maximum divergence: 0.000030517578" ,
  `cargo test -p ocelli-render --test voi_shader -- --ignored --nocapture`
  printed `measured maximum divergence over the sweep: 0.000030517578`. Exact.
- F-041's open finding against `ocelli-pixel` , recomputed in `f32`: at
  `c = 1024.5`, `w = 1.0003662109375`, `c' = 1024`, `w' = 3/8192`, the upper
  breakpoint rounds to 2 ulp above 1024, `(x - c')/w' = 2/3`, body =
  `(2/3 + 0.5) * 255 = 297.5`. The 297.50003 and the 42.5 overshoot both check
  out, and the asymmetry argument (only the upper breakpoint can escape, because
  the lower comparison is `<=`) is correct.
- `CURRENT_SPRINT.md`'s size claim , `ci/wasm-size-budget.json` records 16,388,
  `gate wasm` printed "wasm size 16,455 bytes, baseline 16,388, ceiling 17,207".
  The 67-byte drift and the 5 per cent tolerance are both exactly as written,
  and no story re-baselined.
- Verify ledger , the feature profile is 28 gates and the sprint profile 31,
  matching F-037's and F-041's "28 gates including `corpus` and the new `gpu`"
  and the recorded sprint verification at tree `98c2462347bd`. F-037's staged
  tree `60865666e8bc` is present. F-031's worker-tree entry is not locally
  checkable, because `.claude/verify-ledger.json` is gitignored and lived in the
  worker worktree, its "22 green, four skipped, corpus=absent" is consistent
  with the 27-gate feature profile that tree had (30 gates, no `gpu` yet).
- Review pass counts , F-031 ten and F-037 six agree across
  `SPRINT_TRACKER.md`, `S11-run.json` and the report files. F-041 is D5.
- Ledger agreement , `gate backlog` green, `grep -c '^| F-[0-9]'
  CURRENT_SPRINT.md` is 3, BACKLOG's three S11 rows all read `done`, the
  `Status` column in CURRENT_SPRINT agrees.
- `.claude/handoffs/` holds only `README.md`, so F-031's handoff was consumed
  by integration rather than left behind, which is the README's own test.

**Hygiene over the added lines** (`git diff main...HEAD -U0`, 12,120 added
lines): no `unsafe`, no `.unwrap()`, no `.expect(`, no `panic!` and no `as` cast
in any added Rust or WGSL , every grep hit is the word "as" in prose or a doc
comment quoting the rules. Em-dashes appear only in `.claude/plans/*-design.md`,
all inside HLD quotations, which `docs/hld/` exemption covers and `gate prose`
accepts over 321 files.

**Would the tests fail if the code were wrong.** One mutation, chosen because it
is the sprint's headline arithmetic and crosses two crates: `voi.wgsl`'s
`let cp = c - 0.5;` to `let cp = c;`. Result: 5 of 9 shader tests red, including
the hand-computed section 18.3 rows, the boundary test and the CPU sweep, with
messages naming the expected value ("LINEAR at -159: was 0.31954962, wanted
0.63909775"). Reverted, `git write-tree` returned `98c2462347bd`.

**Reachability.** Nothing added this sprint is run by no gate.
`crates/ocelli-render/tests/ui/workload_dimensions_are_not_interchangeable.rs`
runs under `test` via trybuild (verified: 1 passed).
`cargo test -p ocelli-cache` runs 27 tests across four binaries under `test`.
The 18 `#[ignore]`d tests run under `gpu`, which is in `--sprint` and `--all` and
correctly excluded from `--floor` under D-04, with `NOT_IN_FLOOR` and the
runner's `case` line compared for set equality by `gate ci` (which printed
"agree on corpus, gpu, guards-deep, oracle, quirk-mutations").

---

## Not done in this pass

- `bin/ocelli.sh gate --sprint` was not run end to end. Its 31 gates were run in
  groups plus `gpu` and `wasm`, `oracle`, `guards-deep`, `quirk-mutations`,
  `corpus`, `test`, `clippy`, `fmt`, `types`, `lint`, `native`, `packages`,
  `bench`, `errors`, `panic`, `skills`, `corpus-tests`, `quirks`, `provenance`
  were taken from the recorded sprint verification at this exact tree.
- The pass was NOT recorded with
  `python3 scripts/sprint_workflow.py record-sprint-review`, to leave sprint
  state unmutated for the operator. It reports 5 defects and 3 smells, so it is
  not a clean pass and cannot close the sprint.

**Tree state**: `git status --porcelain` clean apart from this report,
`git write-tree` returns `98c2462347bd09bbb2324b1b2a8c96b6b3b1ddb5`.
