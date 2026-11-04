# F-194 review, pass 2

**Reviewed**: working tree on `sprint/s42` over parent `c90bdcd` after the pass 1 remediation. No tree hash. The reviewer had no shell.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 1 smell, 4 nitpicks

## Acceptance challenged

Pass 1's linear coordinate fix, the off-centre weight, mode 2, and the draw allocation. The new footprint test mixes `&&` and `||`. Nearest and linear do not share one origin inside `blend_display`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read and grep only | working tree | executed now | No test was run in this pass |

## Defects

### D1, the footprint test was not valid WGSL

**Where**: `graph.rs` linear footprint `if`.

**What**: `need_x && ... || need_y && ...` without parentheses. Naga accepts it. The WGSL grammar does not, so a browser Tint parse can reject the whole stack module, including nearest.

**Why it is wrong**: an operand of `||` cannot be an `&&` expression.

**Evidence**: grammar, not a Dawn run. Plausible.

**Scope and regression**: parenthesise both sides. A source test contains that parenthesised text.

### D2, nearest and linear do not share centre-origin

**Where**: `blend_display` nearest calls `texel_of` on the raw coordinate.

**What**: The doc written in the remediation said both modes are centre-origin. `texel_of` reads a corner-origin image coordinate. The approved fixture still wants nearest 0 and linear 138.75 at `(0.5, 0.5)`.

**Why it is wrong**: one sentence cannot call those the same geometric point.

**Evidence**: read the fixture and `texel_of`. The raster nearest path does not call `blend_display`.

**Scope and regression**: document the approved pair. Do not change the nearest number. The page still shows 0 and 138.75.

## Smells

### S1, `allocations` did not see an allocation that forgot to increment

**Where**: `linear_texel_centres_match_the_nearest_texel`.

**Mechanism**: putting a `Vec` back into `fill_display` without touching `allocations` left the equality green. The guard is a linear draw with no reserve, which must return `Extent` and leave the buffer unchanged.

## Nitpicks

- The old mode-2 test still started from nearest. The settled check lived only in `cpu.rs`.
- `available` is true for every real tier, so the new refusal branches have no failing input.
- `Interpolation::mode` has no caller.
- `device_camera_words` still uploads nearest, and `number` still special-cases two bit patterns.

## Verified clean

Pass 1 D1 is fixed for linear. `centre_sample` returns index space. The shader subtracts half a texel only in the linear branch. A zero fraction does not read the next texel. Pass 1 D2's off-centre value 166.5625 is right, and a swapped middle pair is 116.5625. Pass 1 D3 is in the CPU settled test. Pass 1 D4 is in the code. Pass 1 S2 rejects any lane other than exact 0 and exact 1 before the `u32` conversion. Pass 1 S3 shares `centre_sample` and `blend_points`.

## Limitations and handoff

No shader compiler was run. The implementer parenthesises the footprint test, corrects the coordinate doc without moving the approved nearest value, and adds the unreserved-draw refusal. Pass 3 reviews that tree.
