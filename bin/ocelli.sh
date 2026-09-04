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
  native                 the cross-target proof: both targets, and features

Validation
  oracle [args]          the differential harness against cornerstone3D (GPU)
  corpus                 verify corpus/data against corpus/manifest.tsv
  corpus-tests           the corpus tooling suites (see OCELLI_PYTHON below)

Gates
  gate --list            what each gate covers
  gate <name>...         run named gates
  gate --floor           the gates CI runs: no GPU, no corpus, no browser
  gate --sprint          sprint gate, with the S01 pre-oracle exception
  gate --all             every gate, including the GPU and corpus tiers

Environment
  OCELLI_PYTHON          interpreter with pydicom for `gate corpus-tests`,
                         CI override, local default is .venv/bin/python
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
  "nostd|no|no_std crates reach no dependency std feature (D-09)"
  "provenance|no|source-provenance policy, read-blocked projects (HLD C.2.1)"
  "prose|no|voice rules over operator-facing prose"
  "content|no|no DICOM and no build artefacts tracked"
  "backlog|no|BACKLOG, SPRINT_PLAN, tracker and as-built agree"
  "deviations|no|every HLD deviation declared and still true"
  "skills|no|Codex adapters match their canonical command and skill files"
  "lint|no|eslint, including the cached-wasm-view ban (HLD 17.2)"
  "types|no|tsc --build across the TypeScript workspaces"
  "wasm|no|wasm-pack build and the size budget (E1.2, gate A4)"
  "native|no|the cross-target build proof and per-target features (E1.7, HLD 4)"
  "device|no|only ocelli-render creates a GPU device (E1.8, HLD 31)"
  "packages|no|npm tarball contents, exports and a consumer install (E1.3)"
  "ci|no|every floor gate is actually invoked by .github/workflows/ci.yml"
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
    nostd)       python3 scripts/no_std_check.py ;;
    provenance)  python3 scripts/source_provenance_check.py ;;
    prose)       python3 scripts/prose_check.py ;;
    content)     python3 scripts/staged_content_check.py --tracked ;;
    # `&&` and not two statements on two lines. A case arm returns the status
    # of its LAST command, so an unchained first command can fail, print a
    # traceback, and be reported green. That happened here once already.
    backlog)     python3 scripts/backlog_check.py &&
                 python3 scripts/gen_sprint_plan.py --check ;;
    deviations)  python3 scripts/deviation_check.py ;;
    skills)      python3 scripts/sync_agent_skills.py --check ;;
    lint)        [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run lint ;;
    types)       [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run typecheck ;;
    # No skip. F-002 (E1.2) declared wasm-bindgen in ocelli-wasm, so wasm-pack
    # can build it and there is a release artefact to measure. The skip that
    # used to sit here named F-096 as the story that would land the dependency,
    # and it was wrong about which story: the pipeline needs the dependency
    # before the boundary does, because wasm-pack refuses a crate without one.
    #
    # Chained on `&&` for the reason the backlog arm gives.
    wasm)        "$0" wasm && python3 scripts/pin_and_size_check.py --with-size ;;
    native)      "$0" native ;;
    device)      ci/check-device-ownership.sh ;;
    ci)          python3 scripts/ci_floor_check.py ;;
    packages)    [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run test &&
                 python3 scripts/package_check.py ;;
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
                 python3 scripts/corpus_check.py &&
                 python3 scripts/corpus_tests.py --metadata-check ;;
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
    # HLD story E1.7, the cross-target build proof. It is what keeps decision
    # D2 honest over time: if the core stopped being WebAssembly-agnostic,
    # this is where it shows up first.
    #
    # Four steps, and each one's exit code is read from the command itself
    # rather than from the end of a pipe. `set -e` is on, so the first failure
    # ends the arm.
    #
    # Before F-007 this was `cargo build -p ocelli-native` alone, which is a
    # HOST build of ONE crate and proves nothing about wasm32 or about the
    # other eleven.

    # 1. The two entry points LINK, not merely type-check. A stub that only
    #    checks would hide a missing symbol until Phase 2.
    echo "  1/4 native entry points link"
    cargo build -p ocelli-native --bins

    # 2. Every crate HLD section 4 marks `wasm: yes` builds for wasm32.
    #    ocelli-native is excluded because that same table marks it `wasm: no`,
    #    and its lib.rs turns that cell into a compile_error rather than
    #    leaving it as a claim.
    #
    #    NOT --all-targets here, and step 3 is where that flag belongs. For
    #    wasm32 it pulls in dev-dependencies, and `proptest` reaches
    #    `wait-timeout`, which does not compile for wasm32 and is not supposed
    #    to. What ships to a browser is the lib, so that is what is proved.
    #    Running the tests under wasm32 needs wasm-bindgen-test and a browser
    #    runner, which is F-096's and the oracle's ground, not this gate's.
    echo "  2/4 eleven shared crates plus ocelli-wasm build for wasm32"
    cargo check --workspace --exclude ocelli-native \
      --target wasm32-unknown-unknown

    # 3. Every crate the table marks `native: yes` builds natively.
    #    --all-targets IS right here: a native build runs the test suite, so
    #    the tests have to compile.
    echo "  3/4 the same crates build natively, tests included"
    cargo check --workspace --all-targets

    # 4. Resolved features agree across the two targets, or the difference is
    #    declared with a reason. This is the half a build proof cannot cover:
    #    both targets compiling while one quietly resolved a different feature
    #    set is the sprint's stated false-portability defect, and nothing goes
    #    red on its own.
    echo "  4/4 resolved features agree across targets"
    python3 scripts/target_feature_check.py
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
