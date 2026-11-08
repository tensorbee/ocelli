# F-227 review, pass 10

**Reviewed**: library half, plus the one inventory sweep the brief asks for. `git diff 4da5d53 b502385` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `b502385` ("F-227, the AC9 row names AC1's own window"), `git write-tree` `12a4ceffa765650ed9bb123d4dc652a90480ac0c`, no unstaged tracked changes. The diff is one line of `.claude/plans/F-227-design.md`, line 327. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, and the plan sha1 `b2e92511`. Untracked `tools/oracle/out` and the pass 1 to 9 reports are not part of the review.
**Author / reviewer**: the edited row is the sprint lead's, as the caller's brief says. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 9 were written by the same model, so this pass is independent of the correction but not of the finding it answers.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 9 D1, AC9 at line 327.** The row now reads "`AutoWindow::transform(0.0, 255.0)` of AC1 equals `VoiTransform::new` with AC1's own centre and width, `VoiFunction::LinearExact`, 0.0 and 255.0".
  - Against the test that owns it: `auto_window_pool_refusals` asserts `window.transform(0.0, 255.0) == VoiTransform::new(None, &[f32::from_bits(0x41C7_FFF0)], &[f32::from_bits(0x4243_FFF0)], 0, VoiFunction::LinearExact, 0.0, 255.0)`. Those bits are AC1's window as `fixture_pet_like_frame_leaves_out_background_zeros` asserts it.
  - Against the code: `AutoWindow::transform` is `VoiTransform::new(None, &[self.center], &[self.width], 0, VoiFunction::LinearExact, ymin, ymax)`.
  - Probe through the real `auto_window`: equality with the window's own centre and width is `true`. With `VoiFunction::Linear` it is `false`, so the row's "A transform built as LINEAR" rejection holds. With 25 and 49 it is `false`, which is why the old row was wrong. **Closed. The new row is true.**
- **The inventory sweep, AC1 to AC18, once.** Every stated centre, width, low, high, count, basis and grey was compared with the real library's output for that row's inputs (probe table below) and with the test that owns the row. The inventory writes values to the precision it shows, as AC1's "Stored 20 is 49.44" for 49.4388 does. At that precision every library row holds. The exception is AC11, which states exact strings, D1.

Probe output on HEAD, centre and width as `f32` `Display` with bits:

| Row and inputs | Basis, n, low, high | Centre | Width | Greys the row states, measured |
|---|---|---|---|---|
| AC1 | Percentile, 100, 0.5, 49.5 | 24.99997 `41C7FFF0` | 48.99994 `4243FFF0` | m10 49.43884 g49, m25 g128, m0 and m0.5 g0, m49.5 and m500 g255 |
| AC2 pooled | Percentile, 200, 2, 198 | 99.99988 | 195.99976 | m100 g128, m150 192.55127 g193 |
| AC2 A alone | 100, 1, 99 | 49.99994 | 97.99988 | m100 g255 |
| AC2 B alone | 100, 101, 199 | 149.99994 | 97.99963 | not asserted by the test, true at the row's precision |
| AC3 | Percentile, 200, 2, 196 | 98.99988 `42C5FFF0` | 193.99976 `4341FFF0` | m50 63.09286 g63, m100 128.81459 g129 |
| AC3 wrong slopes | slope 1 twice, slope 2 twice | 49.99994, 99.99988 | 97.99988, 195.99976 | the row's 50, 98 and 100, 196 |
| AC4 guarded | 100, 1, 99 | 49.99994 | 97.99988 | |
| AC4 unguarded | 120, -2000, 99 | -950.49915 `C46D9FF2` | 2098.9978 `45032FF7` | the test asserts these bits and derives them |
| AC4 CT | 100, -24, 74 | 24.999954 | 97.99985 | m25 g128, m-1024 g0 |
| AC5 | 262245, 1, 99 | 49.99994 | 97.99988 | whole cap per frame gives high 60000, as the row says |
| AC7 150 | 150, 2, 149 | 75.49988 | 146.99976 | m75 126.63 g127, m75.5 g128 |
| AC7 101 | 101, 2, 100 | 50.999943 | 97.99988 | |
| AC8 constant 7 | Constant, 16 | 7 exact | 1 exact | m7 127.5 g128 |
| AC8 blank and two-frame background | Background, 0 | 0.5 exact | 1 exact | g0, and -2000 g0 |
| AC8 signed | 99, -50, 49 | -0.5 exact | 98.99988 | g126, g129, g131 |
| AC8 stored 3 and 9 | Percentile, 2 | 5.9999943 | 5.999981 | g0, g128, g255 |
| AC9 refused then stored 5 | Constant, 1 | 5 | 1 | `NonFiniteModality` first |
| AC13's frame through the library | 50, 51, 100 | 75.49997 | 48.999817 | the route is not written yet |

- **Independently derived discriminating case.** AC1 "Counting zeros gives centre 24.5 and stored 50 grey 130": counting 200 values puts rank 2 at 0.0 and rank 198 at 49.0, centre about 24.5 and width about 49, and modality 25 draws `((25 - 24.5) / 49 + 0.5) * 255 = 130.1`, grey 130. AC7's linear-interpolation grey 115 recomputed the same way, P1 0.995 and P99 54.005, gives 115.47, grey 115. The margin moves neither by more than about 5e-4.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff 4da5d53 b502385`, `git diff --stat 4da5d53 HEAD`, `git write-tree`, `shasum` | HEAD `b502385` | now | one line of the plan. Tree `12a4ceff`. Source and test sha1 unchanged from pass 7 |
| `cargo test -p ocelli-pixel`, `CARGO_TARGET_DIR` the worktree's `target` | tree `12a4ceff` | now | exit 0. `auto_window` 32 passed in 5.88 s, every other binary ok, 0 failed |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| probe crate `target/pass10-probe`, path dependencies on `ocelli-pixel` and `ocelli-core`, deleted after the run with its build outputs | the real `auto_window`, `AutoWindowPool`, `AutoWindow::transform`, `VoiTransform::new` | now | the table above, and the three AC9 equalities `true`, `false`, `false` |
| `grep` for `series-window`, `Series-Window`, `Auto-Window` over `examples` and `crates` | HEAD | now | no route, page or viewer test yet. The viewer half is unwritten |
| pass 7 and 8 mutations, scans and sweeps | unchanged source | prior receipts | stand for unchanged code |

## Defects

### D1, AC11 states the series window strings as `99` and `194`, which the library and the plan's own text rule cannot produce

**Where**: `.claude/plans/F-227-design.md:329`, AC11: "200, centre `99`, width `194`, sampled 2, skipped 0. Then `/preview` of frame B's file with `X-Ocelli-Series-Window: 99 194` gives grey 129".
**What**: AC11's inputs are AC3's two frames, slope 1 and slope 2. Since the pass 4 rulings the library's window for them is centre `0x42C5FFF0` and width `0x4341FFF0`. Item 17 (line 168) and the `serve.rs` bullet (line 266) fix the reply as Rust `f32` `Display` strings, "the shortest string that parses back to the same `f32`", and line 303 says "The page sends back exactly the strings it received". Those strings are `98.99988` and `193.99976`, not `99` and `194`. The row's values are in code format and the header line is literal, so it is a statement about exact strings, the same kind of exact claim as pass 9 D1, and the pass 4 margin made it false. Before pass 4 the window was exactly 99 and 194 and the row was true.
**Why it is wrong**: microscope section 3, a false claim in prose. The inventory is the contract `/complete-feature` maps tests to. An implementer who writes `the_series_window_pools_each_file_rescale` from the row asserts `"center":"99"` and goes red on a correct route, or makes the route round to match the row, which sends the page a window the library's check never accepted. A test that posts `X-Ocelli-Series-Window: 99 194` exercises a header the page never sends. The grey survives either way: 128.81459 through the real window, 128.81 through 99 and 194, both grey 129, so the grey cannot catch it.
**Evidence**: the probe's AC3 line, `c 98.99988 (0x42C5FFF0) w 193.99976 (0x4341FFF0)`, printed with `{}`, which is `f32` `Display`.
**Scope and regression**: prose only, carried since the pass 4 rulings. It is a viewer-half row, outside this pass's library scope, but the value it states is the library's output and the brief's sweep covers every row from AC1. The route and its test do not exist yet, so nothing is red today. The smallest fix is to state AC11's reply as the `Display` strings of AC3's window, or as "AC3's centre and width as `Display` strings", and the header as the reply's strings sent back. No code or test change is needed.

## Smells

None.

## Nitpicks

- **N1** `.claude/plans/F-227-design.md:266`. The example reply `{"center":"25","width":"49", ... "counted":262144}` names no case, so it is not false, but under the same bullet's `Display` rule an integer pair is a string the moved-pair window will almost never produce. A reader copying it into AC11's test reproduces D1.
- **N2** `.claude/plans/F-227-design.md:315`. The inventory does not say it writes centres and widths to the precision shown. Every row except AC11 holds at that precision, and the test file header (lines 19 to 28) states the reading for the fixtures, but the plan does not, which is how AC9 and AC11 could state exact equality and exact strings beside rounded values. Not blocking, because nothing in the remaining rows is false.
- **Carried**: pass 9 N1 to N3, and the pass 7 nitpicks pass 9 carried, unchanged because their text and code are unchanged.

## Verified clean

- Pass 9 D1 closed. The new AC9 row is true against the owning test, `AutoWindow::transform` and a probe, and its LINEAR rejection is observed false-equal.
- AC1 to AC9 and AC13's library values: every stated count, rank value, basis, centre, width and grey holds at the row's written precision against the probe, and every one a test asserts is asserted by the test the row names. AC4's unguarded window is asserted by bits and derived in its test comment.
- AC6 is untouched by the amendments and asserted exactly by `fixture_series_sample_spacing`.
- AC10, AC12, AC14 to AC18 state no centre or width. AC10's grey 128 for one stored 50 is the Constant window, unchanged by the margin.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` pass.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half (not written), the oracle, real bytes. No mutation was run, because no code changed since pass 7's mutation table. AC11 was checked through the library on AC3's inputs, not through a Part 10 file and the route, which assumes the slopes 1 and 2 parse exactly from DS, as they do in any decimal parser.
- Next owner: the lead restates AC11 (D1), prose only, and a fresh independent pass 11 reviews it. This is a bounded review of the library half and one inventory sweep, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` `12a4ceff` and the three sha1s were re-checked at the end. The probe lived under the worktree's ignored `target/` and was deleted with its build outputs.
