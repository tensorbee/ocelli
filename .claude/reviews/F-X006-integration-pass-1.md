# F-X006 review, integration pass 1

**Reviewed**: `3d5b0bc`, the integrated F-ID commit.
**Reviewer**: the integrator, independent of the implementing agent.
**Context**: the implementing agent terminated on a session rate limit after
writing both answer files and before its handoff. Both answers were complete.
**Result**: 0 defects, 0 smells, 0 nitpicks.

## Defects

None.

## Smells

None.

## Verified clean

**The A1 root cause, confirmed in the crate source rather than from the
report.** `openjp2` 0.6.1 at
`~/.cargo/registry/src/index.crates.io-*/openjp2-0.6.1/src/malloc.rs` declares
`malloc`, `calloc`, `realloc` and `free` inside `extern "C"`, and the file
contains **zero occurrences of `cfg(`**, so nothing guards it by target. Its
`[lib]` declares `crate-type = ["cdylib", "staticlib", "rlib"]`, so cargo links
a cdylib for it even as a dependency, which is where the link fails on a target
with no libc. The story's claim holds exactly as written.

**The answer is a measurement and not an argument.** Four decodes reduced to
one canonical form, 12288 bytes of little-endian u16 at 64 by 96, which is the
shape `scripts/corpus_synth.py` actually produced rather than a shape chosen for
convenience. Two JPEG 2000 Part 1 rows are decoded through the same build as a
control, so "openjp2 does not work on wasm32" and "openjp2's HTJ2K path does not
work" are distinguishable rather than conflated. That control is the single best
thing in this story and nothing required it.

**The comparator was observed red before it was trusted.**
`tools/spikes/common/tests/compare_test.mjs` runs green, exit 0, five checks,
re-run by me. Its two recorded mutations, inverting the equality and removing
the PGM big-endian swap, each turned it red. It refuses any buffer that is not
exactly 12288 bytes, so a truncation cannot pass as equality over a shorter
buffer.

**The anchor's weakness is stated rather than hidden.** For A2, `pyjpegls`
encoded and `dcmdjpls` decodes and both wrap CharLS, so their agreement is not
independent evidence. The answer says so and rests the strong claims on the
uncompressed reference for `.80` and ISO 14495-1's NEAR bound for `.81`. For
A1, the `.203` row has no encoder-independent anchor because OpenJPH encoded it,
and that is recorded as a stated limit on that row's evidence rather than
absorbed.

**The new guard refuses, and with its own message.** I checked the concern I had
raised in advance: adding a path to `ORACLE_OUTPUT_PREFIXES` without touching
its message would produce a correct refusal with a wrong explanation. The story
instead added a SEPARATE `SPIKE_OUTPUT_PREFIXES` constant with its own comment
and its own message. Staging `tools/spikes/out/probe.j2c` with `git add -f`
gives exit **1** and "spike output. Raw codestreams and decoded pixel buffers
under tools/spikes/out/ are extracted from corpus rows and are derived from
them". The control after cleanup is exit 0. The oracle message is untouched.

**Source policy.** Every candidate crate's licence is named from registry
metadata and none is copyleft, so none is read-blocked. `openjph-core`'s missing
repository URL is flagged as a fact that prevents answering one of
`docs/SOURCE-POLICY.md`'s questions, and it does not reach the recommended
route. `gate provenance` passes over the whole tree.

**Gates.** `provenance`, `prose`, `content`, `deviations`, `unsafe`, `pins`,
`skills` and `backlog` all pass in the worker tree, and the whole floor passes
over the merged tree at 23 gates, each read from the command's own exit status.

**No `unsafe` and no `wasm-bindgen`.** The A1 harness uses a hand-written
integer ABI and reads its output from `instance.exports.memory.buffer` on the
JavaScript side, so no raw pointer is dereferenced in a tracked Rust file. It
pins `edition = "2021"` deliberately, because Rust 2024 spells an exported
symbol `#[unsafe(no_mangle)]` and `unsafe_allowlist_check.py` matches the bare
token, so a file containing no unsafe code would otherwise fail `gate unsafe` on
an attribute.

## Carried to the operator, not a defect

A1's outcome is `Fail`, and Appendix A's consequence is therefore in force.
`/spike` step 5 says an answer that reshapes the plan stops for the operator.
The design round pre-answered the immediate question, which is that the fallback
is priced in an S04 story rather than here, so the sprint continues and the
decision is surfaced rather than acted on.

The measurement also falsifies a sentence in HLD section 15.2, which says
openjp2 is what you select on wasm. A deviation is owed when a codec story
activates the feature, and E2.6 raises it.
