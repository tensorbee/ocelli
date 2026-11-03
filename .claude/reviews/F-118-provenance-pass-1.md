# F-118 review, pass 1

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `e49919d201a121f493332ac6bd157a2f13ce5ada` (`e49919d`, F-117, refuse a stable claim). `packages/core/src/provenance.ts` and `packages/core/src/provenance.test.ts` are untracked. The other F-118 edits are unstaged in `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, `CHANGELOG.md`, `docs/lld/public-api.md` and `docs/lld/README.md`. This file is untracked. Nothing was staged or committed. `.githooks/commit-msg` is not in the diff.

**Author / reviewer**: uncommitted implementation / 01a1a1ad-4759-7051-9d73-63172272a1e2 / independent pass 1. This reviewer did not write the change.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-118-design.md`. HLD 27.2 R6 says a provenance trailer is on every commit. It does not say how a reader classifies an existing message. The three words are the approved local decision. They are not an HLD formula.

The unit file locks these cases.

- Both trailers, with `profile=`, `gates=`, `corpus=` and `tree=`, return recorded. An extra `comparison=present` still returns recorded.
- A message with neither trailer returns missing. A non-agent `Co-Authored-By` line with neither trailer returns missing.
- A verify line alone returns forged. A generated line alone returns forged.
- A verify line without `tree=` returns forged.
- Two verify lines return forged. Two generated lines return forged.
- `Co-Authored-By: Claude` with both trailers returns forged. `codex`, `GPT` and `co-authored-by: Copilot` return forged.

An independent probe ran on the original function and was not left in the tree. The restore later matched those bytes. The probe also checked cases the unit file does not name. None of the calls threw.

- A CRLF message with both trailers and the four fields returns recorded.
- The four fields in another order return recorded. Tabs between the fields return recorded.
- The ledger shape, with comma-separated gates and comparison fields, returns recorded.
- A generated line that carries the four fields does not save a verify line that lacks `tree=`. The result is forged.
- A leading space before `Ocelli-Verify`, with a normal generated line, returns forged. A leading space before both trailer keys returns missing. Those lines do not start with the trailer key. The hook uses the same start-of-line check. This does not throw.
- The tokens Claude, GPT and Copilot in the subject, and not after `Co-Authored-By:`, return missing.
- `Co-Authored-By: Ada Lovelace` plus both good trailers returns recorded.
- Two different verify lines return forged.
- An empty message returns missing.
- `notprofile=` does not count as `profile=`. The result is forged. The plan says the verify line contains `profile=`. A raw substring search would treat `notprofile=` as `profile=`. The plan's examples do not use that disguised token. Refusing it does not change AC1 to AC7. It is acceptable. It is not a smell. The hook writes a real `name=` token, and calling the disguised token a field would record a line that lacks `profile`.
- `Ocelli-Verify:profile=feature gates=fmt corpus=pass tree=abc`, with no space after the colon, plus one generated line, returns forged. The plan's recorded rule is that the verify line contains `profile=`, `gates=`, `corpus=` and `tree=`. This line contains all four. That result is D1.

AC1's unit example uses a space after the colon, so the suite stays green on that example. AC7 holds. The index binding is the same function. `recorded` does not read a commit, and `missing` is not a success value. The function does not write a trailer.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git status --short` | Worktree before this review file | Executed now | HEAD `e49919d201a121f493332ac6bd157a2f13ce5ada` on `sprint/s41`. Five modified tracked files and two untracked product files. The hook path is absent from the diff. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Worktree text | Executed now | The strings are equal. Both have 50 ids. Both end with F-118. Length 349. |
| `npx vitest run packages/core/src/provenance.test.ts packages/core/src/index.test.ts` | Original `provenance.ts` sha256 `4ff8c6324501b1bc533f44b0d9a28ade2bb632590e0b9e3567540c0e525b8da2` | Executed now, before the mutation | Exit 0. 10 passed. 6 in `provenance.test.ts` and 4 in `index.test.ts`. |
| Node probe of `classifyProvenance` | Same original bytes. The probe was a process, not a file. | Executed now, before the mutation | Exit 0. 29 calls. 0 throws. CRLF, reordered fields, a generated line carrying the fields, a non-agent co-author with both trailers, an empty message, a subject token, `notprofile=`, and the glued colon were classified as stated above. |
| Vite load of `index.ts` and `provenance.ts` | Same original bytes | Executed now, before the mutation | Exit 0. The index binding is the module function. An empty message is missing. A spaced recorded message is recorded. |
| Same provenance vitest command after the final return changed from recorded to missing | Mutated worktree, not staged | Executed now | Exit 1. 1 failed and 5 passed. `returns recorded when both trailers carry the four fields` expected recorded and received missing at `provenance.test.ts:16`. |
| `cmp`, `shasum -a 256` and `wc -c` after copy-back | Restored bytes | Executed now | Compare empty. sha256 `4ff8c6324501b1bc533f44b0d9a28ade2bb632590e0b9e3567540c0e525b8da2`. Length 1314. |
| `npx vitest run packages/core/src/provenance.test.ts packages/core/src/index.test.ts` | Restored tree | Executed now | Exit 0. 10 passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | Worktree sources after the restore | Executed now | Exit 0. |
| `npx eslint` on `provenance.ts`, `provenance.test.ts`, `index.ts` and `index.test.ts` | Those four files, after the restore | Executed now | Exit 0. ESLint 9.39.5. |
| Python scan of the new prose | Worktree text | Executed now | No em dash and no prose semicolon in the changelog bullet, the provenance section, the new README phrase, or the module comment. |

Environment: Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

### D1, a verify field glued to the colon is forged

**Where**: `packages/core/src/provenance.ts:15` and the call at `packages/core/src/provenance.ts:40`

**What**: `hasField` splits the whole verify line on whitespace and keeps a token only when that token starts with `name=`. On `Ocelli-Verify:profile=feature gates=fmt corpus=pass tree=abc` the first token is `Ocelli-Verify:profile=feature`. It does not start with `profile=`. The other three fields match. With one `Ocelli-Generated-By` line and no agent co-author, the function returns forged.

**Why it is wrong**: The approved plan says recorded when there is one `Ocelli-Verify` line, one `Ocelli-Generated-By` line, and the verify line contains `profile=`, `gates=`, `corpus=` and `tree=`. This message meets that rule. Forged is for a verify line that lacks one of those four fields. This line does not lack them. `docs/lld/public-api.md` states the same recorded rule. The ledger field split takes the text after the first colon and then splits on whitespace, and that split sees `profile`. This is the plan's local rule, not an HLD formula.

**Evidence**: Node probe of the original bytes, before the mutation. The input was the glued verify line plus `Ocelli-Generated-By: worker`. The result was forged. The call did not throw. The unit file does not name this input, so the provenance vitest run stayed at exit 0.

**Scope and regression**: The published `classifyProvenance` export. Hook-written trailers put a space after `Ocelli-Verify:` and are not this input. A message that glues the first field to the colon is classified forged. The smallest regression is that one message, expecting recorded. The defect is in the uncommitted tree, not in HEAD.

## Smells

None.

## Nitpicks

None.

## Verified clean

- AC1 for the spaced shape. Both trailers and the four fields return recorded. An extra comparison field stays recorded. Field order does not matter. A CRLF message stays recorded. Requiring comparison fields would have failed the unit file and the probe. The final return is what the recorded assertion locks. Changing it from recorded to missing went red at `provenance.test.ts:16`.
- AC2. Neither trailer returns missing. The unit file locks an open message and a non-agent co-author with no trailers. The probe locks an empty message. An agent line is not classified as missing.
- AC3. One trailer without the other returns forged. The unit file locks each side. A single line that carries both keys is one verify line and no generated line, so it is forged.
- AC4. A verify line without `tree=` returns forged. The same loop returns forged when `profile=`, `gates=` or `corpus=` is absent and the other three are present. Fields on the generated line do not fill the gap.
- AC5. A duplicated verify line returns forged. A duplicated generated line returns forged. Two verify lines with different values also return forged.
- AC6. An agent co-author returns forged with both trailers and with no trailers. The match is case-insensitive for Claude, Codex, GPT and Copilot, and only on a line that starts with `Co-Authored-By:`. A subject that merely names those tokens stays missing. A non-agent `Co-Authored-By` line plus both good trailers stays recorded.
- AC7. `index.ts` exports `classifyProvenance`. The published runtime list names it. The restored index test matched `Object.keys` of the module. The Vite load saw the same function.
- `notprofile=` is refused. That is the token rule applied to a disguised name the plan's examples do not use. It does not change a required acceptance row. It is acceptable.
- A leading space before a trailer key is not that trailer. One indented verify line beside a real generated line is forged. Two indented keys are missing. Neither call throws.
- The function does not write a trailer, does not call the hook, and does not compare `tree=` with a commit. `missing` is not success. Comparison fields are not required.
- `provenance.ts` has no `addEventListener`, no `wasm-bindgen`, no `unsafe`, and no network call.
- The changelog bullet is the first item under `Unreleased` and `Added`. It does not add a version heading. It says the function does not write a trailer.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-118. Both have 50 ids. Length 349.
- Tiers A, B and C are not applicable in the public-api section. The function does not render and does not branch on a tier.
- The new prose has no em dash and no prose semicolon. That covers the changelog bullet, the provenance section, the new README phrase and the module comment.
- The type-only parenthetical in `packages/core/src/index.test.ts` still stops at `MatrixTier`. This diff adds no type export and does not edit that parenthetical, so it is not a new finding.
- `.githooks/commit-msg` is outside the diff. No new deviation is recorded. The design records none.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0 after the restore. The restored vitest run exited 0.

## Limitations and handoff

The floor gate and the corpus gate were not run. No browser was launched. The story was not committed. `git diff` cannot show a restore of `provenance.ts`, because that file is still untracked. The sha256 match and the empty byte compare are the restore evidence.

The probe ran before the mutation. The restored file is byte-identical to the probed file, and the restored suite locks the spaced recorded result again.

The spaced acceptance rows match the approved plan. D1 is the open failure. The disguised `notprofile=` refusal is not part of that failure.
