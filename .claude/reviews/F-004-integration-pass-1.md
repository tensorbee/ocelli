# F-004 review, integration pass 1

**Reviewed**: `9df8539`, the integrated F-ID commit.
**Reviewer**: the integrator, independent of the implementing agent.
**Context**: the implementing agent terminated on a session rate limit during
its own review loop, so this pass carries more weight than usual. It is the
only review this story has had.
**Result**: 0 defects, 0 smells, 0 nitpicks.

## Defects

None.

## Smells

None.

## Verified clean

**Arithmetic, checked at the site.** Zero `as` casts in `caps.rs` and
`probe.rs`, and the four grep hits are comments. Zero `#[allow]`. Zero `unwrap`
and zero `expect`. The fill-rate comparison is
`pixels * NANOS_PER_SECOND >= threshold * elapsed_nanos` with both sides widened
by `u128::from`, which is cross-multiplication instead of division, so there is
no float for `float_cmp` to object to, no cast for HLD 27.3 to review and no
rounding decision to get wrong. That is the right shape and it was specified in
the plan rather than improvised.

**A design point worth recording because it is easy to lose.** `probe` counts
pixels and does not time itself. `std::time::Instant` panics on
`wasm32-unknown-unknown`, and the alternative is a dependency reaching
`performance.now()` inside `ocelli-render`, which is precisely the browser
binding deviation D-12 says this crate must not grow. The clock is therefore
the caller's, and D2's payoff survives.

**The combination rule.** Exhaustive `match` over the three verdicts, so a
fourth variant fails to compile rather than inheriting a catch-all. The
benchmark decides outright on `Hardware` or `Software` and only `Unknown` falls
through to adapter type, then renderer string, then keeps the candidate. That
is `docs/spikes/A7-tier-c.md`'s "trust the benchmark" implemented rather than
paraphrased, and it is what contains the known `gallium` false positive: a
string is consulted only when the two stronger signals abstained.

**No dead branch, which was the thing I most expected to find.**
`software_ceiling_pps` is `None` in `FillRateBands::RECORDED`, so the
benchmark's `Software` verdict cannot arise in production today. It is not
unreached code: `at_software_ceiling` supplies a synthetic band,
`exactly_at_the_software_ceiling_is_software` and
`one_pixel_above_the_software_ceiling_is_unknown` pin the band edge in both
directions, and `without_a_recorded_ceiling_a_slow_rate_is_unknown_not_software`
asserts the production behaviour separately. Detection still works with the
ceiling absent, because a rasteriser measures slow, falls to `Unknown`, and is
then caught by `wgpu::DeviceType::Cpu`.

**A recorded number that states its own provenance.**
`ci/tier-thresholds.json` gives each figure its own provenance field: one
measured with machine, adapter, elapsed nanoseconds and an observed range, one
DERIVED from it and saying so, and the software ceiling `null` with the
consequence spelled out, that while it is null the benchmark never returns
`Software`. Spike A7.3 says do not invent a number, and this is the difference
between an absent figure that says it is absent and a guess wearing a note.
`the_recorded_bands_match_the_checked_in_file` stops the constant and the file
drifting apart.

**Deviation D-14 applied and independently confirmed.** wgpu 30.0.1's default
features, read from the pinned crate's own manifest, are `std, parking_lot,
dx12, metal, gles, vulkan, wgsl, webgpu`. `webgl` is a real feature at line 117
and is not among them. `ocelli-render` now takes it.

**The ignored test is not a skipped test.** It is the measurement instrument
and it needs a real adapter, which the CI floor does not have. It is named and
its reason is at the site.

**Gates and totals.** `bindgen`, `pins`, `nostd`, `unsafe`, `device`,
`deviations` and `prose` all pass, read from each command's own exit status.
The workspace test count went from 45 at the sprint base to 109 with none
failing. `ci/wasm-size-budget.json` is still 16388, which is F-005's figure,
confirming this story honoured the design round's decision not to touch it.
