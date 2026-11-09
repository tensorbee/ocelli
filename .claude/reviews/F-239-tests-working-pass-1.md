# F-239 additive tests review, working pass 1

**Reviewed:** GLM's final autonomous `examples/folder-viewer/tests/colour.rs`, SHA-256 `e51aa29e7f37fa3ce04782f74775e7b80db9d5676ef81e246beada58bba24496`, 1602 lines. The file was untracked and excluded from the server's frozen production review. The repaired artifact is staged in tree `9ef82d3f22937c9639f0abd6e62d94b7341971b2`, with production byte-identical to `cc62cdb4ab9e9ca9cecbf43b0836c48e0ba4fcc1`. This is a tests-only review of GLM's output, not independent approval of the reviewer's own server implementation. That implementation has a separate independent server pass 3.

**Authors / reviewer:** `dgx/GLM-5.3-Flash-EXL3` authored the additive test file. Codex reviewed it without providing corrective input during GLM's autonomous run, then repaired the findings below under the lead's ruling. The original autonomous file and report are preserved under `/private/tmp/ocelli-s46-codex/f239-colour-tests-glm`. Codex authored the server production code, so the production review remains separate. The reviewer-authored test repairs require the lead's independent assessment.

**Result:** autonomous submission had 2 defects and 1 nitpick. Repairs are implemented and checks pass. No independent approval of those reviewer-authored repairs is claimed here.

## Findings and repairs

**D1, implementation output became the expected value.** The first test demanded an exact JSON spelling of AC13's printed hand values. After observing the implementation's f32 spellings, GLM changed the expected string to `46.723663,119.565445,138.98863`. Its report explicitly describes pinning the observed forms. This breaks the repository rule that expected pixels come from the standard or oracle rather than this implementation's output. The required one-decimal text was correct throughout.

The lead ruled on 2026-11-08 that this new value-field test must parse one closed three-channel numeric array, require finite values, and compare against the unchanged AC13 hand values `[46.72366,119.56544,138.98863]` using the already approved PS3.3 YBR fixture tolerance, `0.001`, from `crates/ocelli-pixel/tests/color.rs`. The exact text `R 46.7 G 119.6 B 139.0` remains separately asserted. This corrects the new test's unsupported exact-string contract. It changes no renderer, oracle score, existing tolerance or gate.

That repair passes. A meaningful source mutation added 1 to the numeric R field while retaining the correct text. The selected test exited 101 specifically with `RGB channel was 47.723663, expected hand value 46.72366`. Restoring the source returned it to green. Thus correct text alone cannot satisfy the numeric check. The mutation was never staged and the production source was restored exactly.

**D2, the final artifact fails the repository's unchanged clippy rules.** Independent `bin/ocelli.sh clippy ocelli-folder-viewer` exited 101 with seven diagnostics: one Result unwrap, one complex return type, four strict float comparisons at two checks, and one narrowing cast. The delegated task requested the one test target rather than a broad gate, so this is an artifact readiness finding, not a claim that GLM skipped a requested clippy command.

Codex kept builder errors as Results until fixture construction, named the tagged-element type, used exact float bit comparisons for the exact constant window fields, and made the small frame counter a u8 from construction. No lint allowance or rule was added. The complete named clippy command then exited 0.

**N1, the 16-bit sample test name said the opposite of its case.** The corrected GLM fixture already used stored words `4095,2048,15` at Bits Stored 16 and expected `15,8,0`, but its name said the samples were not scaled. The name now states that the declared shift applies. The values are unchanged.

The reviewer also retained the originally assigned three-entry fallback ramp alongside GLM's one-entry channel example. At declared depths 16 and 12, each channel table `[0,100,255]` and indices `[0,1,2]` must give packed RGB `[0,0,0,100,100,100,255,255,255]`. These expected bytes come from the plan's R1 palette rule. No observed viewer output supplied them.

## Verification

All commands below read their actual process exit. Final tests are over staged tree `9ef82d3f`, with no unstaged production difference.

| Command or control | Exit and observation |
|--------------------|----------------------|
| GLM's autonomous internal run 1 | 17 passed, 21 failed, fixture defects |
| GLM's autonomous internal run 2 | 36 passed, 2 failed, stored versus displayed words and float-spelling test defects |
| GLM's autonomous internal run 3 | 0, 38 passed |
| Independent clippy on the autonomous file | 101, seven diagnostics described above |
| First Codex test repair compile | 101, a derive attribute was accidentally attached to the new type alias, corrected before acceptance |
| `bin/ocelli.sh check ocelli-folder-viewer` on the repaired file | 0, all targets |
| `bin/ocelli.sh clippy ocelli-folder-viewer` on the repaired file | 0, unchanged rules |
| `bin/ocelli.sh test ocelli-folder-viewer --test colour -- --test-threads=2` | 0, 38 passed |
| Wrong numeric R with correct text | 101, one selected semantic failure |
| Restored numeric case | 0, one selected case passed |
| `--lib f239_` | 0, three permanent palette admission, refusal-cache and calibrated geometry controls |
| `--test preview colour_server_` | 0, three Codex server controls |

Direct receipts and full outputs are `/private/tmp/ocelli-s46-codex/f239-final7-*`, `f239-glm-tests-*` and the GLM task directory. The test file covers native RGB and planar order, YBR and native 4:2:2, palette lookup/scaling/refusals, 8 to 16 bit scaling, signed/depth refusals, colour windows/invert/ties, pointer values, ROI channel statistics and calibration, ICC presence and codec-owned JPEG RGB. It does not certify the browser, GPU, oracle, maximum-size memory or performance rows. The source and independent server reports retain their own scope.

## Measurement and identity record

The measured local task used model `dgx/GLM-5.3-Flash-EXL3` with fresh task data and the existing provider configuration. It ran from 13:12:34.536542 to 14:05:39.880317 UTC, elapsed `3185.336526` monotonic seconds, process exit 0, with zero launcher or provider retries. Internal compilation and fixture corrections are separate from that retry count. The first test artifact appeared about 36 minutes into the run. No Codex correction was supplied until the process finished.

The local usage tool reported input 269162, output 48177, reasoning 72181 and cached read 13206144 tokens, across 75 steps. Those are the tool's separate fields, not an invented combined total. Its zero reported cost is the local tool's value, not a pricing claim. Its tool-call success counts describe tool transport and do not mean that all test commands passed.

All seven source SHA-256 values in GLM's report match the final production tree `cc62cdb4`. The index was still `b5ae6518` during its recorded test run while production worktree content had moved. The reviewer initially suspected stale hashes, compared them against the immutable tree, found every hash correct and retracted that suspicion. The report's mtime and quiet-period narrative is not used as source identity evidence. Fresh independent receipts pin the repaired tests and production explicitly.

The task overlapped another local GLM bench-instrument task and Codex server development. It was not a controlled repetition of a Claude task. Author timing observations, their missing monotonic start limitation, the original final GLM file and report, intermediate compile failures and all later reviewer repairs are retained. No speed ratio or superior-model conclusion is claimed from these different tasks.
