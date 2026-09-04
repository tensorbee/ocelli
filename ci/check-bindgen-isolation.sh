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

# The same loop again, for wasm32. Added by F-002 (E1.2), and it is not
# redundant.
#
# `cargo tree` filters to the HOST platform when no --target is given, so the
# loop above cannot see a dependency declared under
# `[target.'cfg(target_arch = "wasm32")'.dependencies]`. That is exactly the
# form ocelli-wasm itself uses, so it is exactly the form a second crate would
# most plausibly copy. Before F-002 the gap was theoretical, because nothing
# was ever built for wasm32. F-002 makes wasm32 a real build target, so F-002
# is the story that closes it.
#
# The target does NOT need to be installed. `cargo tree` resolves cfg and does
# not compile, so it filters the graph for any target rustc knows, verified
# here against an uninstalled triple. That matters because the CI `guards` job
# installs no extra target, and requiring one would fail it for no reason.
#
# The triple IS checked against rustc's own list, and that check is not
# decoration. `cargo tree --target <typo>` errors, the error goes to
# /dev/null, and `grep -q` then finds nothing in an empty stream, so a
# misspelled triple would report a clean pass over zero crates. "The check
# could not run" and "the check ran and was happy" must never look the same
# here.
WASM_TARGET=wasm32-unknown-unknown
if ! rustc --print target-list 2>/dev/null | grep -qx "$WASM_TARGET"; then
  echo "FAIL: rustc does not know the target $WASM_TARGET, so the wasm32 half"
  echo "      of this check did not run. That is a typo in this script or a"
  echo "      toolchain that cannot build this project at all."
  fail=1
else
  for c in crates/*/; do
    name=$(basename "$c")
    [ "$name" = "ocelli-wasm" ] && continue
    if cargo tree -p "$name" -e normal --target "$WASM_TARGET" 2>/dev/null |
         grep -q 'wasm-bindgen'; then
      echo "FAIL: $name reaches wasm-bindgen under $WASM_TARGET"
      fail=1
    fi
  done
fi

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
