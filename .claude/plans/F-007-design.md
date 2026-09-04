# F-007, Cross-target build proof, native desktop and server binary

**Status**: approved
**Epic ref**: E1.7
**Sprint**: S02
**Estimate**: 2w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation: a prose
semicolon in the source is written as a comma, and an em-dash as a hyphen,
because `scripts/prose_check.py` covers `.claude/plans/` and `docs/hld/` is
exempt. No word is changed. Where the exact bytes matter, the tracked
Markdown under `docs/hld/` wins._

### `docs/hld/03-architecture-and-crates.md`, section 4, the two rows that decide this story

> | ocelli-wasm | The only crate that may import wasm-bindgen. Boundary,
> commands, event ring. | yes | no |
>
> | ocelli-native | Desktop and server entry points. Phase 2 and 3, stubbed
> now. | no | yes |

Every other row in that table reads `yes | yes`. That is the whole content of
the shared-core promise: eleven crates must build for both targets, one is
wasm only and one is native only.

### `docs/hld/12-workspace-and-build.md`, section 15.1, the entry

> ocelli-native/ \# desktop + server entry (Phase 2/3)

### `docs/hld/10-extension-points.md`, section 13, verbatim

> - **A SeriesSource trait** abstracts where bytes come from. Phase 1
>   implements DICOMweb, and Phase 2 adds a DIMSE-backed implementation without
>   touching anything above it.
>
> - **A render-target trait** separating surface from offscreen texture, so
>   server-side rendering reuses ocelli-render unchanged.
>
> - **A dynamic codec registry**, so a native build can link C codecs the
>   browser build cannot.
>
> - **Calibrated display presentation** behind a trait the browser implements
>   as a no-op. DICOM Part 14 greyscale calibration is unreachable from a web
>   page, and is the strongest reason the desktop target exists.

Figure 3's caption, verbatim:

> The single-bindgen-crate rule turns Phases 2 and 3 into new entry points
> rather than new implementations. It costs nothing in Phase 1 and cannot be
> retrofitted cheaply.

### `docs/sprints/allocation.json`, the story note, verbatim

> Guards the shared-core promise before code accumulates

### `docs/sprints/CURRENT_SPRINT.md`, what done means and the defect class, verbatim

> - F-007 compiles the declared native desktop and server entry points using
>   the same core crates as wasm.

> The dangerous build defect is false portability. A workspace can compile on
> one target while Cargo feature unification enables `std`, browser-only
> bindings or a second GPU pathway on another. F-002 and F-007 must prove the
> actual target builds, not infer portability from a host build.

## What the specification does not cover

The HLD says `ocelli-native` holds desktop and server entry points and that it
is stubbed in Phase 1. It does not say:

1. **Whether the entry points are binaries or library functions.** The story
   title says "binary" and the HLD says "entry". This plan reads them together
   as two `[[bin]]` targets, because a library function named `main` proves
   nothing about linking and a binary does.
2. **What a stub does.** A stub that panics contradicts the workspace's
   `panic = "deny"` clippy lint. A stub that does nothing is not observably
   built.
3. **What "the same core crates" is asserted against.** A host build of
   `ocelli-native` says nothing about wasm32, and a wasm32 check of one crate
   says nothing about the other ten.
4. **That feature unification is the actual risk.** Cargo unifies features
   across a workspace build. The check that catches it has to build the two
   target sets separately, and today nothing does.

## Approach

**1. Two binaries in `ocelli-native`, both stubs, both real.**

`src/bin/ocelli-desktop.rs` and `src/bin/ocelli-server.rs`. Each prints its
own name, the crate version, and the four extension points of HLD section 13
that it will eventually implement, then exits zero. No `panic!`, no `unwrap`,
no `expect`, which the workspace lints deny anyway. Printing the extension
points is not decoration: it is what makes the stub's purpose readable from
its output, and it is the list the Phase 2 and Phase 3 stories consume.

The two binaries are separate rather than one binary with a subcommand,
because the HLD names two entry points and because the server one is the entry
that the render-target trait exists to serve. Collapsing them would make the
first Phase 3 story a split rather than a fill.

**2. `bin/ocelli.sh native` becomes the actual proof, in four steps.**

Today it runs `cargo build -p ocelli-native`, which is a host build of one
crate. It becomes:

- `cargo build -p ocelli-native --bins`, host. The desktop and server entry
  points link.
- `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`.
  Every crate the section 4 table marks wasm `yes` builds for wasm32.
- `cargo check --workspace --exclude ocelli-wasm`, host. Every crate the table
  marks native `yes` builds natively.
- `cargo tree -p ocelli-wasm --target aarch64-apple-darwin` must not reach
  `ocelli-native`, and `cargo tree -p ocelli-native --target
  wasm32-unknown-unknown` must not resolve at all. The two `no` cells in the
  table are asserted rather than assumed.

Each step runs as its own command with its own exit code read directly, per
the `AGENTS.md` rule about reading a status from a pipe.

**3. A `native` gate, in the floor.**

`bin/ocelli.sh gate --list` gains `native|no|the cross-target build proof
(E1.7, HLD section 4)`. It needs no GPU and no corpus, so it is in `--floor`
and therefore in CI. The CI workflow's two existing cross-target steps are
replaced by the one gate, so there is one definition of the proof rather than
a shell script and a workflow that can drift apart.

**4. Feature unification, caught rather than hoped for.**

`scripts/no_std_check.py` already asserts that a `no_std` crate reaches no
dependency's `std` feature, which is deviation D-09's guard. This story adds
the complementary assertion the defect class names: the resolved feature set
for each shared crate is captured under both targets with `cargo tree
--format "{p} {f}"` and the two are compared. A feature enabled under one
target and not the other is reported with the crate and the feature named. It
is not automatically a failure, because `getrandom` and friends legitimately
differ by target, so the comparison is against a small recorded allowlist in
`ci/target-feature-baseline.json` with a reason per entry, and an unexplained
difference fails.

**5. Prove the gate goes red.**

Three mutations, all reverted, all recorded in the completion note: add a
`std`-only dependency to `ocelli-core` and watch the wasm32 step fail, make
`ocelli-wasm` depend on `ocelli-native` and watch the tree assertion fail, and
enable a feature on one target only and watch the baseline comparison fail.

## Boundary and tier

- wasm-bindgen: not touched. F-002 declares it in `ocelli-wasm`, and this
  story asserts no other crate reaches it under either target.
- Pixels across the boundary: no.
- Render-loop allocation: none. There is no render loop.
- unsafe: none.
- Tier A (WebGPU): n/a. A build proof resolves no tier.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): n/a, same reason. Note that tier C is a rendering tier and not
  a build target, and the two are easy to confuse here. `ocelli-native` is a
  target. Tier C is what a browser session resolves to when it has no GPU.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | Each binary reports its own name and the crate version | `crates/ocelli-native/src/bin/*.rs` under `#[cfg(test)]` |
| `conformance` | Eleven shared crates build for wasm32 and for the host, `ocelli-wasm` is unreachable from native and `ocelli-native` from wasm32 | `bin/ocelli.sh native`, gate `native` |
| `conformance` | Resolved features agree across targets except for the recorded, reasoned baseline | `bin/ocelli.sh native`, gate `native` |

No `fixture` row. This story computes no pixel and no coordinate, so HLD 27.2
R3 does not apply. Named rather than omitted.

## Parity surface covered

None. Appendix B is a feature surface and this is a build proof.

## Deviations

None.

## LLD impact

`docs/lld/build-targets.md`, created by F-002 for the wasm half, extended here
with the native and cross-target halves: the two binaries, the four assertions
of the proof, and the feature-unification baseline with the reason for each
allowed difference.

## Decisions taken in the design round

1. **The feature-unification baseline is built**, vacuous start and all. Its
   value is entirely in the moment a dependency is added and nobody is
   looking, which is precisely when nobody would build it. It is proved red by
   mutation as step 5 says, so it is a guard that has been observed working
   rather than one that has only ever been observed passing.
2. **The assertion is that `cargo tree -p ocelli-native --target
   wasm32-unknown-unknown` does not resolve**, not merely that the crate has
   no wasm32 dependencies. Section 4's table says `ocelli-native` is native
   only, and "it happens to have no wasm dependencies today" is a weaker
   claim that would stay green through the change that breaks it.

## Open questions

None. Both were resolved above.
