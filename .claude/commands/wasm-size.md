---
description: Build the wasm module, measure it against the recorded budget, and attribute growth.
---

# /wasm-size [--accept] [--attribute]

Story E1.2 and Appendix A gate A4.

```bash
bin/ocelli.sh wasm
python3 scripts/pin_and_size_check.py --with-size
```

## The budget is a measurement, not a guess

The HLD estimates 3 to 8 MB uncompressed before tuning, with Naga dominating,
and says plainly that it is **unmeasured today**. So the first run records the
observed size and passes. After that, growth beyond 5% of the baseline fails.

A budget invented before the first measurement is either meaningless or
immediately wrong, and both teach people to ignore it.

## Reading a failure

Growth is normal as the port proceeds. The gate exists to make it **visible and
attributed**, not to prevent it.

1. **What landed since the baseline?** A story adding a codec or a shader
   family is expected to cost.
2. **Is it Naga?** The shader compiler dominates, and it is pulled in by wgpu.
   A jump with no new shader work points at a dependency change, not at the
   story.
3. **Is it a dependency that should not be there?** `cargo tree` for the wasm
   target. Anything native-only reaching the wasm build is a defect, not a size
   problem, and `bin/ocelli.sh gate bindgen` may already say so.
4. **Was it a release build?** HLD section 15.2's profile, `opt-level = "z"`,
   fat LTO, `codegen-units = 1`, `panic = "abort"`, `strip = true`, applies to
   release only. A dev-profile measurement is meaningless here.

## `--accept`

Re-baselines. **Say why in the design plan of the story that caused it**, and
name the cause rather than the number. "Grew to 4.2 MB" records nothing.
"openjp2 adds 1.1 MB, which is the cost of gate A1's answer" records a
decision someone can revisit.
