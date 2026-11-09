# S46 close-backlog filing draft, review pass 1

**Input**: frozen composition tree `31323a793ea671c11ec1c86374d8bb26f67f3276`.
**Reviewed first output**: tree `2426840c8fdf021cc6c2fadfdfdc2255d6513e8a`.
**Roles**: local GLM authored the first seven draft plans, the F241 append and
three proposed filing files. Codex f244_review independently reviewed that
first output, then authored the bounded corrections below. Root independently
read the drafts and supplied four additional defects, a nit and an isolation
control gap. Root review of the corrected delta remains required. This report
is not approval by an author of its own repair.
**First-output result**: six draft defects, zero smells, one nit, plus one
preventive isolation gap. These are future design and filing defects, not
new observations of executable product failures.

## Scope preserved

The original Claude archive contains nine items. Item2 is already superseded
by F244 and is omitted. Item9 remains four notes in the existing F241 plan.
Seven surviving scopes map to F266 through F272, with E38.61, E38.62, E19.8,
E4.14, E4.15, E38.63 and E2.8 respectively. No F273 was invented. All seven
statuses remain draft/pending, never approved or complete. No experiment,
implementation, corpus change or actual filing occurred.

S47 proposes F266/267/268/269/272 with estimates1/2/1/1/1w. S48 proposes
F270/271 with estimates3/2w. All eleven weeks are inherited provisional lead
suggestions, not capacity approval. Existing M21 authorization and wave
estimates remain intact. The seven inherited M20/P1 follow-ups are explicitly
outside those authorized wave totals. Single-viewer delivery is preserved.

Source inspection confirms every existing allocation record is unchanged.
Exactly seven records were appended. BACKLOG gains seven pending rows.
SPRINT_PLAN changes two generated Goal lines, adds seven rows, adjusts the
M20 count45/weeks146 and appends explanatory allocation prose. It does not
replace the generated tail or remove historical warning prose. The F241
change is append-only and preserves its row, status, estimate and dependencies.

## Findings and bounded corrections

| ID | First-output defect | Finder | Correction authored by Codex f244_review |
|----|---------------------|--------|-----------------------------------------|
| D1 | F269 names the immutable original PACKAGE-INVENTORY as a refreshed pins input and omits the actual patched hash source from its write set | f244_review | Keep original published inventory immutable. Name PATCH-PROVENANCE and patched/provenance hashes in scripts/pin_and_size_check.py. |
| D2 | F271 AC7 assumes the alternative loses on some actual row and that any worse distance makes AC12 fail, neither guaranteed | f244_review | Hand-written mixed, tied and unanimously improved distance controls challenge the strict all-rows adoption predicate without running the matrix experiment or changing AC12. |
| D3 | F268 scope denies both lints while AC2 permits leaving broken links unenforced and AC5 unconditionally demands its mutant fail | root | Consistently propose both lints denied through explicit RUSTDOCFLAGS. Floor/sprint placement remains subject to normal workflow approval. Remove the unproven claim that a manifest lint-table shape applies to every cargo doc. |
| D4 | F270 describes Annex F as codestream/coefficient bit-planes | root | Independently opened the official ITU T800 07/2024 contents. E is quantization, F discrete wavelet transformation, D coefficient bit modelling and A syntax. Add the primary citation, with no diagnosed-cause claim. |
| D5 | F270 says scheduling has no ordering constraint and a merely staged F239 union precedes its tip | root | Require both dependencies DONE before implementation, reschedule after actual completion if carried. |
| D6 | F271 AC4 derives expected fixtures from the edited product constant | root | Derive expected fixtures independently from published primaries and Bradford hand arithmetic before the constant edit, then rerun the full Little CMS comparison. |
| N1 | F270 claims S46 vendor work touched quantization and wavelet files without supported evidence | root | Remove the unsupported statement, retain source locations only as investigation starting points. |

Additional preventive gap raised by root: F272 PID-only directories can
collide between parallel tests and repeated helpers in one process. The draft
now requires unique invocation directories, concurrent independent controls,
and disposable fixture roots for absence and planted-writer cases. A possible
standalone scripts/tests/test_oracle_output_isolation.py is a coordinated
future write-set proposal, not a new gate or implemented test.

The pinned local Cargo manifest reference gives generic namespace guidance,
but this review did not execute a manifest-table experiment and does not
certify an untested workspace rustdoc table. The local rustdoc lint reference
states that its link lints are available during rustdoc, not rustc. The draft
therefore uses explicit flags on its proposed named documentation command.

## Original draft inconsistency handled by GLM

The recovered rustdoc prose says eleven private-link positions, while its own
table enumerates ten private positions plus one unresolved warning. GLM
correctly retained the historical table as historical evidence, removed the
unsupported current count and requires a fresh enumeration at implementation.
The original proposed floor placement was correctly kept a future workflow
decision. These two resolutions were present before controller corrections.

## Actual model evidence and limits

Run s46-close-filing-glm-task2 used dgx/GLM-5.3-Flash-EXL3, from
2026-11-08T17:31:36.485243+00:00 to 18:22:00.686534+00:00. Direct launcher
exit0, wall3024.193381 seconds, zero launcher retries. This was a seven-plan
filing draft task, not a controlled comparison with Claude or Codex. No
isolated controller/reviewer wall-time measurement exists. Raw provider token
and cache counters remain in their original provider form, with unknown
normalization. No ranking or token equivalence is claimed.

Full prompt, start/end receipt, usage/session outputs, log and immutable first
output are under the private s46-close-filing-glm-task2 evidence directory.
The first patch SHA256 is
`2092e6989047bd507093e67f561053a94d0e3e54eabac5d6b580a4c07885a5ab`.
The capture records controller_corrections_before_capture false. All eleven
first-output files and their hashes remain immutable after these repairs.

The GLM log reports static gen_sprint_plan --check, backlog_check and prose
checks exit0, plus source/text consistency inspections. These are author
checks at the original output, not new independent checks of the corrected
plans. The original prose check used git ls-files while seven new plans were
untracked, so it did not cover those seven files. Their later staged check
must be identified separately. No cargo doc, build, native test, Node suite, browser, GPU job or
actual matrix experiment ran. Controller validation is held during the
lead's exclusive measurements. This handoff records source review only and
must not be presented as a passing feature or sprint verification.

## Remaining acceptance

Root reviews the corrected plan delta. After the performance hold releases,
run staged metadata/content/prose checks in this private proposal, preserving
all original bytes and identifying the corrected tree. Actual filing happens
only at sprint close through the existing workflow. Every draft retains its
unresolved design choices, primary-clause review requirements and future
implementation evidence ownership. No canonical backlog, allocation, plan,
review or candidate was written by this preparation.
