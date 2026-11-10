# F-245 native HTTP review, pass 1

**Reviewed**: Worker base d85406a5eed7d4340b3b4e6db961631569bff99a plus frozen native HTTP manifest. Serve source d7884a6a2e3fd9dc145f2f588a4c3fe84e83a64a54bc0c7148f0868f854a73c8, helper 864b7ad2dd7c34b27f7104ff6f2e7e3371a934ba2d3875bd56981c666c825013, library root 4182ba1b0adbdc1309e511e6831f8833c3fa183c7a950db2c71983f129d01e00, manifest 309cfe2268857d90ec6d509ac3ad8f93ee33cb148bcb4f8f3b7bd47dedf18b15 and lock 934b01bc439934da39602ee423d900d51b18e63c2649c72fe516edb9d3bc2bb2. Later configuration or UI changes are outside this snapshot.

**Author / reviewer**: GLM authored semantic source, permanent tests and own review. Worker Codex coordinated exact application and execution. Root Codex independently inspected the production diff and tests, traced unchanged pooled-body ownership, replayed the frozen source, authored two additional consumers and executed a discriminating mutation. Private replay manifest adaptation is controller work, not product implementation.

**Result**: 0 defects, 0 smells, 0 nitpicks in this bounded HTTP slice.

## Acceptance challenged

The actual request reader and native route must enforce existing numeric limits and fixed error forms, including query targets, before oversized body allocation. Parsed identity and hash projection must use the same owned body. The scheduled server must retain its real body permit and charge until their owner drops, release them on refusal and retry, and preserve an already cached local file.

Root's independent consumer uses Viewer::server with its actual two workers. It retains a project-generated synthetic local file, sends alternating invalid and valid QIDO bodies to a query target, checks live slot and inflight ownership before and after route, drops the actual BodyCharge owner, and verifies zero inflight and live slots afterward. The retained file's Arc identity, bytes, file count and accounted cache cost stay unchanged. This is native cache evidence, not browser marks or conformance evidence.

## Evidence

Root evidence: `/private/tmp/ocelli-s46-codex/f245-root-http-review/`.

| Command / cases | Identity and execution | Exit and observation |
| --- | --- | --- |
| Wrapper Cargo test, viewer library, offline/locked, one test thread | Frozen product copied into private workspace, two build jobs, baseline-viewer148.json/log | 0, 148 tests. Source unchanged. Corpus data absent |
| Two appended independent root consumers | Product prefix byte-identical to d788, root-consumers.json/log | 0, exactly two selected tests, 148 filtered. Actual scheduled pool and malformed-target request-reader/route probe |
| Inject charge.release_for_cache at native route entry | Disposable private mutation, early-body-release-mutant.json/log | 101, one selected test fails at owner-lifetime assertion, not compilation or timeout |
| Restore exact private source including unchanged product prefix | pooled-consumer-restored.json/log | 0, one selected ownership test, 149 filtered |
| Final author library, scoped clippy and format | Source-bound handoff receipts inspected | 0, 148 library tests and passing clippy/format, source d788 before/after |
| Six author semantic mutants | Limit extra byte, wrong status, exact-cap comparison, complete-header cap bypass, legacy text EOF mapping, wrong forwarded tuple | Prior actual selected assertion failures 101, restoration then library 148 exit 0 |

The private workspace changes only workspace membership and path roots, retaining versions, features and lints. Offline resolution removes 43 unused packages from the wider workspace lock, adds none and changes no shared checksum. It is a native consumer build, not a new product dependency graph or feature verification receipt. The source context for the library closure guard is a frozen core snapshot, not concurrently changing provider code.

## Own finding dispositions

D1 is not a reproduced native acceptance or privacy defect. Root sent the actual bare-LF malformed target through reader and route. Below the cap it is an unknown request path returning fixed 404 text, not an accepted native endpoint. Above the cap the broader early classifier gives fixed 400 JSON. Neither response contains the request data or enters the native operation. The own report incorrectly says no later CRLF exists and calls fixed text a privacy leak. Root preserves the two observed outcomes rather than changing legacy malformed-target policy.

D2 remains the already-deferred generic header-cap concern. No generic legacy policy change is approved or claimed by this slice. It stays source-observed and unconfirmed as a legacy backlog candidate.

S1 predicts future parser drift without a demonstrated failed invariant. S2 is a style preference. S3 describes the intended retirement followed by charge-owner drop. Root traced BodyPermit retirement and Drop and challenged that lifetime through the actual scheduled consumer and early-release mutant. None supplies a blocking finding here. The raw two-defect/three-smell report remains preserved separately.

## Verified clean

Inspected the new route dispatch, typed request metadata, per-route declared-body admission, native early-refusal handling, canonical query target recognition, closed status/code mapping, MIME boundary checks, bounded JSON sink, body retirement and library-asset closure changes. The same POST paths used by routing select the new limits. Header names are matched without retaining unrelated values in typed metadata. UIDs, nonce and generation validate before projection. Success replies borrow the existing request body through the reviewed helper and serialize under the unchanged 16 MiB written-byte limit.

The existing 2 MiB QIDO, 32 MiB instance and 16 KiB header limits are unchanged. Query targets and exact versus excess boundaries have actual controls. Closed errors carry a code only. Native tuple/hash validation remains in the previously reviewed helper. The asset closure guard now handles both quote styles and includes the new core module without weakening type-only exclusion. Legacy file and frame paths retain their existing behavior in the reviewed diff.

The new production source adds no pixel arithmetic, unsafe block, wasm binding or GPU call. Real buffer retirement uses the original worker owner and keeps the charge through parsing and hash. Root tests are private reviewer artifacts, not GLM-authored permanent tests or a claim of actual clinical or browser observations.

## Limitations and handoff

The successful library suite includes a legacy corpus loop that returns early when corpus data is absent. It is not corpus acceptance. No browser source chooser, local marks/conformance preservation, configured startup/CSP, actual launcher, registered release timing, GPU, physical heap or RSS observation is supplied. Serialized written bytes and accepted payload limits do not establish total allocator peaks.

The browser provider union, transfer-once staging and the next configuration/UI edits require their own source-bound reviews. This pass is not complete F-245, preparation, integration, push or sprint readiness. GLM owns those remaining implementations and own reviews. Root's pooled consumer may be retained with its root authorship stated. Native compilation and source leases have been returned to the worker.
