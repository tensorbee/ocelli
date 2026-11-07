# F-224 review, pass 3

**Reviewed**: `git diff 9f4d85c 308a0f0` in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `308a0f06997ea094a5a4091d0c6f39b7d5f7b24b`, tree `9f4cff636ce5dd2338e5bb197e4507b2270b39bb`. One file, one line added: `examples/folder-viewer/src/use.html:402`. The tree was clean apart from the untracked pass 1 and pass 2 reports.
**Author / reviewer**: remediation by a Claude implementer, per the lead's brief. Reviewer Claude Opus 5.5 as an independent `/microscope` subagent, which did not write this row or any earlier F-224 material.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Prior findings

| Pass 2 | Status | How it was checked |
|--------|--------|--------------------|
| D1, `use.html` says six options above five rows | Closed | The table now has six rows. A script extracted the row names from the `js-dicomweb` section and the keys from `DICOMWEB_CLIENT_OPTION_KEYS`: both are `studiesServiceUrl, wadoUriUrl, fetch, wasm, sink, cache`, in the same order |
| Carried: CHANGELOG 0.x entry | Still owed, not counted | |
| Carried: AC19 viewer-round claims in `use.html` and `more.html` | Still owed to the viewer round, not counted | |

## Acceptance challenged

Every claim in the new row was checked against `packages/core/src/dicomweb.ts` and `options.ts`, not against the README paragraph it echoes.

| Claim in the row | Source | Holds |
|------------------|--------|-------|
| Type `DicomwebCacheMode` | `dicomweb.ts:43`, exported from `packages/core/src/index.ts:55` | yes |
| Optional | `readonly cache?: DicomwebCacheMode` at `dicomweb.ts:68` | yes |
| The fetch cache mode of every request | the only fetch call in `dicomweb.ts` is `#get` at line 287, which passes `cache: this.#cache` (line 290) | yes |
| `"no-cache"` by default | `this.#cache = options.cache ?? "no-cache"` at line 95 | yes |
| Revalidates a stored response with the server before using it | the doc comment at lines 59 to 60, and the Fetch standard meaning of `no-cache` | yes |
| `"no-store"`, `"reload"`, `"default"` accepted | `CACHE_MODES` at lines 45 to 50 holds exactly these plus `"no-cache"` | yes |
| `"default"` opts in to heuristic freshness | doc comment lines 62 to 63 | yes |
| Any other value refused, at construction, `DicomwebError` code `InvalidRequest` | constructor lines 87 to 89, before `safeUrl` and before any field is set, `validCacheMode` is an exact membership test, so a typo is refused as well as the two named modes | yes |

The discriminating case for "any other value" is a near-miss spelling, which a prefix or case-insensitive check would wrongly accept. `dicomweb.test.ts:563` loops over `"force-cache"`, `"only-if-cached"` and `"nocache"` and asserts `InvalidRequest` for each, and it passed in the vitest run below. A `null` or non-string value also fails `validCacheMode` and so is refused, which agrees with "any other value". An explicit `cache: undefined` takes the default, which agrees with "optional".

Refusal order: `refuseUnknownOptions` runs first, so an unknown key is `OcelliError` `UnknownOption` even when `cache` is also bad. The row does not claim otherwise, and README line 357 states the unknown-option rule.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer --test preview` | HEAD 308a0f0 | now | 0, 32 passed |
| `npx vitest run packages/core` | HEAD 308a0f0 | now | 0, 48 files, 440 tests passed |
| `npm run lint` | HEAD 308a0f0 | now | 0 |
| `python3 scripts/prose_check.py` | HEAD 308a0f0 | now | 0, voice rules clean over 864 files |
| `python3 scripts/source_provenance_check.py` | HEAD 308a0f0 | now | 0, clean over 1707 files |
| row and key extraction script | `use.html` `js-dicomweb` tbody, `options.ts` key array | now | identical six-name lists |
| `git status --short` at the end | worktree | now | only the three untracked review reports, HEAD unchanged |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The new row against the code, claim by claim, in the table above.
- Count and table agree: "six options" at `use.html:393`, six rows, matching `DICOMWEB_CLIENT_OPTION_KEYS`.
- No other sentence about `DicomwebClient` options or cache mode is stale. Checked by grep for `cache`, `DicomwebClient`, `no-cache`, `five options`, `six options`, `postWadoInstance` and `wado` over `README.md`, `use.html` and `more.html`:
  - README lines 346 to 358 agree with the row and the code, including the code `InvalidRequest`.
  - The `use.html` example at lines 412 to 427 omits `cache`, which is correct for an optional field with a default.
  - The `use.html` error comment at line 438 lists `InvalidRequest` among the codes, still true.
  - `postWadoInstance` (`use.html:446` to 459, `more.html:394` to 401) passes `cache` through to `DicomwebClient` only when set (`instance-bytes.ts:45`), so "wraps exactly this pattern" stays true. Neither page claims a full option list for it.
  - `more.html` has no sentence about `DicomwebClient` options or cache mode.
- The row's prose has no em-dash and no semicolon, and `prose_check.py` covers the file.

## Limitations and handoff

- Scope was the one-line diff and the three always-in-scope documents' DICOMweb sentences. The rest of F-224 was covered by passes 1 and 2 and was not re-reviewed, since nothing else changed in `9f4d85c..308a0f0`.
- No mutation was run this pass. The diff is prose only, and the one behaviour it describes (refusal of a bad mode) is watched by `dicomweb.test.ts:563`, which passed.
- `postWadoInstance`'s own `cache` field is not listed in any option table. No page claims it does, so this is not a finding, and it is noted here for the viewer round's AC19 work.
- The carried CHANGELOG entry and AC19 viewer claims remain owed. This clean pass is a bounded review of the library and package half, not story completion or sprint readiness.
