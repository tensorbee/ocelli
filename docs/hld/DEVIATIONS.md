<!-- Hand-maintained. Not generated. -->

# Deviations from the HLD

The HLD is normative. Where this repository does something different, it is
recorded here with the reason, not applied quietly. HLD Part II opens with the
rule this file exists to serve:

> Where it gives a formula, a layout or a signature, that is the intended
> implementation and a deviation should be raised rather than improvised.

A deviation is added by a design plan, reviewed like code, and asserted by
`scripts/deviation_check.py`, which refuses a build where a deviation named in
a plan has no row here.

| # | HLD says | We do | Why | Raised |
|---|----------|-------|-----|--------|
| D-01 | §15.2, `rust-version = "1.85"`, `resolver = "2"` | `1.97.1`, `resolver = "3"`, pinned in `rust-toolchain.toml` | Operator instruction. 1.97.1 is above the dicom-rs MSRV floor the HLD cites, so the floor is still satisfied, and resolver 3 is the edition 2024 default. | Bootstrap |
| D-02 | Backlog stories E3.1, E5.1, E6.1, E11.3 name crates `tb-dicom`, `tb-cache`, `tb-render`, `tb-geom` | `ocelli-dicom`, `ocelli-cache`, `ocelli-render`, `ocelli-geom` | HLD §4 and §15.1 both give the `ocelli-` prefix and are the later authority. The `tb-` spellings are a pre-naming artefact in the spreadsheet. | Bootstrap |
| D-03 | The backlog is keyed on E-IDs | Stories are keyed on F-001..F-190, with `Epic ref` carrying the E-ID | The workflow's commands, branch names, plan filenames and sprint state all key on an ID with no dot in it. The E-ID is preserved so Appendix B's `Covered by` column still resolves. | Bootstrap |
| D-04 | §11, "Every pull request renders the corpus in CI" | CI runs no GPU build and no GPU test. The corpus renders locally, in `/verify`, and is required green before a push | Operator constraint, GPU CI minutes are expensive. See the risk below, this one is not free. | Bootstrap |
| D-05 | §11 and E2.1 imply a corpus the project holds | The corpus lives under ignored `corpus/data`, with a committed manifest of per-case checksums and metadata | Operator constraint. A TCIA-derived corpus is large and its redistribution terms are not ours to assume. The manifest makes the corpus verifiable without being present. | Bootstrap |
| D-06 | The .docx code listings carry Word paragraph formatting | `docs/hld/*.md` code fences are re-indented from bracket depth and de-double-spaced | Word stored indentation and line spacing as formatting rather than characters, so pandoc emits every listing flush-left and double-spaced. Presentation only. The .docx wins where exact bytes matter. | Bootstrap |
| D-07 | §7, "Two capability tiers, one codebase", both of them GPU | A third tier, **C, CPU**. The resolved tier may be `Cpu`, and every tier-gated feature declares its CPU answer | §7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing at all, which is a failure mode the specification does not name and does not intend. Operator decision, and spike A7.1 establishes GPU-less sessions as a primary clinical path rather than a fallback. F-X001 to F-X005. | Post-bootstrap |
| D-08 | §16, the marker spaces are `pub enum Canvas {}` and `Pt<S>` carries `#[derive(Debug, PartialEq)]` | The three marker enums derive `Debug, Clone, Copy, PartialEq, Eq, Hash`. The `Pt<S>` block is otherwise unchanged | A `derive` on a generic struct bounds the parameter, so `#[derive(Debug, PartialEq)]` expands to `impl<S: Debug> Debug for Pt<S>` and `Pt<Canvas>` satisfies neither trait while `Canvas` is bare. `assert_eq!` on two `Pt<Canvas>` fails to compile with E0369 and E0277, verified against rustc rather than reasoned about. §16's own note identifies exactly this trap for `Clone` and `Copy` and stops there. Deriving on the markers is the smaller change, because it leaves §16's `Pt` listing character for character as written. | F-001 |
| D-09 | §15.2, `glam = "0.30"` | `glam = { version = "0.30", default-features = false, features = ["libm"] }` | Every core crate carries `#![cfg_attr(not(test), no_std)]`, which the HLD neither requires nor forbids. glam's default feature is `std`, and glam needs either `std` or its optional `libm` dependency to compile at all, so the default entry silently defeats the `no_std` posture the crates declare. The pin itself is untouched and only the feature set changes. | F-001 |
| D-10 | §15.2 lists `wgpu = "=30.0.1"` among the workspace dependencies, and this repository's `Cargo.toml` comment said the entry activates with F-039 until the S03 review's fifth pass corrected it to F-037 | `wgpu` is activated in `ocelli-render` and `ocelli-compute` at F-008, two sprints earlier, and both crates drop `#![cfg_attr(not(test), no_std)]` | §38 makes "ocelli-compute crate exists" a Phase 1 hook whose alternative is "a device-sharing retrofit across the renderer", and §31 states the contract in wgpu terms: ocelli-compute never creates a `wgpu::Device`, it borrows the one ocelli-render owns. A contract expressed only in prose is not a mechanism, so the hook is types and compile errors or it is nothing. The pin itself is untouched, and wgpu needs `std`, which is why the two crates lose a `no_std` posture the HLD neither requires nor forbids. That comment's `F-039` was itself a misattribution, quoted here rather than endorsed, and the S03 review's fifth pass corrected the comment to F-037, E6.1 in S11, device init and capability tiering. F-039 is E6.3 in S13 and is OffscreenCanvas. | F-008 |
| D-11 | Appendix B, "Measured from cornerstone3D v5.8.9 source", and the parity target stated as v5.8.9 | The oracle pins `@cornerstonejs/core`, `@cornerstonejs/tools` and `@cornerstonejs/dicom-image-loader` at exactly `5.8.2` | **v5.8.9 does not exist.** Checked against the npm registry on 2026-09-04: `@cornerstonejs/core` has 1124 published versions, the highest is 5.8.2, and the same holds for the other two packages. 5.8.2 is the highest published 5.8.x and therefore the nearest installable reference. The Appendix B surface counts are left as the author measured them and are re-checked against 5.8.2 by `/parity`. | F-010 |
| D-12 | §15.3's CI invariant, `cargo tree -p <crate> -e normal \| grep -q wasm-bindgen` must find nothing for every crate but `ocelli-wasm` | That loop runs unchanged for the HOST. For wasm32 the rule is DIRECT DECLARATION in a crate's own manifest, not transitive reachability | **The HLD contradicts itself here and something has to give.** §15.2 specifies `wgpu`, §4 says `ocelli-render` builds for wasm, and §15.3 forbids reaching wasm-bindgen. On wasm32 all three cannot hold: wgpu reaches the browser's WebGPU through js-sys and web-sys, which are built on wasm-bindgen. Measured, the only route is `wgpu -> js-sys/web-sys -> wasm-bindgen`, and on the host that route does not exist, so the transcribed loop still means exactly what it says. D2's PURPOSE is untouched: CLAUDE.md says one bindgen crate "is what makes the desktop and server targets new entry points rather than rewrites", and wgpu abstracts the target itself, so `ocelli-render` carries no browser binding in its source and compiles for native unchanged. What D2 forbids is OUR code binding to the browser outside one crate, and direct declaration is our code's own choice where transitive reachability is a specified dependency's. | F-008 |
| D-13 | §18.3's fixture table gives `LINEAR_EXACT(-160) = 1.594` at centre 40, width 400, output range 0 to 255 | Fixtures compute LINEAR_EXACT from §18.2's transcribed formula, which yields `0.000` at that input | §18.2 clamps at `x <= c - w/2`, which is `40 - 200 = -160`, and `-160 <= -160` holds, so the value is `ymin`. The formula body evaluates to `((-160 - 40) / 400 + 0.5) * 255 = 0.000` there in any case, so no boundary convention produces 1.594, which is the value at `x = -157.5`. Rows 2, 3 and 4 of the same table reproduce exactly from the transcribed formulas and the 0.32 headline figure is unaffected. §18.2 is the formula and §18.3 is a worked value, and where they disagree the formula is the specification. Verified in exact rational arithmetic and derived independently twice in the S03 design round. The two lower boundaries coincide at -160, so no input can show LINEAR clamping while LINEAR_EXACT does not, and the asymmetry the row wants lives at the upper bound, where LINEAR clamps at 239 and LINEAR_EXACT at 240. A follow-up story reconciles the table. | F-011 |
| D-14 | §15.2, `wgpu = "=30.0.1"`, taking the crate's default features | `wgpu = { workspace = true, features = ["webgl"] }` in `ocelli-render` | wgpu 30.0.1's default feature set is `std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu`, read from the pinned crate's own manifest. `webgl` is a real feature at line 117 and is not among them, and `gles` is the native GL backend rather than the browser one. So on wasm32 the crate reaches WebGPU and cannot reach WebGL2, and HLD §7 declares tier B a first-class tier that therefore could not resolve in a browser at all. The pin itself is untouched and only the feature set changes, which is D-09's shape. Measured cost today is zero bytes, because `ocelli-wasm` does not depend on `ocelli-render` and so never reaches wgpu, whatever else it depends on. When this row was written that crate had an empty `[dependencies]` table, and F-005 has since added `ocelli-core` to it, which changes the premise and not the conclusion. The size budget moves only when the render path is wired in from S11. | F-004 |
| D-15 | §15.2, `thiserror = "2"`, taking the crate's default features | `thiserror = { version = "2", default-features = false }` at the workspace entry | §4's crate table puts the error model in `ocelli-core`, which carries `#![cfg_attr(not(test), no_std)]`, and §23's first clause is "thiserror in the core crates". thiserror's default feature is `std`, and `cargo tree -e normal,features` reports `thiserror feature "std"`, which is the exact string `scripts/no_std_check.py` searches for, so the default entry turns the `nostd` gate red. Setting `default-features = false` at the member is ignored by Cargo with a warning when the workspace entry does not set it, so the fix has to be at the workspace entry. Measured on thiserror 2.0.20, which compiles under `no_std` either way and emits `core::error::Error`. The pin is untouched and only the feature set changes. | F-005 |
| D-16 | §25.1, colour and ultrasound frames must show "perceptual difference below a stated threshold" | The comparator measures per-channel difference statistics for class-two views and returns `unmeasured`, never `pass` | §25.1 states no threshold and names no metric, so a `pass` would be a claim against a bound nobody wrote, which decision D14 forbids. The measurement is published and the bound is not claimed. Five of the ninety-eight rendered views are class two, which is the 89 stack frames plus the 9 volume reformats F-X007 added and was written here as eighty-nine until the S03 review's fourth pass, and `real/us_cmb_crc/00000001.dcm` is 8-bit greyscale, which §25.1 has no class for at all and which `docs/lld/corpus.md` absorbs into class two by modality. Metric and threshold are chosen together in a later story, because a metric without a threshold produces a number nobody can act on. | F-011 |
| D-17 | §15.3 gives the CI invariant as a self-contained script that needs the pinned toolchain and nothing installed. §11 says only that every pull request renders the corpus in CI and says nothing about what may be installed, so treating §15.3's environment as §11's is this row's inference and not the HLD's text | `scripts/ci_floor_check.py`, a FLOOR gate, imports PyYAML. It is pinned `pyyaml==6.0.3` in `pyproject.toml` and the `guards` job installs it, reading the version out of that file, before any gate runs | The floor's own claim is that `gate --floor` is what CI runs, and this check is what makes that claim true. It reads `.github/workflows/ci.yml`, which is YAML, and it read it line by line until the S03 review's twelfth pass measured FOUR fail-open routes and FIVE refusals of workflows GitHub Actions runs correctly. One of the four is key order, and a mapping has no key order, so no rule a line reader can carry closes it: the two spellings are one workflow and the check gave two answers, on `guards`, the gate that watches every other gate. The dependency-free alternative is to refuse every spelling the reader does not model, which closes three routes, leaves that one open, and makes three of the five false refusals permanent by design. A parser closes all four and accepts all five, because it yields one tree. This is the fourth foreign grammar in that file and the only one still read by hand: the shell arms and the `GATES` array stopped being read with a regex in the ninth and eleventh passes, and the declared-constant ratchet reads HLD 27.1's lint table with `tomllib`. `tomllib` is stdlib and PyYAML is not, and that is the whole of the cost. `BaseLoader` constructs `str`, `list` and `dict` and nothing else. An absent PyYAML is a refusal at import with the install command in it, never a fallback and never a skip, and probe `ci-floor.pyyaml-absent` holds that A THIRD OPTION IS REJECTED HERE RATHER THAN LEFT AS A PREMISE: `ci_floor_check.py` could leave the floor, which keeps the floor dependency-free and costs a developer's `gate --floor` its claim to prove CI equivalence, which is the one thing that check exists to assert. The S03 review's thirteenth pass asked for it to be weighed rather than assumed. | S03 review, pass 12 |

## D-08, and why a derive is not a formatting detail

§16's payoff is that a whole class of tool bugs stops compiling. The mechanism
is `PhantomData<S>` over an uninhabited marker, and the cost of that mechanism
is that every `derive` on `Pt<S>` bounds `S`. `PhantomData` itself implements
`Debug` and `PartialEq` for any `S`, bound-free, which is why the definition
compiles and only the call site fails. That gap is the whole trap: the crate
builds, and the first test that compares two points does not.

The two available fixes are not equivalent. Hand-implementing `Debug` and
`PartialEq` on `Pt<S>` keeps the markers bare, and its `PartialEq` body
compares `f64` fields directly, which the workspace's `float_cmp = "deny"`
lint then has an opinion about inside the one place an exact comparison is
correct. Deriving on the markers instead leaves §16's listing untouched and
keeps the float comparison inside a derive expansion, where it belongs.

**The side effect, which the F-001 review caught and which is easy to miss.**
Deriving `Clone` and `Copy` on the markers retires §16's own note as well.
That note says `derive(Clone, Copy)` on `Pt<S>` "would add an S: Clone bound
that the marker types do not satisfy", and after this deviation they satisfy
it. Confirmed against rustc 1.97.1: with the markers deriving,
`#[derive(Debug, PartialEq, Clone, Copy)]` on `Pt<S>` compiles and works for
all three spaces.

So §16's note is preserved in the source as the quotation it is, and the
hand-written impls stay, but **the reason they stay has changed and the source
says so.** It is no longer that a derive would not compile. It is that
`impl<S> Clone for Pt<S>` and `impl<S> Copy for Pt<S>` are unconditional, so a
`Pt` is `Copy` whatever a future marker does or does not derive. A marker added
later without `Copy` would silently make `Pt` of that space non-`Copy` under a
derive, and the hand-written impls are what stop that.

This is worth writing down because it is the shape a deviation most often goes
wrong in: not by being wrong, but by leaving the reasoning around it describing
the world before it was applied.

## D-07, tier C, and what it does and does not claim

HLD §7 says "Two capability tiers, one codebase", and both are GPU. §31 already
requires a CPU fallback for every tier-A **compute kernel**, so CPU is not
foreign to the design. What is missing is a CPU path for **rendering**, and
without one the resolved-tier logic has no answer for a machine that has
neither WebGPU nor WebGL2. The viewport does not degrade, it fails.

**What tier C claims.** A stack viewport renders, windows, scrolls and measures
on the CPU. That is the highest-volume and lowest-risk surface, it is the first
thing the migration replaces (§12), and it is arithmetic the project already
has to implement exactly once anyway.

**What makes it cheap.** §18 requires the LUT chain to live once in
`ocelli-pixel`, with the shader reading its parameters rather than
reimplementing it. A CPU path therefore reuses that same implementation and
simply does not use the shader. One arithmetic implementation, three
presentation paths, and the oracle can diff tier C against tier A to prove they
agree rather than assuming it.

**What it does not claim.** Interactive volume ray-casting on the CPU. F-X005
decides between a slow path and reporting the feature unavailable, against a
measurement. It must not decide the third thing, which is a CPU path that
quietly produces a different image from the GPU one. §31's rule generalises:
**a feature that cannot run on the resolved tier reports unavailable, and never
silently produces a different answer.**

**Why now rather than later.** This is the same argument the §38 hooks are made
on. Adding a tier to `Caps` before anything reads it costs a few weeks. Adding
one afterwards means changing every viewport, every tool and every feature that
ever asked "am I on A or B" and assumed those were the only answers.

**Spike A7.1 is answered and it raised the stakes.** Deployments are assumed to
span GPU-capable clients and GPU-less ones. The GPU-less class includes
virtualised desktops without GPU passthrough, builds where acceleration is
disabled by policy, and hosts whose driver is blocklisted, and it is exactly
where GPU access is least reliable. Tier C is therefore a rendering path a
substantial share of clinical users may sit on rather than a defensive
fallback. Two consequences follow, both in `docs/spikes/A7-tier-c.md`:

- **Tier resolution must tell a hardware adapter from a software one.** On a
  host with no GPU, a software rasteriser presents a conforming WebGL2
  context, so `Caps` as §7 specifies it resolves tier B and runs GPU paths on
  a rasteriser that is slower than our own CPU path and burns more CPU. On a
  shared host, CPU is the resource that decides how many sessions fit. Worse,
  it is invisible, and presents as "the viewer is slow" rather than as a
  misdetection.
- **The divergence bound has to cover tier A against tier C.** A mixed estate
  means two radiologists can open the same study and see pixels from different
  code paths. Decision D14 already commits to publishing a measured divergence
  bound rather than claiming bit-exactness, and that commitment now extends
  across tiers, not only across GPUs and targets.

**A7.1b answered: assume no GPU passthrough on the GPU-less class.** So
deployments resolve as `GPU client -> tier A` and `GPU-less session -> tier C`,
and **tier C carries a substantial share of clinical use.** Three further
consequences, all in `docs/spikes/A7-tier-c.md`:

- **CPU MPR is required, not optional.** A user on a GPU-less session has no
  other route to a reformat, so without it that whole class gets stack viewing
  and nothing else. Split into F-X004, and F-X005 keeps the volume-rendering
  decision separate because those are a commitment and a question.
- **wasm SIMD128 becomes a requirement** rather than a detection detail, and
  the no-SIMD runtime is measured separately as the worst case.
- **Decision D5 holds, for a second reason.** D5 keeps the build
  single-threaded and says to escalate only on a measurement that demands it.
  A CPU renderer carrying a large share of the load looks like that
  measurement and is not: on a shared host, spending more cores per session
  reduces sessions per host, which is that deployment model's whole
  economics. Tier C is judged on CPU spent, not on wall-clock alone.

**A side effect worth having.** F-X002 puts a software adapter behind the render
tests, so pipeline construction, bind-group layouts, shader compilation and the
tier-B variants become testable with no GPU at all. That widens what the
accepted D-04 arrangement can cover in CI, leaving the local oracle to catch
genuine GPU-behaviour differences rather than everything.

## The risk carried by D-04, stated plainly

HLD §11 makes CI-side corpus rendering the mechanism that "makes generated
Rust safe to merge at volume", and D7 in the decision log calls the oracle the
reason generation speed is an advantage rather than a liability. Moving that
gate off CI moves it onto a human remembering to run it.

Three things carry the load instead, and all three are mechanical:

1. **Outside the bootstrap exception below, `/verify` runs the oracle locally
   and `push` is refused without it.**
   `scripts/verify_ledger.py` records the corpus result against the exact head
   commit. A push whose head has no green corpus record for it is refused by
   `.githooks/pre-push`. A record for an ancestor commit does not count.
2. **CI asserts the ledger, without a GPU.** The CI floor re-reads the ledger
   entry for the pushed head and fails when it is missing, stale or red. This
   costs no GPU minutes and it cannot be satisfied by intention.
3. **A GPU corpus run is available on manual dispatch** for a release or when
   a divergence is suspected, so the expensive path exists and is simply not
   automatic.

**Bootstrap exception, and it is gone.** S01 built the corpus the oracle
consumes and contained no port code to validate, so S01's sprint profile
recorded the absent oracle as a named skip while F-010 remained pending. The
strict `--all` profile never carried the exception and release always required
the oracle. **F-010 landed in S02 and the exception no longer exists.** Its
implementation, `s01_pre_oracle` in `bin/ocelli.sh`, was REMOVED rather than
left standing with a condition that can no longer be true, because a dead
exception is a live misreading: the next person to see a skipped oracle gate
would have to work out that it cannot happen. `--sprint` and `--all` are now the
same set of gates. This paragraph is kept for the record and it describes S01
alone. It read in the present and the future tense until the S03 sprint review's
fourth pass, which made this register the last place in the repository where a
removed exception still read as live. The same pass added a second charge, that
the old text put the skip in S02's profile, and the fifth pass withdrew it: the
old sentence read "S01 contains no port code to validate. Its sprint profile
therefore records the absent oracle as a named skip while F-010 remains pending
in S02", where "Its" is S01's and only F-010's pendency was S02's. The tense
was the whole of the defect.

This is weaker than the HLD's design and it should be revisited if the project
ever has cheap GPU CI. It is recorded here rather than in a commit message
because the next person to ask "why is the corpus not in CI" deserves the
answer without archaeology.

## D-11, and what it does and does not change

The version in Appendix B is not decoration. Decision D7 makes the oracle the
thing that has to exist before port code, and §11 makes cornerstone3D the
reference the oracle measures against. So the pinned version is the definition
of correct for this project, and discovering that the stated one is not
installable is a fact about the specification rather than about the build.

**What is not claimed.** That 5.8.2 and v5.8.9 are the same software. Nobody
can check that, because one of them cannot be obtained. What is claimed is
narrower and checkable: 5.8.2 is the highest published version in the 5.8
series, so it is the closest thing to the stated reference that this project
can actually pin, install and hold still.

**What follows.** Appendix B's counts, twelve viewport types, roughly
sixty-three tool classes and the rest, were measured by the author against a
version this repository has never seen. They are left exactly as written,
because editing a measurement nobody re-took would be worse than carrying one
with a known provenance. `/parity` reports against the checklist, and the
first run that reads 5.8.2's source is the first time those counts are checked
rather than inherited.

**The most likely explanation is a transcription artefact**, and it does not
matter which way it went. The pin is on a version that exists, the reason is
here, and if the operator later identifies what v5.8.9 referred to, this row
is what makes the change a one-line correction instead of an investigation.

## D-12, and the part of it that is easy to get backwards

The tempting reading is that this weakens D2. It does not, and the distinction
is worth being precise about, because the next person to add a dependency will
have to make the same call.

**D2 is a rule about this repository's source, not about the transitive
closure.** Its payoff, stated in `CLAUDE.md`, is that the desktop and server
targets are new entry points rather than rewrites. That payoff is destroyed by
`ocelli-render` importing `wasm_bindgen` and writing browser-specific code,
because then a native build needs a second implementation. It is **not**
touched by wgpu using web-sys internally on one target, because wgpu is what
makes the target difference somebody else's problem. `ocelli-render`'s source
is identical for both targets, which is the whole thing D2 is protecting.

**The check that matters is therefore the one that was always there.** The
source grep for `wasm_bindgen` under `crates/`, exempting `ocelli-wasm`, is
unchanged and is the strongest of the three passes. Direct declaration in a
manifest is the second. Host-side transitive reachability is the third and is
still section 15.3's loop character for character.

**What this deviation gives up**, stated plainly: a crate that acquires
wasm-bindgen transitively on wasm32 through some route other than wgpu will no
longer fail the wasm32 pass. It would still have to name it in source or in a
manifest to be useful, and both of those still fail. The residual case is a
crate reaching wasm-bindgen through a dependency that re-exports it, which the
source grep catches at the point of use.
