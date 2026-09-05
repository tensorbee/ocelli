#!/usr/bin/env bash
# HLD section 15.3, the CI invariant. Given verbatim in the specification.
#
# Decision D2 says wasm-bindgen appears in exactly one crate. That decision is
# what makes Phases 2 and 3 new ENTRY POINTS rather than rewrites. It is nearly
# free to hold and expensive to restore, which is the entire reason it is a
# mechanical check from day one rather than a convention.
#
# This runs on every push and in `/verify`. It needs no GPU and no network
# beyond what `cargo tree` already has cached, so it runs in the CI floor.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0

# HLD section 15.3's loop, unaltered, over the HOST target.
for c in crates/*/; do
  name=$(basename "$c")
  [ "$name" = "ocelli-wasm" ] && continue
  if cargo tree -p "$name" -e normal 2>/dev/null | grep -q 'wasm-bindgen'; then
    echo "FAIL: $name reaches wasm-bindgen"
    fail=1
  fi
done

# The same crates again for wasm32, and NOT as a `cargo tree` reachability
# loop. Deviation D-12, and the reason is a contradiction inside the HLD
# itself rather than a convenience.
#
# Section 15.2 specifies `wgpu`. Section 4 says `ocelli-render` builds for
# wasm. Section 15.3, transcribed above, forbids any crate but ocelli-wasm
# from REACHING wasm-bindgen. On wasm32 those three cannot all hold, because
# wgpu talks to the browser's WebGPU through js-sys and web-sys, which are
# built on wasm-bindgen. Measured:
#
#   wasm-bindgen v0.2.127
#   |-- js-sys -> wasm-bindgen-futures -> wgpu -> ocelli-render
#   `-- web-sys -> wgpu -> ocelli-render
#
# and on the host that route does not exist at all, so pass 1 above still
# means exactly what section 15.3 wrote.
#
# Decision D2's PURPOSE survives intact, and that is what decides the shape of
# this check. CLAUDE.md states the purpose: wasm-bindgen in one crate "is what
# makes the desktop and server targets new entry points rather than rewrites".
# wgpu abstracts the target itself, so `ocelli-render` carries no browser
# binding in its source and compiles for native unchanged. What D2 forbids is
# OUR code binding to the browser outside one crate.
#
# So the wasm32 rule is DIRECT DECLARATION, which is our code's own choice,
# rather than transitive reachability, which a specified dependency decides
# for us. A crate that adds wasm-bindgen to any of its own dependency tables,
# under any target gate, still fails. That is the exact case F-002 added this
# pass for, and it is still caught.
for c in crates/*/; do
  name=$(basename "$c")
  [ "$name" = "ocelli-wasm" ] && continue
  # Any `wasm-bindgen = ...` line in the manifest, whatever section or target
  # gate it sits under. Comments are excluded so the explanatory prose in
  # ocelli-wasm's own manifest cannot trip a sibling.
  if grep -vE '^\s*#' "$c/Cargo.toml" 2>/dev/null |
       grep -qE '^\s*wasm-bindgen\s*(=|\.)'; then
    echo "FAIL: $name declares wasm-bindgen as a direct dependency"
    fail=1
  fi
done

# The rule is about the DEPENDENCY GRAPH, and a crate can also reach
# wasm-bindgen by writing `use wasm_bindgen` against a dev-dependency or a
# re-export. cargo tree -e normal does not see either, so check the source too.
# ocelli-wasm is exempt from both halves and nothing else is.
while IFS= read -r hit; do
  echo "FAIL: $hit names wasm_bindgen in source"
  fail=1
done < <(
  grep -rlE '\bwasm_bindgen\b' crates --include='*.rs' 2>/dev/null \
    | grep -v '^crates/ocelli-wasm/' || true
)

if [ "$fail" -eq 0 ]; then
  echo "OK: wasm-bindgen is confined to ocelli-wasm"
fi
exit "$fail"
