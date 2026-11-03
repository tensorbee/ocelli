# F-117, stable-release claim

**Status**: approved
**Epic ref**: E19.4
**Sprint**: S41
**Estimate**: 2w

## Normative source, transcribed

`docs/RELEASE.md` says a release is a milestone boundary, never a sprint boundary. It says 1.0.0 is gated on M13, where the browser matrix is certified, binary size lands inside budget, and this story sets the semver, changelog and deprecation policy. It says calling something 1.0 before this story lands would claim a stability guarantee no policy exists to honour. It says `/release` is the only command that may publish. It says 0.x carries no stability promise.

D-22 blocks publication. The committed browser matrix has no certified row.

## What the specification does not cover

`docs/RELEASE.md` does not say how many certified rows are enough, and it does not say how long a deprecated name stays.

Those gaps are local decisions. They are not HLD formulas.

`mayClaimStable` returns true only when `d22Open` is false, `matrixRows` is an integer of at least 1, `certifiedRows` is an integer, and `certifiedRows` equals `matrixRows`. Every other input returns false. The function does not read a file and does not bump a version.

`removalMinorsRemaining` is the deprecation window.

- `0.x` returns null. There is no promise.
- `patch` returns null. A patch does not schedule a removal.
- `minor` returns 1. A name deprecated in a published minor stays for one further minor.
- `major` returns 0. A major may remove a name.
- Any other channel returns null.

This story does not publish, does not create a `v` tag, does not run `/release`, and does not change the workspace version from 0.1.0.

## Problem and reproducer

The policy text says 1.0.0 waits on this story. Nothing in the shell returns false for a stable claim while D-22 is open and no browser row is certified. A caller could treat the end of M13 as that claim.

## Scope, API and compatibility

In scope:

- `mayClaimStable` and `removalMinorsRemaining` in `packages/core/src/release.ts`
- Tests and the published names
- `docs/lld/release-policy.md` with the window above
- The public-api row

Out of scope:

- Editing `Cargo.toml` or `package.json` version fields
- Editing `ci/browser-matrix.json`
- A changelog section headed by a version tag
- `cargo publish` or `npm publish`

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A release check the caller writes | `mayClaimStable` | D-22 flag and two counts | true only when the gate is shut and every row is certified |
| A deprecation note | `removalMinorsRemaining` | One channel name | A minor count, or null |
| `/release` | unchanged | Still the only publish command | Unchanged |

## State transitions and invariants

Stateless. Neither function edits the version or the matrix. True is not a publish.

## Ownership, commit and recovery

Not applicable. No tag is created and no registry is contacted.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | D-22 open returns false even when every row is certified | unit | Ignoring the deviation | implementation |
| AC2 | Zero certified rows returns false | unit | Treating an empty matrix as certified | implementation |
| AC3 | Certified count short of the row count returns false | unit | Treating one certified row as enough | implementation |
| AC4 | Closed D-22 and equal positive counts returns true | unit | Also writing a version file | implementation |
| AC5 | A non-integer count returns false | unit | Coercing 1.5 to 1 | implementation |
| AC6 | The four channels return null, null, 1 and 0 | unit | Giving 0.x a window of 0 | implementation |
| AC7 | An unknown channel returns null | unit | Throwing | implementation |
| AC8 | The published names are exported | `index.test.ts` | Leaving the functions private | implementation |
| AC9 | The workspace version string in the existing test stays 0.1.0 | `index.test.ts` | Bumping the version in this story | implementation |

## Approach

One new module and one LLD page. No wasm-bindgen. No pixels. No `unsafe`. No network.

## Tiers

| Tier | Behaviour |
|------|-----------|
| A | Not applicable. The claim does not render. |
| B | Not applicable. The same boolean is returned. |
| C | Not applicable. The same boolean is returned. |

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the claim and the window | `release.test.ts` |
| export | published names | `index.test.ts` |
| version | the shell version stays 0.1.0 | `index.test.ts` |

The committed matrix is not parsed by the function. The LLD states that the current matrix has no certified row and that D-22 is open, so a caller who passes those facts gets false.

## Parity surface covered

None.

## Deviations

None. D-22 is cited and not edited. D-11 is cited by `docs/RELEASE.md` and is not edited.

## LLD impact

`docs/lld/release-policy.md`, `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

None. The full-matrix rule and the deprecation window are approved local decisions.
