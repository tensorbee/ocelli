#!/usr/bin/env bash
# The inner loop and the gate runner.
#
# Everything runs natively on the host. Ocelli needs wasm32 builds, a real GPU
# for WebGPU and a browser for the oracle, and a container can give it none of
# those, so there is no container path here. See docs/DEVELOPER_SETUP.md.
#
# `/verify` is the completion gate and is defined in .claude/commands/verify.md.
# This script is the inner loop and the thing that gate calls, not a second
# source of truth for what must pass.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)

RED=$'\033[31m'; GREEN=$'\033[32m'; DIM=$'\033[2m'; OFF=$'\033[0m'

usage() {
  cat <<'USAGE'
bin/ocelli.sh <command> [args]

Inner loop
  check <crate>          cargo check -p <crate> --all-targets
  build <crate>          cargo build -p <crate>
  test  <crate> [args]   cargo test -p <crate>
  fmt                    cargo fmt --all --check
  clippy [crate]         workspace form, or -p <crate>
  cargo <anything>       raw passthrough

Targets
  wasm [--release]       wasm-pack build crates/ocelli-wasm, then the size gate
  native                 cargo build -p ocelli-native, the cross-target proof

Validation
  oracle [args]          the differential harness against cornerstone3D (GPU)
  corpus                 verify $OCELLI_CORPUS_DIR against corpus/manifest.tsv
  corpus-tests           the corpus tooling suites (see OCELLI_PYTHON below)

Gates
  gate --list            what each gate covers
  gate <name>...         run named gates
  gate --floor           the gates CI runs: no GPU, no corpus, no browser
  gate --sprint          sprint gate, with the S01 pre-oracle exception
  gate --all             every gate, including the GPU and corpus tiers

Environment
  OCELLI_CORPUS_DIR      corpus location, default corpus/data
  OCELLI_PYTHON          interpreter with pydicom for `gate corpus-tests`,
                         default resolved by scripts/corpus_tests.py
  OCELLI_AGENT           recorded in the provenance trailer
USAGE
}

# name|needs_gpu|description
GATES=(
  "fmt|no|cargo fmt --all --check"
  "clippy|no|cargo clippy --workspace --all-targets -- -D warnings"
  "test|no|cargo test --workspace"
  "bindgen|no|wasm-bindgen confined to ocelli-wasm (HLD 15.3, decision D2)"
  "unsafe|no|no unsafe outside the two permitted files (HLD 27.2 R5)"
  "pins|no|wgpu pinned exactly (HLD 15.2, 27.2 R4)"
  "provenance|no|source-provenance policy, read-blocked projects (HLD C.2.1)"
  "prose|no|voice rules over operator-facing prose"
  "content|no|no DICOM and no build artefacts tracked"
  "backlog|no|BACKLOG, SPRINT_PLAN, tracker and as-built agree"
  "deviations|no|every HLD deviation declared and still true"
  "docs|no|docs/hld matches the .docx, no section lost"
  "skills|no|Codex adapters match their canonical command and skill files"
  "lint|no|eslint, including the cached-wasm-view ban (HLD 17.2)"
  "types|no|tsc --build across the TypeScript workspaces"
  "wasm|no|wasm-pack build and the size budget (E1.2, gate A4)"
  "corpus-tests|no|the corpus generator and coverage suites, a skip fails it"
  "corpus|no|corpus coverage over the codec registry, then presence and digests"
  "oracle|YES|the differential corpus against cornerstone3D (HLD 11, D7)"
)

gate_needs_gpu() {
  local entry
  for entry in "${GATES[@]}"; do
    [ "${entry%%|*}" = "$1" ] && { [ "$(echo "$entry" | cut -d'|' -f2)" = "YES" ]; return; }
  done
  return 1
}

run_gate() {
  local name=$1
  case "$name" in
    fmt)         cargo fmt --all --check ;;
    clippy)      cargo clippy --workspace --all-targets -- -D warnings ;;
    test)        cargo test --workspace ;;
    bindgen)     ci/check-bindgen-isolation.sh ;;
    unsafe)      python3 scripts/unsafe_allowlist_check.py ;;
    pins)        python3 scripts/pin_and_size_check.py ;;
    provenance)  python3 scripts/source_provenance_check.py ;;
    prose)       python3 scripts/prose_check.py ;;
    content)     python3 scripts/staged_content_check.py --tracked ;;
    # `&&` and not two statements on two lines. A case arm returns the status
    # of its LAST command, so an unchained first command can fail, print a
    # traceback, and be reported green. That happened here once already.
    backlog)     python3 scripts/backlog_check.py &&
                 python3 scripts/gen_sprint_plan.py --check ;;
    deviations)  python3 scripts/deviation_check.py ;;
    # Both sources live outside the repository, so both may legitimately
    # SKIP with exit 3. Chain on && so a real failure still fails, and pass a
    # skip through rather than converting it to a pass.
    docs)        python3 scripts/split_hld.py --check || return $?
                 python3 scripts/import_backlog_xlsx.py --check || return $? ;;
    skills)      python3 scripts/sync_agent_skills.py --check ;;
    lint)        [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run lint ;;
    types)       [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run typecheck ;;
    wasm)        grep -qE '^\s*wasm-bindgen\s*=' crates/ocelli-wasm/Cargo.toml || {
                   skip "ocelli-wasm declares no wasm-bindgen yet, so wasm-pack cannot build it. It lands with F-096 (E16.2, the boundary)."
                   return 3
                 }
                 "$0" wasm && python3 scripts/pin_and_size_check.py --with-size ;;
    # Needs no corpus, so it is IN the floor. The runner fails on a skipped
    # test rather than on the exit status, because the suites exit 0 under an
    # interpreter with no pydicom while reporting a skip, and this project's
    # rule is that a skip is not a pass. It exits 3, a named skip, only when a
    # prerequisite is genuinely absent.
    corpus-tests) python3 scripts/corpus_tests.py ;;
    # Coverage FIRST, then the digests. Coverage reads the manifest and nothing
    # else, so it answers "does this corpus still cover every transfer syntax
    # the codec registry claims, and both tolerance classes of HLD 25.1" even
    # where the data is absent. Chained on `&&` for the reason the backlog arm
    # gives: a case arm returns the status of its LAST command, so an unchained
    # first command can fail and be reported green.
    corpus)      python3 scripts/corpus_check.py --coverage &&
                 python3 scripts/corpus_check.py ;;
    oracle)      "$0" oracle ;;
    *)           echo "unknown gate: $name" >&2; return 2 ;;
  esac
}

# Exit 3 from a gate means SKIPPED WITH A REASON, and it is counted and named
# separately from a pass. A gate that could not run must never read as one that
# ran and was happy, which is the whole reason this project distrusts a green
# summary it did not watch produce.
skip() { echo "SKIPPED: $*"; return 3; }

s01_pre_oracle() {
  grep -qx '# Current sprint, S01' docs/sprints/CURRENT_SPRINT.md &&
    grep -Eq '^\| F-010 \| E2\.2 \| S02 \| .* \| Test \| 4w \| F-009 \| pending \|$' \
      docs/sprints/BACKLOG.md
}

gates_cmd() {
  local selected=() entry name gpu desc failed=() skipped=() passed=0 status
  local profile=named

  case "${1:-}" in
    --list)
      printf '%-12s %-5s %s\n' GATE GPU COVERS
      for entry in "${GATES[@]}"; do
        IFS='|' read -r name gpu desc <<<"$entry"
        printf '%-12s %-5s %s\n' "$name" "$gpu" "$desc"
      done
      return 0 ;;
    --floor)
      profile=floor
      for entry in "${GATES[@]}"; do
        IFS='|' read -r name gpu desc <<<"$entry"
        # The CI floor. `oracle` needs a GPU and a browser. `corpus` needs the
        # corpus, which is not in git and so is not in CI. Everything else
        # runs, INCLUDING `wasm`: story E1.2's note is "CI fails if the module
        # exceeds the agreed budget", and a wasm-pack build costs no GPU.
        case "$name" in oracle|corpus) continue ;; esac
        selected+=("$name")
      done ;;
    --sprint)
      profile=sprint
      for entry in "${GATES[@]}"; do selected+=("${entry%%|*}"); done ;;
    --all)
      profile=all
      for entry in "${GATES[@]}"; do selected+=("${entry%%|*}"); done ;;
    "")  usage; return 2 ;;
    *)   selected=("$@") ;;
  esac

  for name in "${selected[@]}"; do
    printf '%s>> %s%s\n' "$DIM" "$name" "$OFF"
    status=0
    if [ "$name" = oracle ] && [ "$profile" = sprint ] &&
       [ ! -d tools/oracle/node_modules ] && s01_pre_oracle; then
      skip "S01 precedes F-010, so no oracle exists yet. This exception is" \
        "limited to the sprint profile while F-010 remains pending in S02." || status=$?
    else
      run_gate "$name" || status=$?
    fi
    case "$status" in
      0) passed=$((passed + 1)) ;;
      3) skipped+=("$name") ;;
      *) failed+=("$name") ;;
    esac
  done

  echo
  if [ ${#skipped[@]} -ne 0 ]; then
    printf '%sSKIPPED%s  %s\n' "$DIM" "$OFF" "${skipped[*]}"
  fi
  if [ ${#failed[@]} -eq 0 ]; then
    if [ ${#skipped[@]} -eq 0 ]; then
      printf '%sALL GREEN%s  %d gate(s)\n' "$GREEN" "$OFF" "$passed"
    else
      printf '%sGREEN%s  %d passed, %d skipped. A skipped gate is NOT a pass.\n' \
        "$GREEN" "$OFF" "$passed" "${#skipped[@]}"
    fi
    return 0
  fi
  printf '%sFAILED%s  %s\n' "$RED" "$OFF" "${failed[*]}"
  printf '%d passed, %d failed, %d skipped\n' \
    "$passed" "${#failed[@]}" "${#skipped[@]}"
  return 1
}

command=${1:-}
[ $# -gt 0 ] && shift || true

case "$command" in
  check)   cargo check -p "$1" --all-targets ;;
  build)   cargo build -p "$1" ;;
  test)    crate=$1; shift; cargo test -p "$crate" "$@" ;;
  fmt)     cargo fmt --all --check ;;
  clippy)  if [ $# -gt 0 ]; then cargo clippy -p "$1" --all-targets -- -D warnings
           else cargo clippy --workspace --all-targets -- -D warnings; fi ;;
  cargo)   cargo "$@" ;;

  wasm)
    command -v wasm-pack >/dev/null || {
      echo "wasm-pack is not installed. See docs/DEVELOPER_SETUP.md" >&2
      exit 1
    }
    # Release by default. HLD 15.2's profile (opt-level "z", fat LTO,
    # codegen-units 1, panic abort, strip) only applies to a release build,
    # and a dev-profile size measurement would be meaningless against gate A4.
    wasm-pack build crates/ocelli-wasm --target web --out-dir pkg "$@"
    ;;

  native)
    # HLD story E1.7. The cross-target build proof is what keeps decision D2
    # honest over time: if the core stopped being WebAssembly-agnostic, this
    # is where it shows up first.
    cargo build -p ocelli-native
    ;;

  oracle)
    if [ ! -d tools/oracle/node_modules ]; then
      echo "The oracle is not installed yet. It is built by F-010 (E2.2)." >&2
      echo "Nothing else in the port should start before it works" >&2
      echo "(docs/hld/25-first-ten-files.md, entry 4)." >&2
      exit 1
    fi
    node tools/oracle/run.mjs "$@"
    ;;

  corpus)  python3 scripts/corpus_check.py "$@" ;;
  gate)    gates_cmd "$@" ;;
  ""|-h|--help|help) usage ;;
  *)       echo "unknown command: $command" >&2; usage >&2; exit 2 ;;
esac
