# F-194 review, pass 5

**Reviewed**: working tree on `sprint/s42` after pass 4, plus the unstaged `crates/ocelli-wasm/src/session.rs` allocation assertions saved in `/tmp/ocelli-f194-pass5.diff`. No tree hash. The reviewer had no shell.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The floor test gate failed two wasm session consumers that pass 4 did not execute. Both expected one colour allocation and observed two. The question was whether expecting 2 fits the reserve contract or only the counter.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read and grep only | current `session.rs`, `cpu.rs`, `tests/cpu.rs` | executed now | No test was run in this pass |
| `bin/ocelli.sh test ocelli-wasm session::tests` | host receipt after the edit | prior receipt | 21 passed, including both tests that had failed |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Expecting 2 is the reserve contract. `CpuRaster::allocations` counts times the backing storage has been allocated. Only `reserve_source` increments it. That function grows `rgb_scratch` and `display_scratch`, one increment each. A 2 by 1 colour source therefore goes from empty to 2 RGB entries and 6 display entries. A 1 by 1 source goes to 1 and 3. The CPU colour test at `tests/cpu.rs` already expects 2 after one `reserve_source(1, 1)`.

`draw_cpu_rgb` accepts the source length before `reserve_source`, so a length refusal does not grow either buffer. Later draws of the same source compare against the captured count. A third growth would turn those checks red. Nearest mono draws do not call `reserve_source`, and their counts stay 0. Nothing in `ocelli-wasm` calls `set_interpolation`.

Dropping the display-plane increment would make both new assertions fail. Reallocating `rgb_scratch` on every call would push a later draw to 3. The approved nearest value 0 and the linear value 138.75 were not moved.

## Limitations and handoff

`reserve_source` runs before the stale and idle checks inside `raster_rgb_into`. A stale call at a larger source can grow the scratch buffers while writing no pixels. That was already true on the pass 4 tree, and no pixel changes. This pass did not re-run the tests. It is not sprint readiness by itself.
