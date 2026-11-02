# F-075 review, pass 1

**Reviewed**: HEAD `9ebfbaba1588f79327be8db08fc173492e4642c6` on `sprint/s29`. Nothing was staged. The product diff is unstaged and limited to `crates/ocelli-geom/src/lib.rs` (`2916f28f2a414f3cef5bdbc76463289ad44ef2d1`), `crates/ocelli-geom/src/stats.rs` (`f95b43e5e9e452f5f2d5663cadd6f46251783be7`), and `crates/ocelli-geom/tests/stats.rs`. Those are raw SHA-1 pins from `shasum`. The pass read the test at `32b9efb539275fb1610987bb0c717ebc9d338090`. Rustfmt then collapsed one assertion onto one line, and the formatted file is `6cfcb89c7e4e7c29c49cad8f1ea13421848cf463`. The assertions did not change. Mutations in this pass were restored.
**Author / reviewer**: The F-075 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change. Reviewer id `01a19c17-6269-7310-b3f8-f8c2022a051c`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Samples 1 and 3 with area 4 are mean 2, population sd 1, min 1, max 3 and area 4. The divisor is the count. Area 4 is not the product of the samples.

AC7 and AC8. Elapsed 0 is 140. One half-life is 280, which needs `2^(-1)` to be one half. Changing the SUV factor `1000.0` to `1001.0` failed `elapsed_zero_is_one_hundred_forty` and `one_half_life_is_two_hundred_eighty`. The constant was restored to `1000.0` and the stats tests passed again.

Empty samples are `Degenerate`. A non-finite histogram sample leaves the previous counts in place. An interior edge value belongs to the higher bin. Equal samples stay in bin 0 with width 0. Units other than BQML and a zero weight are `Unavailable`. A non-finite SUV input is `Frame`. The function does not call the modality LUT. The skill formula is not presented as an HLD equation.

## Evidence

| Command | Observation | Exit |
|---------|-------------|------|
| `bin/ocelli.sh test ocelli-geom --test stats` before the mutation | 10 passed | 0 |
| same command with SUV factor `1001.0` | the two SUV fixtures failed | 101 |
| same command after restoring `1000.0` | 10 passed | 0 |
| `bin/ocelli.sh clippy ocelli-geom` | clean | 0 |

No patient data was used. wasm-pack was not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.
