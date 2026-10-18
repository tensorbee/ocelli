# F-X002 review, pass 1

**Reviewed**: working tree on sprint/s14 against 879f80f
**Result**: 1 defect, 0 smells, 1 nitpick

## Defects

### D1, the unit test never executes the exit code the workflow branches on

**Where**: `scripts/lavapipe_layer.py:58-59`, `scripts/tests/test_lavapipe_layer.py:18-20`, `.github/workflows/lavapipe.yml:42-48`

**What**: `main` returns 10 when `ci/lavapipe-layer.json` is missing, and the lavapipe workflow runs the probe only for that exit code. Exit 0 is the drop branch and does not run the probe. The unit test calls `decide` and asserts the string `probe`. It never calls `main`. Replacing `return 10` with `return 0` left all five tests green. The same missing file then exited 0 with empty stdout. That is the workflow's success branch, so the step would end green with neither `layer 2 dropped:` nor `lavapipe resolved`.

**Why it is wrong**: The approved plan says a missing file continues into the probe, and a green step that skips the probe is the failure this story refuses. The sprint text says a named skip printed as a pass is not the drop. The plan's test row says this unit test proves a missing file means the probe is required. A test that stays green when that contract is inverted will be counted as the guard forever. A guard nothing executes is that defect class. `decide` itself is tested. The process exit the workflow reads is not.

**Evidence**: `python3 -m unittest scripts/tests/test_lavapipe_layer.py` exited 0 with 5 tests before the mutation. `python3 scripts/lavapipe_layer.py` with no record exited 10 and printed nothing. A bash copy of the workflow branch, with a stub in place of `cargo run`, then took the probe arm, and a stub exit 2 made that script exit 2. After `return 10` was changed to `return 0`, the same unittest exited 0. `python3 scripts/lavapipe_layer.py` exited 0 with empty stdout, and the workflow branch skipped the probe. The line was restored. The unittest exited 0 again and the direct run exited 10 again. On the restored file, a drop record exited 0 and printed `layer 2 dropped: no ICD on ubuntu-latest`. A `skip` document, a `pass` document, an empty reason, and a whitespace-only reason each exited 1. None of those process exits is an assertion in the unit test.

## Smells

## Nitpicks

### N1, the SwiftShader fixture is shorter than the renderer string this story records

**Where**: `crates/ocelli-render/src/caps.rs:425` and `crates/ocelli-render/src/caps.rs:2528-2530`

**What**: The negative fixture is `ANGLE (Google, SwiftShader Device)`. The layer 3 record also contains `LLVM 10.0.0`. Changing the needle from `llvmpipe` to `llvm` left both predicate tests green. That shorter needle is a substring of the recorded renderer string, so the negative test does not lock SwiftShader against a shortened needle. The shipped needle is `llvmpipe`. The recorded string does not contain `llvmpipe` or `lavapipe`.

## Verified clean

Write set, from `git status --short` and `git diff HEAD --stat` on `sprint/s14` at `879f80f`. Modified files are `crates/ocelli-render/src/caps.rs`, `docs/lld/README.md`, and `docs/lld/tier-resolution.md`. Untracked files are `.github/workflows/lavapipe.yml`, `.github/workflows/swiftshader.yml`, `crates/ocelli-render/examples/lavapipe_probe.rs`, `scripts/lavapipe_layer.py`, `scripts/tests/test_lavapipe_layer.py`, and `tools/swiftshader/run.mjs`. `ci/lavapipe-layer.json` is absent. `docs/hld/DEVIATIONS.md`, `.github/workflows/ci.yml`, `tools/tier-b/run.mjs`, and F-043 product files are not in the diff. The same status held after the mutations were restored.

No new pixel arithmetic. The story adds a string predicate, a probe example, two workflows, and a browser launch. No `as` cast and no DICOM formula. The observed layer 3 report carried the zero placeholder rows, and `tools/swiftshader/run.mjs` does not compare them to anything.

Predicate needle. `bin/ocelli.sh cargo test -p ocelli-render --lib lavapipe_predicate` exited 0, with both tests passing. Replacing `llvmpipe` with `not-a-pipe` made `lavapipe_and_llvmpipe_match_in_any_field` fail at `caps.rs:2523` and the test command exited 101. Restored, the same command exited 0. `fallback_is_lavapipe` is its own function after the `AdapterFacts` impl closes. It does not call `renderer_string_match` and it does not walk `SOFTWARE_RENDERER_STRINGS`. That list still has `swiftshader`, `softpipe`, `gallium`, and the other software names. The predicate matches only `lavapipe` and `llvmpipe`. The recorded SwiftShader string, lowercased, contains neither token.

Decision refusals. Removing the `status != "dropped"` check made `test_skip_and_pass_are_refused` fail with `dropped` against `refused`, and the unittest command exited 1. Restored, it exited 0. Removing the empty-reason check made `test_an_empty_reason_is_refused` fail the same way, exit 1. Restored, exit 0.

Local layer 2. `bin/ocelli.sh cargo run -p ocelli-render --example lavapipe_probe` exited 2. The example's own exit code was 2, read before any later command. Stderr said there was no fallback adapter. It named metal as having no fallback adapters and vulkan as support not compiled in. It also named noop, dx12, gl, and webgpu as support not compiled in. That is not a pass. No `ci/lavapipe-layer.json` was written. The LLD sentence that this machine exited 2, that Metal had no fallback adapter, and that Vulkan support was not compiled in, matches that run.

Pinned wgpu 30.0.1, read from the crate and not from memory. `RequestAdapterOptions.force_fallback_adapter` is a real field in `wgpu-types-30.0.1`. `Instance::request_adapter` returns `Result<Adapter, RequestAdapterError>`. `Instance::new` takes `InstanceDescriptor`, and `new_without_display_handle` exists. The example uses that API and does not call `request_device`. `DeviceType` in this pin has the five variants the example matches.

Vulkan on ubuntu-latest can be compiled. Workspace wgpu default features include `vulkan`. `ocelli-render` adds `webgl` and does not set `default-features = false`. `wgpu-core` 30.0.1 compiles its `vulkan` cfg for Linux when the `vulkan` feature is on. On Apple that cfg requires `vulkan-portability`, which this crate does not enable. The Mac line is that target fact. It is not a reason to add a feature, and the story did not write the drop file.

Workflow, read and then branched in bash with the real Python script. Push and pull_request triggers match `.github/workflows/ci.yml`: `main`, `sprint/**`, `pull_request`, and `workflow_dispatch`. The job installs `mesa-vulkan-drivers`, fails the step when the lavapipe ICD path is empty, and writes `VK_DRIVER_FILES` for the next step. A missing ICD is a failed step, not a drop record. With no record, Python exited 10 and a stub probe exit 2 made the copied step exit 2. A stub probe exit 0 printed `lavapipe resolved` and exited 0. A `skip` document exited 1 and the copied step refused it. `set +e` did not turn the stub probe failure into a green step. `scripts/ci_floor_check.py` binds `WORKFLOW` to `.github/workflows/ci.yml` only. The new workflows do not have to be registered there. D-04 was not edited.

Layer 3. `node tools/swiftshader/run.mjs` exited 0. Its first line was `This run is not the oracle and not lavapipe.` The report was status `demoted`, tier `C`, `decidedBy` `AdapterType`, reason `the WebGL adapter resolved tier C`, and renderer `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (LLVM 10.0.0) (0x0000C0DE)), SwiftShader driver)`. Those are the LLD's status, tier, renderer, and reason. The script requires `SwiftShader` by case-sensitive substring and tier `C`. Any other tier with that substring is `process.exit(1)`. That failure branch was not given a second browser run. The success path above is the run that was executed. Launch flags are `--use-gl=angle`, `--use-angle=swiftshader`, and `--enable-unsafe-swiftshader`, the same three the oracle uses for SwiftShader. The script does not import the comparator and does not open the corpus. `.github/workflows/swiftshader.yml` is `workflow_dispatch` plus cron `0 6 * * *`, not a pull request, and `bin/ocelli.sh` has no swiftshader or lavapipe arm. `gate gpu` is `cargo test -p ocelli-render -- --ignored --test-threads=1`, so it does not run the example.

A console warning during that browser run said `ReadPixels`. The returned rows were still the zero placeholder and the status was `demoted`, so the script did not compare a drawn image. Tier C was not recorded as a different image.

Boundary. No new `wasm_bindgen` attribute. The page calls the existing `tierBSection183` export. No new trait, generic, or `Box<dyn>`. The example lives under `crates/ocelli-render/` and calls `Instance::new` and `request_adapter` only. `bash ci/check-device-ownership.sh` printed `OK: only ocelli-render may create a device, and it still owns GpuContext` and exited 0. No `unsafe` in the new example or the predicate.

Guards. `scripts/guards/discover.py` scans `scripts/`, `ci/`, `.githooks/`, `bin/`, and `tools/`, and not `.github/` and not `scripts/tests/`. Against its shapes, `scripts/lavapipe_layer.py` and `tools/swiftshader/run.mjs` had no hits. `sys.exit(main())` and `process.exit(n)` are not the string-exit or `throw new Error` shapes. `.github/workflows/lavapipe.yml` contains an `exit 1` that the shell regex would see, and the file is outside the scan roots. No catalogue entry is required for the new Python or the new mjs.

Other commands, each exit code from the command itself. `bin/ocelli.sh fmt` exited 0. `bin/ocelli.sh clippy ocelli-render` exited 0, which includes the example under `--all-targets`. The final predicate test and the final unittest, after every mutation was restored, each exited 0.
