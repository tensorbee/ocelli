# F-245 launcher repair review, pass 3

**Author / reviewer**: GLM source and tests, GLM own source review, independent Codex root review.
**Reviewed**: untracked worker launcher SHA256 7332c0dc6885e2c6abd93191caa8d61778de03f19a51f9d0928de9da07fb4130 and permanent tests SHA256 6f4e6fac4ca829d7ec843fe5158bc2c1a68fb5285479b6771b193053358238d9, based on worker HEAD d85406a5eed7d4340b3b4e6db961631569bff99a.
**Result**: 0 defects, 0 smells, 0 nits in this repair scope. Full launcher acceptance remains open.

## Acceptance challenged

AC1, normal Cargo exit with a living descendant retaining piped stdout. Cleanup must begin from exit without waiting for pipe closure, then confirm owned group and pipe closure before advancing. The unchanged independent control failed on the previous source at the descendant-alive assertion. It now passes at the reviewed source. Root additionally challenged Cargo failure and a compiler artifact naming an absent backend. Both must refuse without starting a backend or hanging.

## Evidence

Private evidence directory: /private/tmp/ocelli-s46-codex/f245-root-launcher-review.

| Command and selected cases | Executed now or prior receipt | Exit and observation |
| --- | --- | --- |
| Node test cargo-pipe.test.mjs, pattern root Cargo parent exit | Root now, cargo-pipe-after.json and log | 0, exactly 1 selected and passed, source hash unchanged |
| Node test adjacent-phase.test.mjs, pattern root adjacent phase refusal | Root now, adjacent-phase.json and log | 0, exactly 2 selected and passed, source hash unchanged |
| Same original Cargo pipe control on source 7dc1a225 | Prior root receipt cargo-pipe-before.json and log | 1 at actual descendant-alive assertion |
| Permanent launcher test file and targeted ESLint | Author receipts launcher-cargo-fixed-final-five.json and launcher-cargo-fixed-final-eslint.json | Both 0, five controls, exact reviewed hashes |

No browser, user listener or global process-name cleanup is used. Root fixtures clean up their recorded owned processes only.

## Prior findings and own review disposition

Pass 2 D1 is closed. A separate exit promise starts normal cleanup. The existing close promise confirms pipe drainage after group shutdown. Failed spawn still reaches close, and errors retain exit1. Signal handling preserves the first signal and the original non-extending cleanup deadline. N1 is closed by the factual permanent test title.

The GLM own response is fenced JSON. Its wrapper JSON decoding failure is a protocol result, not a failed product test. Preserve both the raw response and mechanically parsed report. Its alleged redundant await is a single await on a function named awaitClose. Its fixed-deadline objection conflicts with the approved cleanup bound, which the control intentionally enforces. Normal exit cannot merely probe because descendants may still live. The reported process identifier reuse window is an inherent limitation of POSIX group signaling, not a demonstrated new source regression or an implemented atomic alternative. This review does not claim protection against adversarial kernel identifier reuse.

## Verified clean

The reviewed repair separates process exit from stdio close without introducing a whole-phase timeout. Normal orphan cleanup, bounded confirmation, missing backend refusal and Cargo nonzero refusal behave as required in the exercised synthetic callers. Listeners are removed after settlement. Root has not authored the product repair.

## Limitations and handoff

This pass accepts the exit-versus-close repair only. Real npm and Cargo startup, real browser observations, Cargo interruption, malformed and duplicate artifact refusals, platform documentation, permanent test registration and the complete feature acceptance inventory remain owed. The source is not integrated or ready for push. The sole F245 writer continues those steps and returns final source for full review.
