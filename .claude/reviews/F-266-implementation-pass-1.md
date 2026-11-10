# F-266 implementation microscope, pass 1

**Reviewed**: partial implementation against base `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`. Frozen complete partial patch SHA256 `42b2c8a271ab25d7c23fb319fbb82f1c022436a0d8f439a9b279744a58668f23`. Checker SHA256 `fa9b9a7de41dd9fee3cfafcfc482a71e0a3e38790d28c4c47c4f2212c9c8e3a0`, CSP probe `2d6ca7b14ef1d03bf71d05669602ce908becee6399696e25e9ffdee0203b6de5`. A new design amendment is being drafted separately. It is not approved or included as implemented scope here.

**Author / reviewer**: GLM authored all semantic changes. Codex independently inspected the consumers and ran the isolated checker and catalogue controls below. The author has not completed a clean full implementation microscope or acceptance inventory.

**Result**: 3 defects, 0 smells, 1 nitpick. Partial implementation only. No completion or prepared handoff is accepted.

## Defects

D1. AC1 protects every emitted file from the build machine's absolute path. emitted() reads names and worker text only, and main() passes only that text into present_problems(). A healthy worker with an absolute fetch in index-a1.js or ocelli_wasm-Bcd23.js is accepted. The independent disposable emitted-tree control calls those actual functions and obtains an empty problem list for both cases, with unchanged checker source. Proof: /private/tmp/ocelli-s46-codex/f266-root-nonworker-refusal-control.json. Extend the existing check through its actual callers to cover every emitted output in the declared contract, including build-root paths beyond a single exact package URL. Preserve intentional absent-build behavior and all unrelated refusals. Add a source-bound negative outside the worker, then restoration GREEN. This observation is an isolated helper control, not a real browser run.

D2. The new strict plain-HTTP positive still fails. The controller's actual latest CSP run reports undefined logged policy for the document: startStaticServer passes the policy directly to writeHead and then reads res.getHeader to record it. The current code does not obtain the policy actually received by the browser. There is also a subsequent source-visible mismatch: page.goto requests '/', staticLog retains '/', while documentEntry only searches for '/index.html'. After repairing header observation, that document lookup remains unsatisfied. Record and assert actual response headers, bind '/' to the served index file without changing emitted bytes, and rerun the genuine strict positive plus all existing policy controls. Policy text stays unchanged. Earlier 14-pass runs lacked the real positive's HTTP policy header and do not supply this acceptance.

D3. The existing guard catalogue remains incompatible with the changed checker. Running the exact AST-extracted _VIEWER_PRESENT_HEALTHY program against the candidate returns exit 1 and the absolute-path refusal, because its old healthy fixture contains the removed preflight fetch. Proof: /private/tmp/ocelli-s46-codex/f266-root-restored-catalogue-healthy-proof.json. The necessary consumer and budget changes require the currently pending design amendment before implementation. Then update the healthy shape, meaningful absolute-fetch refusal and the corresponding actual refusal-site accounting without changing unrelated ratchets, budgets, gates or profiles. No green floor is possible from leaving this consumer stale.

## Nitpick

N1. _absolute_reach and _partial_spellings describe more mixed forms than are enumerated. The code enumerates one segment decoded at a time, not arbitrary encoded and decoded combinations. Keep the documented guarantee bounded to implemented coverage or provide an appropriate broader refusal that satisfies AC1. Do not claim exhaustive spellings from these controls.

## Verified clean and retained evidence

The worker removes the redundant fetch and still imports before awaiting default init. The config selects the decoded specifier only for present builds and the encoded browser URL for the remaining mode and presence combinations. The old-fetch browser negative and restoration are retained as prior partial evidence. The absent build deliberately keeps its external encoded import. Unit controls cover the renamed browser and Rollup forms, worker absolute fetches and several single-segment mixed forms. These observations do not close the full dev and build, present and absent path matrix.

The no-connect consumer adaptation is appropriate: script-src can allow module glue while connect-src refuses the wasm fetch within it. The author's earlier old-probe versus new-dist comparison was not a base runtime observation and cannot excuse this adaptation. That attribution error was corrected and preserved.

## Limits and handoff

The author exceeded soft tool targets and its steered second attempt was interrupted at 828.182 seconds of the original 900-second allowance. No timer reset or process exit is a correctness verdict. A guard-catalogue edit made outside the approved write set was preserved as an unapproved proposal and mechanically restored to HEAD before this review. It is not adopted by this report or by the forthcoming amendment review.

The actual strict positive, full path and mode matrix, causal same-input allowance control, final guard coverage and complete author microscope remain open. GLM owns the semantic repairs after design approval. Root will review the repaired complete union and its actual evidence. No S47 integration into S46, publication, operator-check waiver or sprint closure is granted.
