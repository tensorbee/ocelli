# S46 CI harness repair, independent bounded review

Reviewer: Codex f244_review. Author of this two-file repair: root Codex. This reviewer authored earlier parts of the unchanged comparison actor and collector. Those production functions are context only and receive no independent approval here. This pass reviews only root's CI dependency/package corrections and isolated test-fixture platform repair against HEAD `8fdee9143dc09891f5540619dc0ee4cee9d7bbdf`.

## Result and source

0 defects, 0 smells in the bounded delta. The exact two-file patch is `/private/tmp/ocelli-s46-codex/s46-ci-harness-independent-review/reviewed-two-path.patch`, SHA256 `6e14f1768e4bb04910b61abefc70d123eb397eddb62153f089d985f46e7364ca`. Source identities and private controls are in `/private/tmp/ocelli-s46-codex/s46-ci-harness-independent-review/identity.json`.

- `.github/workflows/ci.yml`, SHA256 `6995389cc541fb3dbf59ca5a0c82e1f3d2ad0b51b6e810ae3ba6d60e8d5b1097`.
- `tools/bench/tests/comparison_sampler_context_test.mjs`, SHA256 `3add17fb8329718b554fdc8e72d0ad71c1153ebd603a9e9695e200bd0ffa8fe1`.

The workflow installs the locked oracle package dependencies before the Guards benchmark gate. The comparison controls import the parser and page builder even when opted-out browser controls do not execute. No pin, lockfile, gate, browser requirement or threshold is relaxed. The new step supplies the imported dependencies rather than declaring the tests skipped.

The Corpus setup uses the actual Ubuntu binary package `libopenjp2-tools`. The existing subsequent `command -v opj_compress` remains a real failure if installation does not supply the required executable. The official Ubuntu catalogue lists that package for Noble and Resolute as the JPEG 2000 command-line tools. These source checks do not claim an Ubuntu installation or successful post-push CI execution. Package evidence: [Ubuntu package catalogue](https://packages.ubuntu.com/search?keywords=libopenjp2) and [Resolute package](https://packages.ubuntu.com/resolute/libopenjp2-tools). Direct file-list fetches were unavailable, so executable availability is left to the retained CI check.

The sampler fixture already substitutes OS and browser transports. Its new default Darwin platform represents that declared footprint transport, regardless of the actual host running Node. The exact platform-guard string must occur once before it is replaced in a disposable module copy. Only that consumer guard's platform expression is changed. Production source, guard behavior and other platform decisions remain untouched. A separate Linux fixture now exercises the real unavailable-return path, checks the closed response and budget, observes zero sampling calls and requires actor ownership cleanup.

## Executed consumer controls

Used actual Node24.13.0 and existing dependencies, with no browser, native viewer or OS footprint process.

- Full permanent mocked sampler suite: direct0,18pass,0skip, elapsed0.666485375026241 seconds. Receipt `s46-ci-repair-independent-sampler-green.json` and adjacent log.
- Private copy of the unchanged new Linux case, with only import locations bound to the reviewed source: direct0,1pass, elapsed0.07150637498125434 seconds. Receipt `s46-ci-repair-linux-copy-green.json`.
- Separate private consumer mutation omits only `actor.close()` in the Linux copied module's finally block. Direct1,1failed, elapsed0.0754152910085395 seconds. Failure is the actual closed-actor assertion, not a fixture replacement or import assertion. Receipt `s46-ci-repair-linux-cleanup-mutant-red.json`. The production and candidate test files were not mutated.

This challenges the newly added unavailable-route lifecycle control while retaining the existing safe-stage, no-retry, successful-receipt, failed-ordinal and unaccepted-output controls. All reviewed file hashes remained unchanged after execution.

## Limits

No benchmark measurements, GPU, browser, real OS sampler, dependency installation, Ubuntu package installation, clinical data, candidate/index edits or publication. Existing production collector arithmetic and its earlier authored tests are outside approval. Root owns adoption and final source verification. Successful post-push CI and corpus generation remain separate observations.
