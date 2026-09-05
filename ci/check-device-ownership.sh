#!/usr/bin/env bash
# HLD section 31, the device-sharing contract. Story E1.8, the section 38 hook.
#
# > **Shares the renderer's device.** ocelli-compute never creates a
# > wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot
# > share textures, which would defeat the entire point.
#
# Modelled on ci/check-bindgen-isolation.sh, and for the same reason section
# 15.3 gives for that one: a decision is worthless unless it is enforced.
#
# TWO MECHANISMS, AND THIS IS THE WEAKER ONE. The strong one is the type
# system: `GpuContext` hands out shared borrows and nothing else, and
# `crates/ocelli-compute/tests/ui/` asserts as compile errors that no owned
# device escapes it and that a `ComputeCtx` cannot outlive its borrow.
#
# What this script catches is the case the type system cannot: a crate that
# creates a device it never puts in a `GpuContext` at all. No type is involved
# in that, so no type can refuse it.
#
# Needs no GPU and no network. Runs in the CI floor.
set -euo pipefail

cd "$(dirname "$0")/.."

# The calls that BRING A DEVICE INTO EXISTENCE. Not every wgpu call, because
# every crate under crates/ will legitimately name wgpu types eventually.
CREATORS='wgpu::Instance::new|Instance::new|request_adapter|request_device|create_surface'

fail=0

while IFS= read -r hit; do
  echo "FAIL: $hit creates a GPU device or surface"
  fail=1
done < <(
  grep -rlE "$CREATORS" crates --include='*.rs' 2>/dev/null \
    | grep -v '^crates/ocelli-render/' || true
)

# The mirror of the rule, and it is not the same check. Above asserts that
# nobody ELSE creates a device. This asserts that ocelli-render still OWNS the
# type that holds one, so the rule cannot be satisfied by deleting the contract
# and letting every crate be equally device-free.
if ! grep -qE '^\s*pub struct GpuContext' crates/ocelli-render/src/gpu.rs 2>/dev/null; then
  echo "FAIL: ocelli-render no longer defines GpuContext, so the contract of"
  echo "      HLD section 31 has nowhere to live. It is not satisfied by"
  echo "      nobody holding a device, it is satisfied by ONE crate holding it."
  fail=1
fi

# An accessor that yields an owned device defeats the contract without any
# crate calling a creator. The compile-fail case in
# crates/ocelli-compute/tests/ui/ is the real assertion, and this is the cheap
# one that names the shape in the place someone would add it.
if grep -qE 'pub fn (into_device|into_queue|take_device|clone_device)' \
     crates/ocelli-render/src/gpu.rs 2>/dev/null; then
  echo "FAIL: GpuContext has an accessor that hands out an owned device or"
  echo "      queue. Section 31's rule then holds only by convention."
  fail=1
fi

# A `Clone` derive is a second way to the same place, and `wgpu::Device` being
# `Clone` (measured, see the test in gpu.rs) means `#[derive(Clone)]` on
# GpuContext would compile. This is a grep rather than a test because a
# compile-time assertion of the ABSENCE of a trait impl needs specialisation.
#
# To be clear about what it is and is not defending: a clone of a refcounted
# handle is the SAME device, so this is not the two-devices problem. It is the
# one-owner rule for the device, queue and resolved Caps triple.
if awk '/^#\[derive\(/{d=$0} /^pub struct GpuContext/{print d}' \
     crates/ocelli-render/src/gpu.rs 2>/dev/null | grep -q 'Clone'; then
  echo "FAIL: GpuContext derives Clone. The device, the queue and the caps"
  echo "      they resolved to should have one owner. A second owner is not a"
  echo "      second device, it is a second place to look."
  fail=1
fi

if [ "$fail" -eq 0 ]; then
  echo "OK: only ocelli-render may create a device, and it still owns GpuContext"
fi
exit "$fail"
