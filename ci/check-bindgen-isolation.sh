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
for c in crates/*/; do
  name=$(basename "$c")
  [ "$name" = "ocelli-wasm" ] && continue
  if cargo tree -p "$name" -e normal 2>/dev/null | grep -q 'wasm-bindgen'; then
    echo "FAIL: $name reaches wasm-bindgen"
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
