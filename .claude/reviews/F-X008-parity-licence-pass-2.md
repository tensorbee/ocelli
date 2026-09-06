# F-X008 review, pass 2

**Reviewed**: staged working tree against `deb7a3e`
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, the live sprint plan can contradict the effective parity target

**Where**: `tools/oracle/tests/pins_test.mjs`, operational consumer set

**What**: The test reads the parity command and `scripts/gen_sprint_plan.py`,
but not `docs/sprints/SPRINT_PLAN.md`. F-X020 made that tracked plan
hand-curated and made the generator refuse to overwrite it by default. The
generator is therefore no longer a proof that its live output says the same
thing.

**Why it is wrong**: `SPRINT_PLAN.md` is an operator-facing statement of the
Phase 1 target and currently carries the D-11 correction. A test advertised as
holding the operational parity consumers to one version cannot leave that live
claim outside its consumer set after the generator and output acquired
independent lifecycles.

**Evidence**: I changed the plan's D-11 sentence to claim 5.8.9 and to call
5.8.2 unavailable. `node --test tools/oracle/tests/pins_test.mjs` exited 0 with
8 of 8 tests passing. `python3 scripts/gen_sprint_plan.py --check` exited 0 and
`bin/ocelli.sh gate backlog` exited 0. The mutation was restored. The working
file and index now both hash to
`31b9a1987079f83f609eaa6da46d5f2d9c098fe9`.

### D2, the relative-escape test does not prove both containment checks

**Where**: `scripts/tests/test_guard_catalogue.py`,
`SandboxCopyPreservesTrackedShape`

**What**: The one relative-escape fixture places the same link at the same
depth below both roots. Its target therefore escapes both roots at once. The
test stays red-capable when either the source containment condition or the
destination containment condition is deleted, because the surviving condition
still rejects the fixture.

**Why it is wrong**: Pass 1 required source-root and destination-root
containment proofs. The implementation has both checks, but the standing tests
do not independently hold either one in place. The LLD statement that the
focused suite watches all branches is false.

**Evidence**: With only destination containment left, the focused catalogue
suite exited 0 with 53 of 53 tests passing. With only source containment left,
the same suite again exited 0 with 53 of 53 passing. Both mutations were
restored. Separate fixtures can keep the source target contained while making
the destination target escape, and vice versa, by placing the destination at
a different depth.

### D3, the package proof accepts links instead of packaged licence files

**Where**: `scripts/pin_and_size_check.py`, `check_package_licences`

**What**: `Path.is_file()` and `Path.read_bytes()` follow symlinks. A generated
package whose `LICENSE-MIT` and `LICENSE-APACHE` entries are symlinks to files
outside the package passes the check.

**Why it is wrong**: The LLD claims the generated package contains regular
copies. A link to bytes outside the publish directory is not a licence text
contained in that package, even if following it from the working tree yields
the right bytes. The gate must prove both entry shape and byte identity.

**Evidence**: A temporary package containing only two absolute symlinks to the
repository fixture grants made `check_package_licences` return an empty problem
list. The adversarial command exited 0 and printed
`packaged_symlinks=[True, True]`.

### D4, the absent repository-grant refusal is not watched

**Where**: `scripts/tests/test_pin_and_size_check.py`

**What**: Production code refuses an absent repository original, but the
focused suite has no case for it. Its four tests cover a green pair, an absent
packaged Apache grant, changed packaged bytes and `--with-size` wiring.

**Why it is wrong**: `docs/lld/guards.md` says the package-licence suite watches
the absent repository grant, absent packaged grant and differing-byte outcomes.
Only the latter two refusal branches are exercised, and the catalogue has no
separate absent-source probe.

**Evidence**: I deleted the absent-source branch from
`check_package_licences`. The focused suite exited 0 with 4 of 4 tests passing.
The mutation was restored.

## Gate connection and verified clean surface

- The three crate-local licence entries are tracked mode-120000 relative links
  to `../../LICENSE`, `../../LICENSE-MIT` and `../../LICENSE-APACHE`.
- The generated package currently contains three regular licence files. MIT is
  1,066 bytes with SHA-256
  `4f65e077edb846129a99e66efe73892282ec9525525f2062dd79bf105a3f4df0`.
  Apache is 11,358 bytes with SHA-256
  `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.
  Both `cmp` checks exited 0 against the repository originals.
- `test_pin_and_size_check.py` is invoked by the `wasm` gate, and CI invokes
  that gate. `test_guard_catalogue.py` is invoked by the `guards` gate.
  `pins_test.mjs` is in the oracle unit list, and its two new refusal probes
  are in the floor guard profile. The tests are connected, though the defects
  above show that three claims are incompletely specified.
- The canonical parity command and generated adapter agree. The three D-11
  packages in `tools/oracle/package.json` are exactly 5.8.2.
- No runtime, pixel, LUT, tolerance, unsafe or wasm-boundary change is present.

## Commands and exact outcomes

- `python3 -B -m unittest discover -s scripts/tests -p test_pin_and_size_check.py`
  exited 0, 4 tests passed.
- `python3 -B -m unittest discover -s scripts/tests -p test_guard_catalogue.py`
  exited 0, 53 tests passed.
- `node --test tools/oracle/tests/pins_test.mjs` exited 0, 8 tests passed.
- `python3 scripts/guard_probe.py --only pins.package-licence-absent` exited 0,
  1 refusal probe drove 1 guard red and 1 control was green.
- `python3 scripts/guard_probe.py --only pins.stale-operational-parity` exited
  0, 1 refusal probe drove 1 guard red and 1 control was green.
- `python3 scripts/guard_census.py` exited 0. It counted 629 refusals in 61
  files, 235 probes, 9 watched by nothing and 11 shared-word refusals outside
  the site count.
- `python3 scripts/sync_agent_skills.py --check` exited 0, 20 adapters matched.
- `python3 scripts/pin_and_size_check.py --with-size` exited 0. The wasm was
  16,388 bytes against a 16,388-byte baseline and a 17,207-byte ceiling.
- `bin/ocelli.sh gate wasm` first exited 1 inside the restricted sandbox because
  wasm-opt could not execute. Re-run with execution permission, it exited 0,
  1 gate passed and its 4 focused tests passed.
- `bin/ocelli.sh gate guards` exited 0, 1 gate passed. It ran 149 refusal
  probes across 29 guards, 23 accept probes, 38 controls, 1 declared known
  defect, then 53, 4 and 71 unit tests, all passing.
- `python3 scripts/prose_check.py --staged` exited 0 over 8 staged prose files
  after this pass-2 review was added.
- Two attempted `guard_probe.py --probe ...` invocations exited 2 because the
  runner's selector is `--only`. They produced no review evidence and were
  rerun with the correct selector above.
- One attempted path argument to `prose_check.py --staged` exited 2 because
  that command accepts the staged set only. It was rerun in its supported form
  above.

The three source mutations, the sprint-plan mutation and all temporary package
fixtures were restored or existed only below the system temporary directory.
No remediation was applied.
