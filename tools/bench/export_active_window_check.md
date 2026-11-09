# F227 private acceptance export helper: procedure

Private caller-input preparation for the independent F227 check. Not product UI and not an AC16 completion. No actual study has been accessed during preparation.

## Steps

1. Open the release folder viewer locally and inject `export_active_window_check.js` so the page defines `exportActiveWindowCheck`. Change nothing else.
2. Load the selected folder, select the current series and its default auto window. The helper refuses an in-flight series job and any other failure with only the closed message "Window check unavailable." It never logs, prints or returns study data.
3. Call `exportActiveWindowCheck("auto")` for the cached automatic-window route, or `exportActiveWindowCheck("file-voi-control")` for a manual VOI control route. The call always returns undefined.
4. Keep the downloaded `ocelli-window-check.json` private. It is the only authorized output: `{mode, records:[{path,frame},...], observed:[{center,width},...]}`. Records map `series.images` in exact current page order, `frame` being each image's zero-based source-frame index, never its page-order position. Each `path` is the file's `webkitRelativePath` with only the common first folder segment stripped. In auto mode `observed` holds the one cached centre and width pair exactly as the `/series-window` reply text arrived, unrounded and unreformatted. In file-voi-control mode `observed` is empty and no automatic-window claim is made.
5. Supply the selected folder to the existing calculator as its `--input-root`. Record paths had the common first folder-name prefix stripped. The calculator runs one series per invocation.
6. Check the unrounded observed string provenance by hand against the page.

## Limits

Exporting one cached pair is not first, middle or last evidence and does not replace manual observation. Separately perform manual first, middle, last, scroll and Auto label checks. For file-voi-control, selected sample VOI ineligibility is established later by the independent calculator, not by this export.

## Synthetic controls

Run `node --test tools/bench/tests/export_active_window_check_test.mjs`. These permanent VM controls use only synthetic caller state and are registered in the benchmark gate. They complement the explicit Python suite and do not replace actual browser evidence.
