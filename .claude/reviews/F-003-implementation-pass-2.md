# F-003 implementation review, pass 2

**Scope**: pass 1's two remediations, which were unreviewed work until this
pass, and a second read of the whole F-003 diff.
**Result**: 0 defects, 0 smells, 0 nitpicks. **Clean.**

## Pass 1's remediations, re-read

The licence and readme files are real files in each package rather than
symlinks, because `npm pack` does not reliably carry a symlink's content into
the tarball. Verified by listing the tarball rather than by trusting `files`.

The credential refusal is gone, including the `os` import it needed, and the
reasoning survives in both the script and the LLD so the next person to have
the idea finds the answer rather than the code.

## Second read of the whole diff

| Plan step | Landed | Evidence |
|-----------|--------|----------|
| 1, no bundler, with the reason written down | yes | `docs/lld/typescript-packaging.md`, and the revisit condition for F-096 is named |
| 2, a `packages` gate | yes | in `bin/ocelli.sh` and in the floor, so CI runs it |
| 3, consumer resolution proof outside the workspace | yes | `node` import plus `tsc` under `bundler` and `node16` |
| 4, `npm publish --dry-run` as the last step | yes | runs only when everything else passed, so a failure names the real cause |
| 5, the absent-core seam stays honest | yes | `coreAvailable()` still returns `false`, and a test now asserts it so F-096's change is visible in a diff |

## Two things checked specifically

**The publish dry run runs last and only when nothing else failed.** That is
deliberate. It is the slowest step and the one whose failure output is least
specific, so letting the tarball and consumer checks report first means a
failure names the actual cause instead of a registry error downstream of it.

**`@ocelli/react` resolves `@ocelli/core` from the sibling tarball, not from
the registry.** `@ocelli/core@0.1.0` is not published anywhere, so a registry
lookup would 404 and the consumer install would fail. It passes, which
confirms npm resolved the dependency from the tarball given on the same command
line. Worth recording, because if that behaviour ever changed the failure would
look like a network problem rather than a resolution one.
