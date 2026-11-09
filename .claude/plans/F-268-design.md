# F-268, rustdoc links resolve, with a workspace doc check proposed through the normal workflow

**Status**: draft
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
| AC3 | Each enumerated position is fixed in place, by making the target visible or the link correct, never by deleting the doc comment | Diff review | Silencing by removal | implementation, reviewer |
| AC4 | The proposed check runs `cargo doc` for the workspace with the lints denied and fails with the link named | The check run before and after planting one private link | A check that greps source instead of building docs | implementation |
| AC5 | Negative controls. Reintroducing one private intra-doc link turns AC1 red, and planting an unresolved link turns AC2 red | Recorded in the review | A suite that passes on the wrong tree | implementation, reviewer |
| AC6 | Public API unchanged. `cargo check --workspace` and the existing suites pass | The feature profile at the tree | A rename smuggled in as a fix | implementation |
| AC7 | The check's profile placement is decided through the normal workflow and recorded where the workflow keeps it, never asserted by this filing | The workflow record | This filing claiming an authorized gate or floor change | operator, workflow |

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

Rejected: `#![allow]` at the call sites, rewriting docs to avoid linking, and
a grep-based check, which reads source rather than what rustdoc resolves.

## Dependencies, write ownership and resources

No dependencies, as the recovered draft had. Writes doc comments across the
crates the fresh run names, `bin/ocelli.sh` and the workflow files only
through the normal workflow. No exclusive resource. No wasm or size impact,
since docs do not enter the modules.

## Verification and evidence reuse

AC1's run is both the evidence and the fix list. The feature profile covers
the changed crates. Evidence invalidates on any later link-affecting edit,
which is what the proposed check exists to catch.

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
| unit | The planted-link negative controls | AC5, recorded in the review |
| integration | The workspace doc build exits clean | AC1, AC2 |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/build-targets.md` gains the check's description if the workflow
accepts it.

## User-facing documentation impact

`README.md` no change. The workspace docs are the deliverable.

## Open questions

1. Floor or sprint profile? Affects AC4 and AC7. Owner: the operator through
   the normal workflow. This filing proposes and never authorizes. Both
   lints denied is this draft's proposed command contract, not an authorized
   profile or workflow change.
