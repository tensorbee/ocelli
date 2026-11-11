/* exported exportActiveWindowCheck */
// Private acceptance helper for the independent F227 check. Not product UI.
// Injected locally by the operator into the release folder viewer. Always
// returns undefined. On any failure it shows only the fixed closed message
// and never exposes error detail. No network, no console, no logs. This is
// caller-input preparation, not an AC16 completion, and it is never run
// against an actual study during preparation.
function exportActiveWindowCheck(mode = "auto") {
  function fail() {
    try { window.alert("Window check unavailable."); } catch { /* No fallback may expose error detail. */ }
  }
  let url = null;
  let anchor = null;
  try {
    if (mode !== "auto" && mode !== "file-voi-control") return fail();
    const generation = cacheGeneration;
    const vp = activeViewport();
    const series = seriesOf(vp);
    if (!series || !series.key || !Array.isArray(series.images)) return fail();
    // Refuse an in-flight series job in either mode.
    if (seriesWindowJobs.has(series.key)) return fail();
    const current = currentImage(vp);
    if (!current) return fail();
    let entry = null;
    let observed = [];
    if (mode === "auto") {
      if (seriesWindowRefused.has(series.key)) return fail();
      entry = seriesWindows.get(series.key);
      if (!entry) return fail();
      if (typeof entry.center !== "string" || entry.center === "") return fail();
      if (typeof entry.width !== "string" || entry.width === "") return fail();
      if (vp.draftWindow) return fail();
      // The existing helper itself rejects manual override, preset,
      // ineligibility and non-default effective window by returning "".
      if (seriesHeader(current, vp) !== entry.center + " " + entry.width) return fail();
      observed = [{ center: entry.center, width: entry.width }];
    }
    const records = [];
    let root = null;
    // Map series.images in exact current page order. No sorting, no inference.
    for (let i = 0; i < series.images.length; i++) {
      const image = series.images[i];
      const rel = image && image.file && image.file.webkitRelativePath;
      if (typeof rel !== "string" || rel === "") return fail();
      if (rel.indexOf("\\") !== -1 || rel.indexOf("\0") !== -1) return fail();
      if (rel.charAt(0) === "/" || /^[A-Za-z]:/.test(rel)) return fail();
      const cut = rel.indexOf("/");
      if (cut === -1) return fail(); // single-file or non-folder input
      const first = rel.slice(0, cut);
      const segments = rel.split("/");
      for (let s = 0; s < segments.length; s++) {
        if (segments[s] === "" || segments[s] === "." || segments[s] === "..") return fail();
      }
      if (root === null) root = first;
      else if (first !== root) return fail(); // common first root segment
      if (!Number.isSafeInteger(image.frame) || image.frame < 0) return fail();
      records.push({ path: rel.slice(cut + 1), frame: image.frame }); // first segment only
    }
    // Reverify immediately before download. No stale generation downloads.
    if (cacheGeneration !== generation) return fail();
    if (activeViewport() !== vp || seriesOf(activeViewport()) !== series) return fail();
    if (seriesWindowJobs.has(series.key)) return fail();
    if (mode === "auto") {
      if (seriesWindows.get(series.key) !== entry || vp.draftWindow) return fail();
      const now = currentImage(vp);
      if (!now || seriesHeader(now, vp) !== entry.center + " " + entry.width) return fail();
    }
    const json = JSON.stringify({ mode: mode, records: records, observed: observed });
    url = URL.createObjectURL(new Blob([json], { type: "application/json" }));
    anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "ocelli-window-check.json";
    document.body.appendChild(anchor);
    anchor.click();
  } catch {
    return fail();
  } finally {
    if (anchor) { try { anchor.remove(); } catch { /* Cleanup must remain silent. */ } }
    if (url) {
      // Cleanup only: never re-read study state or expose diagnostics.
      setTimeout(function () { try { URL.revokeObjectURL(url); } catch { /* Cleanup must remain silent. */ } }, 0);
    }
  }
}
