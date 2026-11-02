Result: 0 defects, 0 smells, 0 nitpicks

# S25 sprint review, pass 1

**Reviewed**: `sprint/s25` at `9c772372b007904b0feec8cef6254658d3387498` (`F-070, hit-testing and measurement`). Parent `9e0e53d447ddd363b2011de136bc4e1da46b31df`. Sprint open `9d7f46ed01df6d86a87e3e0df99e5570b892281b`. Design approval `06768e19630e6336bc7824ff16eb81168ffb7db7`. Base `main` is `71ac28d516cc2c7a9ed9db337f1528e47cc397e6`. Annotated tag `s24` peels to that merge. Working tree clean. No product file was edited.
**Author / reviewer**: The story commits carry `Ocelli-Generated-By: grok`. A generated trailer does not establish the author. This reviewer did not write F-068 or F-070 and did not resume the earlier F-068 or F-070 passes. The assignment forbade staging, so this pass was not recorded with `scripts/sprint_workflow.py`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The ledger sentences named for this pass were checked on this HEAD.

M6 is F-063, F-064, F-065, F-066 and F-067. The estimates are 4, 4, 3, 3 and 2 weeks, which is 16. All five backlog rows are `done`. Tag `s24` peels to `71ac28d516cc2c7a9ed9db337f1528e47cc397e6`, subject `Merge sprint/s24, M6 is complete`. `main` is that commit. The root `Cargo.toml`, `packages/core/package.json` and `packages/react/package.json` are still `0.1.0`. The range `71ac28d..HEAD` does not change those files or `ci/wasm-size-budget.json`. `scripts/pin_and_size_check.py` takes the ceiling as `int(baseline * (1 + tolerance))`. For 2151196 and 0.05 that is 2258755, which is the 2,258,755 stated in the current sprint. No release commit is in the S25 range.

M7 in `docs/sprints/SPRINT_PLAN.md` is S25 to S27, five stories and 16 engineer-weeks. F-069, F-071 and F-072 stay `pending`. The sprint diff adds no pointer dispatcher, no SVG annotation layer and no cursor state machine. Closing S25 would not complete M7.

F-068 and F-070 are `done` in `docs/sprints/BACKLOG.md` and in the current-sprint status table. F-X003 is `in-progress`. F-X011 is `pending`. F-100 and F-048 are `done`. F-068 depends on F-100. F-070 depends on F-048. The two S25 rows do not depend on each other or on F-X003.

`createToolRegistry` closes over its own group array. A second registry does not see the first. `active` requires a non-empty binding list. `passive` and `off` require an empty list. Taking a binding walks tools in registration order, sets each other active owner to `passive`, and replaces that owner's bindings with an empty list. The returned names are that walk. Refusal paths return before `push`, `splice` or a mode write. `group`, `groups`, `bindingOwner` and `registerTool` return new arrays or `copyTool`. The module does not import a command packer and does not add a listener.

`project_onto_segment` divides by `len2` at the written quotient. `hit_segment` uses `distance <= tolerance`. `project_onto_plane` sets `unit = n / n.length()`. `GeomError::number` matches 400 and 401. `ErrorCode` is 1, 700 and 701. `packages/core/src/errors.ts` has those three numbers. `ci/error-codes.json` reserves 400 to 499 and has no code entry 400 or 401.

Commit `06768e1` says `No glam call is added in this crate.` The current plan routes length and the angle through workspace glam. `docs/sprints/AS_BUILT.md` records that design-plan change and says it is not a `D-NN`. `docs/hld/DEVIATIONS.md` is not in the sprint diff. On rustc 1.97.1 the inherent `f64::sqrt` and `f64::atan2` live in `std`. Stable `core` does not provide those methods. `crates/ocelli-seg/src/contour.rs` already says `f64::round` is unavailable in that `no_std` crate. The sigmoid arm in `ocelli-pixel` already calls glam because scalar `exp` is not in `core`. Workspace glam is `default-features = false` with `features = ["libm"]`. `ocelli-geom` uses that workspace entry.

The sha256 of `packages/core/src/tools.ts` is `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`. The sha256 of `crates/ocelli-geom/src/measure.rs` is `2b00b1406e5b07cebeda9fc863bee927f9615df65b68299fde1d6f179b3902e8`. Both match the story notes. The F-068 and F-070 tracker cells start with `not measured`. The five F-068 reviews report 0/3, 0/3, 0/1, 0/1 and 0/0. The F-070 review reports 0 defects and 0 smells. Those counts match the tracker and `AS_BUILT.md`.

Hand evaluation of the local rules agrees with the fixtures. `(0,0,0)` to `(3,4,0)` is 5. The polyline through `(3,4,0)` and `(3,4,12)` is 17, not the chord 13. Segment `t` for `(1,3,0)` on `(0,0,0)` to `(4,0,0)` is `4/16`. The axial plane foot is `(1,2,0)`. Skipping the normal division yields z of `-9`. `atan2(1, 0)` is a right angle. `atan2(0, 1)` is 0. A pair out of the xy plane, `(0,0,1)` and `(0,1,0)`, has the same cross length and dot. A plane through `(0,0,5)` with normal `(0,0,1)` sends `(1,2,8)` to `(1,2,5)`. The last two cases were not executed.

This rejects a module singleton, a demotion that keeps the previous owner's other bindings, a hit that uses `<`, a constant segment parameter, and a new boundary code 400. The HLD names `ocelli-geom` and gives none of these formulas. The transcribed shell sentence in the F-068 plan matches `docs/hld/03-architecture-and-crates.md` section 3. The crate row matches section 4. Section 10 of `docs/hld/07-concurrency-and-typescript.md` places tool state in the shell and hit-testing in `ocelli-geom`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | host before any compile | executed now | no compiler process |
| `git rev-parse HEAD`, `s24^{commit}`, `main`, status | `sprint/s25` | executed now | HEAD `9c77237`. Tag commit and `main` are `71ac28d`. Clean tree. |
| `git diff --stat 71ac28d..HEAD` | four S25 commits | executed now | 25 files. No version file, no wasm budget, no `DEVIATIONS.md`, no F-069, F-071 or F-072 path. |
| `shasum -a 256` of `tools.ts` and `measure.rs` | current bytes | executed now | `ca6a9007...c2c9` and `2b00b140...02e8` |
| `git rev-parse` of the two story trees against `.claude/verify-ledger.json` | `9e0e53d^{tree}` and `9c77237^{tree}` | executed now | both trees are recorded `profile=feature`, `corpus=pass`, agent `grok`, with the gate list in the commit trailers |
| `git show 06768e1:.claude/plans/F-070-design.md` | design approval | executed now | contains `No glam call is added in this crate` |
| current F-070 plan and workspace `Cargo.toml` | HEAD | executed now | plan says workspace glam. glam line is `0.30` with `libm` and default features off |
| rustc 1.97.1 `library/core` and `library/std` `f64.rs` | installed sysroot | executed now | inherent `sqrt` and `atan2` are in `std`. Stable `core` has neither method |
| backlog status parse and tracker prefix | HEAD ledgers | executed now | F-068 and F-070 `done`. F-069, F-071, F-072 and F-X011 `pending`. F-X003 `in-progress`. S22 to S25 rows read `not measured` |
| `python3` ceiling `int(2151196 * 1.05)` | same expression as `pin_and_size_check.py` | executed now | `2258755` |
| review result lines | F-068 passes 1 to 5 and F-070 pass 1 | executed now | 0/3, 0/3, 0/1, 0/1, 0/0 and 0/0 |
| count of `it(` and `#[test]` | `tools.test.ts`, `measure.rs` | executed now | 27 registry tests and 13 fixture tests, plus the scaffold test in `lib.rs` |
| F-068 and F-070 mutation catalogues | prior reviews | not repeated | prior receipts stand. Hashes still match those reviews |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

F-068 keeps one active owner per normalised binding inside a group. Demotion clears every binding of that owner, not only the binding that was taken. Names come back in registration order. The test `demotion names follow registration order` expects `Z` then `A`. A refused duplicate or bad binding returns before that walk writes. Viewport ids stay in at most one group. `viewportId(0)` is accepted by `Number.isSafeInteger` and `id >= 0`. Reads copy tools, modifiers and viewport ids. Two registries do not share groups. `packages/core` has no `Math.hypot`, `Math.atan2` or `Math.sqrt`. Tier rows for the registry are n/a in the design and in `AS_BUILT.md`. The lifecycle section matches the code. The public-api row in `docs/lld/README.md` names the tool registry and F-068.

F-070 keeps points in one `Pt<S>`. It does not call an F-048 transform. Distance is the Euclidean length, and a point to itself is 0. Polyline length sums windows in order and refuses a later `NaN` before returning a partial sum. The angle is `DVec2::new(dotted, cross_len).to_angle()`, which is `atan2` of the cross length and the dot, so a non-negative cross length stays in `0..=PI`. Exact zero `len2`, using both zero bit patterns, is `Degenerate`. There is no epsilon. The hit check order is non-finite input, then a negative tolerance, then a zero segment. The plane divides by the normal length. `GeomError::number` is a match, not a cast. No `as` cast was added in `measure.rs`. The crate is `no_std` outside tests. Dependencies are `ocelli-core` and workspace glam. There is no `unsafe`, no `wasm-bindgen` and no `wgpu`. Tier A, B and C are n/a in the design, in `docs/lld/geometry.md` and in `AS_BUILT.md`. The geometry row in `docs/lld/README.md` names F-070. Fixture expectations are literals or `f64` constants, not a second call to the function under test. The sprint diff adds no patient identifier and no DICOM preamble.

Both story commits name a feature profile. The ledger trees equal `git rev-parse` of those commits. The change log bullets for the registry and the measurements match the code. D-02 already names `ocelli-geom`. The glam route is not a new `D-NN`.

## Limitations

No product file was mutated, so nothing needed a restore. The F-068 mutation catalogue and the F-070 plane, epsilon and swapped-angle mutations were not repeated. `bin/ocelli.sh gate`, wasm-pack, vitest and `cargo test` were not run. The feature-profile receipts are historical and were checked by tree id, not re-executed. No GPU and no browser were used.

The out-of-plane angle and the non-zero plane origin were derived from the written formula and were not executed. `Index` was not instantiated. An infinite segment parameter that the clamp turns into 0 or 1 was traced to the later finite check and was not run. The string scan in `the registry does not dispatch` does not name `submitCommands`. The import list was read and does not call it.

This pass does not close S25. There is no sprint-profile ledger entry for a later union, and none was claimed. M7 stays open because F-069, F-071 and F-072 are still pending. The result is a bounded review of the landed S25 stories.
