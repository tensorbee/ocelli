---
description: Draft, validate or render the CHANGELOG section for a release tag. The notes are a reviewed artefact, not a generated changelog.
---

# /release-notes vX.Y.Z [--check] [--render]

The release notes are the `CHANGELOG.md` section headed by the exact tag.
`--check` validates it. `--render` prints the exact body `/release` will
publish.

`/release` compares the published GitHub release body byte for byte against a
fresh render, because a body that drifted from the changelog means one of the
two is lying and there is no way to tell which.

## Drafting

Sections, in this order, omitting any that is empty:

```markdown
## vX.Y.Z

### Highlights

{One paragraph. What changed for someone using the library, not what the
team did. "Volume rendering degrades to WebGL2" is a highlight. "Completed
M5" is not.}

### Added
### Changed
### Fixed
### Deprecated
### Known limitations
```

## Rules

**Write for a consumer of the library.** F-IDs, sprint numbers and milestone
names mean nothing outside this repository. Translate them.

**A tolerance or divergence change is always a Highlight**, never a Fixed
bullet. Someone integrating this library may have written a conformance
statement against the previous number.

**Behaviour that differs from cornerstone3D goes in `Known limitations`,
named.** The whole product argument is measured divergence rather than claimed
equivalence (decision D14), so a difference that is documented is a feature and
a difference that is discovered is a defect.

**Every deviation in `docs/hld/DEVIATIONS.md` that affects observable
behaviour gets a `Known limitations` line.** A deviation that ships in a 1.x
release is supported behaviour, whether or not anyone intended it, and the
first release where it goes unmentioned is the release where it became a
promise.

**Do not claim a capability the corpus did not exercise.** If a transfer syntax
has no corpus case, it is not supported, it is untested. Say which.

**Do not claim bit-exact reproducibility, ever.** Decision D14. Publish the
measured bound.

## `--check`

1. A section headed by exactly `## vX.Y.Z` exists.
2. It contains at least one of the content sections and is not empty.
3. No `TODO`, `TBD` or placeholder text.
4. The voice rules pass over it.
5. It is the topmost released section, and `## Unreleased` above it is either
   absent or empty. **A non-empty `## Unreleased` above a release section means
   changes landed after the notes were written**, and the notes therefore
   describe something other than what is being released.
6. Every deviation currently affecting observable behaviour appears.

## `--render`

Prints the section body with the heading removed, deterministically. Rendering
twice must produce identical bytes, which is what makes the byte-for-byte
comparison in `/release` meaningful.
