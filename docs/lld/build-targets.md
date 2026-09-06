# Build targets

**F-IDs that contributed:** F-002, F-004, F-005, F-007, F-008
**Last updated:** 2026-09-05

The wasm build pipeline, the size budget, and the invariants that keep the
core target-agnostic. This describes what the code does today.

## The two targets, and the two crates that are not shared

HLD section 4's crate table has a `wasm` and a `native` column. Eleven crates
read `yes` in both. Two do not, and they are the whole content of the
shared-core promise.

| Crate | wasm | native |
|-------|------|--------|
| `ocelli-wasm` | yes | no |
| `ocelli-native` | no | yes |
| everything else | yes | yes |

### The two `no` cells are not symmetric, and the asymmetry is deliberate

**`ocelli-native` is `wasm: no` and that is enforced.** Its `lib.rs` carries a
`#[cfg(target_arch = "wasm32")] compile_error!` naming section 4 as the source.
Before F-007 the cell was not enforceable at all: `cargo check -p ocelli-native
--target wasm32-unknown-unknown` **succeeded**, because the crate is a stub
with no native-only dependency and nothing declared it native-only. A guard
asserting "it does not build for wasm32" would have been asserting something
untrue. F-007 made it true instead of asserting it.

**`ocelli-wasm` is `native: no` and that is NOT enforced, on purpose.** The
crate compiles natively today, because `wasm-bindgen` is target-gated, and
`cargo test -p ocelli-wasm` depends on that. The cell means the crate is not
*shipped* natively. Its native compilation is what lets the boundary's logic be
unit-tested without a browser, which the project needs, so a `compile_error`
here would cost real coverage to enforce a claim the table is not making.

## The wasm pipeline

```bash
bin/ocelli.sh wasm            # wasm-pack build, release
bin/ocelli.sh gate wasm       # the same build, then the size check
```

`bin/ocelli.sh wasm` runs `wasm-pack build crates/ocelli-wasm --target web
--out-dir pkg`. Release is the default and is not a convenience. HLD section
15.2's profile is `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`,
`panic = "abort"` and `strip = true`, all of which apply to release only, so a
dev-profile size measurement is not a smaller version of the same number. It
is a different number.

`crates/ocelli-wasm/pkg/` is generated and gitignored.

### wasm-opt runs, and is not disabled

`crates/ocelli-wasm/Cargo.toml` carries
`[package.metadata.wasm-pack.profile.release]` with an explicit `wasm-opt`
flag list. The flags are needed because rustc for `wasm32-unknown-unknown`
enables several WebAssembly proposals by default and the `wasm-opt` that
wasm-pack downloads validates without them. A stock build fails with
`Bulk memory operations require bulk memory [--enable-bulk-memory]`, measured
on rustc 1.97.1 and wasm-pack 0.15.0.

**The other documented fix is `wasm-opt = false` and it is the wrong one
here.** Turning the optimiser off produces a green build and a larger
artefact, so the number the budget records stops describing what would ship.
That is the same defect class as widening a tolerance to make a test pass.

### The exported surface

Four functions, listed in the table under "The exported surface, and the panic
probe" below. `ocelli_version()` returns the workspace version and the other
three are the panic record of HLD section 23. The boundary itself arrives with
F-101 (E16.2).

It exists for measurement rather than for features. Fat LTO with `strip =
true` lets the linker discard anything unreachable, so a module with no
exported root would measure the size of nothing. It also meets a real seam:
`packages/core/src/index.ts` carries a `VERSION` constant and a
`coreAvailable()` that returns `false` because no core has ever been built,
and this is the value those eventually agree with.

Its test asserts the literal `"0.1.0"` rather than `env!("CARGO_PKG_VERSION")`.
The obvious form restates the function body, so it passes whatever the body
returns, which is HLD 27.2 R2's failure exactly. `/release` updates the
literal with the version bump.

## The cross-target proof

```bash
bin/ocelli.sh native           # the proof
bin/ocelli.sh gate native      # the same thing, as a gate, in the floor
```

Four steps, each exit code read from the command itself.

| Step | What it proves |
|------|----------------|
| 1 | `cargo build -p ocelli-native --bins`. The two entry points **link**, not merely type-check |
| 2 | `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`. Every crate the table marks `wasm: yes` builds for wasm32 |
| 3 | `cargo check --workspace --all-targets`. Every crate the table marks `native: yes` builds natively, tests included |
| 4 | `scripts/target_feature_check.py`. Resolved features agree across the two targets, or the difference is declared |

**Step 2 deliberately omits `--all-targets` and step 3 keeps it.** For wasm32
that flag pulls in dev-dependencies, and `proptest` reaches `wait-timeout`,
which does not compile for wasm32 and is not meant to. What ships to a browser
is the lib. Running the test suite under wasm32 needs `wasm-bindgen-test` and a
browser runner, which is the oracle's and F-101's ground. A native build does
run its tests, so step 3 has to compile them.

### The two entry points

`crates/ocelli-native/src/bin/ocelli-desktop.rs` and `ocelli-server.rs`. Both
print `entry_point_banner()`, which names the binary and the four extension
points of HLD section 13 that Phase 2 and Phase 3 will fill, and then, since
F-004, the tier they resolved to and the evidence for it.

They are two binaries rather than one with a subcommand because section 13
names two entry points, and because the server one is what the render-target
trait exists to serve. Collapsing them would make the first Phase 3 story a
split rather than a fill.

### Step 4, and why a build proof is not enough on its own

A build proof catches a target that stops compiling. It does not catch **both
targets compiling while one quietly resolved a different feature set**, which is
the sprint's stated false-portability defect and the more dangerous half,
because nothing goes red and the difference is in the artefact rather than in
the log.

`scripts/target_feature_check.py` makes **one** claim: every dependency this
workspace declares directly, meaning the entries in
`[workspace.dependencies]`, resolves the same features on both targets. A
difference there is our decision and we have to say so, in
`ci/target-feature-baseline.json`.

**It was broader in F-007 and F-008 narrowed it. That was a correction, not a
retreat.** The first version compared the whole transitive closure, which was
easy to believe when the only dependency was `glam`. Activating wgpu produced
42 findings, 32 of them packages present on one target only, and every one of
them legitimate. Worse, **most were specific to the machine**: `objc2-metal`
and `raw-window-metal` are macOS host-only, where a Linux runner reports `ash`
and `gpu-alloc`. A baseline listing them would have been correct on one laptop
and red in CI, and the fix for a red CI would have been to re-declare it. That
is tolerance-tuning wearing a different hat, and it is the exact thing
`docs/hld/22-testing-and-tolerance.md` section 25.1 says destroys a suite.

The transitive closure of a cross-platform GPU library differs per target by
design. That is wgpu doing its job, and asserting otherwise measures the
dependency rather than this project.

**A parsing bug went with it.** `cargo tree` marks an already-printed subtree
with a trailing ` (*)`, which lands after the `--format` string and so arrives
inside the feature field, turning `default` into `default (*)`. It produced
four phantom differences between a package and itself. Stripped before the
split.

**Step 4 does not assert section 4's crate table**, and an earlier draft tried
to. `cargo tree` lists every workspace member whatever target it is given,
because nothing in a manifest restricts a member to a target, so the assertion
reported `ocelli-native` present under wasm32 and could not tell that from a
real violation. The table is enforced where it can be: the `compile_error!` in
`ocelli-native`, and steps 1 to 3 building each target for real.

## The size budget

`ci/wasm-size-budget.json` holds a recorded measurement and a 5% tolerance.
`scripts/pin_and_size_check.py --with-size` compares the built module against
it. The first run records and passes, because a budget invented before the
first measurement would be either meaningless or immediately wrong.

**First measurement: 14,104 bytes**, on 2026-09-04.

**Current baseline: 16,388 bytes**, re-baselined by F-005 on 2026-09-05. The
delta is 2,284 bytes, which is 16.2 per cent and therefore over the tolerance,
and it was declared rather than discovered. `ci/wasm-size-budget.json` carries
the attribution and the two measurements it rests on:

- Base commit `d74ad3a` rebuilt on the same toolchain reproduces 14,104 bytes
  exactly, so the delta is F-005's and not drift.
- Raising the panic record's `MESSAGE_CAPACITY` from 512 to 1,536 bytes leaves
  the module **byte-identical** at 16,388, because a zeroed static needs no
  data segment. So the 528-byte record costs the module nothing, and
  `ocelli-core` becoming a dependency of `ocelli-wasm` for the first time costs
  nothing measurable either, since the only thing reached is one enum
  discriminant and fat LTO drops the rest.

**What the delta actually is** is the panic hook's code:
`std::panic::set_hook`, the boxed closure it takes, the `core::fmt` machinery
that formats the panic location, and the `Any` downcast that reads the payload.
It has no bearing on gate A4, whose estimate is three orders of magnitude
larger and dominated by Naga.

**That number is not an answer to Appendix A gate A4 and must not be read as
one.** A4 asks whether binary size and cold start land within budget and
estimates 3 to 8 MB uncompressed with Naga dominating. This module contains
four exported functions, no wgpu and no Naga. A4 stays open.

### The other half of A4, cold start

A4 names two figures and the paragraphs above are only the first. **Cold start
was never measured at all until F-006**, which recorded **2.3 ms** on
2026-09-05 for the release artefact: fetch, `WebAssembly.compile`, instantiate
and the first `ocelli_version()` call, timed in headless Chromium from the page
rather than from inside the module, so `crates/` gained not one line. The figure
and everything about how it was taken are in `ci/bench-baseline.json`, and the
design is `docs/lld/benchmarks.md`.

**The same caveat applies to it, unchanged, and it is the reason both halves
live in one place rather than in two documents.** A module holding four exported
functions, no wgpu and no Naga tells you nothing about the cold start of a
feature-complete module, exactly as its 16 KB tells you nothing about A4's 3 to
8 MB. Separating the two numbers is how one of them eventually gets read as
answering the gate. **A4 stays open**, and 2.3 ms is a regression baseline for
the build-out phase.

**Re-baselining is expected, repeatedly, for the whole build-out phase.** A 5%
tolerance on a 14 KB module is blown by the first story that adds anything
real. That is the mechanism working rather than failing: during buildup the
gate does not mean "you exceeded a budget", it means **"the module changed
size and the change was not declared"**. `--accept-size` is the declaration,
and the design plan that used it says why. The gate only starts meaning the
other thing once the module is feature-complete.

## The exported surface, and the panic probe

The module exports four functions of ours, up from one. wasm-bindgen
adds its own, and `memory`, which are the glue's and not this table's.

| Export | Called |
|--------|--------|
| `ocelli_version()` | Any time. F-002's measurement root |
| `install_panic_hook()` | Once, by a worker, before any other call |
| `panic_record_ptr()` | Once, straight after instantiation. Cached |
| `panic_record_len()` | Once, straight after instantiation. Cached |

The last three are F-005's, and `docs/lld/errors.md` is where they are
specified. The property that matters for this file is that **none of them is
called after a trap**, so the panic path adds no post-trap requirement on the
module.

**`wasm32-unknown-unknown` is `panic = "abort"` in every profile**, not only in
the release profile HLD section 15.2 sets it in, because it is the target's own
default. `rustc --print cfg --target wasm32-unknown-unknown` reports
`panic="abort"` with no profile involved. That is worth stating here because
this file describes the build, and the obvious reading of section 15.2 is that
a dev wasm build unwinds. It does not.

`bin/ocelli.sh gate panic` builds a **second** module, carrying the
`panic-probe` cargo feature, into `crates/ocelli-wasm/target/panic-probe`,
which is gitignored. That feature adds one export that panics on purpose, and
the shipped artefact does not carry it, so the module `bin/ocelli.sh wasm`
measures has no way to be asked to panic. `AGENTS.md` forbids a feature flag
without a named user, and the named user is `scripts/panic_probe.mjs`.

## The isolation invariant

`ci/check-bindgen-isolation.sh` enforces decision D2, wasm-bindgen in exactly
one crate. It has three parts.

1. HLD section 15.3's loop verbatim, over the **host** target.
2. The same loop over **wasm32-unknown-unknown**, added by F-002.
3. A source grep for `wasm_bindgen`, which catches a dev-dependency or a
   re-export that `cargo tree -e normal` does not see.

**Part 2 is not redundant and the reason is worth keeping.** `cargo tree`
filters to the host platform when no `--target` is given, so part 1 cannot see
a dependency declared under `[target.'cfg(target_arch = "wasm32")'.dependencies]`.
That is precisely the form `ocelli-wasm` itself uses, so it is the form a
second crate would most plausibly copy. Measured while adding it: with
`wasm-bindgen` added to `ocelli-core` under a wasm32 target gate, part 1
reports zero hits and would have passed, and part 2 fails.

**The target does not need to be installed**, because `cargo tree` resolves
cfg rather than compiling. That was verified against an uninstalled triple
rather than assumed, and it matters: the CI `guards` job installs no extra
target, so requiring one would fail that job for no reason.

The triple is still checked against `rustc --print target-list`, and that
check is not decoration. `cargo tree --target <typo>` errors, the error goes
to `/dev/null`, and `grep -q` then finds nothing in an empty stream, so a
misspelled triple would report a clean pass over zero crates. "The check could
not run" and "the check ran and was happy" must not look the same.

## The exact pins

`scripts/pin_and_size_check.py` refuses a range form for:

| Crate | Why a range is refused |
|-------|------------------------|
| `wgpu` | HLD section 15.2. Agents reliably emit wgpu 0.19-era pipeline code, and a range lets that compile against something subtly different from what the shader expects |
| `wasm-bindgen` | `wasm-pack` runs a CLI whose version must match the crate version. A range lets the two drift, and the mismatch reads as a build break rather than as a resolution change |

### `ocelli-render` adds one wgpu feature, and it is deviation D-14

`wgpu = { workspace = true, features = ["webgl"] }`. The pin and the workspace
entry are untouched and only the consuming crate's feature set changes, which
is the shape D-09 set for glam.

wgpu 30.0.1's default set is `std, parking_lot, dx12, metal, gles, vulkan,
wgsl, webgpu`, read from the pinned crate's own manifest. `webgl` is a real
feature and is not among them, and `gles` is the **native** GL backend rather
than the browser one. Without the feature the crate reaches WebGPU on wasm32
and cannot reach WebGL2 at all, so HLD section 7's tier B could never resolve
in a browser, and a tier the resolver can never return is not a tier.

**Step 4 is unaffected and that was checked rather than assumed.** The feature
is enabled unconditionally, so both targets resolve it, and the packages it
pulls in on wasm32 are already target-gated inside wgpu's own manifest.
`ci/check-bindgen-isolation.sh` part 1 is unaffected for the same reason:
`wasm-bindgen`, `js-sys` and `web-sys` sit under wgpu's
`cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))` tables, so
the host tree does not reach them.

**Measured cost today is zero bytes**, because `ocelli-wasm` does not depend on
`ocelli-render` and so never reaches wgpu, whatever else it depends on. It had
an empty `[dependencies]` table when this was written and F-005 has since added
`ocelli-core`, which changes the premise and not the conclusion. The size budget
moves only when the render path is wired in from S11.

### `ocelli-native` now depends on `ocelli-render`

F-004 gives the two entry points a real job: read `OCELLI_TIER`, resolve a
tier, and print the evidence. That is the operator override's named user, and
`AGENTS.md` forbids a flag without one.

The cost is that **step 1 of the cross-target proof starts linking wgpu**,
which is a real build-time increase to a gate in the floor. It is accepted
because the alternative is shipping an override nobody can observe. See
[tier-resolution.md](tier-resolution.md).

## Known gaps

- **wasm-pack warns that `crates/ocelli-wasm/` carries no LICENSE file.** The
  licences are at the repository root. The generated `pkg/` is not published
  by anything today, and whether it is published at all is F-101's and
  `/release`'s question, not this one.
- **Step 4 starts vacuous.** It compares the dependencies this workspace
  declares directly, not the workspace's own crates, and today not one of them
  resolves a different feature set on the two targets, so `allowed` in
  `ci/target-feature-baseline.json` is empty. That file's
  `checked_dependencies` is the list being watched and
  `python3 -c "import json;print(len(json.load(open('ci/target-feature-baseline.json'))['checked_dependencies']))"`
  prints how long it is. It was built anyway, and proved red by
  construction, because its value is entirely in the moment a dependency is
  added and nobody is looking, which is precisely the moment nobody would
  build it.
