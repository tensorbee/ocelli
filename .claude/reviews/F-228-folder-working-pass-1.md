# F-228 folder review, working pass 1

**Reviewed:** staged tree `f76a42fd9d40194e5a2eb62321bff23c93068175` on HEAD `58e71431f0f4e737e29122b0f4c920ec061feee2`. Both staged-to-snapshot and unstaged diffs were empty before and after execution. The bounded delta against integration baseline `61efc91b` has 14 paths. The entire `csp.rs` and `csp-probe.mjs` were reviewed, including the module material already present at that baseline. The inherited F-215/F-225 implementation was examined only where it supplies the actual page, import map, library providers and browser composition. This is folder pass 1, following example-half pass 7. It is not a repeat of that example-half review.

**Authors / reviewer:** `dgx/GLM-5.3-Flash-EXL3` authored the policy module and its 15 tests. Codex root authored runtime wiring, the typed page identity, static style replacements, docs and AC2 HTML quote support. The independent `f244_review` Codex agent authored the browser probe. This reviewer authored none of those product changes. Reviewer-generated drivers and mutants stayed under `/private/tmp`.

**Result:** 0 defects, 0 smells, 0 nitpicks.

## Scope and source identity

The 14 paths are the design amendment, README, TypeScript packaging LLD, ESLint config, folder probe, `csp.rs`, `lib.rs`, `more.html`, `page.html`, `serve.rs`, `use.html`, HTTP preview tests, the embedding policy comment and its quote test. The entire approved plan and example pass 7 were read as context. The delta introduces no tolerance change or gate bypass.

The production module SHA-256 is `01c2d6c25f775684ff22af092cbdf09ca033bb2a80160584118c86deb3d8aba1`. The probe SHA-256 is `1f1d7c2f4756dded0f544e34d1f1d35670b454bf8af7c753bbaa7621ec93e460`. An exact `git archive` of the staged tree is at `/private/tmp/ocelli-f228-review-folder-p1`. Receipts, direct command exits, independent drivers and mutation logs are at `/private/tmp/ocelli-f228-review-folder-p1-evidence`.

The final snapshot was rebuilt with `cargo build --locked --release -p ocelli-folder-viewer`, two jobs, exit 0 in 12.53 seconds. Release binary SHA-256 was `fbb1f8d38682b4d00dcddb5ee46f213fcda4cd31097aace8e7c1e5b46534bc9a`. All browser evidence below uses that build, rather than the author's earlier tree.

## Acceptance challenged

**AC11, exact extraction and encoding.** The scanner was read in full against the three trusted embedded literals. It preserves body bytes, excludes scripts with an actual `src` attribute, handles quoted attributes containing `>` and does not rescan script text. Both main-page script elements, including the import map, contribute hashes. Main, Use and More have respectively two, one and zero inline scripts, and each has one style element. Chromium's inert DOM parser independently supplied the actual element text, and Node crypto supplied the SHA-256 and base64. Every complete policy matched byte for byte. This tests browser text semantics against the production byte scanner on the actual supported sources. It does not establish a general HTML parser contract for arbitrary malformed input.

The [CSP3 element matching rule](https://www.w3.org/TR/CSP3/#match-element-to-source-list) hashes UTF-8 element source. [RFC 4648 section 10](https://www.rfc-editor.org/rfc/rfc4648#section-10) supplies the base64 vectors. The direct native tests pin those vectors and SHA-256 of `abc`. The reviewer also ran all 15 tests in an isolated copy, plus actual-page expected policies and a 24-thread barrier test. Each thread repeatedly received the same policy address for its page, with different pages receiving their own expected policy.

**Cache and header identity.** The sole production caller passes the source selected by private `HtmlPage`. `html_page` derives the response body from that same identity, and `write_reply_by` derives its policy from the identity. Other reply constructors initialize it to `None`. Aliases select the same identity. `OnceLock` initializes one mutex, and lookup, policy construction and insertion occur under that mutex. Consequently concurrent first use cannot insert a second value for the same literal. The process-lifetime allocation is bounded by the three production literals. No request text enters that cache key.

**AC12 and AC13, actual HTTP and static DOM.** Ninety-six responses in batches of six covered all six aliases, with no admission refusal or retry. Browser-parsed body text and independently derived complete policy matched for every response. Alias bodies and policies matched. The probe recursively inspected inert template content, detected its planted mixed-case style and event attributes, and found none in the actual static pages. Each of the three pages loaded under its policy with no CSP violation or page error. The main page booted. The two CSS replacements retained computed SVG position `absolute` and More-page top margin `40px`.

The author's one-space main-script mutation retained the original header and was blocked as an inline script. The main function remained undefined. Independently, removing only the import-map hash blocked the map while allowing the main script to boot, and importing `@ocelli/core` failed with a module-specifier TypeError. Removing only More's style hash caused a style CSP violation and changed its computed margin. These failures are semantic controls, with the other policy sources retained.

**AC14, compressed pixels.** Under the unchanged exact policy, all seven approved lossless corpus rows equalled the uncompressed reference's actual backing-canvas RGBA. The reference had varied pixels. Generation and active-canvas paint instrumentation were reviewed, then challenged independently: a JPEG-LS preview reply was actually intercepted, every grey byte inverted, and fulfillment delayed 350 ms. The probe rejected that case specifically because JPEG-LS backing pixels differed from native. One corrupted response was consumed. It did not pass by reading an identical retained canvas from the preceding file.

**AC15 and AC16, dependent flows.** The actual region gesture loaded the core provider through the import map, changed the window and changed painted pixels. A rectangle was drawn, Save marks produced a real DICOM Part 10 download, and a fresh page loaded that file with matching type, point count and coordinates. No CSP or page error appeared. Those positive flows, including compressed decode, ran without `wasm-unsafe-eval`. The browser version was Chromium `151.0.7922.34`.

**AC2 extension and prose.** The existing embedding policy implementation is unchanged apart from its comment. The marked quote parser adds the Use page and requires its HTML opener, one exact header line and closing delimiter. In the archived snapshot, original and restored states passed. Independently changing the Use header to permit `unsafe-eval`, removing its marker, and replacing its HTML opener with a Markdown fence each failed with the corresponding Use-page finding. README and LLD describe the actual caching, hashing and probe behavior. Use and More consistently distinguish the embedding worker policy from the local folder policy. ESLint adds the probe and explicit execution globals without changing its enforcement rules or wasm-memory allowance list.

## Commands and controls executed now

Every exit below was read from the process itself. No piped tail status was used.

| Check | Exit and observation |
|-------|----------------------|
| Final locked release build | 0, final binary identity above |
| `bin/ocelli.sh test ocelli-folder-viewer --lib csp::` | 0, 15 passed, 90 filtered |
| `bin/ocelli.sh test ocelli-folder-viewer --test preview the_pages_carry_their_own_policy` | 0, 1 passed, 121 filtered |
| Independent native copy with actual browser-derived expectations and 24-thread cache test | 0, 17 passed |
| Native hash-start-tag mutant | 101, 12 semantic failures, 5 passed |
| Native trim-body mutant | 101, 3 semantic failures, 14 passed |
| Native bad-base64-padding mutant | 101, 14 semantic failures, 3 passed |
| Native constant-cache-key mutant | 101, 11 semantic failures, 6 passed |
| Restored native copy | 0, 17 passed |
| Final release browser composition and independent header/pixel negatives | 0, 15.54 seconds |
| AC2 archived targeted test, original and restored | 0, one selected test passed, 12 intentionally filtered |
| AC2 wrong Use header, absent marker, wrong opener | 1 each, assertion failures naming Use |

Two reviewer harness setup failures are retained separately. The first HTTP driver assumed all 96 simultaneous connections would be admitted and exited 1 before policy comparison. A separate diagnostic showed 32 successful responses and 64 explicit connection-limit 503 responses, so the successful run used batches of six. This was no policy failure and no production change. The first archived Vitest invocation exited 1 because the sandbox denied its dependency-cache write. The authorized rerun supplied the semantic results above. Neither initial failure is counted as a product RED or a passing check.

Final report guards passed: staged prose checked 185 files, the CSP delta against `61efc91b` passed `git diff --check`, and the unstaged diff remained empty. A whole staged diff against HEAD reported five inherited JPEG XL manifest rows ending in an empty TSV field. The identical warning is present in the pinned source tree, outside the 14-path CSP delta. The manifest was not edited. Only this report was added to the index relative to the reviewed tree.

## Limitations and handoff

No production edits, commits or history changes were made by this reviewer. Browser instances and the ephemeral server were stopped. Synthetic downloaded SR and test copies remain under private temporary evidence paths, never staged. The archived source used for mutations was restored.

This is a clean bounded review of the folder CSP implementation and its composition. It does not reapprove inherited SR or conformance algorithms, exercise an interactive human click-through, rerun the example-half probe, certify wasm cross-target compilation, run a GPU oracle, measure performance budgets or complete sprint close. Those retain their existing owners and acceptance requirements. This report must be staged before any guard records its content.
