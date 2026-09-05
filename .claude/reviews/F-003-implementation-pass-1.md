# F-003 implementation review, pass 1

**Scope**: the working diff for F-003 (E1.3), TS package scaffold, bundling,
npm publish pipeline.
**Result**: 2 defects, 0 smells, 0 nitpicks. Both remediated.

## Defects

### D1. The packages shipped no licence text

`npm pack --dry-run` on `@ocelli/core` before this story listed nine files:
`dist/` and `package.json`. No `LICENSE`, no `README.md`.

Both manifests declare `"license": "MIT OR Apache-2.0"`. A published tarball
that carries neither licence text makes that a claim rather than a grant, and
it is the kind of thing nobody notices until a downstream consumer's legal
review does.

**Remediation.** `LICENSE-MIT`, `LICENSE-APACHE` and a `README.md` are in both
packages and named explicitly in `files`, so the packaging check asserts an
intent rather than an npm default. Proved red by dropping them from `files`.

### D2. A credential refusal that would have got the gate disabled

`scripts/package_check.py` initially refused to run when `NPM_TOKEN` was set or
an `.npmrc` carried an `_authToken`, on the theory that it defends against a
future edit removing the `--dry-run`.

**The cost is larger than the protection.** `npm publish --dry-run` cannot
publish, so the guard defends against a hypothetical edit rather than a real
failure mode. Meanwhile any developer logged into npm for an unrelated project
would have had this gate fail on them, with no action available except deleting
their credentials or disabling the gate. `AGENTS.md`'s escalation table makes
disabling a gate a change to `.claude/WORKFLOW.md`, so building one that
invites it is the wrong trade.

**Remediation.** Removed before it landed, with the reasoning recorded in the
script and in the LLD so it does not get re-added by the same argument.

## What was checked and found clean

- Every `as` cast: none, this is TypeScript and Python.
- Arithmetic: none. No pixel and no coordinate, so HLD 27.2 R3 does not apply.
- **`VERSION` is asserted against a literal**, not against `package.json`.
  Comparing the constant to the file it was copied from restates it. Comparing
  the two FILES is `package_check.py`'s job and it does that against
  `Cargo.toml` too.
- **The consumer install is outside the npm workspace**, which is the whole
  point of it. Inside, `@ocelli/core` resolves through the workspace link to
  `src/` and the tarball is never consulted, so the check would pass while the
  defect it exists for was present.
- `vitest.config.ts` sets `passWithNoTests: false`. Vitest passes an empty run
  by default, and this project's rule is that a check which could not run must
  not read as one that ran and was happy. Proved: with the test file moved
  away, `vitest run` exits 1 with `No test files found`.
- The gate calls `npm run test` rather than `npx vitest run`, so the gate and
  `npm test` are one definition.

## The seven mutations

| Mutation | Expected | Observed |
|----------|----------|----------|
| `exports.types` points at a file the tarball lacks | red | `advertises exports...types = './dist/nonexistent.d.ts' and the tarball has no dist/nonexistent.d.ts` |
| `files` reduced to `["dist"]` | red | `tarball has no LICENSE-MIT`, `no LICENSE-APACHE` |
| npm version set to `0.2.0` | red | `is version '0.2.0' and the Rust workspace is '0.1.0'` |
| `@ocelli/react` depends on `^0.9.0` | red | `which is not the version @ocelli/core publishes` |
| `src` added to `files` | red | `tarball carries src/bulk.ts, which a consumer downloads and never uses` |
| `VERSION` changed to `0.1.1` | red | `expected '0.1.1' to be '0.1.0'` |
| the test file removed | red | `No test files found, exiting with code 1` |

All reverted, and `gate packages` is green after.
