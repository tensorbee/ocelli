# F-268, rustdoc links resolve, with a workspace doc check proposed through the normal workflow

**Status**: approved
**Epic ref**: E19.8
**Sprint**: S47
**Estimate**: 1w

Filed at the S46 close from the recovered close-backlog draft, original item
4, with that draft's internal count inconsistency resolved in the row. The
estimate is inherited and is provisional, lead-suggested and not
capacity-approved. This draft claims no implementation, no review cleanliness
and no completion.

## Normative source, transcribed

`docs/hld/12-workspace-and-build.md` 15.3: "Decision D2 is worthless unless it
is enforced. This runs on every pull request." The HLD's pattern for a build
property is a check that runs, not a convention.

No HLD section names rustdoc, `cargo doc` or a documentation gate. A search of
`docs/hld/` for rustdoc and cargo doc finds nothing. The two lints are
rustdoc's own: `rustdoc::private_intra_doc_links` denies a public item's
documentation linking to a private item, and `rustdoc::broken_intra_doc_links`
denies an unresolved intra-doc link.

## What the specification does not cover

Where the check sits between the floor and the sprint profile is a design
decision. This draft proposes both `broken_intra_doc_links` and
`private_intra_doc_links` denied consistently, through explicit
`RUSTDOCFLAGS` on the workspace documentation command.
The floor is what CI runs, and `bin/ocelli.sh gate --floor` is compared for
set equality against `scripts/ci_floor_check.py`'s `NOT_IN_FLOOR` by the `ci`
gate, so changing the floor is a workflow change. This filing makes no such
change and proposes the check through the normal workflow.

For the docs Cargo child only, remove CARGO_ENCODED_RUSTDOCFLAGS from the child environment (unset, never set to an empty string) and set RUSTDOCFLAGS to the exact existing authoritative paired denial. Precedence follows current Cargo docs: encoded flags would override raw ones, so an inherited encoded -A for both lints would defeat the paired denial. Do not mutate the caller or global environment and do not alter other gates.

Ordinary Cargo command examples are baseline invocations assuming CARGO_ENCODED_RUSTDOCFLAGS is absent, not retrospective proof of an old environment. The actual docs gate must apply the separately declared child-only normalization before invoking Rustdoc.

## Problem and evidence

At the S46 close the recovered draft ran, with a scratch `CARGO_TARGET_DIR`:

```text
RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc -p ocelli-render --no-deps
```

exit 101 with four errors in `crates/ocelli-render`: `graph.rs:1856:11`,
`register_stored` linking to `selected_stored_format`, and
`interpolation.rs:92:23`, `:92:41` and `:95:54`, `blend_display` linking to
`texel_of`. The same flag over `cargo doc --workspace --no-deps --keep-going`
then failed five crates. The draft's prose said eleven private errors plus one
unresolved warning. Its own location table enumerates ten private positions
plus that warning: `ocelli-geom` two, `ocelli-native` one, `ocelli-volume`
one, the folder viewer two, `ocelli-render` four, and
`crates/ocelli-viewport/src/snapshot.rs:7:34` an unresolved link to
`LutChain::new`. **The prose and the table disagree by one, so this filing
carries no count.** The table is a historical enumeration at one tree, not a
current measurement, and the list moves as stories land, so the story re-runs
the check at implementation rather than fixing the table's rows only.

Source check at 31323a79, read-only: the named items still exist, including
`register_stored` in `crates/ocelli-render/src/graph.rs` and `blend_display`
in `crates/ocelli-render/src/interpolation.rs`. No `cargo doc` run was made
for this filing, and none of the counts above is claimed as current.

The cause is that nothing builds the docs.
`grep -c "cargo doc" bin/ocelli.sh .github/workflows/ci.yml scripts/*.py`
names no match at 31323a79, so no gate catches the links. F-229 working pass 4
N2 found the same gap for `--document-private-items` in `ocelli-codec`.

## Scope, API and compatibility

In scope: the private and unresolved positions a fresh workspace doc run at
implementation enumerates, fixed in place, and a workspace `cargo doc` check
with the two rustdoc lints denied, proposed through the normal workflow.

Non-goals: writing missing documentation, changing any public item's name to
silence a link, `--document-private-items` scope, deleting a doc comment to
make an error go away, and any policy or floor edit inside this filing.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Every workspace crate's docs | `cargo doc --workspace --no-deps` under the denied lints | Scratch target dir, keep-going | Exit status and the error list |
| The proposed check | A named `bin/ocelli.sh` gate function | Profile placement a design decision | Failure names the link |
| Reviewers and `/verify` | The same check, once approved | Floor or sprint as decided | Evidence the docs build |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Stage / owner |
|----|-------------------------------|---------------------|-------------------------------|---------------|
| AC1 | A fresh `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps --keep-going` with a scratch target dir exits 0 at the story tip | The recorded command and exit | Fixing the draft's table rows only | implementation |
| AC2 | The same run with both `private_intra_doc_links` and `broken_intra_doc_links` denied exits 0 | The recorded command and exit | A check that passes with unresolved links silently | implementation |
| AC3 | Every freshly enumerated doc reference is fixed in place with its meaningful description unchanged. Private helper names appear as code identifiers where needed. The public constructor link is fully qualified as `ocelli_pixel::LutChain::new`. API, visibility, signatures, and reexports remain unchanged. | Diff review | Silencing by removal | implementation, reviewer |
| AC4 | The proposed check runs `cargo doc` for the workspace with the lints denied and fails with the link named | The check run before and after planting one private link | A check that greps source instead of building docs | implementation |
| AC5 | Negative controls. Reintroducing one private intra-doc link turns AC1 red, and planting an unresolved link turns AC2 red | Recorded in the review | A suite that passes on the wrong tree | implementation, reviewer |
| AC6 | Public API unchanged. `cargo check --workspace` and the existing suites pass | The feature profile at the tree | A rename smuggled in as a fix | implementation |
| AC7 | The check's profile placement is decided through the normal workflow and recorded where the workflow keeps it, never asserted by this filing | The workflow record | This filing claiming an authorized gate or floor change | operator, workflow |
| AC8 | Run the actual bin/ocelli.sh gate docs target on a valid synthetic Rust fixture while CARGO_ENCODED_RUSTDOCFLAGS is inherited carrying -A for both lints, with a private link planted, then repeat with an unresolved link planted, then restore the healthy fixture and run once more under the same inherited override. No mock wrapper is substituted for the real child spawn. This is a permanent control, not a one-off diagnostic. | The gate must reject the private-link fixture with its intended paired-denial diagnostic (nonzero exit), reject the unresolved-link fixture with its intended diagnostic (nonzero exit), and then pass with exit 0 on the restored healthy fixture despite the inherited encoded override. The gate record must show its own child exits and that the caller environment, including CARGO_ENCODED_RUSTDOCFLAGS and RUSTDOCFLAGS in the caller shell, is unchanged after each run. | A wrong implementation sets CARGO_ENCODED_RUSTDOCFLAGS to an empty string or appends to it, mutates the caller or global environment, touches other gates, swaps in a Cargo-only invocation or a mock wrapper instead of the actual gate, or accepts the private or unresolved fixture because the inherited encoded -A masked the paired denial. | implementation, reviewer |

Unrun ownership. No doc build, gate or test ran for this filing. AC1's fresh
run is the story's first act, with a scratch `CARGO_TARGET_DIR`, and its list
may differ from the draft's table in both directions.

## Approach

1. Run the fresh workspace check and record the list.
2. Fix each position in place.
3. Draft the check as a named `bin/ocelli.sh` gate function calling
   `cargo doc --workspace --no-deps` with the lints denied, and take its
   floor-or-sprint placement through the normal workflow.
4. Pass `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links -D rustdoc::broken_intra_doc_links"`
   to that command. The check deliberately makes no claim that an unadorned
   `cargo doc` inherits these flags. The pinned local rustdoc book confirms
   that these lints are available when running rustdoc, not rustc. The
   pinned Cargo manifest reference gives generic lint namespace guidance,
   which does not establish workspace inheritance or command coverage for
   an untested `[workspace.lints.rustdoc]` shape. No manifest-table API is
   proposed by this draft.

Rejected: deleting or weakening meaningful doc comments, exposing private helpers, changing visibility or API surface, adding lint allow/suppress attributes, and substituting source-grep checks for actual rustdoc resolution. Allowed: referencing a private helper as a plain code identifier in prose and linking an existing public contract. No blanket prohibition applies to those allowed edits.

## Dependencies, write ownership and resources

No dependencies, as the recovered draft had. Writes doc comments across the
crates the fresh run names, `bin/ocelli.sh` and the workflow files only
through the normal workflow. No exclusive resource. No wasm or size impact,
since docs do not enter the modules.

## Verification and evidence reuse

AC1's run is both the evidence and the fix list. The feature profile covers
the changed crates. Evidence invalidates on any later link-affecting edit,
which is what the proposed check exists to catch.

Full native/default workspace docs gate exits 0. A tiny two-member fixture gate exits healthy 0, private 1, broken 1. Raw Cargo baseline EXIT 101 was fixed to 0, not 101 failures. 68 unit tests total including 3 new stdlib tests, including the selector-ordering regression. Two deep refusal probes and a healthy control are green. Three wrong-gate mutants are semantic RED. Staged feature floor/corpus and independent final union review are pending. No whole-feature completion.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: n/a. Documentation only
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three tier rows are not applicable. The story touches no rendering or
compute.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| integration | The planted-link negative controls | AC5, recorded in the review |
| integration | The workspace doc build exits clean | AC1, AC2 |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/build-targets.md` gains the check's description if the workflow
accepts it.

## User-facing documentation impact

`README.md`, `use.html`, and `more.html` no change, because the edit set
introduces no viewer, runtime, or API behavior change. The workspace docs
are the deliverable.

## Gate providers and mandatory controls

- Doc-only fixes land in the actual lib source files: crates/ocelli-geom/src/lib.rs, crates/ocelli-native/src/lib.rs, crates/ocelli-render/src/graph.rs, crates/ocelli-render/src/interpolation.rs, crates/ocelli-viewport/src/snapshot.rs, crates/ocelli-volume/src/ortho.rs and examples/folder-viewer/src/measure.rs.
- The named docs arm plus GATES registry go in bin/ocelli.sh.
- The actual CI invocation goes in the Rust job of .github/workflows/ci.yml.
- Guard registration goes through scripts/guards/catalogue.py.
- Workflow edits cover .claude/WORKFLOW.md, the affected .claude/commands/verify.md and generated adapters via scripts/sync_agent_skills.py, as a separate reviewed policy commit.
- Any new refusal, budget or generated runbook delta follows the existing guard process and is never reset.
- docs/lld/build-targets.md is written only when the check is actually implemented.
- No new forwarding module, generic framework, trait or dependency is introduced.

Otherwise valid fresh source with a healthy link must pass the actual check. Separately, one private-target link must RED with its actual named diagnostic and own exit, then restore GREEN. Separately, one unresolved link must RED with its named diagnostic and own exit, then restore GREEN. Full workspace and default-selection scope, the fresh unique target, source, tool and effective flag identity, and cleanup and failure-exit preservation all stay in force. Existing correct paired flags remain untouched.

## Open questions

The lead settled the additive docs check placement in the floor because no GPU or corpus is needed. A normal separately reviewed WORKFLOW, affected-command, and adapter-sync change is neither implemented nor adopted. No other material doc-fix choice remains. Independent design approval is recorded in `.claude/reviews/F-268-design-pass-2.md`. Open question owner: Codex lead (parent integrator) is the assigned independent design review owner, distinct from the GLM author.
