# F-002, wasm-pack build pipeline with a hard size budget gate

**Status**: approved
**Epic ref**: E1.2
**Sprint**: S02
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/12-workspace-and-build.md`, section 15.1, the layout entry

> ocelli-wasm/ \# \*\* the only wasm-bindgen crate \*\*

### `docs/hld/12-workspace-and-build.md`, section 15.2, `[profile.release]`, verbatim

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

This block is already in the workspace `Cargo.toml` verbatim and is not
changed by this story. It is transcribed because it is the reason the size
measurement is release-only. Every one of the five settings moves the number
the budget gate reads, so a dev-profile measurement is not a smaller version
of the same figure, it is a different figure.

### `docs/hld/12-workspace-and-build.md`, section 15.3, the CI invariant, verbatim

> Decision D2 is worthless unless it is enforced. This runs on every pull
> request.

```bash
#!/usr/bin/env bash
# ci/check-bindgen-isolation.sh
set -euo pipefail
fail=0
for c in crates/*/; do
name=$(basename "$c")
[ "$name" = "ocelli-wasm" ] && continue
if cargo tree -p "$name" -e normal 2>/dev/null | grep -q 'wasm-bindgen'; then
echo "FAIL: $name reaches wasm-bindgen"; fail=1
fi
done
exit $fail
```

### `docs/hld/A-spike-gates.md`, Appendix A gate A4, verbatim

> | A4 | Do binary size and cold start land within budget? | Estimated 3-8 MB
> uncompressed before tuning, Naga dominating. Unmeasured today |

### `docs/sprints/allocation.json`, the story note, verbatim

> CI fails if the module exceeds the agreed budget

### `docs/sprints/CURRENT_SPRINT.md`, what done means, verbatim

> - F-002 produces the release wasm package through `wasm-pack` and enforces
>   the HLD size budget against that release output.

and the defect class, verbatim:

> The dangerous build defect is false portability. A workspace can compile on
> one target while Cargo feature unification enables `std`, browser-only
> bindings or a second GPU pathway on another. F-002 and F-007 must prove the
> actual target builds, not infer portability from a host build. The size
> budget is measured only from a release wasm artefact.

## What the specification does not cover

The HLD gives the release profile, the isolation invariant and the budget
question. It does not say any of the following, and this plan decides each.

1. **What `ocelli-wasm` exports before F-096 builds the boundary.** wasm-pack
   refuses to build a crate that does not depend on `wasm-bindgen`, so the
   pipeline cannot exist at all until the dependency is declared. The current
   `wasm` gate skips for exactly this reason and names F-096 as the story that
   lands it. `CURRENT_SPRINT.md` assigns the removal of that skip to F-002,
   which is the later authority and which this plan follows.
2. **Which `wasm-bindgen` version.** The HLD's section 15.2 dependency table
   does not list it.
3. **Where the first baseline number comes from.** `scripts/pin_and_size_check.py`
   already answers this: the first run records the observed size and passes,
   and subsequent runs fail beyond a 5% tolerance. That mechanism is F-001's
   and is not redesigned here.
4. **That the isolation check is target-blind.** `cargo tree` filters to the
   host platform by default, so a `wasm-bindgen` dependency added to another
   crate under `[target.'cfg(target_arch = "wasm32")'.dependencies]` would not
   appear in the check the HLD transcribes. Before this story that gap was
   theoretical because nothing was built for wasm32. F-002 is the story that
   makes wasm32 a real build target, so it is the story that closes it.

## Approach

**1. Declare the dependency, target-gated, in `ocelli-wasm` only.**
`wasm-bindgen` goes into the existing
`[target.'cfg(target_arch = "wasm32")'.dependencies]` block, which F-001
created empty with a comment deferring to F-096. The comment is replaced with
one saying what is actually true: the dependency is here because the build
pipeline is a story of its own, and the boundary that uses it is F-096.

**2. Give the module a real export, and only one.**
A wasm module with no export measures nothing, because a linker with fat LTO
and `strip = true` can discard an unreferenced world. The export is
`ocelli_version()`, returning the crate version string. It is honest, it is
not a placeholder pretending to be the boundary, and it is a value the
TypeScript shell will genuinely want. `packages/core/src/index.ts` already
declares a `VERSION` constant and a `coreAvailable()` function that returns
`false` because no core has ever been built, which is the seam this export
eventually meets in F-003 and F-096.

**3. Take the skip out of the gate.**
The `wasm)` arm of `run_gate` in `bin/ocelli.sh` currently greps
`crates/ocelli-wasm/Cargo.toml` for `wasm-bindgen` and skips when it is
absent. The grep goes. The same skip text exists in `.github/workflows/ci.yml`
and is removed there too, so the CI floor runs the build rather than reporting
a skipped gate as a green tick.

**4. Close the target-blindness in `ci/check-bindgen-isolation.sh`.**
The loop runs a second time with `--target wasm32-unknown-unknown`. The HLD's
listing is preserved as the first pass, unaltered, and the second pass is
added beneath it with a comment saying what it catches that the first does
not. This is an addition to a transcribed listing rather than an edit of one,
which is the smaller change.

**5. Record the baseline.**
`bin/ocelli.sh wasm` then `python3 scripts/pin_and_size_check.py --with-size`
writes `ci/wasm-size-budget.json` on the first run. The recorded number is
reported in the completion note against gate A4's 3 to 8 MB estimate, because
A4 is unanswered and this is the first measurement the project has ever had.
A4 is not closed by this story: A4 asks about the shipped module including
Naga, and this module contains neither wgpu nor Naga.

**6. Prove the gate goes red.**
Per HLD 27.3, a new guard is observed failing before it is claimed. Two
mutations, both reverted: lower the recorded baseline so the real module
exceeds the ceiling, and add `wasm-bindgen` to a second crate under a wasm32
target gate so the strengthened isolation check fails. Both are recorded in
the completion note.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only, target-gated to wasm32, and the isolation
  check is strengthened in this story to see the target gate.
- Pixels across the boundary: no. Nothing crosses it yet.
- Render-loop allocation: none. There is no render loop.
- unsafe: none. `crates/ocelli-wasm/src/ring.rs` is not created by this story.
- Tier A (WebGPU): n/a. This is a build pipeline and resolves no tier.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): n/a, same reason.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The exported version string equals `CARGO_PKG_VERSION` | `crates/ocelli-wasm/src/lib.rs` under `#[cfg(test)]` |
| `browser` | `wasm-pack build --target web` produces a loadable module whose export returns that same string | `bin/ocelli.sh wasm`, asserted by the gate |

No `fixture` row. This story computes no pixel and no coordinate, so HLD 27.2
R3 does not apply. The row is named rather than omitted, because an omitted
row and a deliberate "no arithmetic here" read identically later.

The size gate is not a test in the taxonomy's sense. It is a recorded
measurement with a tolerance, and its red path is observed by mutation rather
than asserted by a case.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` enumerates viewport types, tools, blend
modes, VOI functions, transfer syntaxes, segmentation representations, events
and adapters. A build pipeline is on none of those axes.

## Deviations

None. The one thing that looks like a deviation is not: `bin/ocelli.sh`'s
skip message names F-096 as the story that lands `wasm-bindgen`, and this
story lands it instead. That is a stale comment in a repository file, not a
departure from `docs/hld/`, and the comment is corrected here.

## LLD impact

A new `docs/lld/build-targets.md`, covering the wasm build pipeline, the size
budget mechanism and the isolation invariant. F-007 extends the same file with
the native and cross-target halves, which is why it is one file and not two.

## Decisions taken in the design round

1. **`wasm-bindgen` is pinned exactly**, at `=0.2.127`, in
   `[workspace.dependencies]`, and `wasm-bindgen` joins `wgpu` in
   `EXACT_PINNED` in `scripts/pin_and_size_check.py`. The reason is stronger
   here than for wgpu: `wasm-pack` runs a CLI whose version must match the
   crate's, so a floating version produces a version-mismatch failure that
   reads as a build break rather than as a resolution change. 0.2.127 is the
   highest published version, checked on 2026-09-04.
2. **The export is `ocelli_version()`**, returning the crate version, for the
   reason in step 2 of the approach. A `#[wasm_bindgen(start)]` with no
   exported function would measure a module the linker is free to shrink, and
   would give the shell nothing.

## Open questions

None. Both were resolved above.
