# F-240 synthetic ICC kind guard, working pass 1

**Result:** 0 defects, 0 smells, 0 nitpicks in the four-path guard-regression and census scope.

**Reviewed:** immutable tree `ac1837e9e0b8e385d659e9b8580586b4e71d81ac` against merge tree `ed389574440aa48b803f8e0e3559a3e78041c737`. The exact reviewed paths are `scripts/tests/test_corpus_synth.py`, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json` and `docs/runbooks/guard-verification.md`. The complete tree difference also includes five Rust composition or formatting paths. Their author explicitly identified them as separate F-227 compatibility work and they are excluded from this approval. This reviewer authored neither the new regression nor the census amendment. The production ICC kind guard itself is unchanged by this delta.

The new permanent regression tries four invalid names, including empty, near-spelling and case-different inputs. Each must raise ValueError with the exact fixed message. It does not derive that expected message from the implementation or accept a fallback gamma profile. The method itself reads no corpus object. The unittest module's setup generates private synthetic inputs, so this is independent of a pre-existing ignored corpus, not a claim that module setup does no fixture work.

The catalogue adds an explanatory standing-test citation and the generated runbook reproduces it. A parsed comparison of the complete budget JSON found exactly one change: `/entry_sites/corpus-synth`, 17 to 18. No uncovered allowance, exclusion, constant or other entry changed. The new production refusal is accounted for rather than hidden or absorbed by a broader allowance.

Runtime review used `/private/tmp/ocelli-icc-kind-review`, an archive initialized as a local Git repository with exactly the frozen tracked paths staged. `git write-tree` equals `ac1837e9e0b8e385d659e9b8580586b4e71d81ac`. This preserves the census's actual `git ls-files` inventory while isolating concurrent author work.

| Independent check | Direct exit and observation |
|-------------------|-----------------------------|
| Exact new permanent unittest | 0, all four subcases pass |
| Complete `IccProfileFixtures` class | 0, five tests passed, including supported fixture controls |
| Remove only the closed-kind guard in private source | 1, all four subcases reject missing ValueError |
| Keep ValueError but change its text | 1, exact anchored name assertions reject it |
| Restore source and repeat permanent unittest | 0 |
| `python3 scripts/guard_census.py --check-runbook` on the indexed snapshot | 0, 1199 discovered refusals all claimed, zero watched by nothing |
| Restore old budget 17 beside the new actual guard | 1, specifically reports corpus-synth growing from 17 to 18 and requires reviewed accounting |
| Restore reviewed budget and repeat census/runbook check | 0 |
| `git diff --exit-code` after private restoration | 0 |

An initial census attempt on the archive before Git initialization exited 1 because `git ls-files` had no repository. That setup failure is retained and is not semantic mutation evidence. The indexed rerun proves the frozen source inventory.

Receipts and full logs are `/private/tmp/ocelli-s46-codex/icc-kind-independent-*`. The independent mutation driver is `/private/tmp/icc-kind-independent-mutations.py`. The author's separate AST mutation was inspected but is not substituted for these fresh controls. No shared corpus, production source, guard policy or gate was changed, and no build, browser, GPU, actual measurement, commit or push was performed. This approval is limited to the four stated paths and does not complete F-240 or accept the excluded F-227 changes.
