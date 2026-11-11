# F-240 viewer review, working pass 2

**Result:** F-240 viewer pass 2, 0 defects, 0 smells, 0 nitpicks in the bounded remediation scope.

**Reviewed:** staged tree `326584b4992220934beab071ff70b9aa04ef6ebc` against pass-1 source `81987bd7c13f8e981b62838e18d8afa21b9b7ce4`. The delta is the prior independent pass-1 report and exactly five repaired documentation paths: README, the ICC heap doc comment, and the frame-open, pixel-pipeline and render-graph LLDs. `git write-tree` directly returned the reviewed tree. `git diff --exit-code` returned 0, confirming no unstaged tracked change at review time.

**Independence:** the lead authored these repairs. This reviewer authored none of the five repaired paths. The reviewer-authored pass-1 report is an evidence attachment, not material independently approved by its own author. There is no new functional source or test change.

D1 is closed. `IccTransform::heap_bytes` now names its `ColorTransform::heap_bytes` caller and the viewer's pre-admission charge. Those statements match the actual call chain and pass-1 exact 8-bit and 16-bit capacity controls. The private missing-charge mutation failed at the intended table-capacity assertion, and restored source passed.

N1 is closed. README now reads `A matrix or tone-curve ICC profile`.

The adjacent LLD repairs were also read in full. The frame-open document now describes the current classifier and malformed byte-payload case. The pixel-pipeline document describes the five generated profiles, retained applied transform or refused kind, scan classification without device-code tables, and pre-admission accounting. The added viewer-consumer section matches the band size, source-plane gathering, three-byte replies, raw pointer and ROI conversion, channel-aware page paths and mixed-kind window rules reviewed on the frozen implementation. The render-graph paragraph correctly distinguishes the CPU viewer consumer from the unavailable GPU ICC route.

The factual profile and corpus statements are supported by pass-1 direct fixture reproduction, header and ID checks, five-profile Little CMS 2.18 run, native tests and browser observations. Applied profiles remove the warning. Unsupported profiles retain device bytes and a closed kind. No new size, timing, GPU, oracle or display-calibration claim was introduced.

No runtime check was repeated for this documentation-only follow-up. Its functional evidence remains the source-pinned pass-1 receipts `icc-independent-*` under `/private/tmp/ocelli-s46-codex`, including the restored three native cases, table-capacity control and unchanged browser positive. The initial review-harness mistakes remain disclosed in pass 1. No tolerance or gate changed.

This closes the viewer remediation review only. AC13 size, AC14 to AC16 timing and comparison, feature profile, integration and whole-sprint acceptance remain open. No complete-feature acceptance, commit or push is claimed.
