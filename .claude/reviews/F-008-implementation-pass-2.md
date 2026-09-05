# F-008 implementation review, pass 2

**Scope**: pass 1's three remediations, which were unreviewed work until this
pass, and a second read of the whole F-008 diff.
**Result**: 2 defects, 0 smells, 0 nitpicks. Both remediated.

## Defects

### D4. A claim in the LLD that nothing enforced, and that was overstated anyway

`docs/lld/gpu-ownership.md` said `GpuContext` "is not `Clone`" as part of the
defence against a second device. Two things wrong with that sentence.

**Nothing enforced it.** The compile-fail case covers an owned accessor and the
guard script covered four accessor names. A `#[derive(Clone)]` would have
passed all of them, and it compiles, because `wgpu::Device` is `Clone`.

**And the reasoning was too strong.** `wgpu::Device` is a refcounted handle, so
a clone is the **same** device, not a second one. Measured rather than
reasoned about, with a test that also covers `Queue`. Section 31's "two devices
cannot share textures" is about a second `request_device`, which the guard
script already refuses and which is the load-bearing check.

**Remediation.** The LLD and the type's documentation now say precisely what
each mechanism defends. `GpuContext` is still not `Clone`, for the smaller and
separate reason that the triple should have one owner, and that is now a grep
in `ci/check-device-ownership.sh`, proved red by adding the derive. The
`wgpu::Device: Clone` fact is a test, so if a future wgpu changes it the
reasoning gets revisited rather than silently becoming wrong. That is the
pattern F-001 used for `Transform::inverse` returning non-finite values.

### D5. A test that could not fail, written while fixing D4

While closing D4 the first attempt added `gpu_context_is_not_clone`, which
asserted `!fallback::<GpuContext>()` where `fallback` returns `false`
unconditionally. It was a tautology dressed as a test and it would have been
counted as coverage forever.

**Remediation.** Deleted. The check moved to the guard script, where it can
actually fail, and it was proved failing.

Recording it as a defect rather than quietly deleting it, because it is the
third instance of this shape in this story alone, after the compile-fail case
that compiled. **The pattern is that it appears while fixing something else**,
when attention is on the fix rather than on whether the new test discriminates.

## What was re-checked and found clean

- `GpuContext` has no `Clone` derive and no owned-value accessor. Both verified
  by grep against the source rather than from memory of having written it.
- `ComputeCtx`'s two fields are private.
- `Kernel`'s signature still matches section 31 character for character.
- The reshaped `ci/check-bindgen-isolation.sh` still refuses a direct
  wasm-bindgen declaration in both the plain and the wasm32-gated form,
  re-proved after the reshape.
- `gate --floor` reports 19 gates green with none skipped.
