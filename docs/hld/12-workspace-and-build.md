<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Workspace and build

**Source**: bootstrap import from `Ocelli-HLD.docx`, Part II, section 15. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## Part II, Low-level implementation guidance

This part is prescriptive. Where it gives a formula, a layout or a signature, that is the intended implementation and a deviation should be raised rather than improvised. It exists because the dangerous defect in medical imaging is not the crash — it is the pixel that is quietly wrong, and quietly wrong code is produced by reasonable people making locally reasonable choices.

## 15. Workspace and build

### 15.1 Layout

> ocelli/
>
> Cargo.toml \# workspace root
>
> crates/
>
> ocelli-core/ \# types, spaces, errors (no I/O)
>
> ocelli-dicom/ \# parse, metadata, providers
>
> ocelli-codec/ \# decoder registry + adapters
>
> ocelli-pixel/ \# LUT chain, frame model
>
> ocelli-volume/ \# volume assembly, reslicing
>
> ocelli-cache/ \# budgeted LRU
>
> ocelli-render/ \# wgpu, render graph, WGSL
>
> ocelli-viewport/ \# viewport + scene model
>
> ocelli-geom/ \# hit-test, measurement math
>
> ocelli-seg/ \# segmentation state
>
> ocelli-wasm/ \# \*\* the only wasm-bindgen crate \*\*
>
> ocelli-native/ \# desktop + server entry (Phase 2/3)
>
> packages/
>
> core/ \# @ocelli/core (TypeScript shell)
>
> react/ \# @ocelli/react
>
> tools/oracle/ \# differential harness vs cornerstone3D
>
> corpus/ \# golden fixtures (external store + manifest)

### 15.2 Workspace manifest

```toml
[workspace]
resolver = "2"
members = ["crates/*"]
[workspace.package]
edition = "2024"
rust-version = "1.85" # dicom-rs MSRV floor
[workspace.dependencies]
wgpu = "=30.0.1" # pin EXACTLY - breaking changes ~quarterly
dicom = { version = "0.10", default-features = false }
glam = "0.30"
bytemuck = { version = "1", features = ["derive"] }
thiserror = "2"
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

*The exact wgpu pin is not fussiness. Agents reliably emit wgpu 0.19-era pipeline code; a caret range lets that compile against something subtly different from what the shader expects.*

|  |
|----|
| **DICOM-RS FEATURES** Disable default features. The defaults are rayon and simd, and the gdcm feature is native-only. On wasm you want: default-features = false, then jpeg, rle, deflate and openjp2 selected explicitly. Note also that the inventory-based transfer-syntax plugin registry does not work on wasm at all — which is why §21 specifies an explicit runtime registry instead. |

### 15.3 The CI invariant

Decision D2 is worthless unless it is enforced. This runs on every pull request.

```bash
#!/usr/bin/env bash
# ci/check-bindgen-isolation.sh
set -euo pipefail
fail=0
for c in crates/*/; do
name=\$(basename "\$c")
[ "\$name" = "ocelli-wasm" ] && continue
if cargo tree -p "\$name" -e normal 2>/dev/null | grep -q 'wasm-bindgen'; then
echo "FAIL: \$name reaches wasm-bindgen"; fail=1
fi
done
exit \$fail
```
