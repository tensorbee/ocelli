# F-198 review, pass 5
**Reviewed**: staged tree faf36b0cf1cc1f2a7d19c1b647dfc040420feadd
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

The question for this pass is whether `#[cfg(all(feature = "render-worker", any(test, target_arch = "wasm32")))]` on `mod tier_label` (`crates/ocelli-wasm/src/lib.rs:40`) compiles the module in every configuration that uses it and in none that cannot resolve `ocelli_render`. I checked it against the only non-test user, `mod canvas` at `lib.rs:34`, which is `all(target_arch = "wasm32", feature = "render-worker")`, and against the manifest.

| Configuration | `canvas` | `tier_label` | `ocelli_render` linked | Outcome |
|---|---|---|---|---|
| host, default, non-test | no | no | yes | no dead code, nothing to resolve |
| host, default, test | no | yes | yes | the two unit tests use both helpers |
| host, decode-worker only, test | no | no | no | closes the pass 4 failure |
| host, decode-worker only, non-test | no | no | no | clean |
| wasm32, default | yes | yes | yes | canvas calls both helpers |
| wasm32, decode-worker only | no | no | no | clean |
| either target, `tier-b-probe` | follows render-worker | follows render-worker | yes | `tier-b-probe = ["render-worker"]` at `Cargo.toml:99` |

On wasm32 the two predicates reduce to the same thing. On the host, `tier_label` exists only under `test`, and its tests call both `tier_name` and `stored_tier_label`, so neither is dead. No configuration compiles `tier_label` without `ocelli-render`, because `render-worker = ["dep:ocelli-render", ...]` at `Cargo.toml:82` is now a conjunct.

## Evidence

- `crates/ocelli-wasm/src/lib.rs:32-41`, both cfgs and the new three-line comment.
- `crates/ocelli-wasm/Cargo.toml:77-99` and `:114`, where `ocelli-render` is optional and enabled only through `render-worker`.
- `crates/ocelli-wasm/src/tier_label.rs`, all 48 lines. The import at line 6 is the one pass 4 saw fail to resolve.
- `crates/ocelli-wasm/src/canvas.rs:402-472, 621-651`, every call into `crate::tier_label`. All of them are inside the wasm32 and render-worker module.
- `/tmp/ocelli-f198.diff:575-655`, the AS_BUILT, BACKLOG, CURRENT_SPRINT and SPRINT_TRACKER hunks.
- The command results in the brief. I did not run them again, because this pass has no shell.

## Defects

None.

## Smells

None.

## Nitpicks

**N1, carried.** The production `/@fs` vite define. It is unchanged and stays non-blocking.

**N2, carried.** The empty tier line when `available` is false. It is unchanged and stays non-blocking.

**N3, new.** The AS_BUILT note at diff line 608 says "Host clippy passed." That is true for `cargo clippy -p ocelli-wasm --all-targets -- -D warnings` with default features. The decode-worker clippy run exited 101 on `examples/presented_frame.rs`, which is outside this diff. "Host clippy with default features passed" would stop a reader taking it as cover for decode-worker. Non-blocking.

## Verified clean

- **The predicate closes pass 4 S1.** `tier_label` can no longer compile without `render-worker`. The decode-worker host lib test that exited 101 is now reported as passing 11 tests with `tier_label` absent, and every configuration in the table above resolves.
- The predicate does not narrow anything `canvas` needs. On wasm32 it is the same as `canvas`'s own cfg.
- The comment at `lib.rs:37-39` describes the cfg accurately.
- `Tier` in `crates/ocelli-render/src/caps.rs:24` has exactly `A`, `B` and `Cpu`, so the match in `tier_name` is exhaustive with no wildcard.
- The AS_BUILT pass 4 sentences are true against the brief: the smell, the failing command, the new predicate, 11 decode-worker lib tests, 2 `tier_label` tests, and the wasm32 check.
- The six mutations in the AS_BUILT note match the six items in the tracker row: the flag, the truncated post, the frame commit, the accept check, the adapter branch and the reused canvas.
- The tracker row leaves out the "the last at 0 defects and 0 smells" clause that the F-195 to F-197 rows carry. That is correct, because the last pass recorded there, pass 4, had a smell.
- The BACKLOG and CURRENT_SPRINT status flips agree, and the rewritten CURRENT_SPRINT bullet matches what `loadCore` is described as doing.
- I found no em dash and no prose semicolon in the new AS_BUILT or tracker lines.

## Limitations and handoff

- I ran nothing. Every build and test result above is from the brief.
- **The AS_BUILT Verification paragraph and the tracker's "staged feature verification" are not yet true for faf36b0c.** The floor and corpus gates have not been run on this tree. Both sentences point at a verify ledger entry that does not exist yet. They become true only if `/verify` with the feature profile runs on this exact tree before commit. The commit-msg hook keys the trailer to the tree hash, so committing without that run should be refused.
- The AS_BUILT notes stop at pass 4. Once this pass is accepted, add one sentence recording pass 5 as 0 defects, 0 smells and 3 nitpicks. That changes the staged tree, so verification has to run on the final tree and not on faf36b0c.
- `examples/presented_frame.rs` failing decode-worker clippy predates this diff and is not an F-198 finding. It may deserve its own backlog row.
The two prose edits applied after this pass, as this handoff asked, are the pass 5 count in AS_BUILT and the words "Host clippy with default features passed". Verification has to name the tree that contains those sentences.
