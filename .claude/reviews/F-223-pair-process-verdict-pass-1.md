# F-223 comparator process verdict, bounded independent review

Reviewed 2026-11-09 by root Codex. Result: 0 defects, 0 smells, 0 nitpicks in private patch `22b83aac38eb767dca27cb9078a4326284f118fa8566f04b01dacc3dded0d4d7`, based on canonical 9a4456b6. The f244 participant authored this remediation and the original controller. Root authored neither. This review covers the process repair and its synthetic controls, not the separate unresolved study pixel divergence or F-223 acceptance.

## Defect and correction

The actual `ocelli-compare pair` command returns 1 with a structured seven-field verdict when a comparison fails or has insufficient information. The former promisified process call treated that expected refusal as an exception, discarded the verdict and surfaced only the closed generic error. The private study observation reached that exact boundary. The repair preserves a validated refusal and the existing no-timings path. It changes no pixel, numerical predicate, tolerance, input-selection rule or publication permission.

The consumer calls the new process helper directly. Only exit 0 or 1 without signal, termination or stderr is eligible. The exact seven keys, duplicate-key rejection, finite numeric ranges, bounded sample counts and boolean types prevent foreign strings or diagnostics entering the report. Exit 0 requires passes and excludes low-information success. Exit 1 requires a failed verdict. Unknown output, malformed data, excess bytes, other exit codes and contradictory channels produce one fixed error. This validation checks the process protocol and does not recompute the numerical tolerance.

## Evidence and adversaries

The private comparison-pair-process-repair directory retains the exact source and direct command receipts. The registered full comparison_run_test suite passed all 20 tests, exit 0 in 0.656584 seconds, and passed after restoration in 1.407878 seconds. Targeted ESLint over both real paths exited 0 in 0.910705 seconds. The new default controls run actual synthetic child processes without requiring an otherwise absent release comparator or claiming browser evidence.

An explicit observation used the existing release comparator, SHA-256 prefix 4c316c97, on generated informative identical, informative biased and low-information RGBA fixtures. The actual exits were 0, 1 and 1. The same driver challenged the real pixelPrecheck-to-controller integration, retaining a refusal report, zero timing trials and cleanup. Its direct command exited 0 in 0.075183 seconds. It is an explicit observation, not a new default binary dependency or skipped default test.

Restoring the old direct process call while keeping the helper present failed the actual controller control, exit 1 in 0.135324 seconds, at the generic error instead of a retained refusal. Accepting foreign stderr failed in 0.881053 seconds. Removing duplicate-key rejection failed in 0.066421 seconds. Exact repaired source was restored before the final 20-test pass. Source identity and private raw receipts remain available for final integration verification.

## Limits and permanent regression

A structured refused_pixels result supplies no passing pixel precondition and no timings. The actual study comparison remains open, with no publication of its derived report authorized. The new tests cover the actual consumer integration, expected process refusal and malformed channels. The existing workflow already requires consumer-level adversaries and closed private-data errors. The missed step was testing a real expected nonzero comparator result through that consumer, so no new workflow rule or gate waiver is introduced. Final canonical feature verification and qualification CI remain required.

## Advisory GLM findings, lead disposition

The later source-only GLM report is preserved under comparison-pair-protocol-risk-glm. It supplies no runtime approval. Its conditional D1 argument-order concern is closed by the actual SIDES order `[ocelli, cornerstone]`, the capture array and the real comparator's positive one-code candidate-minus-reference control. The reference is raw[1], the candidate raw[0].

D2 is a preventive pre-spawn validation suggestion. The actual consumer obtains positive integer native canvas dimensions and the trusted CLI rejects malformed dimensions. The validator rejects an invalid sample count. No invalid verdict or successful malformed capture is demonstrated. No broader caller API is introduced. D3 would expose actual foreign process codes as public outcomes, contrary to this repair's closed error contract. The fixed exit 1 means comparison failure, not a claim that the child itself exited 1. The raw private diagnostic remains separate. D4's claimed inferred TypeScript result type does not exist in this JavaScript module, and the explicit numeric/string checks reject foreign code and byte-buffer shapes. D5 is a minor explicit-boolean suggestion. The success adapter constructs killed=false, while the actual Node rejection adapter preserves the child's termination flag and the validator rejects true and signals. No accepted terminated result is shown. These do not establish a new defect or require loosening the protocol. Original model text and participant qualification remain preserved, with no claim that advisory finding labels were independently confirmed.
