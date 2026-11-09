# F-240 viewer review, working pass 1

**Result:** F-240 viewer pass 1, 1 defect, 0 smells, 1 nitpick. The defect is a false integration comment. No functional defect was found in the bounded controls below. This is not a clean whole-feature review.

**Reviewed:** frozen staged tree `81987bd7c13f8e981b62838e18d8afa21b9b7ce4` against colour baseline `7dd567302ac5abc31634238b409c6646fd04fbbf`. The complete difference has 35 paths, including the already reviewed F-240 library slice and its four review records. This pass focuses on the new viewer, fixture, corpus and documentation composition. It does not repeat the full earlier library arithmetic review. Compared with reviewed library commit `def9`, the ICC parser source and integration test file differ only in their two test-builder version words, `04440000` to `04400000`.

**Independence:** the lead authored this integration and the six new browser cases. This reviewer authored neither the production ICC changes nor those cases. Earlier F-239 browser fixtures authored by this reviewer are excluded from independent approval. The author worktree stayed untouched during all runtime challenges. Private controls and mutations used an immutable archive of the frozen tree at `/private/tmp/ocelli-icc-review-p1`, with dedicated target `/private/tmp/icc-review-p1-target` and jobs 2.

## Findings

**D1, the ICC heap-accounting comment is false after integration.** `crates/ocelli-pixel/src/icc.rs`, immediately above `IccTransform::heap_bytes`, says nothing in that crate calls it and describes viewer charging as future work. `ColorTransform::heap_bytes` calls it in the same crate. The viewer now calls the latter from `ColourRecipe::heap_bytes` and includes that cost before sample admission. The comment must describe the actual owner and current charge rather than say it has no caller. The functional accounting control passes and a removed ICC heap charge is detected, so this finding concerns the false prose, not observed under-accounting.

**N1, README grammar.** The new sentence reads `An matrix or tone-curve ICC profile`. Use `A`.

## Source and normative checks

The scan and held recipe use the same `read_profile` classifier. Its named strings come from a closed match over `IccError` and recognized four-byte class or space signatures. The profile payload is never interpolated into the page. A supported profile attaches before the colour window. Missing and unsupported profiles retain the F-239 device route. The held recipe carries the result, and redraw reads the attached transform rather than reparsing it.

`ColourRecipe::heap_bytes` includes the box contents and `ColorTransform::heap_bytes`, which counts both palette and ICC retained heap capacities. The exact predecode claim continues through the F-239 acquisition path. A gamma profile at 8 bits retains 3072 bytes of device-code tables. The 16-bit case retains 786432 bytes. Both were checked independently against the actual recipe and claim. Untagged RGB retains zero transform heap.

The published [ICC.1:2022](https://www.color.org/specification/ICC.1-2022-05.pdf) was opened directly. Sections 7.2.4, 7.2.18, 7.3 and 8.10 support the version encoding, profile-ID calculation, tag alignment and lookup precedence checks. Version 4.4.0.0 is encoded `04400000`. The two inherited fixture corrections are factual header fixes, with no parser arithmetic or expected pixel changes.

All five committed raw profiles were independently checked for declared size, version, signature, contiguous aligned unique tags and the profile ID after zeroing the prescribed fields. The generated DICOM rows reproduce all five manifest digests in a private output directory. The manifest has 114 rows in this snapshot. Each raw fixture also reproduces byte-for-byte from the generator. No shared corpus file was written.

Little CMS 2.18 accepted and converted every profile through the host tool `transicc -t1 -n -i <fixture> -o '*sRGB'`. The exact input triples, commands and outputs are retained in `/private/tmp/icc-independent-profile-results.json`. The four applied profiles agree with the approved plan's byte gold after quantization. The lookup profile is accepted by the external CMM, while the viewer deliberately retains device RGB under the approved subset policy. No Little CMS source was read or copied.

## Executed controls

All listed statuses come directly from the process. Browser checks used the frozen author's release binary, Chromium `151.0.7922.34`, a port-zero server and cleanup by PID. The complete harness has 25 fixtures, including the six new ICC cases. This pass approves only the new ICC assertions and their integration behavior, not this reviewer's earlier F-239 test authorship.

| Command or challenge | Direct exit and observation |
|----------------------|-----------------------------|
| `bin/ocelli.sh test ocelli-folder-viewer --test preview icc_ -- --test-threads=2` in fresh snapshot | 0, three permanent ICC cases |
| Independent `--test review_icc` | 0, 16 unsupported profile shapes keep exact device bytes and closed kind labels |
| Strengthened independent raw pointer and ROI controls | 0, each finite channel quantizes to AC1 byte 137, exact pointer text is `R 137.2 G 137.2 B 137.2`, ROI count 1 and population SD 0, raw value stays fixed while window draws 255 |
| `--lib independent_icc_claim_review` | 0, absent, 8-bit and 16-bit table costs equal exact predecode and retained charges |
| Delete only the ICC heap charge in private source | 101, literal table-capacity check fails at actual 0 against expected 3072 |
| Restore source and repeat ownership control | 0 |
| Repeat three permanent ICC cases after restoration | 0 |
| Browser base on unchanged frozen source | 0, 25 fixtures, decoded-pixel negative control retained |
| Falsify `icc_applied` in actual file-post inspect response | 1, one response changed, rejected at gamma1 corner warning assertion |
| Change lookup-table kind to printer in actual response | 1, one response changed, rejected at exact lookup kind assertion |
| Repeat unchanged browser positive | 0, 25 fixtures |
| Independent five-profile header, ID and CMM check | 0 |
| Private corpus regeneration and fixture equality | 0, all five manifest digests match |

The browser checks cover actual canvas and thumbnail pixels, profile-before-window order, removal of the warning on converted images, exact lookup warning in corner and help, and selection between applied, untagged and unsupported images in one folder. The native controls also cover 12-bit normalization and profile retention after encoded bytes are dropped.

## Review setup failures retained

The first two browser mutation attempts intercepted only `/inspect`, while folder opening obtains inspection JSON from `/file`. They changed zero responses and failed only the review driver's sentinel. They are retained as harness failures and provide no semantic product evidence. The corrected route intercepts both endpoints and the semantic failures above each changed one response.

A temporary strengthened raw-value control initially used the idealized common AC1 value 137.2066 with the unrelated YBR bound 0.001. It failed at red 137.2054443359375. The generated profile has quantized colourants, and the fresh CMM prints 137.2055, 137.2083 and 137.2069. That temporary common-channel expectation was incorrect for this fixture. It was replaced with the plan's existing literal byte 137, finite numeric channels, exact one-decimal text and population statistics. The failed receipt is preserved. No product tolerance or acceptance rule was changed, and no new sub-code float-accuracy claim is made.

The initial `git write-tree` call was blocked by sandbox permissions. It was rerun with the permitted Git escalation and directly returned the frozen tree above. Native compilation and browser setup failures are not reported as semantic mutation evidence.

## Evidence and remaining scope

Receipts and full logs are `/private/tmp/ocelli-s46-codex/icc-independent-*`. The browser driver is `/private/tmp/icc-independent-browser.mjs`, fixture generator `/private/tmp/icc-review-fixtures.py`, and corpus/CMM controls `/private/tmp/icc-independent-corpus-repro.py` and `/private/tmp/icc-independent-profile-check.py`. Synthetic fixture digests are `/private/tmp/icc-review-inputs/digests.json`. The browser harness SHA-256 is `5b57de14f24a62291e66c71601ea4953e3807975088553cf0e952a451bdbff07`.

D1 and its repair require a fresh bounded review. AC13 size, AC14 to AC16 timing and comparison, feature profile, final integration and sprint checks remain outside this pass. No GPU oracle acceptance, physical-display colour calibration, complete feature acceptance, commit or push is claimed.
