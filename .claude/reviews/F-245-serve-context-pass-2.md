# F-245 serve context and /dicweb/config review, pass 2

**Reviewed**: `examples/folder-viewer/src/serve.rs` 6ad54f1d7c8d0148b897370ec4923aa5c094a7198e8b84e71968d6a7675415af, `examples/folder-viewer/src/dicomweb.rs` 4af2d82b886268313c387bc61c7bf9ce384522e228c2798711791d97df92fea4, `examples/folder-viewer/src/csp.rs` 47bcf21b3fea9f62be4018af37cdedeeaecda052041b9004ecf544fa6a94ca41. Diffed against the three `claude-*-before-review1-repair.rs` baselines. Hashes are the same before and after this pass.

**Author / reviewer**: GLM authored the repairs. Claude (root independent reviewer, subagent) reviewed, as in pass 1.

**Result**: 1 defect, 0 smells, 4 nitpicks. Not clean.

## Pass 1 dispositions

D1, the from_os test gap. Fixed. Two new tests cover a lone invalid value in each position. The studies-site mutant rerun here now exits 101 on `viewer_dicomweb_config_from_os_refuses_non_unicode_studies_with_absent_wado`, and the lone-invalid probe still shows the mutant would return `Ok(None)`, so the new test is the thing catching it. The wado-site receipt shows the twin test red.

D2, the `handle_connection_view` comment. Replaced, but the new sentence is half false. See D1 below.

S1, the weak policy test. Fixed. `/` and `/index.html` are compared for equality with `configured_main_policy(PAGE, &["https://pacs.example:8443"])` and the dead binding is gone.

N4. Fixed. Every `OsStrExt` test carries `#[cfg(unix)]`.

N1, N2 and N3 are carried as dispositions and stay nitpicks. N5 is superseded by the new receipt, whose restored hash matches the current dicomweb.rs.

CSP comments. True as written. `page_policy` caches `configured_main_policy(page, &[])` at `csp.rs:25`, and the configured form is built once in `ServeContext::configured` and not cached.

## Findings

**D1** `serve.rs:380`. "The optional dicomweb config and main policy override the defaults for the served page when present." The main policy does replace the image page policy. The dicomweb config does not touch any page. It enables the `GET /dicweb/config` reply in `connection_session`. Suggested fix: say that the config enables `/dicweb/config` and the policy replaces the main page policy.

**N1** `GET /dicweb/config?x` reaches the config. This is the same for every GET route.

**N2** The config JSON is rebuilt on every config request.

**N3** `handle_connection_in` only forwards.

**N4** `serve.rs:380` and `csp.rs:5`. The new comment lines run past 100 columns. rustfmt does not wrap comments, and the neighbouring lines are wrapped.

## Evidence

| Command | Exit and observation |
| --- | --- |
| studies-site `None => None` mutant plus lone-invalid probe, `cargo test --lib dicomweb::tests`, two jobs | 101, the new absent-wado test fails, probe `ok_none=true` |
| restore from byte copy, then `touch` to force a rebuild | dicomweb.rs 4af2d82b again |
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer --lib` | 0, 163 passed |
| `cargo clippy -p ocelli-folder-viewer --all-targets` | 0, no warnings |
| `cargo fmt -p ocelli-folder-viewer --check` | 0 |

A caution for anyone restoring a mutant with `cp -p`: the restored file keeps its old mtime, and cargo then reuses the mutant build. My first full run after the restore reported 164 tests with one failure for that reason. `touch` fixed it, and the content hash did not change.

## Verdict

Not clean. One sentence at `serve.rs:380` is left to fix.
