# F-273 component test placement plan review, pass 1

**Reviewed**: Approved plan 3ca72c7add5f80a5d4c2641a9529bcf3d394b5853f5ad2f36c294252295ae384 plus one amendment inserted at the end of `## Exact write and registered test map`, resulting plan f8e18bbf3eff90a7. The operator ground truth section is byte-identical before and after.

**Author / reviewer**: GLM authored the amendment text in two passes. Claude coordinated and independently reviewed. No GLM packet contained the operator ground truth section.

**Result**: 0 defects, 0 smells, 0 nitpicks in the amended text after v2.

## Why an amendment was needed

The write map placed the helper at `tools/oracle/page/reference_loader.mjs` and said no new file or registration. Root review of the private helper found four guards that survive the helper's own tests, and GLM wrote five killing controls, each now qualified against a named mutant. Those nine Node component controls had no planned home, and their private copies resolved dependencies through absolute worker paths.

## Review

The v1 text called the file both new and existing, contained a prose semicolon and offered an example where one mechanism was required. v2 fixes all three and changes nothing else. The decision is one new file, `tools/bench/tests/reference_loader_component_test.mjs`, with exactly the two registrations that every bench test already has, `tools/bench/package.json` scripts.test and the `bin/ocelli.sh` bench arm list. Dependency resolution uses `createRequire(new URL('../../oracle/package.json', import.meta.url))`, which from `tools/bench/tests/` resolves to `tools/oracle/package.json`, the package that already owns dicom-parser and Cornerstone.

## Acceptance challenged

A reader could take nine green Node controls as evidence for AC17 or AC22. The amendment says they do not satisfy either and that BR-CTX and BR-MUT stay required in `comparison_page_test.mjs`. No acceptance criterion, endpoint, workload, N, tolerance or baseline changes.

## Evidence

Qualification of the nine controls is `claude-reference-killing-tests-qualification-v5.json` in the F-273 evidence root: 9 of 9 pass on module 94124be7, and the cpu-delete, window-noeq, ref-check, postfinite and vm-truncate mutants each fail exactly their own control. The byte-whitelist mutant survives because the DS regex refuses the same bytes, which is nit N1 of `F-273-reference-module-pass-1.md`.
