# F-118 review, pass 2

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `e49919d201a121f493332ac6bd157a2f13ce5ada` (`e49919d`, F-117, refuse a stable claim). `packages/core/src/provenance.ts` and `packages/core/src/provenance.test.ts` are untracked. The other F-118 edits are unstaged in `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, `CHANGELOG.md`, `docs/lld/public-api.md` and `docs/lld/README.md`. The pass 1 review is untracked and was not edited. Nothing was staged or committed. `.githooks/commit-msg` is not in the diff. Pass 1 finding D1 is closed.

**Author / reviewer**: uncommitted implementation / 01a1a1ba-79c5-78b3-90e7-67b075efbd03 / independent pass 2. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-118-design.md`. HLD 27.2 R6 requires a provenance trailer on every commit. It does not say how a reader classifies an existing message. The three words are the approved local decision. They are not an HLD formula.

Pass 1 D1 was the verify line `Ocelli-Verify:profile=feature gates=fmt corpus=pass tree=abc` plus one generated line. The old split treated `Ocelli-Verify:profile=feature` as one token, so the line was forged. The plan says recorded when that line contains `profile=`, `gates=`, `corpus=` and `tree=`. The remediation searches for each `name=` and accepts it unless the character before it is an ASCII letter, digit, or underscore. The exact two-line input now returns recorded. D1 is not carried forward.

The adjacent case uses the same four fields in another order and glues the first field to the colon. `Ocelli-Verify:tree=abc corpus=pass gates=fmt profile=feature` plus `Ocelli-Generated-By: worker` returns recorded. The same shape with `gates` first, and again with `corpus` first, also returns recorded.

`notprofile=` stays forged. That includes the colon-glued form `Ocelli-Verify:notprofile=feature gates=fmt corpus=pass tree=abc`. Pass 1 said this refusal is acceptable and is not a smell. A blocked `notprofile=` followed by a real `profile=` on the same line still returns recorded, so the search does not stop at the first hit.

The unit file has 7 tests. One recorded assertion is the glued colon. One test expects `notprofile=` to be forged. Changing the predecessor class so a colon blocks a field made only the glued assertion go red.

`recorded` does not read a commit. `tree=not-a-commit` still returns recorded. `missing` is a different word from `recorded`. Comparison fields are not required. A ledger-shaped line with them returns recorded, and the spaced line without them returns recorded. The function does not write a trailer and does not call the hook.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git log -1 --oneline` | Worktree | Executed now | HEAD `e49919d201a121f493332ac6bd157a2f13ce5ada` on `sprint/s41`. Subject is `e49919d F-117, refuse a stable claim`. |
| `git status --short` and `git diff` on `.githooks/commit-msg` | Worktree before this review file | Executed now | Five modified tracked files, two untracked product files, and the untracked pass 1 review. The hook path is absent from the diff. |
| Python compare of the public-api F-ID header with the last cell of the public-api row | Worktree text | Executed now | The strings are equal. Both have 50 ids. Both end with F-118. Length 349. |
| Vite load of `classifyProvenance` from `provenance.ts` and `index.ts` | Original bytes sha256 `3b239a28c11c1ea9e41a61f746ff30782f05d6222482d0ec386f842eaa5beef2`, length 1569 | Executed now, before the mutation | Exit 0. The index export is the same function. 48 calls. 0 throws. |
| Exact two-line D1 replay and the adjacent glued order | Restored bytes, same sha256 | Executed now, after the restore | D1 returned recorded. `Ocelli-Verify:tree=abc corpus=pass gates=fmt profile=feature` plus one generated line returned recorded. Colon-glued `notprofile=` returned forged. No call threw. |
| Wider probe on the same bytes | Original bytes, before the mutation | Executed now | Gates-first and corpus-first glued forms returned recorded. Tabs, CRLF, reordered fields, and a generated line placed before the verify line returned recorded. Each single missing field returned forged. One trailer, a repeated trailer, both keys on one line, and an agent co-author returned forged. An empty message returned missing. |
| `npx vitest run packages/core/src/provenance.test.ts` after the predecessor class also rejected `:` | Mutated bytes sha256 `55a3f463d52e57a40ba2a124cb40cb93489954268118e9296548903c73c1a812`, not staged | Executed now | Exit 1. 1 failed and 6 passed. `returns recorded when both trailers carry the four fields` expected recorded and received forged at `provenance.test.ts:24`. |
| `cmp`, `shasum -a 256` and `wc -c` after byte copy-back | Restored bytes | Executed now | Compare empty. sha256 `3b239a28c11c1ea9e41a61f746ff30782f05d6222482d0ec386f842eaa5beef2`. Length 1569. |
| `npx vitest run packages/core/src/provenance.test.ts packages/core/src/index.test.ts` | Restored tree | Executed now | Exit 0. 11 passed. 7 in `provenance.test.ts` and 4 in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | Restored sources | Executed now | Exit 0. TypeScript 5.9.3. |
| `npx eslint` on `provenance.ts`, `provenance.test.ts`, `index.ts` and `index.test.ts` | Those four files, after the restore | Executed now | Exit 0. ESLint 9.39.5. |
| Python scan of the new prose | Worktree text | Executed now | No em dash and no prose semicolon in the changelog bullet, the provenance section, the new README phrase, or the module comment. |

Environment: Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass 1 D1 is closed. The exact input `Ocelli-Verify:profile=feature gates=fmt corpus=pass tree=abc` plus `Ocelli-Generated-By: worker` returns recorded. The call does not throw. The unit file locks that input. The predecessor mutation, which also treats `:` as a blocking character, turned that assertion red at `provenance.test.ts:24` and left the other six tests green. The spaced recorded assertion in the same test had already passed.
- The search does not add a new refusal for a real field. A field glued to the colon is recorded for `profile`, `gates`, `corpus` and `tree`. A field after a space, a tab, or a punctuation mark is recorded, which is what `contains` `name=` asks for. A comma in `corpus=pass,tree=abc` and a hyphen in `corpus=pass-tree=abc` still return recorded. Neither character is an ASCII letter, digit, or underscore. Refusing them would repeat the token-start failure from D1.
- The identifier boundary from pass 1 still holds. `notprofile=`, `_profile=`, `9profile=`, `my_tree=` and `street=` return forged. The longer-token test stayed green under the colon mutation. A blocked prefix does not hide a later real field.
- AC1. Both trailers and the four fields return recorded. Field order does not matter. An extra comparison field stays recorded. The ledger shape, with comma-separated gates and comparison fields, returns recorded. Requiring comparison fields would have failed the unit file and the probe.
- AC2. Neither trailer returns missing. The unit file locks an open message and a non-agent co-author with no trailers. The probe locks an empty message. An agent line is not classified as missing.
- AC3. One trailer without the other returns forged. The unit file locks each side. A single line that carries both keys is one verify line and no generated line, so it is forged.
- AC4. A verify line without `tree=` returns forged. The same loop returns forged when `profile=`, `gates=` or `corpus=` is absent and the other three are present. Fields on the generated line do not fill the gap. A space before `=` does not count as `name=`.
- AC5. A duplicated verify line returns forged, including when the two values differ. A duplicated generated line returns forged.
- AC6. An agent co-author returns forged with both trailers and with no trailers. The match is case-insensitive for Claude, Codex, GPT and Copilot, and only on a line that starts with `Co-Authored-By:`. A subject that merely names one of those tokens stays missing. A non-agent `Co-Authored-By` line plus both good trailers stays recorded. An indented co-author line is not that trailer.
- AC7. `index.ts` exports `classifyProvenance`. The published runtime list names it. The restored index test matched `Object.keys` of the module. The Vite load saw the same function.
- A leading space before a trailer key is not that trailer. One indented verify line beside a real generated line is forged. Two indented keys are missing. Neither call throws.
- The function does not write a trailer, does not call the hook, and does not compare `tree=` with a commit. `missing` is not success. Comparison fields are not required.
- `provenance.ts` has no `addEventListener`, no `wasm-bindgen`, no `unsafe`, and no network call. The only code references are the definition, the index re-export, and the two test files.
- The `line === undefined` branch is the `noUncheckedIndexedAccess` narrowing after a length check that already returned. It is not a second classification rule.
- The changelog bullet is the first item under `Unreleased` and `Added`. It does not add a version heading. It says the function does not write a trailer, and that missing is not success.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-118. Both have 50 ids. Length 349.
- Tiers A, B and C are not applicable in the public-api section. The function does not render and does not branch on a tier.
- The new prose has no em dash and no prose semicolon. That covers the changelog bullet, the provenance section, the new README phrase and the module comment.
- The type-only parenthetical in `packages/core/src/index.test.ts` still stops at `MatrixTier`. This diff adds no type export and does not edit that parenthetical.
- `.githooks/commit-msg` is outside the diff. No new deviation is recorded. The design records none.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0 after the restore. The restored vitest run exited 0.

## Limitations and handoff

The floor gate and the corpus gate were not run. No browser was launched. The story was not committed. `git diff` cannot show a restore of `provenance.ts`, because that file is still untracked. The empty byte compare and the sha256 match are the restore evidence. The restore was a byte copy of the pre-mutation file.

The wide probe ran before the mutation. The exact two-line replay ran after the restore. Both saw the same sha256. The restored suite locks the spaced result and the glued colon again.

No new defect was found in the field search. D1 is closed.
