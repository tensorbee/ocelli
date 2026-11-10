# F-245 borrowed source review, pass 1

**Author / reviewer**: GLM source and tests plus completed readable own microscope with format failure preserved, independent Codex root review.
**Identity**: native-borrowed-root-review-manifest.json binds dicomweb.rs SHA256 f2f214d83cec2f8ccc18ffedc13d3f884558a3d63b9a029127fa4869c70f75d5, lib.rs SHA256 9130e69ae02432698c93cf8b7053b66a68e01bc49b9d38cf0edc15bb48623604 and tests/dicomweb.rs SHA256 d96a1dce6f89b223edf69bf2fe406805112c15b163ba96d6b32eb6ecb8adf6b8.
**Result**: 0 defects, 0 smells, 1 nonblocking documentation nit. Scope is the borrowed DICOMweb provider only. Part10 internals retain their separate earlier review. No native viewer consumer or full feature acceptance is supplied.

## Source and actual consumer combination

The borrowed entry checks the encoded body bound, admits only QidoJson and WadoRsInstances, propagates the decoded cap to each bounded Part10 parse, and maps its SizeLimit to the source limit class. Multipart count is checked before admitting another part and before the caller parses any Part10 payload. QIDO applies max_rows only at the root array. Nested attribute arrays keep duplicate-member checking and remain subject to the body bound, with no object-heap claim. Legacy owned consumption retains all four representations and passes no new limits to its shared helpers.

Root directly reran the full DICOMweb integration suite, 19 tests passed, with all three reviewed hashes checked before and after. The separate private consumer control constructs two independently hand-encoded valid Part10 objects with distinct Rows values. It accepts both at an exact per-part decoded cap, retains both distinct values, rejects a one-byte-tighter cap and confirms legacy consumption stays uncapped. This also exercises max_rows zero with a WADO batch, rather than treating every numeric limit as a common batch policy. The actual selected private test passes. Its first invocation accidentally used an abbreviated name with --exact and selected zero tests. That root command construction error is retained separately and supplies no behavioral evidence. The corrected full exact name selected one test and passed.

Receipts and complete output are /private/tmp/ocelli-s46-codex/f245-root-native-review/borrowed-full.json and borrowed-composition.json with their logs. Author fmt and scoped clippy successes remain separately recorded. Root inspected all four author mutant logs: encoded and root-row limit changes fail their SizeLimit assertions, removing the part-count cap reaches invalid Part10 part 1 instead of SizeLimit, and removing decoded-limit propagation incorrectly succeeds. Each is semantic assertion RED, not compilation failure. Restored source and 19 tests pass at the bound identity.

## Own report qualifications

The raw own response has readable content but invalid JSON from an unescaped quoted word. No repaired JSON clean report is claimed. Its seven smells do not establish a new contract defect. Non-array error precedence is the same InvalidJsonRoot in both paths. Root-only row counting and unsupported representation refusal are intended caller policies. Media-type order, delimiter allocation and content-length parsing describe existing behavior. The claimed repeated-scan complexity has no demonstrated witness. The 19 tests and four separate failing mutant runs remain distinct observations.

## N1, make the limits documentation local and precise

In a later owned documentation edit, describe decoded_max as decoded raw data-set bytes per Part10 part, with the existing ParseLimits exclusions for File Meta and completion sentinel. State that it does not bound aggregate parsed-object heap or RSS. The consume_borrowed Errors list should include root-row SizeLimit and explain that the encoded limit is checked before representation dispatch. No code behavior or new error precedence is requested. The independent two-object control above records the actual per-part contract.

## Remaining acceptance

The native viewer still needs its HTTP limits, QIDO projection, actual UID tuple and digest checks, one-part policy, reply cap, safe error projection and browser consumer. This generic API has no viewer defaults. The new method borrows its input slice while returning parsed objects. It does not retain the original raw response as its output. No clinical files, corpus rows, browser, GPU or release performance evidence is supplied by this slice.
