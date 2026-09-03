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
| D-05 | §11 and E2.1 imply a corpus the project holds | The corpus lives outside git at `$OCELLI_CORPUS_DIR`, with a committed manifest of per-case checksums and metadata | Operator constraint. A TCIA-derived corpus is large and its redistribution terms are not ours to assume. The manifest makes the corpus verifiable without being present. | Bootstrap |
| D-06 | The .docx code listings carry Word paragraph formatting | `docs/hld/*.md` code fences are re-indented from bracket depth and de-double-spaced | Word stored indentation and line spacing as formatting rather than characters, so pandoc emits every listing flush-left and double-spaced. Presentation only. The .docx wins where exact bytes matter. | Bootstrap |
| D-07 | §7, "Two capability tiers, one codebase", both of them GPU | A third tier, **C, CPU**. The resolved tier may be `Cpu`, and every tier-gated feature declares its CPU answer | §7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing at all, which is a failure mode the specification does not name and does not intend. Operator decision, and spike A7.1 establishes GPU-less sessions as a primary clinical path rather than a fallback. F-X001 to F-X004. | Post-bootstrap |
| D-08 | §16, the marker spaces are `pub enum Canvas {}` and `Pt<S>` carries `#[derive(Debug, PartialEq)]` | The three marker enums derive `Debug, Clone, Copy, PartialEq, Eq, Hash`. The `Pt<S>` block is otherwise unchanged | A `derive` on a generic struct bounds the parameter, so `#[derive(Debug, PartialEq)]` expands to `impl<S: Debug> Debug for Pt<S>` and `Pt<Canvas>` satisfies neither trait while `Canvas` is bare. `assert_eq!` on two `Pt<Canvas>` fails to compile with E0369 and E0277, verified against rustc rather than reasoned about. §16's own note identifies exactly this trap for `Clone` and `Copy` and stops there. Deriving on the markers is the smaller change, because it leaves §16's `Pt` listing character for character as written. | F-001 |
| D-09 | §15.2, `glam = "0.30"` | `glam = { version = "0.30", default-features = false, features = ["libm"] }` | Every core crate carries `#![cfg_attr(not(test), no_std)]`, which the HLD neither requires nor forbids. glam's default feature is `std`, and glam needs either `std` or its optional `libm` dependency to compile at all, so the default entry silently defeats the `no_std` posture the crates declare. The pin itself is untouched and only the feature set changes. | F-001 |

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

**What it does not claim.** Interactive volume ray-casting on the CPU. F-X004
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

1. **`/verify` runs the oracle locally and `push` is refused without it.**
   `scripts/verify_ledger.py` records the corpus result against the exact head
   commit. A push whose head has no green corpus record for it is refused by
   `.githooks/pre-push`. A record for an ancestor commit does not count.
2. **CI asserts the ledger, without a GPU.** The CI floor re-reads the ledger
   entry for the pushed head and fails when it is missing, stale or red. This
   costs no GPU minutes and it cannot be satisfied by intention.
3. **A GPU corpus run is available on manual dispatch** for a release or when
   a divergence is suspected, so the expensive path exists and is simply not
   automatic.

This is weaker than the HLD's design and it should be revisited if the project
ever has cheap GPU CI. It is recorded here rather than in a commit message
because the next person to ask "why is the corpus not in CI" deserves the
answer without archaeology.
