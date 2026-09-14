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
  bench [args]           the benchmark harness (E1.6, HLD 26). --help for flags.
                         Records durations. It compares nothing unless asked
  oracle [args]          the differential harness against cornerstone3D (GPU)
  compare [args]         the pixel-diff comparator over the oracle's output
  corpus                 verify corpus/data against corpus/manifest.tsv
  corpus-tests           the corpus tooling suites (see OCELLI_PYTHON below)

Gates
  gate --list            what each gate covers
  gate <name>...         run named gates
  gate --floor           the gates CI runs: no GPU, no corpus, no browser
  gate --sprint          sprint gate, every gate with no exception
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
  "errors|no|error codes agree across Rust, TypeScript and the registry (HLD 23)"
  "panic|no|the wasm panic record survives the trap and needs no export (HLD 23)"
  "bench|no|the benchmark harness's registry, refusals and pins (E1.6, HLD 26)"
  "provenance|no|source-provenance policy, read-blocked projects (HLD C.2.1)"
  "prose|no|voice rules over operator-facing prose"
  "content|no|no DICOM and no build artefacts tracked"
  "backlog|no|BACKLOG, SPRINT_PLAN, tracker and as-built agree"
  "deviations|no|every HLD deviation declared and still true"
  "skills|no|Codex adapters match canonical sources and marked examples run"
  "lint|no|eslint, including the cached-wasm-view ban (HLD 17.2)"
  "types|no|tsc --build across the TypeScript workspaces"
  "wasm|no|wasm-pack build and the size budget (E1.2, gate A4)"
  "native|no|the cross-target build proof and per-target features (E1.7, HLD 4)"
  "device|no|only ocelli-render creates a GPU device (E1.8, HLD 31)"
  "packages|no|npm tarball contents, exports and a consumer install (E1.3)"
  "ci|no|every floor gate is actually invoked by .github/workflows/ci.yml"
  "guards|no|every declared guard still refuses what it is for (F-X009)"
  "guards-deep|no|the guard probes needing a cargo toolchain (F-X009)"
  "quirks|no|field quirks retain synthetic provenance and active regression evidence"
  "quirk-mutations|no|checker-owned quirk mutations fail at their fixed boundaries"
  "corpus-tests|no|the corpus generator and coverage suites, a skip fails it"
  "corpus|no|corpus coverage over the codec registry, then presence and digests"
  "gpu|YES|ocelli-render's #[ignore]d tests, the hardware tier (E6.1, D-04)"
  "oracle|YES|the differential corpus against cornerstone3D (HLD 11, D7)"
)

# `gate_needs_gpu()` used to sit here and was called by nothing. It is REMOVED
# rather than wired in, because the GPU column already has a reader that
# matters: `scripts/ci_floor_check.py`'s `gpu_gates()` parses this array and
# uses `YES` to decide which excluded gate CI is not supposed to run at all,
# which is deviation D-04. A second reader nothing calls is a place to look
# that answers no question, and its absence would never have been noticed,
# which is the same argument that removed `s01_pre_oracle` below.

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
    # F-005. The guard, then its own negative cases. Chained on `&&` for the
    # reason the backlog arm gives: a case arm returns the status of its LAST
    # command, so an unchained first command can fail and be reported green.
    errors)      python3 scripts/error_code_check.py &&
                 python3 -m unittest discover -s scripts/tests \
                   -p test_error_code_check.py ;;
    # F-005, HLD section 23. wasm32-unknown-unknown is `panic = "abort"` in
    # every profile, so nothing on the host can observe what a real trap leaves
    # behind. This builds a SECOND module carrying the `panic-probe` feature,
    # into its own out-dir under the gitignored crates/ocelli-wasm/target, so
    # the artefact `wasm` measures never carries a way to be asked to panic.
    panic)       command -v wasm-pack >/dev/null || {
                   echo "wasm-pack is not installed. See docs/DEVELOPER_SETUP.md" >&2
                   return 1
                 }
                 wasm-pack build crates/ocelli-wasm --target web \
                   --out-dir target/panic-probe -- --features panic-probe &&
                 node scripts/panic_probe.mjs ;;
    # F-006, HLD section 26. The gate asserts the INSTRUMENT and never a
    # duration: the registry parses, every subject_story resolves in
    # allocation.json, no subject whose story is pending has a runner or a
    # recorded number, and the two harnesses' playwright pins are equal. That
    # is deterministic, needs no GPU and no browser, and so it is in the floor.
    #
    # The DURATION COMPARISON is deliberately not here and not in --sprint.
    # `bin/ocelli.sh bench --compare` runs it on the machine that owns the
    # baseline. A duration taken on a machine that did not record the baseline
    # is either noise or a skip, a skipped gate is not a pass here, and a
    # permanently amber gate is a gate that gets disabled.
    #
    # Three commands, chained on `&&` for the reason the backlog arm gives: a
    # case arm returns the status of its LAST command.
    #
    # tests/cold_start_test.mjs IS here, and it was not until the S03 review's
    # fourth pass. Six of its seven tests need no browser, and the whole file
    # was excluded because the runner it imports pulled in `playwright` at
    # module scope, so the file could not load without an install. The runner
    # now imports playwright inside `run()`, and the one test that does launch
    # a browser is opted into with OCELLI_BENCH_BROWSER=1, which
    # `npm run test:browser` in tools/bench sets. So this stays a no-browser
    # gate and gains workspaceVersion, median, atClockPrecision and
    # resolveServedPath.
    bench)       python3 scripts/bench_check.py &&
                 python3 -m unittest discover -s scripts/tests \
                   -p test_bench_check.py &&
                 node --test tools/bench/tests/hostclass_test.mjs \
                   tools/bench/tests/paths_test.mjs \
                   tools/bench/tests/record_test.mjs \
                   tools/bench/tests/registry_test.mjs \
                   tools/bench/tests/run_test.mjs \
                   tools/bench/tests/state_test.mjs \
                   tools/bench/tests/cold_start_test.mjs \
                   tools/bench/tests/decode_frame_test.mjs \
                   tools/bench/tests/decode_htj2k_test.mjs \
                   tools/bench/tests/decode_jpeg2000_test.mjs \
                   tools/bench/tests/decode_jpegls_test.mjs \
                   tools/bench/tests/tier_startup_test.mjs ;;
    provenance)  python3 scripts/source_provenance_check.py ;;
    prose)       python3 scripts/prose_check.py ;;
    content)     python3 scripts/staged_content_check.py --tracked ;;
    # `&&` and not two statements on two lines. A case arm returns the status
    # of its LAST command, so an unchained first command can fail, print a
    # traceback, and be reported green. That happened here once already.
    backlog)     python3 scripts/backlog_check.py &&
                 python3 scripts/gen_sprint_plan.py --check ;;
    deviations)  python3 scripts/deviation_check.py ;;
    skills)      python3 scripts/sync_agent_skills.py --check &&
                 python3 scripts/skill_examples_check.py &&
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_skill_examples_check.py ;;
    lint)        [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run lint &&
                 node --test scripts/tests/test_eslint_wasm_memory_view.mjs ;;
    types)       [ -d node_modules ] || { skip "node_modules is absent, run npm ci"; return 3; }
                 npm run typecheck ;;
    # No skip. F-002 (E1.2) declared wasm-bindgen in ocelli-wasm, so wasm-pack
    # can build it and there is a release artefact to measure. The skip that
    # used to sit here named F-096 as the story that would land the dependency,
    # and it was wrong about which story: the pipeline needs the dependency
    # before the boundary does, because wasm-pack refuses a crate without one.
    #
    # Chained on `&&` for the reason the backlog arm gives.
    wasm)        "$0" wasm &&
                 python3 scripts/pin_and_size_check.py --with-size &&
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_pin_and_size_check.py ;;
    native)      "$0" native ;;
    device)      ci/check-device-ownership.sh ;;
    ci)          python3 scripts/ci_floor_check.py ;;
    # F-X009. A green run of the gates is not evidence that the gates work, it
    # is evidence that nothing was wrong or that nothing was checked, and only
    # this tells those apart (docs/runbooks/guard-verification.md).
    #
    # Three commands, chained on `&&` for the reason the backlog arm gives: a
    # case arm returns the status of its LAST command. The census proves the
    # catalogue is complete and no guard has been widened, the probe runner
    # drives each declared refusal red inside a disposable repository, and the
    # new lint-policy guard asserts HLD 27.1's deny list is still denied.
    #
    # Every probe here runs with no cargo, no npm, no wasm-pack, no browser,
    # no corpus and no GPU, which is what puts it in the floor. The census
    # REFUSES an entry that declares otherwise and sits in the floor anyway.
    #
    # The GATE is not under that rule and one command in this arm needs cargo.
    # scripts/lint_policy_check.py reads the workspace member set from
    # `cargo metadata --no-deps`, because `[workspace] members` is not that set
    # and a crate reached as a path dependency was measured linted by cargo and
    # never walked by the guard. `nostd` is in the floor on the same footing
    # and has been since it started running `cargo tree`, and the CI job that
    # runs this gate installs the pinned toolchain. What follows for the
    # PROBES is that every `lint-policy` one declares `needs="cargo"` and runs
    # in `guards-deep` below.
    guards)      python3 scripts/lint_policy_check.py &&
                 python3 scripts/guard_census.py &&
                 python3 scripts/guard_probe.py --self-test &&
                 python3 scripts/guard_probe.py --profile floor &&
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_guard_catalogue.py &&
                 # F-X014. The integration-handoff grammar is part of the
                 # guard contract, including forms the catalogue samples do
                 # not repeat. Keep this suite named so it cannot silently
                 # leave the required gate.
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_sprint_workflow.py &&
                 # F-X020. The sprint-plan writer protects hand-curated prose,
                 # and these tests cover its bootstrap, refusal, forced
                 # replacement and read-only check modes.
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_gen_sprint_plan.py &&
                 # The two readers that stopped being regexes in the S03
                 # review's eleventh pass, checked against bash and against
                 # TOML rather than against themselves. Named rather than
                 # globbed, so a file added under scripts/tests/ does not
                 # silently join or leave this gate.
                 python3 -B -m unittest discover -s scripts/tests \
                   -p test_guard_readers.py ;;
    # The level-3 runs that need a toolchain. NOT in the floor, and excluded
    # by name in the --floor arm below and in scripts/ci_floor_check.py's
    # NOT_IN_FLOOR. The two lists are compared for set equality there, so
    # missing either is refused rather than being a matter of care.
    #
    # scripts/ci_floor_check.py also refuses the case where the CI step goes
    # away entirely: a gate outside the floor that needs no GPU must still
    # be run by some CI step.
    #
    # It is a STEP in the `guards` job since the S03 review's ninth pass, and
    # it was a separate job gated to push-to-main and workflow_dispatch
    # before that. So a weakened deep guard is caught on the pull request that
    # weakened it, which matters most for the `lint-policy` probes: every one
    # of them is here, and the sprint review has found a route past that guard
    # on every pass since the fifth. The two reasons the old trigger carried
    # were both false, and .github/workflows/ci.yml records which.
    guards-deep) python3 scripts/guard_census.py --profile deep &&
                 python3 scripts/guard_probe.py --profile deep ;;
    # F-014. The registry is data only. Its checker parses the named source
    # and fixture symbols but never executes a command read from JSON.
    quirks)      python3 scripts/quirk_check.py &&
                 python3 -B -m unittest discover -s scripts/tests \
                   -p 'test_quirk*.py' ;;
    # The executable half of F-014's mutation evidence. This is outside the
    # floor because it needs both the locked DICOM environment and cargo. The
    # corpus-tooling CI job installs both and runs it on every event.
    quirk-mutations) python3 scripts/quirk_mutations.py ;;
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
                 python3 scripts/corpus_tests.py --metadata-check &&
                 cargo test -p ocelli-dicom --test corpus -- --ignored ;;
    # F-011. The reference half renders and the comparator judges, and the gate
    # means both. Chained on `&&` for the reason the corpus arm gives above: a
    # case arm returns the status of its LAST command, so an unchained
    # `"$0" oracle` could fail and be reported green by a passing comparison.
    # F-037. THE SET IS EVERY TEST MARKED `#[ignore]` IN `ocelli-render`, and
    # that is the whole definition. It is deliberately not described as "the
    # ones needing an adapter": most do, and
    # `a_cpu_override_yields_no_adapter_and_spends_no_startup_cost` does not.
    # That one is ignored because its MUTATION only dies where adapters exist,
    # not because it needs one to run, and a gloss over a growing set goes stale
    # the first time a test joins it for a new reason. This one already has.
    #
    # What the set is today: the device lifecycle, that short-circuit test, and
    # the fill-rate instrument noted below. F-041's LUT-shader comparison joins
    # it when that story lands, and the gate needs no edit for it because this
    # names no test file.
    #
    # `-- --ignored` runs ONLY the ignored tests, which is the whole set and
    # nothing else. The default `cargo test --workspace` in the `test` gate runs
    # every test NOT marked `#[ignore]`, so the two gates partition the crate's
    # suite on that attribute and neither is a subset of the other. The
    # partition is by the attribute and not by what a test needs, for the reason
    # the paragraph above gives.
    #
    # NOT a named list of test files. `--test device --test voi_shader` was the
    # design plan's spelling and it omits `ocelli-render`'s own `--lib` ignored
    # tests, one of which is the device-loss REBUILD path: that arm needs a
    # `pub(crate)` injection seam, because the pinned wgpu can produce only a
    # `Destroyed` loss on demand and `Unknown` is the one this project rebuilds
    # from, so the test has to live inside the crate. A named list would have
    # left the recovery arm run by nothing while reading as though it covered
    # the story.
    #
    # `probe::tests::measures_a_fill_rate_on_this_machine` is swept up by this
    # and asserts nothing, so it is not evidence of correctness. It is not
    # nothing either: it drives `resolve` end to end on a real adapter, so a
    # panic there fails this gate. A hang would hang it rather than fail it,
    # which is what `COMPLETION_TIMEOUT_NANOS` inside the probe is for. Its
    # printed figure is for a human
    # recording a band in `ci/tier-thresholds.json` and that is a separate,
    # deliberate, release-profile run.
    #
    # No `skip` arm. A machine with no adapter resolves tier C, and the tests
    # themselves report that they did not apply and pass, which is deviation
    # D-07's honesty rule rather than a gate pretending to have run.
    #
    # `--test-threads=1` because THE ADAPTER IS AN EXCLUSIVE RESOURCE.
    # `docs/sprints/CURRENT_SPRINT.md` says two workers running device tests
    # concurrently on one machine contend for it and produce timeouts that read
    # exactly like rendering failures. Most of the tests here open one, the
    # default harness runs them on one thread per core, and the gate was
    # therefore doing inside itself what the wave plan forbids between workers.
    gpu)         cargo test -p ocelli-render -- --ignored --test-threads=1 ;;
    oracle)      "$0" oracle && "$0" compare ;;
    *)           echo "unknown gate: $name" >&2; return 2 ;;
  esac
}

# Exit 3 from a gate means SKIPPED WITH A REASON, and it is counted and named
# separately from a pass. A gate that could not run must never read as one that
# ran and was happy, which is the whole reason this project distrusts a green
# summary it did not watch produce.
skip() { echo "SKIPPED: $*"; return 3; }

# The S01 pre-oracle exception used to live between `skip` and `gates_cmd`, as
# `s01_pre_oracle`. It let `gate --sprint` report an absent oracle as a named
# skip while S01 was building the corpus the oracle needs. F-010 built the
# oracle, so the condition can no longer be true, and the function is REMOVED
# rather than left in place with one that cannot fire. A dead exception is a
# live misreading: the next person to see a skipped oracle gate would have to
# prove it could not have applied instead of reading that it cannot exist.
# `.claude/WORKFLOW.md` records the same removal, in the same change.

# Select and run gates, counting a pass, a named skip and a failure apart.
gates_cmd() {
  local selected=() entry name gpu desc failed=() skipped=() passed=0 status

  case "${1:-}" in
    --list)
      printf '%-12s %-5s %s\n' GATE GPU COVERS
      for entry in "${GATES[@]}"; do
        IFS='|' read -r name gpu desc <<<"$entry"
        printf '%-12s %-5s %s\n' "$name" "$gpu" "$desc"
      done
      return 0 ;;
    --floor)
      for entry in "${GATES[@]}"; do
        IFS='|' read -r name gpu desc <<<"$entry"
        # The CI floor. `oracle` needs a GPU and a browser. `corpus` needs the
        # corpus, which is not in git and so is not in CI.
        #
        # `guards-deep` is here for a reason that has now been written three
        # ways and was false twice. It is not npm or wasm-pack, which the
        # eighth pass corrected. It is not the clock, and it is not "a runner
        # has to install the toolchain" either, which is what this comment said
        # until the S03 review's ninth pass: the `guards` CI job installs the
        # pinned toolchain for `gate guards` itself, so the runner that would
        # run the deep probes already has one. That job RUNS `gate guards-deep`
        # on every event since the ninth pass, so the trigger is no longer a
        # difference between the two at all.
        #
        # What is left, and it is the whole of it: `--profile deep` is a strict
        # SUPERSET of `--profile floor`, so a `gate --floor` that included this
        # would run every floor probe twice. THE TIMINGS ARE NOT WRITTEN
        # HERE. They are in ci/guard-probe-budget.json under
        # `wall_clock_seconds` and `--record-budget` writes them. This comment
        # carried deep 23.8s against floor 15.9s while the recorded pair said
        # otherwise, the tenth pass fixed that by copying the recorded pair
        # into four files, and the eleventh pass found all four saying 27.2
        # and 18.3 while the file said 28.4 and 18.6, because the recording
        # run moved them in the same commit that quoted them. A copy of a
        # measurement goes stale the next time the measurement is taken, so
        # read the file. CI pays that duplication deliberately,
        # because the alternative is `gate guards` running a different probe
        # set there from the one a developer gets. `gate --sprint` and
        # `gate --all` run both and so does the `guards` CI job.
        # `python3 scripts/guard_probe.py --list --profile deep` prints each
        # probe's profile and what it needs, and reading that beats reading
        # this. The PROFILE filters the listing to the probes this paragraph
        # is about. It was load-bearing until the tenth pass, when bare
        # `--list` printed the floor set alone and showed none of them.
        # Everything else runs, INCLUDING `wasm`: story E1.2's note is "CI
        # fails if the module exceeds the agreed budget", and a wasm-pack
        # build costs no GPU.
        #
        # `quirk-mutations` is also outside the floor. It needs both the locked
        # DICOM Python environment and cargo, which no single floor job owns.
        # The corpus-tooling job installs both and runs it on every event. The
        # stdlib-only `quirks` checker remains in the floor.
        #
        # This list and scripts/ci_floor_check.py's NOT_IN_FLOOR must agree,
        # and since the S03 review's fourth pass something joins them: that
        # file PARSES the `case` line below and refuses a set that differs
        # from NOT_IN_FLOOR in either direction. This comment used to say a
        # name in one list and not the other "makes the `ci` gate demand a
        # CI step for a gate the floor never runs", and the reviewer
        # measured that adding `prose` here alone left the `ci` gate at 0
        # while `gate --floor` silently stopped running `prose`. A gate
        # leaving the floor removes work rather than adding a demand, so
        # that direction had no detection at all. Now it does.
        #
        # `gpu` is the second GPU gate and joins `oracle` for the same reason
        # and under the same deviation D-04. It runs `ocelli-render`'s
        # `#[ignore]`d tests. Added by F-037, because `cargo test --workspace`
        # has `needs_gpu = no` and an `#[ignore]`d test therefore ran in NO
        # profile at all, including `--sprint`. It is an ADDITION to the set CI
        # does not run, so D-04's row stays true.
        case "$name" in oracle|corpus|guards-deep|quirk-mutations|gpu) continue ;; esac
        selected+=("$name")
      done ;;
    --sprint|--all)
      for entry in "${GATES[@]}"; do selected+=("${entry%%|*}"); done ;;
    "")  usage; return 2 ;;
    *)   selected=("$@") ;;
  esac

  for name in "${selected[@]}"; do
    printf '%s>> %s%s\n' "$DIM" "$name" "$OFF"
    status=0
    run_gate "$name" || status=$?
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
    # Eight steps, and each one's exit code is read from the command itself
    # rather than from the end of a pipe. `set -e` is on, so the first failure
    # ends the arm.
    #
    # Before F-007 this was `cargo build -p ocelli-native` alone, which is a
    # HOST build of ONE crate and proves nothing about wasm32 or about the
    # other eleven.

    # 1. The two entry points LINK, not merely type-check. A stub that only
    #    checks would hide a missing symbol until Phase 2.
    echo "  1/11 native entry points link"
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
    #    runner, which is F-101's and the oracle's ground, not this gate's.
    echo "  2/11 eleven shared crates plus ocelli-wasm build for wasm32"
    cargo check --workspace --exclude ocelli-native \
      --target wasm32-unknown-unknown

    # 3. Every crate the table marks `native: yes` builds natively.
    #    --all-targets IS right here: a native build runs the test suite, so
    #    the tests have to compile.
    echo "  3/11 the same crates build natively, tests included"
    cargo check --workspace --all-targets

    # 4. Resolved features agree across the two targets, or the difference is
    #    declared with a reason. This is the half a build proof cannot cover:
    #    both targets compiling while one quietly resolved a different feature
    #    set is the sprint's stated false-portability defect, and nothing goes
    #    red on its own.
    echo "  4/11 resolved features agree across targets"
    python3 scripts/target_feature_check.py

    # 5. The same production JPEG 2000 proof executes natively. Steps 6 and 7
    # run that exact source as plain and SIMD-enabled wasm under Node.
    echo "  5/11 codec wasm runner refusals and control"
    node --test scripts/tests/test_run_codec_wasm.mjs

    echo "  6/11 JPEG 2000 production decoder executes natively"
    cargo run -p ocelli-codec --release --example verify_jpeg2000

    echo "  7/11 JPEG 2000 production decoder builds as plain and SIMD wasm"
    cargo build -p ocelli-codec --release --example verify_jpeg2000 \
      --target wasm32-unknown-unknown --target-dir target/codec-wasm-plain
    cargo rustc -p ocelli-codec --release --example verify_jpeg2000 \
      --target wasm32-unknown-unknown --target-dir target/codec-wasm-simd -- \
      -C target-feature=+simd128

    echo "  8/11 plain and SIMD JPEG 2000 wasm execute under Node"
    node scripts/run_codec_wasm.mjs \
      target/codec-wasm-plain/wasm32-unknown-unknown/release/examples/verify_jpeg2000.wasm \
      target/codec-wasm-simd/wasm32-unknown-unknown/release/examples/verify_jpeg2000.wasm

    # 9 to 11 are the standing version of F-X013's central claim, that
    # openjph-core produces identical samples on native, plain wasm and
    # +simd128 wasm. The spike that measured it once is deleted. D-22.
    echo "  9/11 HTJ2K production decoder executes natively"
    cargo run -p ocelli-codec --release --example verify_htj2k

    echo "  10/11 HTJ2K production decoder builds as plain and SIMD wasm"
    cargo build -p ocelli-codec --release --example verify_htj2k \
      --target wasm32-unknown-unknown --target-dir target/codec-wasm-plain
    cargo rustc -p ocelli-codec --release --example verify_htj2k \
      --target wasm32-unknown-unknown --target-dir target/codec-wasm-simd -- \
      -C target-feature=+simd128

    echo "  11/11 plain and SIMD HTJ2K wasm execute under Node"
    node scripts/run_codec_wasm.mjs \
      target/codec-wasm-plain/wasm32-unknown-unknown/release/examples/verify_htj2k.wasm \
      target/codec-wasm-simd/wasm32-unknown-unknown/release/examples/verify_htj2k.wasm
    ;;

  bench)
    # The benchmark harness HLD section 26 names. F-006.
    #
    # It refuses an absent install the way `oracle` does, and for a sharper
    # reason: the ONE subject that has a subject today is timed in a browser,
    # so a missing playwright is not a degraded run, it is no run at all. The
    # harness would then report its single measurable subject as a runner
    # failure, which is correct output and is not what anyone typing this
    # wanted.
    #
    # tools/bench keeps its own install rather than sharing the oracle's, and
    # scripts/bench_check.py asserts the two playwright pins are equal so the
    # separation cannot become a drift.
    #
    # `--help` and `--list` are exempt, because neither loads a runner and
    # neither touches playwright. Refusing them would mean a developer could
    # not read what the harness measures without first installing a browser,
    # and the registry is the part most worth reading.
    case " $* " in
      *" --help "*|*" -h "*|*" --list "*) ;;
      *)
        if [ ! -d tools/bench/node_modules ]; then
          echo "The benchmark harness's browser is not installed." >&2
          echo "Run: (cd tools/bench && npm ci && npx playwright install chromium)" >&2
          echo "See docs/lld/benchmarks.md for what it measures and what it does not." >&2
          echo "\`bench --list\` and \`bench --help\` work without it." >&2
          exit 1
        fi ;;
    esac
    node tools/bench/run.mjs "$@"
    ;;

  oracle)
    # The refusal has two halves, and the second is the one that bites later.
    #
    # Absent is obvious. PRESENT BUT NOT AT THE PINNED VERSIONS is the case
    # that would otherwise produce reference frames from a reference nobody
    # pinned, and output from a moving reference is not reference output. That
    # half lives in run.mjs, because node can read an installed package.json
    # and shell cannot without another dependency, and it is a refusal rather
    # than a warning.
    #
    # `run.mjs` is the whole harness: the pins, the pure unit tests, two passes
    # over the corpus for determinism, the pydicom cross-read of the sidecars,
    # and the fault-injection self test. One place to look, and `--help` says
    # what each flag turns off.
    if [ ! -d tools/oracle/node_modules ]; then
      echo "The oracle's reference stack is not installed." >&2
      echo "Run: (cd tools/oracle && npm ci && npx playwright install chromium)" >&2
      echo "See docs/lld/oracle.md for what it is and why it is pinned." >&2
      exit 1
    fi
    node tools/oracle/run.mjs "$@"
    ;;

  compare)
    # The comparator, F-011. It reads two directories of reference-half output
    # and returns a verdict per view against HLD 25.1. See
    # docs/lld/comparator.md.
    #
    # Two exercises with no arguments, and both are needed. `identity` proves
    # the loader, the identifier mapping, the class resolution, the sidecar
    # contract and the report shape over every view, and it proves NOTHING
    # about detection. `mutations` replays the declared catalogue and requires
    # each entry to produce the verdict written beside it, which is the half
    # that proves detection. Chained on `&&` for the reason the corpus gate arm
    # gives.
    #
    # RELEASE, and not for speed alone. A debug build of a comparison over
    # ninety-nine frames plus twenty-one mutation replays is minutes rather
    # than seconds, and a check nobody wants to wait for is a check that stops
    # being run.
    #
    # An argument is passed straight through. F-012's candidate-evidence form
    # is `bin/ocelli.sh compare gate --reference REF --candidate CANDIDATE`.
    # It requires two distinct resolved directories and never defaults the
    # candidate to the reference. F-X021 connects it to Ocelli output.
    if [ ! -d tools/oracle/out ]; then
      echo "There is no oracle output to compare." >&2
      echo "Run: bin/ocelli.sh oracle" >&2
      echo "See docs/lld/comparator.md for what this compares and what it does not." >&2
      exit 1
    fi
    cargo build --release -p ocelli-oracle --bin ocelli-compare
    if [ "$#" -gt 0 ]; then
      ./target/release/ocelli-compare "$@"
    else
      ./target/release/ocelli-compare identity &&
      ./target/release/ocelli-compare mutations
    fi
    ;;

  corpus)  python3 scripts/corpus_check.py "$@" ;;
  gate)    gates_cmd "$@" ;;
  ""|-h|--help|help) usage ;;
  *)       echo "unknown command: $command" >&2; usage >&2; exit 2 ;;
esac
