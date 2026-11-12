# F-245 configured CSP helper review, pass 1

**Reviewed**: Worker base d85406a5eed7d4340b3b4e6db961631569bff99a plus the applied `examples/folder-viewer/src/csp.rs` at 83d015acff62541c709245c242593b2121ac9ae0f85b1457630f6dfae5c439a3. Only `configured_main_policy`, the `page_policy` delegation and the three new tests. Startup configuration, the serve context and the HTTP binding do not exist yet and are outside this snapshot.

**Author / reviewer**: GLM authored the helper, its three permanent tests and its own review. Codex coordinated exact application and the first execution. Claude (root independent reviewer) inspected the diff against the approved plan, re-executed the library selection and qualified the GLM own report.

**Result**: 0 defects, 0 smells, 1 nitpick in this bounded helper slice.

## Acceptance challenged

The plan says the main page and `/index.html` receive the configured policy, other paths keep the cached self-only policy, only the studies origin is allowed to connect and there are no wildcards or other directive changes. The helper is a pure builder over already validated origins, so the challenge is whether the cached default can be contaminated and whether an empty list is byte-identical to the previous policy. `page_policy` now calls `configured_main_policy(page, &[])`, so the default shape is produced by the same code path with the origin loop executing zero times. The directive order and every pushed literal are unchanged from the previous `build_policy`, which is what the first new test pins.

The helper performs no origin validation and the plan does not ask it to. The obligation that every origin is a scheme and authority with no character a CSP parser treats as a separator or wildcard therefore falls on the startup validator. That validator is the unapplied URL pair parser, and its v2 draft fails it, see the disposition below.

## Evidence

| Command | Identity | Exit and observation |
| --- | --- | --- |
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer --lib csp::` in the worker | csp.rs 83d015ac before and after | 0, 18 passed, 133 filtered |
| Prior missing-origin and cache-pollution mutants | `native-csp-missing-origin-mutant.json`, `native-csp-cache-origin-mutant.json` in the evidence root | Inspected receipts. Each selected assertion failed, restored 18 passed |
| GLM own review | `claude-csp-own-review-json-low-v1/` in the evidence root | Complete in 35.4 s, finish stop. Helper exit 1 is a format outcome because the JSON arrived inside a Markdown fence, not a semantic failure |

## Findings

**N1** `csp.rs:31`. `configured_main_policy` has no doc comment, while every other crate-visible item here has one. The module comment at line 5 still describes only cached per-page policies. A reader auditing cache isolation is not told that the cached default is built through the configured path with an empty list, or that the helper trusts its caller to validate origins. Fix with the next edit that touches this file.

## Own finding dispositions

GLM raised three smells and two nits. S1, no origin validation, is correct as a fact and is a caller obligation rather than a defect in this file, as GLM itself says. S2, no routing test for page to policy, describes the serve binding that does not exist yet. It is open coverage owned by the startup slice, not a smell in this helper. S3, the wildcard assertion only covers well-formed fixtures, is the same caller obligation. The first nit matches N1. The second, prefix matching in the test-only `directive` helper, has no failing case today and the policy has no directive name that prefixes another.

## Cross-slice finding recorded here

The unapplied URL pair parser `native-dicomweb-config-pair-no-panic-low-v2` derives `connect_origin` with `format!("{}://{}", scheme, authority)` and accepts any `Host::RegisteredName`. RFC 3986 section 3.2.2 reg-name admits sub-delims and percent-encoding, so `http://*.example.com/studies` validates and would place a wildcard source in `connect-src`. A `;` ends the directive and a `,` starts a second policy. That is a defect in the parser draft, not in this helper. It was returned to GLM as a tests-first repair before any application.

## Verified clean

The cache is keyed by the page constant and only the empty-origin form is ever stored. Configured strings are owned per call and never leak. No forbidden source is introduced. Hash collection is unchanged.
