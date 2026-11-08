# CPU final byte store, independent source review pass 1

Reviewer: Codex `/root/f225_review`. Author: root Codex. The reviewer authored
neither changed production code nor its tests or lead amendment. The earlier
GLM proposal and its two erroneous illustrative IEEE examples remain preserved
separately. Those examples are absent from this implementation.

Result: 0 defects, 0 smells, 0 notes in this bounded source and arithmetic
review. This does not supply runtime, codegen, benchmark or feature acceptance.

## Exact scope

Reviewed immutable tree `784546c0a918404859ae73f13dc61de1ed2ab741` against
`9b586d477dc5172969253f6a9d8bc44437214a63`. Only `cpu.rs` and the F-239
lead amendment differ. Exact source and unchanged core-helper hashes are in
`root-byte-store-review-identity.json`. The working source matched the target
when reviewed. No reviewer source or index edits occurred.

## Whole-integer conversion proof

The original leading guard still sends every non-finite and non-positive
input to zero. This includes positive infinity, negative infinity, all NaN
payloads and signs, both zeros and negative subnormals. The next guard still
sends every finite value at or above 255 to 255. Their ordering is unchanged.

The remaining input is finite and strictly between 0 and 255. The pinned
Rust 1.97.1 standard API guarantees the precise nearest integer with exact
halves to even. Its result is therefore an exactly representable integer in
0 through 255, with no negative nonzero result. This is the same interval and
exact-half partition implemented by the old binary search.

The existing shared `exact_f32_integer` is unchanged. Its zero branch accepts
zero with magnitude zero. For a positive result from 1 through 255, the
binary exponent is from 0 through 7, the significand fits its u64 storage,
the right shift is from 16 through 23, and discarded fraction bits are zero
because the input is integral. It recovers the exact non-negative magnitude.
`u8::try_from` therefore succeeds with the same integer. Ignoring the separate
negative flag is sound in this guarded domain. Neither the helper's overflow
branch nor its fraction refusal can be reached here. Fallback zero does not
replace a reachable valid byte under this proof.

This reuses the single existing bit decomposition and checked integer
conversion. No float-to-integer cast, allocation, dependency, unsafe code,
public API change or additional numerical formula was introduced.

## Consumer and test scope

Only the shared final channel store changes. RGB, palette, ICC output and
linear render consumers retain their existing order through windowing and
then this store. The separate exact-byte membership predicate, monochrome
table-address ties-up rule and table construction are unchanged. The core
integer-reader implementation is unchanged.

The new literal bit-class test asserts both infinity signs, both quiet-NaN
signs, nonzero signalling-NaN payloads, both zeros, both minimum subnormal
signs and finite extremes. Positive infinity to zero is the decisive control
for dropping or moving the non-finite guard behind the upper clamp. The old
negative-one and finite endpoint cases remain in the existing tie fixture.
The established public-entry fixture declares all 255 halves with both IEEE
neighbours. No erroneous GLM predecessor decimal was copied.

The new guard test precedes the existing tie-test documentation. That comment
still attaches to `a_tie_rounds_to_the_even_channel_byte`, rather than to the
new non-finite test. Existing coverage was retained.

The lead amendment describes the supported domain and conversion accurately,
keeps R2 and budgets unchanged, and does not claim measured benefit or an
exhaustive all-bit-pattern observation. Root's pending target and runtime
checks and unchanged registered instrument remain separate requirements.

## Commands and limits

Read-only source and exact-tree consistency checks were performed. An initial
file-discovery filter returned exit 1 because its filename expression did not
match `exact_f32.rs`. The actual shared helper was then read directly. No
success was claimed from that failed discovery. No native build, product test,
wasm build, profiling, benchmark, browser, GPU or mass enumeration was run by
the reviewer while root owns the CPU and wasm pair.
