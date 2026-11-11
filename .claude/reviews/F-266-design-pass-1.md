# F-266 design microscope, pass 1

Reviewer: Codex, independent of the GLM author. Design only.

Source SHA256: `6d29ffdb2ae09a80966423aa54498623db656612b41fa0cc91d63e64e53ea014`, private proposal based on `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`. GLM authored and repaired the design and performed its own source microscope first. Its final zero-defect and zero-smell report is not independent approval.

Result: 2 defects, 0 smells, 2 nits. Status remains draft.

D1. Scope and AC6 ban absolute emitted paths without conditioning that restriction on a present package, while Approach 5 deliberately externalizes the encoded absolute browser URL when the package is absent. Apply the present-package restriction consistently. Approach 7's statement that the checker has no absent-build refusal to make is too broad: only the intentional external URL is exempt from the present-package path ban. Every other existing absent-shape refusal remains required. No guard is disabled.

D2. AC7's mutation restores a /@fs define, but the healthy present-package Rollup branch already uses that filesystem specifier before bundling. That change alone need not fail. Restore the actual redundant preflight fetch to the absolute URL on a present-package build, plus the old unconditional decoded form if needed, and demonstrate the intended emitted-path and real static-HTTP readiness failures. Restore and observe GREEN. The control must challenge the real consumer, not only configuration text.

N1. The consumer map says selection is by command alone. Name command and package presence, as the concrete approach already does.

N2. The dependency section still postpones implementation until S46 completion. The operator now explicitly allows isolated design-approved S47 work while S46 remains open. Record that ordering exception without implying permission to integrate, push or close.

The proposed import-only route is otherwise concrete: preserve the currently proven decoded Rollup specifier only for a present-package build, use the encoded browser URL for serve and absent-build cases, externalize exactly the absent URL, and decode the development filesystem allowance separately. Readiness still requires successful module import and default initialization, as the current loadCore path already does. A genuine positive CSP observation must serve unmodified emitted bytes over plain HTTP without repository mapping, fallback or route interception. Instrumented negative scenarios remain separately identified.

The five affected config, worker, checker, checker-test and CSP-probe paths are mapped. No F-245 writer is active. The existing packages gate carries the checker, so no new gate or profile is necessary. No product implementation or browser acceptance is claimed here.
