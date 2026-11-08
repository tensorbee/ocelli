# F-239 additive tests review, working pass 2

**Result:** F-239 additive test repairs, pass 2, 0 defects, 0 smells, 0 nitpicks in the bounded repair scope.

**Source:** staged tree `094411c04b4ff1a753434321cdc68b54e6cf8d27`, comprising source-and-tests tree `9ef82d3f22937c9639f0abd6e62d94b7341971b2` plus four review records. Production is byte-identical to `cc62cdb4ab9e9ca9cecbf43b0836c48e0ba4fcc1`. `f239-server-final-handoff.json` records that composition. The test-only delta from production is one 1619-line file, `examples/folder-viewer/tests/colour.rs`.

**Independence:** local GLM authored the original Rust test file. Another Codex participant authored the subsequent repairs. This reviewer authored neither that file nor its repairs and made no production changes. Earlier reviewer-authored browser fixtures are outside this approval. The prior server production review remains a separate record.

The preserved autonomous file has SHA-256 `e51aa29e7f37fa3ce04782f74775e7b80db9d5676ef81e246beada58bba24496`. The repaired file has SHA-256 `c2a2482ca6a5eaf73fc6356d98b9ca366b5b6b0cf5ca3912c21419fd653d7b09`. Both were independently hashed, then their complete diff was read alongside the prior tests report and the original GLM report. Original GLM output and Codex corrections remain distinct.

## Repair checks

D1 is repaired. The YBR pointer test now requires one closed three-channel numeric array, finite values and the unchanged hand values `[46.72366,119.56544,138.98863]`. It separately requires exact text `R 46.7 G 119.6 B 139.0`. It no longer pins observed shortest-f32 spellings.

The lead explicitly authorized the existing `0.001` PS3.3 YBR fixture rule for this new numeric assertion. `crates/ocelli-pixel/tests/color.rs` already defines that bound and uses strict less-than comparison. The repaired test rejects error greater than or equal to that bound and rejects nonfinite channels. No existing tolerance, oracle score, renderer or gate changed. After the frozen server snapshot, the lead recorded the ruling in the canonical F-239 plan, reported staged in colour tree `7dd567302ac5abc31634238b409c6646fd04fbbf`. This later documentation does not change the reviewed test or production bytes.

An independent exact-fraction inversion of PS3.3 C.7.6.3.1.2's four-decimal forward matrix for YBR `100,150,90` gives RGB approximately `46.7236620367,119.5654530135,138.9886327379`. Each differs from the plan's unchanged printed hand value by less than `0.001`. This calculation reads no implementation output.

D2 is repaired without lint allowances. The seven prior clippy diagnostics correspond exactly to one Result unwrap, one complex return type, four strict float comparisons and one narrowing cast. The repair carries builder failures as Results, names the tagged-element type, compares exact window-field bits, and constructs the small frame index as u8. Valid fixture bytes and expected pixels remain unchanged by these mechanics. Named target clippy passed with warnings denied.

N1 is repaired. The 16-stored-bit test name now describes the declared shift. Its stored words `4095,2048,15` still require `15,8,0`, whereas the 12-stored-bit sibling requires `255,128,0`. The additional three-entry palette ramp uses indices `0,1,2` and every channel table `[0,100,255]`, at declared depths 12 and 16. R1 therefore reads each table as 8-bit and requires packed triples `0,0,0,100,100,100,255,255,255`. This is the approved interoperability fallback, not a claim that 12-bit palette descriptors conform to the standard.

The remaining numeric gold in this file is literal fixture evidence. The JPEG case deliberately compares preview bytes with the codec's own RGB output to test absence of a second colour conversion. That relational check does not establish decoder correctness or supply independent golden pixels. It was unchanged by these repairs.

## Executed controls

Runtime checks used `/private/tmp/ocelli-f239-server-review-p3`, restored to exact frozen production with the repaired test file copied byte-for-byte. Its dedicated target is `/private/tmp/f239-review-p3-target`. Review-only controls outside this target remain excluded from the named test and clippy invocations. `CARGO_BUILD_JOBS=2` was set. Every status below is the subprocess's direct exit.

| Check | Exit and actual observation |
|-------|-----------------------------|
| `bin/ocelli.sh test ocelli-folder-viewer --test colour -- --test-threads=2` | 0, 38 passed |
| `bin/ocelli.sh cargo clippy -p ocelli-folder-viewer --test colour -- -D warnings` | 0 |
| Change only numeric G serialization by +0.01, keep correct text | 101, rejected `119.57545` against hand value `119.56544` |
| Serialize numeric R as NaN, keep correct text | 101, finite-value check rejected NaN |
| Change only text channel G to X, keep correct numeric channels | 101, exact one-decimal text assertion failed |
| Restore immutable production and rerun complete named target | 0, 38 passed |

All three mutations were confined to the private review copy and restored. The author worktree was never mutated. Receipts are `/private/tmp/ocelli-s46-codex/f239-tests-independent-*.json` with corresponding full logs. The mutation driver is `/private/tmp/f239-tests-review-mutations.py` and the identity record is `/private/tmp/f239-test-review-identity.json`.

This pass covers the repairs to the additive Rust test artifact. It does not approve browser behavior, ICC implementation, GPU rendering, oracle agreement, performance, maximum-size memory, integration or the complete feature. No source remediation, shared corpus write, commit or push was performed.
