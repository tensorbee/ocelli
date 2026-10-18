# F-042 review, pass 4

**Reviewed**: working tree against b93eef02. The only unstaged path is `eslint.config.js`. Behaviour sources are older than the pass 3 review.
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, The native draw-failure arm asserts a variant it already excluded

**Where**: crates/ocelli-render/tests/tier_b.rs:116
**What**: The `NotSubmitted` and `Readback` arm asserts `matches!(error, Section183Error::Unavailable)`. That pattern cannot succeed. The test still fails closed, because the assertion message fires. `tests/tier_b.rs` was not edited after pass 3. Carried from pass 3.

## Verified clean

The lint fix is the block at `eslint.config.js:317`. `git diff HEAD --numstat -- eslint.config.js` is 15 insertions and 0 deletions. The bytes before `const config = tseslint.config(` match `HEAD`. `ALLOWED_TO_DISABLE` is still the two file lists at lines 142 to 145. The wasm-memory rule and `assertTheBanIsIntact` are those same bytes. The new block has no `rules` key. It does not set `no-undef` to off, and it does not disable either half of the view ban.

Loading the config runs `assertTheBanIsIntact`. `ESLint.calculateConfigForFile("tools/tier-b/run.mjs")` succeeded. For that file, `no-undef` is severity 2 with `{ typeof: false }`. The globals are `console`, `process`, `URL`, and `document`, each `readonly`. `Worker` and `window` are absent. `ocelli/no-wasm-memory-view` and `no-restricted-syntax` are absent on this `.mjs` file. That is the existing TypeScript scope of the ban, not a new allowance.

`tools/tier-b/nested/extra.mjs` receives the same four globals. That path is under `tools/tier-b`. `tools/tier-b-other/run.mjs` receives none of them. On `tools/oracle/src/voi.mjs`, `document` is absent. On `packages/core/offscreen-canvas.mjs`, the F-039 set is unchanged and still includes `Worker`, `setTimeout`, `clearTimeout`, and `Error`. A snippet `document;` linted as `scripts/example.mjs` reports `no-undef` for `document`.

`eslint.lintText` on `tools/tier-b/run.mjs` reported 0 errors and 0 warnings. Inserting `notARealGlobal` reported `no-undef`. Assigning to `document` reported `no-global-assign`. `npm run lint` exited 0.

The new comment has no semicolon and no em dash. It names the node globals the script uses, plus `document`. That identifier is the Playwright `waitForFunction` callback at `tools/tier-b/run.mjs:150`. `find tools/tier-b -type f` lists only `run.mjs`.

`git diff --name-only` lists only `eslint.config.js`. `probe.rs`, `graph.rs`, `tier_b.rs`, `canvas.rs`, and `tools/tier-b/run.mjs` are older than the pass 3 review file. This pass did not re-run the GPU and did not rebuild wasm. Pass 3's behaviour findings were not re-opened.

Prose written after pass 3 was read in full. The design plan names `resolve_webgl` as the second `create_surface`. The call sites are `probe.rs:497`, inside `resolve_webgl` and before `detect_from` at line 505, and `gpu.rs:308`, inside `attach_target`. `request_adapter` is `probe.rs:590`. `ocelli-wasm` has no `create_surface` call. `ocelli-render` has no `web_sys` name. The recorded pass counts 4/0, 1/0, and 0/0 match pass 1, pass 2, and pass 3. `ci/wasm-size-budget.json` is not in the diff. Its recorded size is still 1999061 bytes with tolerance 0.05. No new false claim was found in those pages.
