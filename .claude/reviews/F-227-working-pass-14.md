# F-227 review, pass 14

**Reviewed**: `git diff 8b2e979 58e6db4` on branch `work/f-227-xai`, HEAD `58e6db4a52071ac6e0a3b15fa77bfeac8bc78346`, tracked tree clean. Files: `.claude/plans/F-227-design.md` (2f02e51, lead) and `crates/ocelli-pixel/src/auto_window.rs` (58e6db4, implementer). Library half only.
**Author / reviewer**: plan edits by the sprint lead (Claude), source change by a Claude implementer, review by an independent Claude reviewer who authored none of the reviewed material. All are the same model family, which is the independence limit.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 13 D3, the pool buffer growing past 4 MiB.** The structural claim in the new comment at `auto_window.rs:221` is that no push can reach past `sample_budget(K) * K`. Derived independently: with `b = cap / K` and `s = ceil(len / b)`, `s >= len / b` so `len / s <= b`, and since `b` is an integer `ceil(len / s) <= b`. At most `K` frames are committed (`TooManyFrames` returns before any push), and a refused frame truncates to `mark`, so the committed total is at most `b * K <= 2^20`. Checked by execution, not only by the argument, below.
- **The plan item 7 sentence** "reserved once for the whole pool, at most 4 MiB of `f32`, and never grows". True: the only allocation is `Vec::with_capacity` in `new`, and `finish` allocates nothing (read: `select_nth_unstable_by` is in place, `VoiTransform::new` with `lut = None` builds a `VoiSelection::Window` of plain `f32` fields, `lut.rs:326` to `363`, and `search_window`, `display_bits`, `margin_pair` and `outward_pair` hold only scalars).
- **Pass 13 D1 and D2** (prose). The LINEAR sentence is deleted from item 8. `PixelError is not extended.` is true: `git diff $(merge-base main) HEAD` touches no `crates/ocelli-pixel/src/error.rs`. Both closed.
- **Pass 13 N3.** `#[derive(Clone, Copy, Debug)] // PartialEq by hand, bitwise` matches source lines 63 to 80. Closed.
- **Pass 13 N2.** `width = 1` and "Exact while" are gone. The remaining `center = m_max + 0.5` is N1 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Arithmetic sweep: every `K` in 1..=16, every `len` in 1..=17 * 2^20, assert `len.div_ceil(len.div_ceil(b)) <= b` | scratch copy of HEAD under `target/p14-scratch`, `--release` | now | pass. `debug_assertions = false` printed. Reserves 1,048,567 (K=13) to 1,048,576. Lengths that fill the budget exactly: 153 at K=1 up to 37,128 at K=16, so the bound is reached, not just approached |
| Pool probe through the `#[cfg(test)] capacity()` accessor: every K 1..=16, lengths 1, 2, 3, 7, b-1, b, b+1 and m*b-1, m*b, m*b+1 for m 2..=8 (K=7 gives 299,591, 299,592, 299,593 and so on), K frames each, then a `K+1`th | same scratch, `--release`, no `debug_assert` | now | 868 cases pass. Capacity equal to `b * K` before and after every frame, `len == capacity` exactly at `len = b`, `TooManyFrames` after K with capacity unchanged |
| Refused frame in the middle: K-1 good, one with `Stored(INFINITY)` at its last strided index, then one good | same | now | `NonFiniteModality`, length back to its mark, capacity unchanged, final count `K * ceil(len / stride)` |
| Mutant M1: reserve `sample_budget(frames)` | scratch, `auto_window::tests` | now | both tests red in debug (the `debug_assert` at line 280) and in release (the capacity assertion at line 697) |
| Mutant M3: reserve `sample_budget(frames) * (frames - 1)` | same | now | both red, debug and release |
| Mutant M2: reserve `sample_budget(frames) * frames - 1` | same, the two new unit tests | now | both GREEN, debug and release. N2 |
| Mutant M2, whole `cargo test -p ocelli-pixel` (debug) | same | now | red: `fixture_cap_is_split_across_frames` panics at the `debug_assert` (line 280) through its one-frame 2^20 case, which fills the cap exactly |
| Background centre through `auto_window` on all-zero frames, slope 1, intercept m | scratch, debug | now | m = 0 and 1000 give `m + 0.5`, width 1. m = 1.5000001 gives centre 2.0000002, width 1.0000001. m = 4194303.75 gives 4194304.5, m = 8388608 gives 8388609.0, both width 1.0000001. N1 |
| `cargo fmt --all --check` | worktree | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | worktree | now | 0 |
| `cargo test -p ocelli-pixel` | worktree | now | 0. lib 16, auto_window 32, color 15, image_plane 4, lut_chain 16, modality 5, palette 7, stored_pixel 2, voi 6, doc 0 |
| `cargo build -p ocelli-pixel`, and `--target wasm32-unknown-unknown` debug and release | worktree, `#![cfg_attr(not(test), no_std)]` | now | 0, 0, 0. No bare-metal target is installed, so `no_std` is proved by the attribute compiling on host and wasm32 |
| `python3 scripts/prose_check.py` | worktree | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | worktree | now | 0, "0 watched by nothing" |
| `git status --short`, `git diff --quiet`, `git diff --cached --quiet` | worktree after deleting `target/p14-scratch` | now | tracked tree unchanged, HEAD still 58e6db4, only the pre-existing untracked reviews and `tools/oracle/out` plus this report |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1** plan line 156, the Background bullet, "`center = m_max + 0.5`". This is the search's first try, not always the result: m = 1.5000001, 4194303.75 and 8388608 widen to 1.0000001 and the centre moves with `m_max + width / 2` (probe above). Covered under the precedence sentence at line 144, because the pass 3 amendment's widening search for a background window "keeping the centre rule of each window kind" visibly changes this bullet, which is why pass 13 counted the same shape a nitpick. Writing `center = m_max + width / 2` would be true at every probed value and stop it reopening. Not blocking.
- **N2** `auto_window.rs:670` to `719`, the two new unit tests never fill the reserve. At K=5 they push 1,039,360 of 1,048,575 and at K=3 786,432 of 1,048,575, so a reserve short by up to 9,215 values (M2 is short by one) passes both in either profile. The suite still catches M2 in the default debug run, through `fixture_cap_is_split_across_frames`'s exact 2^20 fill tripping the new `debug_assert`, so the off-by-one is covered, but only via a debug-only assertion in another file and only at K=1. A case at `len = sample_budget(K)` for some K above 1 (for example K=7, len 149,796) would fill to capacity exactly and pin the boundary in the unit tests themselves. Not blocking, because a mutant of the guarded line goes red in the gate's own `cargo test`.
- **Carried**: pass 12 N1, pass 11 N1 (pass 10 N2), pass 9 N1 to N3 and the pass 7 nitpicks pass 9 carried, unchanged because their text and code are unchanged in this diff.

## Verified clean

- The reserve `sample_budget(frames) * frames` is at most `AUTO_WINDOW_SAMPLE_CAP`, 2^20 values, 4 MiB of `f32`, for every K. The allocator returned exactly that capacity in all 868 probe cases.
- No push exceeds it for any K from 1 to 16 and any length: proved above and executed for every length to 17 * 2^20 arithmetically and for the boundary lengths through the real pool in a release build with `debug_assert` compiled out. Length 0 returns `EmptyFrame` before any mutation. Lengths below the budget read every sample (stride 1).
- The `debug_assert!(len < capacity)` is reached: M1, M2 and M3 each trip it in a debug run. It is the right inequality, since a push at `len == capacity` is the one that reallocates.
- The `#[cfg(test)] fn capacity` accessor is test-only, so the public API is unchanged and `clippy --all-targets` sees no dead code.
- The test doc arithmetic: 1421 * 1024 = 1,455,104, budget 209,715, stride 7, 207,872 a frame, 1,039,360 total, doubling to 1,677,720 values and 6,710,880 bytes. 1024 * 1024 at budget 349,525 gives stride 4, 262,144 a frame, 786,432 total, doubling to 1,398,100 values and 5,592,400 bytes. All recomputed.
- The new comment at line 221 (stride bound, at most `frames` frames) and the test comment that `finish` allocates nothing are true by the reading above.
- Plan item 7, item 8 after the deletion, the `StoredRange` derive line and the `PixelError` line are true against the source.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half, the oracle, real corpus bytes. The probes and mutants ran in a scratch copy of HEAD under `target/p14-scratch`, deleted at the end, and nothing in the worktree was edited except this report.
- `no_std` is shown by the crate compiling with `#![no_std]` on host and wasm32. A bare-metal target without `std` was not available.
- Pass 13 D1, D2 and D3 are closed, and N3 is closed. N1 and N2 here are optional for the lead. This is a clean bounded review of the library half at 58e6db4, not acceptance of F-227 or sprint readiness. The viewer half still needs its own review.
