# S46 deep lint fixture repair, bounded independent pass1

Final reviewed source: tree945b058679d04f19582c23b94bb9df072d08ef40 on basecba838a660968fa179cb0f9c576bada3fe71fe07. Four-path patch SHA256dd9d2d32d09086dafd4b3eb83eb2768244cf5db9c87b9afddd636a4e2a2633d0. Verdict:0D0S remaining in this bounded source review.

## Independence and scope

Root Codex authored the fallback. This reviewer authored neither its helper nor permanent tests. An earlier independent GLM control-risk proposal saw only immutable c27 source and the actual old CI/replay failures. It was frozen before this repair was read. That proposal is not source approval. Root's original implementation-model attempt was unused and late-censored without an artifact or correctness verdict. It receives no credit for the fallback code.

Reviewed all four changed paths: catalogue helper/probe data, catalogue tests, generated runbook and budget. The production lint checker and probe runner are byte-identical to the base. No compiler policy, tolerance, gate selection, harness polarity or expected group-refusal text changed.

## Source and lifecycle challenge

The exclusion helper parses the complete original TOML before editing. It appends only when the member is absent, preserves other parsed values and lets TOML delimit valid multiline arrays, comments and quoted brackets. Existing malformed input does not become an unrelated valid fixture. It explicitly refuses unsupported fixture spellings/layouts rather than inserting a duplicate key. It is a fixture editor, not a general TOML formatter. Formatting/comments within the rewritten value are not preserved as an API promise.

Both clean module builders record a real successful guard baseline before adding the Rust source. Their wrapper requires exactly N+1 from the next successful run before emitting its acceptance witness. The baseline JSON cannot become a scanned Rust module. Each probe mutation starts after Sandbox.reset, writes its own baseline anew and has an explicit ordinary-checker control. The shared healthy-control cache therefore never invokes the wrapper without baseline preparation or shares a mutated count between probes.

The unchanged plain and cfg_attr group-allow companions use the same source layout. The author's fake-count-without-queue mutant demonstrates why both halves matter: the count positives stay green, but both content negatives go red. Their failure is an unseen forbidden allowance, not a broadened success fragment. The skipped-path mutant separately rejects missing traversal. No literal whole-repository3-to4 repair is used.

Only catalogue entry_sites85to90 changed in the parsed budget. All constants, gate data and other entries compare equal. The five new setup refusals are explicitly covered by the two named unit classes, and the generated runbook describes their fixture-precondition limit without claiming another automatic deep probe.

## Actual checks and precursor correction

Reviewer direct pure-Python controls on frozen function ASTs exited0 in0.187068seconds. They independently cover five valid exclusion shapes, two malformed originals, twelve count deltas, two baseline advances, guard-failure propagation and missing-count preconditions. They also assert the unchanged production files and exact budget delta. These controlled returns are unit evidence, not actual Cargo or CI execution.

Root identified a future fixture fragility in v1: its source-derived expected list always appended a member even when already present. The reviewer reproduced the real permanent test with that valid future source shape: v1 exited1 in0.197262seconds at the duplicate-member assertion. V2 changes only that expected-value preparation. The identical challenge then exited0 in0.206508seconds, with helper/runtime unchanged. This was not a failure on the actual Cba manifest and is now closed.

Separately inspected author receipts: final v2 unit suite65tests exited0 in3.375276seconds, and census/runbook exited0 in0.962845seconds. Earlier real selected deep execution exited0 with3 intended refusals,2 accepts and1 healthy control. Old-helper duplicate-TOML, skipped-path and discarded-exclusion mutations were red. Fake traversal counts passed the positive count pair but failed both required content negatives, followed by restoration. Historical warning/earlier64-test receipts remain distinct from the final65-test result.

## Limits and disposition

Reviewer controls used Python3.14.5 and AST-extracted frozen functions, without module import/Cargo execution. Author's actual Python3.12 suite and real sandbox probes are separate evidence. No new browser, native build, actual study or full-profile job ran here. Unsupported valid TOML fixture layouts fail closed as documented, rather than being silently repaired. This review does not claim a fresh Linux CI result or completed feature profile.

Root must integrate the exact patch and run the promised final floor/corpus/deep/candidate checks before committing. No canonical file or index was changed by this reviewer. Private receipts and source hashes are bound by handoff.json.
