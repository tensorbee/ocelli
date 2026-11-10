# F-245 serve context and /dicweb/config review, pass 1

**Reviewed**: `examples/folder-viewer/src/serve.rs` 1b46291d553105316dd6e56494f2e826d5ca6e2eac03e7cffc12500367fe60f0 (unchanged from the stated hash, before and after this pass), and `examples/folder-viewer/src/dicomweb.rs` b1230307f702a27e88c7d0c0f6d411552d692507bed2d11ba2bbf5a39078ebaf, `viewer_dicomweb_config_from_os` and its five tests only. Diffed against `claude-serve-before-context.rs` and `claude-dicomweb-before-from-os.rs` in the evidence root. Plan lines 272 and 274 of `.claude/plans/F-245-design.md`.

**Author / reviewer**: GLM authored. Claude (root independent reviewer, subagent) reviewed.

**Result**: 2 defects, 1 smell, 5 nitpicks. Not clean.

## What was checked and holds

- `HtmlPage::Image` is produced only by `page_reply` at `serve.rs:866`, reached only from `get_reply` for `/` and `/index.html` at `serve.rs:713`. Use and More keep `page_policy`. The query is stripped by `parse_header`, so `/?x` also gets the configured policy, which is the same page.
- Busy 503 and `ReadStop::Reply` replies carry `html_page: None`, so passing `main_policy` to them changes nothing. Local and configured contexts behave identically on both.
- The config arm at `serve.rs:464` retires the body through `retire_request_body(&charge, ...)` exactly as the `("GET", _)` arm of `route` does, and `charge` drops on the same line afterwards. Probe below shows `inflight` 0 after a GET with a body.
- POST `/dicweb/config` reaches `route` and 404s. HEAD is refused 400 by `parse_header`. `/dicweb/config/` and `/dicweb/Config` 404.
- `handle_connection` passes `None, None` and is otherwise unchanged. Exported items in `lib.rs:51` and the public signatures are unchanged. There is one connection loop, `connection_session`.
- The refusal message is the fixed constant. No env value is echoed.
- No allocation is added on non-config requests. The policy is borrowed.
- Both socket tests use real listeners and the coordinator receipts show three serve mutants red.

## Findings

**D1** `dicomweb.rs:580` and `:587`. A test that passes both ways. Mutating either `None => return Err(invalid())` to `None => None` leaves all 26 dicomweb tests green, and the receipt `non-unicode-as-absent` shows the same. Scenario: `OCELLI_VIEWER_STUDIES_SERVICE_URL` holds a non-UTF-8 byte and `OCELLI_VIEWER_WADO_URI_URL` is unset. The mutant returns `Ok(None)` and the viewer starts local only instead of refusing, which the probe confirmed. The existing single-site tests pair the invalid value with a valid one, so the partial-pair refusal hides the mutant. Add the lone-invalid case for each position.

**D2** `serve.rs:380`. The doc comment on `handle_connection_view` says "keyed on the viewer alone" while the function takes the config and policy as well. False prose.

**S1** `serve.rs:5024` and `:5040`. `image_policy` is computed and discarded with `let _ = &image_policy`. The test asserts only that one `connect-src` directive is present and that the policy differs from `page_policy`, so a configured policy with an extra or altered directive would pass. Compare to `configured_main_policy` exactly, or delete the binding.

**N1** `serve.rs:464`. `GET /dicweb/config?x` returns the config. Consistent with every other GET route, but not stated in the plan.

**N2** `serve.rs:467`. The JSON is rebuilt per config request. It could be serialized once in `ServeContext::configured`. One request per page load, so harmless.

**N3** `serve.rs:330`. `handle_connection_in` only forwards to `handle_connection_view`.

**N4** `dicomweb.rs` tests use `std::os::unix::ffi::OsStrExt`, so the test module does not build on Windows.

**N5** The receipt `claude-serve-context-mutants.json` records `dicomweb_sha256` 4314bd45..., not the current b1230307.... The dicomweb receipts predate the reviewed file.

## Lifecycle challenge

One keep-alive connection on a configured context, seven requests: `GET /` 200 with the configured CSP, `GET /dicweb/config` with a 5-byte body 200 JSON, `GET /dicweb/config?x` 200 JSON, `POST /dicweb/config` 404, `GET /dicweb/config/` 404, `GET /dicweb/Config` 404, `GET /held` 200 with `inflight` 0. The connection stayed open throughout. The claim holds.

## Evidence

| Command | Exit and observation |
| --- | --- |
| `shasum -a 256` both targets, before and after | serve 1b46291d, dicomweb b1230307, unchanged |
| `CARGO_BUILD_JOBS=2 cargo test -p ocelli-folder-viewer --lib` | 0, 161 passed |
| temporary keep-alive probe test in serve.rs, then exact restore | 0, 1 passed, output as above |
| studies-site `None => None` mutant plus lone-invalid probe, then exact restore | 0, 26 passed, probe `ok_none=true` |

Both files were restored from byte copies and the hashes rechecked.

## Verdict

Not clean. D1, D2 and S1 block. Pass 2 needs the lone-invalid tests, the doc fix and an exact policy assertion.
