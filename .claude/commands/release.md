---
description: Publish a prepared and reviewed version to crates.io and npm. The only command that creates a v* tag or publishes anything.
---

# /release vX.Y.Z

Release the exact reviewed HEAD. **This is the only command allowed to create
or push a `v*` tag, run `cargo publish`, or run `npm publish`.** It never merges
to `main` and never creates an `sNN` sprint tag.

The version bump lands earlier, through its own F-ID. **This command does not
edit a version, create a release commit, or repair a red gate.**

Cadence and the published package set are in `docs/RELEASE.md`.

## Preconditions

Refuse before any tag or push if one fails:

1. The argument is exactly `vX.Y.Z`, and `[workspace.package].version` is
   exactly `X.Y.Z`, and every published package inherits it.
2. The branch is the active `sprint/sNN` and the tree is clean.
3. **`bin/ocelli.sh gate --all` is green at this exact HEAD, including
   `corpus` and `oracle`.** CI runs neither (`DEVIATIONS.md` D-04), and that
   deviation is recorded on the understanding that this gate still runs here.
   A release is the one moment the GPU tier is not optional.
4. `python3 scripts/verify_ledger.py check-commit HEAD --require-corpus`
   passes. `corpus=absent` is permitted during early development and is **not**
   permitted here.
5. `python3 scripts/sprint_workflow.py release-notes vX.Y.Z --check` passes,
   and the rendered body is inspected. Its source is the `CHANGELOG.md`
   section headed by the exact tag at this HEAD.
6. `cargo publish --workspace --dry-run` succeeds with path patches, so
   internal dependencies resolve against this reviewed source graph rather than
   registry placeholders. A dry run uploads nothing.
7. `npm pack` for each published package, contents listed. **The wasm module
   must be present in `@ocelli/core` and must be the one this HEAD builds**,
   not a stale artefact left in `pkg/` by an earlier build. Rebuild it first
   and compare the digest.
8. `ocelli-wasm` is `publish = false`, and no test-only or private package is
   in either allowlist.
9. `docs/hld/DEVIATIONS.md` carries no row whose reason has expired. **A
   deviation shipped in a 1.x release becomes supported behaviour whether or
   not anyone intended it.**
10. Fetch remote tags. The exact tag must be absent locally and on `origin`.
    Refuse an already-published version rather than treating it as success.

## Final approval

Report the HEAD SHA, the tag, the exact package set for both registries, the
version, the remote, and the rendered release notes.

**Ask for a separate, explicit go or no-go immediately before the first
external mutation.** Approval given earlier in the feature or the sprint does
not carry to this boundary. crates.io has no unpublish. npm has none after 72
hours.

## Publish, in this order

The order runs from most recoverable to least, and reverses nothing.

1. Push the sprint branch at the reviewed HEAD.
2. Create one annotated tag `vX.Y.Z` at that exact HEAD, message
   `Release vX.Y.Z`.
3. Push only that tag.
4. **crates.io**, in dependency order, one at a time, with a wait between
   layers for the index to propagate. A publish that races the index fails on
   a dependency that exists but is not yet visible.
5. **npm**, `@ocelli/core` then `@ocelli/react`. After crates.io, because the
   npm package embeds the wasm module and a failed crate publish should stop
   the release before anything reaches npm.
6. Create the GitHub release from the rendered notes, and compare the published
   body byte for byte against a fresh render from the released SHA.

## After

Verify, do not assume:

- `cargo info <crate>@X.Y.Z` and the owner, for every crate in the set.
- `npm view @ocelli/<pkg>@X.Y.Z`, for every package.
- The GitHub release tag and its target SHA.

**A failed job is a failed release.** Do not re-run blindly. Do not convert an
authentication, network, compilation, duplicate-version or registry failure
into success.

If the branch push succeeds and the tag push fails, report that exact state. If
crates.io succeeds and npm fails, **retain the tag**, report the exact package
and error, and fix forward with a patch version. Never delete or move a
published tag.

## Finalise

Only after every registry version and the GitHub release are verified: write
the AS_BUILT entry with the release evidence, complete the release story's
ledger rows, and record it in sprint state.

## Refused situations

- A version bump or an uncommitted change is still required.
- Verification or the sprint review covers a different SHA.
- The corpus or the oracle did not run, or ran red.
- The CHANGELOG section for the tag is missing or differs from the published
  body.
- A local dry run is offered as a substitute for successful publication.
- The command would merge to `main` or create an `sNN` tag.
- The operator has not given the separate final approval.
