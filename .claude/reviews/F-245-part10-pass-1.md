# F-245 bounded Part10 review, pass 1

**Author / reviewer**: GLM parser and fixtures plus completed own microscope, independent Codex root review.
**Identity**: native-part10-root-review-source.json and native-part10-root-review.patch bind parse.rs SHA256 7a806bdc6b8cd5e4766c65b2067b56ba6b532bd2bc5889b08ab306cd73da5692, lib.rs SHA256 83dbc8a002b40e7ad914952a1d765e2574158f6c35688f5bfc4d3df1fbc794dc and tests/part10.rs SHA256 f7e65801b23356b5ebe40bf16221e4b226837795035ef835c659ddc5d6d183de.
**Result**: 0 defects, 0 smells, 0 nitpicks in this bounded Part10 slice. No whole feature, borrowed source API, WADO projection, browser, corpus or performance acceptance is supplied.

## Source and policy

The new ParseLimits wrapper keeps the old entrypoint uncapped. Native and encapsulated dataset routes check encoded dataset length before copying. Adapted dataset readers append only within remaining room, retry Interrupted, accept short nonzero reads, probe after an exact cap to distinguish actual EOF, and refuse excess output before vector growth. The Deflate route retains its existing consumed-stream and required padding checks. File Meta Information and the completion sentinel remain excluded as documented. No total heap, capacity, object allocation or RSS bound follows from the decoded byte cap.

Root inspected the source and complete changed tests. The own report identifies no concrete defect. Its proposed richer I/O diagnostics conflict with no required behavior and the fixed parser error is consistent with existing patient-safe error policy. Equivalent helper extraction, comments, placement of a constant and defensive slicing preferences are not blockers. The encoded versus decoded route distinction is accurate, not a smell. The raw report remains preserved at native-part10-own-microscope-off-v1/content-parsed.json.

## Independent observations

Root directly reran the complete existing Part10 suite, 61 passed, and the four real capped-reader controls, four passed, at the exact parse source above. Receipts and complete output are /private/tmp/ocelli-s46-codex/f245-root-native-review/part10.json and helper.json with their matching logs. These are focused runs, not a whole-crate claim.

To challenge the shared-entrypoint assumption through different consumers, root replayed the existing complete 61-fixture suite in a private harness, mechanically replacing only its legacy parse entry with parse_part10_with_limits at a generous 128 MiB cap. All 61 passed, covering implicit VR, big endian, encapsulated empty offset tables, nested icons, Deflate, structural refusals and sequence-depth controls through the new bounded consumer. No fixture bytes or worker source changed. The private bounded-consumer/run.json and run.log record the result and source identity. The first harness attempt failed to compile because root omitted its direct dicom-core dependency. That setup failure is separately retained and is not product RED. The corrected manifest replay exited 0.

Root also read the actual author mutant logs. Treating a short read as EOF fails at the expected full byte vector, not compilation. Failing to retry Interrupted fails at the expected successful result. Permitting a one-byte cap excess fails both explicit and Deflate SizeLimit assertions. Restored source and controls pass. These are author-run sensitivity controls independently checked by root, distinct from root execution.

## Remaining work

The borrowed source API test currently requires missing implementation. Its compile failure is not evidence against this parser slice and does not constitute semantic RED. The borrowed parser, native transport/projection, core provider, canonical page, launcher, browser suites and release measurements still require implementation and their own reviews. Do not convert this clean slice result into completed F-245 status.
