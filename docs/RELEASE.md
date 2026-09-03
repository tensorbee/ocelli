# Release policy and cadence

Ocelli publishes to two registries from one repository: the `ocelli-*` crates
to crates.io, and the `@ocelli/*` packages to npm. They version together and
release together, because `@ocelli/core` ships the wasm module built from the
crates and a version skew between them is not a supported configuration.

## Cadence, tied to milestones

**A release is a milestone boundary, never a sprint boundary.** A sprint closes
with an `sNN` tag created by `/close-sprint`. That tag is history, it is not a
release, and it publishes nothing.

| Milestone | Version | What it claims |
|-----------|---------|----------------|
| M1 | 0.1.0 | The namespace is reserved and the oracle exists. Nothing is ported. |
| M2 to M12 | 0.2.0 to 0.12.0 | One minor per milestone. Anything may change. |
| M13 | **1.0.0** | Parity with cornerstone3D v5.8.9, and semver starts meaning something. |
| M14 to M18 | 1.1.0 to 1.5.0 | Phase 1.5, the differentiating capabilities. |

**0.x carries no stability promise and says so.** The first publish at M1 is
there to reserve `ocelli` on crates.io and `@ocelli` on npm, both of which HLD
section 1 records as free and worth claiming. Publishing early is cheap and
losing a namespace is not recoverable.

**1.0.0 is gated on M13, not on a date.** M13 is where the browser matrix is
certified, binary size lands inside budget, and story F-118 (E19.4) sets the
semver, changelog and deprecation policy. Calling something 1.0 before that
story lands would be claiming a stability guarantee no policy exists to honour.

A patch release happens whenever a defect warrants one and does not wait for a
milestone.

## What is published, and what is not

**crates.io, in dependency order:** `ocelli-core`, `ocelli-geom`,
`ocelli-pixel`, `ocelli-codec`, `ocelli-dicom`, `ocelli-cache`, `ocelli-volume`,
`ocelli-render`, `ocelli-compute`, `ocelli-seg`, `ocelli-viewport`.

**npm:** `@ocelli/core`, then `@ocelli/react`.

**Never published:** `ocelli-wasm` is `publish = false`. It is a `cdylib` whose
artefact ships inside `@ocelli/core`, so publishing it to crates.io would offer
a crate nobody can use from Rust. `ocelli-oracle` is test infrastructure and
`@ocelli/example-viewer` is private.

npm publishes **after** crates.io, because the npm package embeds the wasm
module and a failed crate publish should stop the release before anything
reaches npm. npm has no unpublish after 72 hours, crates.io has none at all.
Order the irreversible steps from most recoverable to least.

## `/release` is the only command that may publish

It is the only thing permitted to create or push a `v*` tag or run
`cargo publish` / `npm publish`. It never merges to `main` and never creates an
`sNN` sprint tag. The version bump itself lands earlier, through its own F-ID,
so `/release` never edits a version.

**It asks for a separate go or no-go immediately before the first external
mutation.** Approval given earlier in the feature or the sprint does not carry
to this boundary. Publication is irreversible and a release tag is not moved
once published.

## Release notes are a reviewed artefact

The notes are the `CHANGELOG.md` section headed by the exact tag, at the
released commit. `/release-notes` renders them and `--check` validates them.
The published GitHub release body is compared byte for byte against a fresh
render from the released SHA, because a release body that drifted from the
changelog means one of the two is lying and there is no way to tell which.

## The preflight, and why each item is there

`/release` refuses before any tag or push if one of these fails:

1. The tag is exactly `vX.Y.Z`, and `[workspace.package].version` is exactly
   `X.Y.Z`, and every published package inherits it.
2. The branch is the active `sprint/sNN`, the tree is clean, and the tag does
   not already exist locally or on `origin`.
3. `bin/ocelli.sh gate --all` is green at this exact HEAD, **including the
   corpus and the oracle**. A release is the one moment the GPU tier is not
   optional, and `DEVIATIONS.md` D-04 moves that gate off CI precisely on the
   understanding that it still runs here.
4. `python3 scripts/verify_ledger.py check-commit HEAD --require-corpus`
   passes, so the evidence is in the commit and not only in someone's memory.
5. `cargo publish --workspace --dry-run` succeeds with path patches, so
   internal dependencies resolve against this reviewed source graph rather
   than registry placeholders. A dry run uploads nothing.
6. `npm pack` for each published package, with its contents listed. The wasm
   module must be present in `@ocelli/core` and must be the one this HEAD
   builds, not a stale artefact from a previous build.
7. The `CHANGELOG.md` section for the exact tag exists and `--check` passes.
8. `docs/hld/DEVIATIONS.md` has no row whose "Why" is a temporary measure that
   has since expired. A deviation shipped in a 1.x release is a supported
   behaviour whether or not anyone intended it.

## After publication

Verify `cargo info <crate>@X.Y.Z` and the owner for every crate, `npm view
@ocelli/<pkg>@X.Y.Z` for every package, and the GitHub release tag and target
SHA. Do not claim a package was published without checking the registry.

A failed job is a failed release. Do not re-run blindly, and do not convert an
authentication, network, compilation, duplicate-version or registry failure
into success. If the crate publish succeeds and npm fails, retain the tag,
report the exact state, and fix forward with a patch version. Never delete or
move a published tag.
