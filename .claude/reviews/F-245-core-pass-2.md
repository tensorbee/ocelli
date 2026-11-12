# F-245 bounded client review, pass 2

**Author / reviewer**: GLM client and tests, completed GLM own microscope, independent Codex root review.
**Identity**: core-final-own-source.json binds all five files. dicomweb.ts SHA256 ab104227213966d7c6e047954c2b30fbb4aae52b8afd97475120ccd14b5b6189. Root complete patch SHA256 0d39b11af8a104eaf62c28ab90984589d9b6879cf52eefd9ac7a5e50fde4cb51.
**Result**: 0 defects, 0 smells, 0 nitpicks for the core client slice. No viewer provider, native, browser, performance or full feature acceptance is supplied.

## Findings resolved

Pass 1 cancellation precedence is repaired for the bounded branch after fetch resolves and before status or content classification. Non-aborted status and content refusals remain distinct and the legacy branch is preserved. Root replayed the same six-case independent consumer composition with all cases passing at the reviewed source.

The author subsequently found that invoking receive as a delivery-record method exposed the private record through callback this. The permanent regression actually failed before the fix: the callback enlarged the cap, replaced itself and allowed a second response that should be refused. Root read the exact assertion failure, distinct from compilation or missing implementation. The repaired call captures and invokes the function unbound. Root added independent series and instance QIDO consumer combinations: a same-size second response still invokes the original receiver, and a larger second response still refuses at the captured cap. Both pass. No new public API or frozen-object workaround is introduced.

The earlier URL-constructor and misleading Content-Length repairs remain intact. Byte counting uses actual streamed body bytes, exact cap requires EOF, excess refuses before assembly, abort precedence is checked around pending reads and callback completion, and the receiver is awaited. Own OcelliError identity is preserved, including its DicomwebError subclass. Foreign callback errors become fixed CommitRefused. Constructor primitives and callback are captured rather than read through the caller's mutable options object.

## Actual observations

Root directly ran the complete focused dicomweb/options suite, 45 tests passed, and eight independent composition tests, eight passed. All five handed-off source/test hashes matched before and after both runs. Exact commands, direct exits and complete output are /private/tmp/ocelli-s46-codex/f245-root-core-review/complete-focused.json and root-composition.json with matching logs. TypeScript and ESLint successes are separately author-recorded. Existing callback await and stream-cap sensitivity evidence is preserved at its actual earlier source identity, not relabeled as a fresh whole-feature run.

The public bounded WADO instance, frame and URI controls check bytes, representation metadata and one callback invocation. These are transport controls over opaque synthetic bytes. They are not claims that their tiny payloads are valid multipart DICOM objects or a native parse result. The native consumer is a separate acceptance surface.

## Own review qualifications and limits

The raw final own report puts an observation under defects while explicitly saying it is not a correctness violation. Its claim that the chunk element annotation differs from pushed values is unsupported by source and passing typecheck. Deliberate bounded versus legacy constructor errors are required policy, not a smell. A comment on the either-key selector and equivalent type spelling are preferences. Additional invalid-cache coverage is a nonblocking inventory note, not a demonstrated defect. No product change is requested for these remarks.

This slice establishes no network, native object-heap, allocator, crypto-buffer or RSS bound. The accepted raw payload cap remains the approved claim. Full F-245 still needs its source provider, native transport/projection, canonical page, launcher cleanup, complete browser inventory and release measurements. The clean slice result does not complete the story.
