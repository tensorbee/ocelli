# F-245 WADO provider review, pass 3

**Reviewed**: Worker `/private/tmp/ocelli-s47-f245-glm`, HEAD d85406a5eed7d4340b3b4e6db961631569bff99a, dirty tree preserved. Hashes verified before and after:

| Path | SHA-256 |
|------|---------|
| packages/core/src/dicomweb-viewer.ts | 2817635599c9fb05fbf3d7a9186a87abfaee404748468c7ac9c5f4efb82bbdcf (unchanged since pass 2) |
| packages/core/src/dicomweb-viewer.test.ts | a2481dde9e3d8bf82ce25b0959b2fc7c10d222a3d1ccfef029ffc98b31416e39 |
| packages/core/src/index.ts | d84fc60627d8f5ff0c04f9a26a8e4b124b7a833cc1aa48ccd768894a948cdd4b |
| packages/core/src/options.ts | 5c4d313550fb9941b1119fb18d5bf4621e8960ebdd4765b6b19ec540156b5597 |

Test delta reviewed against `/private/tmp/ocelli-s46-codex/f245-implementation/claude-provider-test-before-uid-receipt.ts`.

**Author / reviewer**: GLM authored the test additions, which Codex applied. Reviewer is Claude (root independent reviewer, subagent), who authored none of the reviewed material.

**Result**: 0 defects, 0 smells, 5 nitpicks.

## Acceptance challenged

AC9's same-length wrong UID and length-only rule, and the S1 amendment's malformed-receipt refusal, through the pass 2 mutants that survived plus one new receipt mutant the coordinator's receipt did not name, a length-only series comparison in `takeStagedInstance`.

## Evidence

| Command / selected cases | Source identity | Executed now / prior | Exit and observation |
|--------------------------|-----------------|----------------------|----------------------|
| Three-file vitest command, worker root, Node 24.13.0, Vitest 4.1.11 | 28176355, a2481dde | Executed now | 0, 123 passed |
| LEN_UID, native projection `instanceUid` compared by length, provider suite | 25d0d288 | Executed now | 1, only "same-length wrong instance UID" projection case fails, 77 passed |
| R1_KEYS, outer `exactKeys` on the receipt removed | 3ec679aa | Executed now | 1, only "extra top-level key" fails, 77 passed |
| LEN_RCPT_SERIES, receipt `seriesUid` compared by length | 20997476 | Executed now | 1, only receipt "same-length wrong series UID" fails, 77 passed |
| Scratch restored, worker rehashed | 28176355 | Executed now | match |
| Other length and key mutants, tsc, ESLint | Coordinator receipt | Prior receipt only | 1 per mutant, 0, 0 as recorded |

## Defects

None.

## Smells

None. S2 and S3 from pass 2 are closed.

## Nitpicks

- **N1 to N4**: carried as dispositions from passes 1 and 2.
- **N5**: carried. The pre-aborted control holds no live operation. Not raised to a smell because the realistic regression is caught.
- The "missing top-level key" receipt case does not single out `exactKeys`, since the `typeof byteLength` check refuses it as well. It is a valid refusal case, not a tautology. The extra-key cases are the ones that discriminate, and R1_KEYS shows they do.

## Verified clean

Each new case changes exactly one field of an otherwise valid receipt or DTO and is followed by a successful take or refetch on the same provider, so a provider that refused everything would fail. The same-length UIDs `2.25.9`, `2.25.8` and `2.25.7` each match the length of the correct value and differ from it. No eslint disable was added. The production source is unchanged, so the pass 1 and pass 2 clean findings still apply.

## Limitations and handoff

Native HTTP, page, CSP, browser, performance and memory are not exercised. No browser, cargo or GPU was run. This is a bounded clean review of the four files, not F-245 acceptance.
