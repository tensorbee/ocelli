# A1, does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH?

**Gate**: `docs/hld/A-spike-gates.md`, Appendix A, A1.
**Answered by**: F-X006, from the approved plan `.claude/plans/F-X006-design.md`.
**Status**: **RESOLVED. Outcome `Fail`.**

`docs/hld/A-spike-gates.md`, Appendix A, the row, transcribed character for
character. It sits in a table because `scripts/prose_check.py` relaxes the
voice rules inside a table row, which is what lets an author's text be quoted
exactly.

| **Gate** | **Question** | **Consequence if it fails** |
|----|----|----|
| A1 | Does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH? | You maintain C codec builds regardless, and one of the four arguments for the rewrite weakens |

**`openjp2` 0.6.1 does not decode anything at all on
`wasm32-unknown-unknown`.** It does not link as published, and once linked with
a supplied C allocator it traps on the first decode, on HTJ2K and on JPEG 2000
Part 1 alike. Natively it is exact. The Appendix A consequence column applies
verbatim and `/spike` step 5 stops here for the operator.

---

## What a pass and a fail look like

Written before the work, per `/spike` step 1. Transcribed from
`.claude/plans/F-X006-design.md`, which was approved and committed before any
measurement in this file was taken. `docs/hld/A-spike-gates.md` is cut from the
author's document and is not hand-edited, so the criteria live in the plan and
are reproduced here rather than added there.

| Outcome | Meaning |
|---------|---------|
| **Pass** | `openjp2` builds for `wasm32-unknown-unknown` in both SIMD configurations. For `.201` and `.202`, all of `D_wasm`, `D_native`, `D_ojph` and `R` have the same sha256. For `.203`, `D_wasm`, `D_native` and `D_ojph` have the same sha256. Then `openjp2` is the HTJ2K decoder, no C codec build is needed for HTJ2K, and the rewrite argument the gate names holds |
| **Constrained** | `.201` and `.202` pass in full, and `.203` differs from `D_ojph` while matching `D_native` exactly. The irreversible 9/7 path is where two conforming implementations may legitimately round differently, so this is an explained difference rather than a defect. `openjp2` is still the decoder, the record publishes the max absolute difference and its distribution as a D14 measured divergence, and the E2.6 codec story carries a standing test pinning that bound |
| **Fail** | Any one of: the crate does not build for wasm32, the module refuses an HTJ2K codestream, `D_wasm` differs from `D_native` for the same library, or a lossless row does not reproduce `R` exactly. Then Appendix A's consequence applies verbatim, C codec builds are maintained regardless and one of the four arguments for the rewrite weakens. The record prices the fallback rather than leaving it as a gap, and the two candidates to price are an OpenJPH build, which is what the reference itself uses at `@cornerstonejs/codec-openjph` 2.4.9, and the pure-Rust `openjph-core` |

Two notes from the plan, carried here because they decide how the result below
is read. **`D_wasm` differing from `D_native` is a `Fail` and not a
`Constrained`, whatever the size of the difference**, because a library
disagreeing with itself across targets is a wasm defect and the whole gate is
about wasm. And **the anchor `R` is what makes the lossless rows
encoder-independent**, because it comes from `scripts/corpus_synth.py`'s ramp
and not from OpenJPH.

**Two of the four `Fail` clauses fired.** The crate does not build for wasm32,
and once made to link it refuses every codestream by trapping.

---

## Method

### The four decodes and the anchor

| Symbol | What it is |
|--------|-----------|
| `D_wasm` | `openjp2` 0.6.1 compiled to `wasm32-unknown-unknown`, executed under node v24.16.0 |
| `D_simd` | the same, built with `-C target-feature=+simd128` |
| `D_native` | the same `openjp2` 0.6.1 compiled to `aarch64-apple-darwin` |
| `D_ojph` | `ojph_expand`, openjph 0.31.0, the independent decoder A1 names |
| `R` | `syntax/explicit_vr_le.dcm` PixelData, the uncompressed reference every case was encoded from |

`R` is exact for `1.2.840.10008.1.2.4.201` and `1.2.840.10008.1.2.4.202` only.
`.203` is irreversible and has no exact anchor. See **The `.203` limitation**
below.

### The canonical form

Every decode is reduced to the same thing before comparison: **12288 bytes,
little-endian `u16`, 6144 samples, row-major, 64 rows by 96 columns**. That
shape is not chosen, it is what `scripts/corpus_synth.py` produced, and all
three HTJ2K rows were encoded from `syntax/explicit_vr_le.dcm` by
`ojph_compress` with `-reversible true` for the first two.

`ojph_expand` writes a PGM, and `P5` with a maxval above 255 is big-endian by
the PGM specification, so the conversion is a header skip and a byte swap. The
swap happens in the comparator and never inside a decoder, so no decode is
touched by our arithmetic.

### The comparator, and that it was observed red

One comparator, `tools/spikes/common/compare.mjs`, exact equality with no
tolerance, because none of these comparisons is a rendering comparison and HLD
section 25.1 does not reach them. It refuses any buffer that is not exactly
12288 bytes, so a truncation cannot pass as equality over a shorter buffer.

`tools/spikes/common/tests/compare_test.mjs` was run green, then two mutations
were applied in their own commands and each was observed red before being
reverted:

| Mutation | Result |
|----------|--------|
| `if (sa[i] === sb[i]) continue;` inverted to `!==` | 2 of 5 checks red, exit 1 |
| the PGM big-endian byte swap removed | 1 of 5 checks red, exit 1 |
| both reverted | 5 of 5 green, exit 0 |

### The harness, and the two things about it that are not obvious

`tools/spikes/a1-htj2k/`, a `cdylib` with a hand-written integer ABI, no
`wasm-bindgen`, no `unsafe`, and no `as` cast. It is not a workspace member and
carries its own empty `[workspace]` table, so the root `Cargo.toml` is
untouched. It is pinned to `edition = "2021"` deliberately: Rust 2024 spells an
exported symbol `#[unsafe(no_mangle)]` and
`scripts/unsafe_allowlist_check.py` matches the bare token `unsafe`, so a
tracked spike file containing no unsafe code at all would fail `gate unsafe` on
an attribute.

**The plan says the harness depends on `openjp2` and it depends on `jpeg2k`
with `openjp2` selected. This is the one place the tree contradicted the plan
and it is reported rather than absorbed.** `openjp2` 0.6.1 exposes no safe
in-memory stream. `Stream::new_file` sits behind its `file-io` feature, which
needs `libc` and a filesystem, and the only other route is
`Stream::new_custom` with `extern "C"` callbacks over a `*mut c_void`, which
cannot be used without dereferencing a raw pointer in a tracked file. The
adaptation keeps the plan's intent exactly, because
`dicom-transfer-syntax-registry` 0.10 declares

```toml
# JPEG 2000 support via the OpenJPEG Rust port,
# works on Linux and a few other platforms
openjp2 = ["dep:jpeg2k", "jpeg2k/openjp2"]
```

so `jpeg2k` with `default-features = false, features = ["openjp2"]` is the
route the codec registry would take, and the only JPEG 2000 code in the tree is
`openjp2` 0.6.1:

```
a1-htj2k v0.0.0
├── jpeg2k v0.10.1
│   ├── anyhow v1.0.104
│   ├── log v0.4.34
│   ├── openjp2 v0.6.1
│   │   ├── bitflags v1.3.2
│   │   ├── byteorder v1.5.0
│   │   ├── log v0.4.34
│   │   ├── smallvec v1.16.0
│   │   └── sprintf v0.1.4
│   └── thiserror v1.0.69
└── openjp2 v0.6.1 (*)
```

**dicom-rs hedges in its own comment**, "works on Linux and a few other
platforms", and does not claim wasm. HLD section 15.2 does claim it: "On wasm
you want: default-features = false, then jpeg, rle, deflate and openjp2
selected explicitly." The measurement below is against that sentence.

### The control, and why there is one

Two extra rows are decoded that A1 does not ask about:
`1.2.840.10008.1.2.4.90` and `1.2.840.10008.1.2.4.91`, JPEG 2000 Part 1. They
run the same build through a different code path. Without them, "openjp2 does
not work on wasm32" and "openjp2's HTJ2K path does not work on wasm32" are
indistinguishable, and they have different consequences.

---

## Raw result

### Question 1, does `openjp2` build for `wasm32-unknown-unknown`?

**No, not as published, in any feature configuration.**

With `default-features = false, features = ["std"]`, which is what `jpeg2k`
selects, it compiles and then fails to link. `openjp2` 0.6.1's lib target
declares `crate-type = ["cdylib", "staticlib", "rlib"]`, so cargo links a
cdylib for it even as a dependency, and that link is where it fails:

```
rust-lld: error: openjp2-<hash>.openjp2.<hash>-cgu.0.rcgu.o: undefined symbol: malloc
rust-lld: error: openjp2-<hash>.openjp2.<hash>-cgu.0.rcgu.o: undefined symbol: calloc
rust-lld: error: openjp2-<hash>.openjp2.<hash>-cgu.0.rcgu.o: undefined symbol: realloc
rust-lld: error: openjp2-<hash>.openjp2.<hash>-cgu.0.rcgu.o: undefined symbol: free
error: could not compile `openjp2` (lib) due to 1 previous error
```

Counted over one complete link, with `-C link-arg=--error-limit=0` so nothing
is truncated, **under the `[profile.release]` this harness pins**, which is HLD
15.2's: 197 undefined-symbol errors, 175 `free`, 17 `calloc`, 3 `malloc`, 1
`realloc` and 1 `strcpy`. `memcpy` does not appear at all, because
`compiler_builtins` supplies it on this target.

**The count is a property of the profile and is meaningless without it.** The
same link under the default release profile gives 432, 279 `free`, 60 `malloc`,
51 `calloc`, 41 `realloc` and 1 `strcpy`, measured the same way. Five distinct
symbols either way, and `memcpy` absent either way, which is the part of this
that is an answer. The totals differ because `lto = "fat"` and
`codegen-units = 1` change how many call sites survive into the object file.

The cause is named and is not a mystery. **The upstream README calls it a
"C2Rust port" under that heading and describes it as "An experimental Rust
port is being developed"**, and `src/malloc.rs` declares its allocator as C
with no `cfg` at all:

```rust
extern "C" {
  fn malloc(_: usize) -> *mut core::ffi::c_void;
  fn calloc(_: usize, _: usize) -> *mut core::ffi::c_void;
  fn realloc(_: *mut core::ffi::c_void, _: usize) -> *mut core::ffi::c_void;
  fn free(_: *mut core::ffi::c_void);
  fn memcpy(...) -> *mut core::ffi::c_void;
}
```

A native target gets those from the system libc. `wasm32-unknown-unknown` has
no libc.

With `default-features` left on, so that `file-io` and the `libc` crate are
active, it does not even compile:

```
error[E0425]: cannot find function `fwrite` in crate `libc`   (58 occurrences)
error[E0425]: cannot find type `FILE` in crate `::libc`
error[E0432]: unresolved import `libc::FILE`                  (3 occurrences)
error: could not compile `openjp2` (lib) due to 62 previous errors
```

**That alone fires the first `Fail` clause.** The measurement was carried
further anyway, because "does not build" and "builds and is wrong" price the
fallback differently.

### Question 1b, what does it take to make it link, and does that help?

Two things, and they were supplied to find out:

1. `-C link-arg=--allow-undefined`, so `openjp2`'s own intermediate cdylib may
   leave the four symbols as wasm imports.
2. A C allocator in the downstream crate. `tools/spikes/a1-htj2k/src/wasm_libc.rs`
   supplies `malloc`, `calloc`, `realloc` and `free` **with no `unsafe`**, by
   keeping every allocation as an owned `Box<[u64]>` in a side table keyed by
   address, so `realloc` copies between two owned slices and never reads
   through a raw pointer. `u64` and not `u8` because the C code stores
   `int32_t` and pointers in these blocks.
3. One JavaScript import, `env.strcpy`, which is the fifth undefined symbol in
   the list above and the only one left after the allocator. It comes from
   `openjp2`'s event and error reporting and is implemented in the driver over
   the module's own linear memory. The final module has exactly one import,
   `env.strcpy`, and its own `memory` plus the four allocator functions among
   its exports.

The allocator was exercised directly from JavaScript before anything was
concluded from it: `malloc(1024)` returns an 8-byte-aligned in-range address,
`calloc(16, 64)` likewise, `realloc` to 4096 preserves all 1024 written bytes
exactly, `free` succeeds, `malloc(0)` returns null, and the module's memory
grows as expected.

**With all three supplied, both wasm builds link, and every decode traps.**

```
[wasm-plain]
  decode(0) THREW memory access out of bounds     .201 HTJ2K reversible
  decode(1) THREW memory access out of bounds     .202 HTJ2K reversible RPCL
  decode(2) THREW memory access out of bounds     .203 HTJ2K irreversible
  decode(3) THREW memory access out of bounds     .90  Part 1, CONTROL
  decode(4) THREW memory access out of bounds     .91  Part 1, CONTROL
[wasm-simd]
  decode(0) THREW memory access out of bounds
  decode(1) THREW memory access out of bounds
  decode(2) THREW memory access out of bounds
  decode(3) THREW memory access out of bounds
  decode(4) THREW memory access out of bounds
```

**The shim is not the cause, and that was tested rather than assumed.** A
diagnostic variant that never reclaims an address, making a reused address or a
use-after-free impossible, traps identically on all five cases.

**The controls trap too**, so this is not an HTJ2K-path defect. It is
`openjp2` 0.6.1 on `wasm32-unknown-unknown`.

### Question 1c, where exactly

A debug build, which keeps the name section and Rust's debug assertions, names
the site:

```
RuntimeError: unreachable
  at core::ptr::NonNull::new_unchecked::precondition_check ... 7openjp2
  at alloc::alloc::dealloc ... 7openjp2
  at openjp2::tcd::opj_tcd_code_block_dec_deallocate
  at openjp2::tcd::opj_tcd_free_tile
  at <openjp2::types::opj_tcd as core::ops::drop::Drop>::drop
```

`openjp2` 0.6.1 `src/tcd.rs` calls Rust's own `std::alloc::dealloc` on
`decoded_data` with **no null check**, at two sites, lines 1432 and 2510:

```rust
dealloc(
  (*l_code_block).decoded_data as _,
  (*l_code_block).decoded_data_layout,
);
(*l_code_block).decoded_data = core::ptr::null_mut::<OPJ_INT32>();
```

`decoded_data` is null for a code block that never had decoded data allocated.
`std::alloc::dealloc` requires a non-null pointer. The system allocator behind
`aarch64-apple-darwin` tolerates the call. The `dlmalloc` behind
`wasm32-unknown-unknown` does not, and in a release build, where the
`NonNull::new_unchecked` precondition check is compiled out, it presents as
`memory access out of bounds`.

**This is a latent defect on every target, not a wasm-specific one.** It is
undefined behaviour that a host allocator happens to survive. That matters for
how the fallback is priced, and it is the sharpest single finding in this gate.

### Questions 2 and 3, the digests

`D_wasm` and `D_simd` produce **no pixels on any row**, so every comparison
involving them is reported as NOT MEASURED and none is inferred.

```
R (syntax/explicit_vr_le.dcm PixelData): b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609

--- htj2k_lossless, 1.2.840.10008.1.2.4.201, reversible ---
  D_wasm    NO PIXELS. trap: memory access out of bounds
  D_simd    NO PIXELS. trap: memory access out of bounds
  D_native  b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  D_ojph    b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  EQUAL  D_native vs D_ojph
  EQUAL  D_native vs R
  EQUAL  D_ojph   vs R
  NOT MEASURED  D_wasm vs D_native, D_simd vs D_wasm, D_wasm vs D_ojph, D_wasm vs R

--- htj2k_lossless_rpcl, 1.2.840.10008.1.2.4.202, reversible ---
  D_wasm    NO PIXELS. trap: memory access out of bounds
  D_simd    NO PIXELS. trap: memory access out of bounds
  D_native  b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  D_ojph    b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  EQUAL  D_native vs D_ojph
  EQUAL  D_native vs R
  EQUAL  D_ojph   vs R
  NOT MEASURED  D_wasm vs D_native, D_simd vs D_wasm, D_wasm vs D_ojph, D_wasm vs R

--- htj2k_lossy, 1.2.840.10008.1.2.4.203, irreversible ---
  D_wasm    NO PIXELS. trap: memory access out of bounds
  D_simd    NO PIXELS. trap: memory access out of bounds
  D_native  123e04f90236421caa3d173937dd527a9487f6093230ba95e128775f5e2d6c67
  D_ojph    41a94cc4db2b871e16e50c738512413db800657b0dbeab7872415504be87c138
  DIFFER D_native vs D_ojph
    differing samples: 1250 of 6144
    first difference: index 64, D_native=701 D_ojph=700
    max absolute difference: 1
    histogram by magnitude: 1:1250
  NOT MEASURED  D_wasm vs D_native, D_simd vs D_wasm, D_wasm vs D_ojph

--- j2k_lossless, 1.2.840.10008.1.2.4.90, reversible, CONTROL ---
  D_wasm    NO PIXELS. trap: memory access out of bounds
  D_simd    NO PIXELS. trap: memory access out of bounds
  D_native  b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  EQUAL  D_native vs R

--- j2k_lossy, 1.2.840.10008.1.2.4.91, irreversible, CONTROL ---
  D_wasm    NO PIXELS. trap: memory access out of bounds
  D_simd    NO PIXELS. trap: memory access out of bounds
  D_native  e1d2914d695ea1f2efaa6b5c13b249f457bee82cf8152811b170ed1b54c27efa

1 differing comparison(s), 16 not measured, 10 decode(s) produced no pixels, 10 wasm trap(s)
```

The native decoder also reports its own sample range on every run, which is
what proves the `u16::try_from` in the harness could never have refused a
legitimate lossy reconstruction: `.201` and `.202` span 0 to 65535, `.203`
spans 29 to 65504, `.90` spans 0 to 65535, `.91` spans 0 to 65533.

---

## Interpretation

**A1 is a `Fail`, and two of the clause's four conditions fired.** The crate
does not build for wasm32, and when forced to link it refuses every codestream.

Three things are true at once and none of them should be collapsed into the
others.

**One. `openjp2` 0.6.1 is exact natively.** For `.201` and `.202` all three
independent parties produce the same digest: it agrees byte for byte with
`ojph_expand` 0.31.0 and with the uncompressed reference. **`.90` has two
parties, not three**, because `ojph_expand` is an HTJ2K decoder and `run.mjs`
does not run it on the Part 1 controls, as the method section above says. There
it is `openjp2` against the uncompressed reference, and they agree. That is a
real and useful result. It says the HTJ2K decode arithmetic in `openjp2` is
right, which is not what failed.

**Two. `.203` differs from OpenJPH by exactly one level, on 1250 of 6144
samples, maximum absolute difference 1, every difference of magnitude 1.**
That is the signature of two conforming implementations rounding the
irreversible 9/7 inverse transform differently, and it is precisely the
divergence the `Constrained` row anticipated. **It does not make the outcome
`Constrained`, because `Constrained` requires `.201` and `.202` to have passed
in full, and passing requires `D_wasm`, which does not exist.** The figure is
recorded because it is what the E2.6 codec story would have to pin if `openjp2`
were ever adopted natively, and because it is a clean D14 measured-divergence
number that cost nothing to obtain.

**Three. The wasm failure is not about HTJ2K.** The Part 1 controls trap
identically. Anyone reading this result as "HTJ2K is the problem" would go and
price an HTJ2K-specific fallback and would be solving the wrong problem. The
whole of `openjp2` is unusable on `wasm32-unknown-unknown` today.

### What HLD section 15.2 says, and what is now measured

Section 15.2 says: "On wasm you want: default-features = false, then jpeg, rle,
deflate and openjp2 selected explicitly." **The `openjp2` half of that sentence
does not hold against `openjp2` 0.6.1 and dicom-rs 0.10.** dicom-rs's own
feature comment is more careful and says "works on Linux and a few other
platforms". The other three features named in that sentence were not measured
here and nothing in this record says anything about them.

### The `.203` limitation, stated rather than implied

For `.201` and `.202` there are three independent parties. OpenJPH encoded,
`openjp2` decodes, and `R` comes from `scripts/corpus_synth.py`'s ramp with a
third decode already asserted by
`scripts/tests/test_corpus_synth.py::test_lossless_syntaxes_round_trip_exactly`.

**For `.203` there is no such anchor.** It is irreversible, so `R` is not
exact for it, and the only comparison available is `openjp2` against OpenJPH
while OpenJPH also encoded it. The comparison is still cross-implementation on
the decode side and is what A1 literally asks for, and the strength of that
one row's evidence is lower than the other two. No corpus row is added here.
Adding a `.203` case from a second encoder is a corpus story with a licence and
a manifest row.

### SIMD128, and the boundary with F-004

Two wasm builds were produced, with and without `-C target-feature=+simd128`.
Both link and both trap identically, so **the two digests A1 wanted from them
do not exist and no SIMD conclusion can be drawn from this gate.** F-004 owns
runtime SIMD128 detection and its `WebAssembly.validate` probe is the runtime
half of the same question. Neither record should restate the other's
conclusion, and this one has nothing to contribute.

### What R5's audit-surface claim does and does not cover

HLD 27.2 R5 keeps `unsafe` to two files so a reviewer auditing this project for
a device submission reads two files.
`scripts/unsafe_allowlist_check.py` reads `git ls-files`, so **it never sees a
dependency.** `openjp2` is a C2Rust port and is substantially `unsafe`
internally, and the specific defect this gate found is a null pointer reaching
`std::alloc::dealloc` inside it.

**That is not a reason to reject `openjp2`, and it is a reason the record must
not claim an audit surface the project does not have.** If a codec dependency
is ever adopted, R5 continues to hold mechanically and its stated purpose is
weakened, and that sentence belongs in the device submission rather than being
discovered by a reviewer.

### Section 21's "Must not allocate", read narrowly

Per the design round's decision 2, section 21's
`/// Decode one frame into `out`. Must not allocate.` is read as **no
allocation per decode call**, with the output buffer caller-provided and
internal state allocated at registration or first use outside the rule. Under
the broad reading no route passes, including the one the HLD names, since
`openjp2` carries its own `malloc.rs`. This reading is shared with F-006, which
transcribes the same comment.

---

## Recommendation

**Do not adopt `openjp2` for the browser target. Do not adopt it for the native
targets either without addressing the null-`dealloc` defect, which is undefined
behaviour on every target and not only on wasm.**

Three routes exist and this gate does not choose between them, because
**decision 6 of the design round says the fallback is priced in an S04 story
and not here.**

| Route | What it is | What is already known |
|-------|-----------|----------------------|
| **F1** | Upstream fix to `openjp2`, a null check at `src/tcd.rs` 1432 and 2510, plus a `cfg` for the C allocator | Both changes are small and are in a BSD-2 crate. It is a third-party release cycle, and until it lands the wasm route also needs the `--allow-undefined` link argument and an allocator shim, which is a bigger commitment than a patch |
| **F2** | `@cornerstonejs/codec-openjph` 2.4.9, which is what the reference itself uses | It is a JS-side wasm module, so it has R1's shape and R1's hole: no JavaScript on the native desktop and server targets. `docs/SOURCE-POLICY.md` already permits OpenJPH |
| **F3** | `openjph-core` 0.1.0, pure Rust, BSD-2-Clause | Declares itself "a faithful port of the OpenJPH C++ library (v0.26.3)". OpenJPH is in the policy's yes column. First and only publish 2026-03-20, 3643 downloads, and **crates.io reports no repository URL for it**, so `docs/SOURCE-POLICY.md` question 1 cannot be answered from a repository and only question 2 can. Not measured by this gate |

**F3 is the one to measure first in S04**, because it is the only route that is
one implementation on every target, which is the same property that decided A2.
It is also the route this gate can say least about, so it should be measured
rather than argued.

---

## What changes in the plan

`/spike` step 5 applies. **This stops for the operator.**

| Item | Before | After |
|------|--------|-------|
| HTJ2K decoder for the browser | `openjp2` under wasm32, per HLD 15.2 | **None. Unresolved, and an S04 story** |
| HTJ2K decoder natively | `openjp2` | `openjp2` is exact and carries a latent null-free defect. Not recommended without the fix |
| HLD 15.2's `openjp2` wasm feature | assumed to work | **Measured not to.** A deviation is owed when a codec story activates the feature, and E2.6 raises it |
| Appendix B transfer syntax count | 13, with HTJ2K an open gate | Unchanged in count. HTJ2K moves from "open gate" to "route unresolved, S04" |
| Appendix A's A1 consequence | conditional | **In force.** C codec builds are maintained regardless, and one of the four arguments for the rewrite weakens |

**Two consequences for other stories, neither actioned here.**

- **E2.6, the codec registry story**, cannot register an HTJ2K decoder until
  the S04 fallback story answers. The registry design is unaffected, because
  it keys on the transfer syntax UID and a missing decoder means the syntax
  reports unavailable, which is section 31's rule and deviation D-07's rule
  generalised.
- **`crates/ocelli-codec/src/lib.rs` keeps `#![cfg_attr(not(test), no_std)]`
  for now.** The plan flagged that activating `openjp2` would drop it, in the
  same shape as D-10. That pressure does not arrive with this answer.

---

## Reproducing this

An answer that cannot be re-run is an opinion. Everything below was recorded
from the run that produced the digests above.

| Thing | Version |
|-------|---------|
| rustc, cargo | 1.97.1 (`8bab26f4f 2026-07-14`), cargo 1.97.1 |
| target | `wasm32-unknown-unknown`, the only one `rust-toolchain.toml` pins. `wasm32-wasip1` is NOT installed and is NOT added |
| node | v24.16.0 |
| `openjp2` | 0.6.1, BSD-2-Clause, `https://github.com/Neopallium/openjp2` |
| `jpeg2k` | 0.10.1, MIT/Apache-2.0, `https://github.com/Neopallium/jpeg2k` |
| OpenJPH | `ojph_expand`, openjph 0.31.0 |
| corpus manifest | sha256 `a6151d3c289a6938dec98f6cd8f42231e1daeb7fefb72fb7455e68e77618bf49` |
| corpus rows | manifest lines 34, 35, 36 for HTJ2K, 38 and 39 for the Part 1 controls, 33 for `R` |

```bash
uv run tools/spikes/common/extract.py
node tools/spikes/common/tests/compare_test.mjs
cd tools/spikes/a1-htj2k
cargo build --release
RUSTFLAGS="-C link-arg=--allow-undefined" \
  cargo build --release --lib --target wasm32-unknown-unknown \
  --target-dir target/wasm-plain
RUSTFLAGS="-C link-arg=--allow-undefined -C target-feature=+simd128" \
  cargo build --release --lib --target wasm32-unknown-unknown \
  --target-dir target/wasm-simd
cd ../../.. && node tools/spikes/a1-htj2k/run.mjs
```

To see the failure as published, drop the `--allow-undefined` link argument and
the `mod wasm_libc;` line with its `cfg`. The undefined-symbol counts above
come from dropping the link argument only, run from `tools/spikes/a1-htj2k`:

```bash
RUSTFLAGS="-C link-arg=--error-limit=0" \
  cargo build --release --lib --target wasm32-unknown-unknown
```

`--error-limit=0` is not optional, since `rust-lld` stops at 20 errors by
default and a count taken from a truncated log is wrong. Dropping
`mod wasm_libc;` is not needed for the count, because the link that fails is
`openjp2`'s own cdylib, built as a dependency before this crate's code is
reached.

**Record the profile with the number.** The command above picks up the
`[profile.release]` in this crate's `Cargo.toml`, which is HLD 15.2's, and that
is where 197 comes from. Delete that table and cargo's default release profile
gives 432 from the same command. A count quoted without its profile cannot be
checked, and this figure has been restated twice for that reason.

**The harness is throwaway and is not held to the gate set**, per `/spike`
step 2, and it is deleted when this gate and A2 are closed. The gates that do
reach it anyway are `unsafe`, `provenance` and `content`, which read
`git ls-files`, and `lint`, which reaches `run.mjs` through the
`tools/spikes/**/*.mjs` block in `eslint.config.js`. It passes all four.
`clippy`, `test`, `bindgen` and `nostd` are scoped to the workspace or to
`crates/` and never see it.

**`prose` does not reach this directory**, which earlier revisions of this file
claimed it did. `scripts/prose_check.py` includes a path only if it is in
`INCLUDE_EXACT` or it ends in `.md` and starts with one of `INCLUDE_PREFIXES`.
`tools/spikes/` is on neither list and holds no `.md`, so an em-dash in
`src/lib.rs` is unchecked. **This file is a different matter**: `docs/spikes/`
is a prefix, so the answer files are covered and the harness sources are not.

`ci/check-bindgen-isolation.sh` scopes all three of its passes to `crates/*/`,
and `grep -c wasm-bindgen tools/spikes/a1-htj2k/Cargo.lock` returns 0, so there
is nothing here to find either. D2 and D-12 are untouched. **The A2 harness's
lockfile is not empty of it**, and `docs/spikes/A2-jpeg-ls.md` records why and
which candidate it belongs to. Nothing under `tools/spikes/out/` is committed: `.gitignore` covers
it and `scripts/staged_content_check.py` refuses it by path, because
`git add -f` exists and a codestream extracted from a corpus row is derived
from that row.
