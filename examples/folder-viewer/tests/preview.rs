//! Synthetic Part 10 frames. The expected grey bytes are computed from
//! PS3.3 C.11.2.1.3.2, not from a previous run of this preview.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ocelli_core::{Index, Pt};
use ocelli_dicom::{FrameGeometry, MetadataSet, MultiframeMetadata, parse_part10};
use ocelli_folder_viewer::{
    AutoSource, FRAME_LIST_CAP, FrameRequest, ImagePoint, MeasureShape, Preview, PreviewError,
    SLICES, Viewer, ViewerConfig, age_text, dicom_date_text, dicom_time_text, exposure_mas,
    handle_connection, headers_json, inspect_json, measure_frame, patient_orientation_letters,
    person_name_text, plane_from_token, plane_token, preview_first_frame, preview_frame, serve_on,
    slice,
};
use ocelli_pixel::VoiFunction;
use ocelli_viewport::presentation::WindowPreset;

#[test]
fn an_enhanced_frame_uses_its_functional_group_window() -> Result<(), String> {
    // No top-level rescale or window. Shared functional groups carry slope 1
    // and intercept 0. Frame 0 has LINEAR_EXACT center 100 width 200.
    // Frame 1 has LINEAR_EXACT center 50 width 1. Both frames store 50.
    // Frame 0: ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    // Frame 1: ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    // Ignoring the groups would draw both frames at the automatic window.
    let file = enhanced_frames()?;
    let first = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the first enhanced frame was refused".to_owned())?;
    if first.auto != AutoSource::None {
        return Err("the enhanced frame was labelled auto".to_owned());
    }
    if only_byte(&first)? != 64 {
        return Err("the first enhanced frame was not grey 64".to_owned());
    }
    let second = preview_frame(
        &file,
        FrameRequest {
            frame: 1,
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "the second enhanced frame was refused".to_owned())?;
    if only_byte(&second)? != 128 {
        return Err("the second enhanced frame was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_linear_exact_window_maps_stored_50_to_grey_64() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 100, width 200.
    // ymin 0, ymax 255. x = 50 lies inside [0, 200].
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75.
    // `display_index` adds 0.5 in f32 and lands on table address 64.
    let preview = preview_first_frame(&mono_frame(true)?)
        .map_err(|_| "the synthetic windowed frame was refused".to_owned())?;
    assert_eq!(preview.width, 1);
    assert_eq!(preview.height, 1);
    assert_eq!(preview.frames, 1);
    assert_eq!(preview.auto, AutoSource::None);
    assert_eq!(only_byte(&preview)?, 64);
    Ok(())
}

#[test]
fn a_zero_length_window_is_auto() -> Result<(), String> {
    // F-227 AC10. Stored 50. Slope 1 and intercept 0 are present. Window
    // Center and Window Width are present with length 0, so they carry no
    // number. That is the same as an absent window. One counted modality
    // value is the constant automatic window, LINEAR_EXACT center 50, width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_first_frame(&mono_zero_window()?)
        .map_err(|_| "a zero-length window was refused".to_owned())?;
    if preview.auto != AutoSource::Image {
        return Err("a zero-length window was not the image's automatic window".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("a zero-length window was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_zero_length_rescale_keeps_the_explicit_window() -> Result<(), String> {
    // Stored 50. Rescale Slope and Rescale Intercept are present with
    // length 0, so they carry no number. Absent rescale is slope 1 and
    // intercept 0. The window is LINEAR_EXACT center 100 and width 200.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    let preview = preview_first_frame(&mono_zero_rescale()?)
        .map_err(|_| "a zero-length rescale was refused".to_owned())?;
    if preview.auto != AutoSource::None {
        return Err("a zero-length rescale was labelled auto".to_owned());
    }
    if only_byte(&preview)? != 64 {
        return Err("a zero-length rescale was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn a_zero_length_voi_function_uses_linear() -> Result<(), String> {
    // Window Center 100 and Window Width 200 are present. VOI LUT Function
    // is present with length 0, so it carries no token. An absent function
    // is LINEAR. The frame is not drawn at an automatic window.
    let preview = preview_first_frame(&mono_zero_function()?)
        .map_err(|_| "a zero-length function was refused".to_owned())?;
    if preview.auto != AutoSource::None {
        return Err("a zero-length function was labelled auto".to_owned());
    }
    if preview.window_function != "LINEAR" {
        return Err("a zero-length function was not LINEAR".to_owned());
    }
    Ok(())
}

#[test]
fn a_missing_window_is_labelled_auto() -> Result<(), String> {
    // F-227 AC10. One stored sample 50. Absent rescale is slope 1 and
    // intercept 0. Absent center and width take the automatic window of that
    // one counted value, the constant window, LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_first_frame(&mono_frame(false)?)
        .map_err(|_| "the synthetic frame with no window was refused".to_owned())?;
    assert_eq!(preview.auto, AutoSource::Image);
    assert_eq!(only_byte(&preview)?, 128);
    Ok(())
}

#[test]
fn bytes_without_dicm_are_refused() -> Result<(), String> {
    let bytes = vec![0_u8; 200];
    match preview_first_frame(&bytes) {
        Err(PreviewError::NotPart10) => Ok(()),
        Ok(_) => Err("bytes without DICM were accepted".to_owned()),
        Err(error) => Err(format!("bytes without DICM were refused as {error}")),
    }
}

#[test]
fn the_page_posts_a_frame_and_returns_its_grey_byte() -> Result<(), String> {
    let file = mono_frame(true)?;
    let request = http_post(&file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the preview response was not 200".to_owned());
    }
    if !header.contains("X-Ocelli-Width: 1") || !header.contains("X-Ocelli-Height: 1") {
        return Err("the preview response omitted the frame size".to_owned());
    }
    if !header.contains("X-Ocelli-Auto-Window: none") {
        return Err("a windowed frame was labelled auto".to_owned());
    }
    if body.len() != 1 {
        return Err("the preview response was not one byte".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the preview response was empty".to_owned())?;
    if grey != 64 {
        return Err("the preview response was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn the_page_offers_a_folder_picker() -> Result<(), String> {
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    if !page.contains("webkitdirectory") {
        return Err("the page has no folder picker".to_owned());
    }
    if !page.contains("id=\"series\"") || !page.contains(">Headers<") {
        return Err("the page is not the series viewer".to_owned());
    }
    if !page.contains(">Freeform<") || !page.contains(">Text<") {
        return Err("the page has no freeform or text tool".to_owned());
    }
    if !page.contains(">Rectangle<") || !page.contains(">Ellipse<") || !page.contains(">Remove<") {
        return Err("the page has no rectangle, ellipse, or remove control".to_owned());
    }
    if !page.contains("system-ui") || !page.contains("#141414") {
        return Err("the page is not the sans gray layout".to_owned());
    }
    if page.contains("Georgia") || page.contains("#16130f") {
        return Err("the page still uses the old type or the warm ground".to_owned());
    }
    if !page.contains("if (event.ctrlKey) {\n    event.preventDefault();") {
        return Err("a pinch does not call preventDefault".to_owned());
    }
    if !page.contains("if (Math.abs(wheelAcc) >= 40) {\n    wheelAcc = 0;\n    queueSlice(sign);") {
        return Err("a swipe still steps once per event".to_owned());
    }
    // F-215 AC16 and AC17. Marks are saved as documents, and the retired
    // sentences are gone. Lead amendment 2 keeps "It is not saved." on the
    // kinds this story does not save, so that phrase is refused only in the
    // marks note and the key image sentence, and the tool sentences are
    // pinned one by one below.
    for needle in [
        "TID 1500",
        "Key Object Selection",
        ">Save marks<",
        "Nothing is written into the opened folder",
        "type=\"importmap\"",
        "\"@ocelli/core\":\"/lib/ocelli-core/index.js\"",
        "import(\"/mark-documents.mjs\")",
    ] {
        if !page.contains(needle) {
            return Err(format!(
                "the page does not say how marks are saved: {needle}"
            ));
        }
    }
    for retired in [
        "not DICOM SR",
        "gone when you reload",
        "does not write a Key Object Selection",
        "for this session. It is not saved",
    ] {
        if page.contains(retired) {
            return Err(format!(
                "the page does not say how marks are saved: {retired}"
            ));
        }
    }
    let wrap_at = page
        .find("id=\"wrap\"")
        .ok_or_else(|| "the page has no viewport".to_owned())?;
    let note_at = page
        .find("id=\"note\"")
        .ok_or_else(|| "the page has no note field".to_owned())?;
    let inspector_at = page
        .find("id=\"inspector\"")
        .ok_or_else(|| "the page has no inspector".to_owned())?;
    if wrap_at > note_at || note_at > inspector_at {
        return Err("the note field is outside the viewport".to_owned());
    }
    if !page.contains("id=\"draft-under\"") && !page.contains("\"draft-under\"") {
        return Err("a draft stroke has no underlay".to_owned());
    }
    if !page.contains("Series shortened") {
        return Err("a shortened series is not labelled".to_owned());
    }
    // F-210. The cross asks the process for the value by file id, with no
    // body, and shows a reply only for the cell and image it was asked for.
    let call_at = page
        .find("fetch(\"/value\"")
        .ok_or_else(|| "the page does not ask for the value".to_owned())?;
    let call = page
        .get(call_at..)
        .and_then(|rest| rest.find("});").and_then(|end| rest.get(..end)))
        .ok_or_else(|| "the value call does not close".to_owned())?;
    if !call.contains("\"X-Ocelli-File\": id") || call.contains("body:") {
        return Err(format!(
            "the value call is not by id without a body: {call}"
        ));
    }
    if !page.contains(
        "\"toggle-cross\": \"Two lines meet at the pointer. The numbers are the column and row under it, counted from 1, and the modality value of that pixel, which is Hounsfield units on a CT whose file states HU, or states no unit and is not multi-energy. The local process computes the value.\"",
    ) {
        return Err("the cross sentence does not name the value".to_owned());
    }
    if !page.contains("if (point && want.generation === valueState.generation &&\n      point.index_column === valueState.column && point.index_row === valueState.row) {") {
        return Err("a value reply is shown without the stale cell check".to_owned());
    }
    for arithmetic in ["slope", "intercept", "Rescale"] {
        if page.contains(arithmetic) {
            return Err(format!("the page computes a modality value: {arithmetic}"));
        }
    }
    // F-211 AC17. The five new tools exist with their keys, every length,
    // angle, area and statistic comes from `/measure` by file id, and a
    // reply is kept only for the revision it was asked for.
    for (tool, key) in [
        ("Angle", "Q"),
        ("Cobb", "J"),
        ("Bidirectional", "B"),
        ("Arrow", "U"),
        ("Probe", "X"),
    ] {
        if !page.contains(&format!(">{tool}<"))
            || !page.contains(&format!(
                "data-key=\"{key}\"><svg class=\"i\"><use href=\"#i-"
            ))
        {
            return Err(format!("the page has no {tool} tool on {key}"));
        }
    }
    let measure_at = page
        .find("fetch(\"/measure\"")
        .ok_or_else(|| "the page does not ask for a measurement".to_owned())?;
    let measure_call = page
        .get(measure_at..)
        .and_then(|rest| rest.find("});").and_then(|end| rest.get(..end)))
        .ok_or_else(|| "the measure call does not close".to_owned())?;
    if !measure_call.contains("\"X-Ocelli-File\": id")
        || !measure_call.contains("body: pointsAttr(points)")
    {
        return Err(format!(
            "the measure call is not by id with the points: {measure_call}"
        ));
    }
    // F-215 AC17. The five kinds Save marks writes say so. Lead amendment 2:
    // the kinds it does not write keep "not saved".
    for sentence in [
        "\"tool-rect\": \"A region. The local process reports its area and the mean, SD, minimum and maximum of the modality values whose pixel centres are inside it. Save marks writes it as TID 1500.\"",
        "\"tool-ellipse\": \"A region. The local process reports its area and the mean, SD, minimum and maximum of the modality values whose pixel centres are inside it. Save marks writes it as TID 1500.\"",
        "\"tool-contour\": \"A freehand region with at least three points. The local process reports its area and statistics. An outline that crosses itself is not measured. Save marks writes it as TID 1500.\"",
        "\"tool-length\": \"The distance between the two points, from the local process. Millimetres from Pixel Spacing, or from Imager Pixel Spacing with a magnification factor, millimetres at the detector from Imager Pixel Spacing alone, and pixels when the file gives no spacing. Save marks writes it as TID 1500.\"",
        "\"tool-text\": \"A note on this image. Size and colour can be changed. Save marks writes it as TID 1500.\"",
        "\"tool-angle\": \"Drag the first arm to the vertex, then click the end of the second arm. The local process reports the angle at the vertex. It is not saved.\"",
        "\"tool-cobb\": \"Drag one line, then drag a second line. The local process reports the acute angle between them. It is not saved.\"",
        "\"tool-bidirectional\": \"Drag the long axis. A short axis is placed across its middle at half its length. The local process reports both lengths as drawn. It is not saved.\"",
        "\"tool-arrow\": \"Drag from the tail to the head, then type an optional note and press Enter. It has no number and it is not saved.\"",
        "\"tool-probe\": \"Click a pixel. The local process reports its modality value with its unit. It is not saved.\"",
        "\"key-image\": \"Marks this image in the series list. Save marks writes the key images as a Key Object Selection.\"",
    ] {
        if !page.contains(sentence) {
            return Err(format!("the page lacks the sentence {sentence}"));
        }
    }
    if page.contains("Math.hypot(dCol * facts.column") || page.contains("lengthText") {
        return Err("the page keeps its own length formula".to_owned());
    }
    // The page's outline cap is the server's, so an outline over it gets the
    // page's own sentence exactly when the route refuses it.
    let cap = format!(
        "const OUTLINE_CAP = {};",
        ocelli_folder_viewer::FREEFORM_POINT_CAP
    );
    if !page.contains(&cap) {
        return Err(format!("the page's outline cap is not {cap}"));
    }
    if !page.contains(&format!(
        "This outline has more than {} points, so it is not measured.",
        ocelli_folder_viewer::FREEFORM_POINT_CAP
    )) {
        return Err("the outline sentence does not name the server's cap".to_owned());
    }
    // The sentence is used for a freeform the route refuses with `points`
    // for its length, not only declared.
    if !page.contains(
        "if (code === \"points\" && shape === \"contour\" && points.length > OUTLINE_CAP) return { result: null, error: OUTLINE_LONG };",
    ) {
        return Err("the outline sentence is not used for a long outline".to_owned());
    }
    if !page.contains("if (!live || live.revision !== revision) return;") {
        return Err("a measure reply is applied without the revision check".to_owned());
    }
    // PS3.3 10.7.1.3: the first Pixel Spacing value is the row spacing, the
    // vertical distance between rows, and the second is the column spacing.
    // The display stretches x by the column spacing and y by the row spacing.
    if !page.contains("return { x: facts.column / least, y: facts.row / least };") {
        return Err("the display aspect does not follow row and column spacing".to_owned());
    }
    for used in [
        "const pictureW = Math.max(1, imageWidth * scale * aspectX);",
        "const pictureH = Math.max(1, imageHeight * scale * aspectY);",
        "node.dataset.aspect = String(aspect.y / aspect.x);",
    ] {
        if !page.contains(used) {
            return Err(format!(
                "the display aspect is not applied where it is used: {used}"
            ));
        }
    }
    // F-212 AC11. The corner text is formatted by the local process, so the
    // page keeps no name or date formatter of its own.
    for formatter in ["function personName", "function dicomDate"] {
        if page.contains(formatter) {
            return Err(format!("the page still formats corner text: {formatter}"));
        }
    }
    for field in [
        "info.patientNameText",
        "info.birthDate",
        "info.sex",
        "info.age",
        "info.institution",
        "info.studyDateText",
        "info.studyTime",
        "info.sliceLocation",
        "info.sliceThickness",
        "info.kvp",
        "info.exposureMas",
        "info.repetitionTime",
        "info.echoTime",
    ] {
        if !page.contains(field) {
            return Err(format!("the corners do not read {field}"));
        }
    }
    // The bar reads the calibration headers the preview reply writes.
    for read in [
        "response.headers.get(\"X-Ocelli-Calibration-Source\")",
        "headerNumber(response, \"X-Ocelli-Calibration-Column\")",
    ] {
        if !page.contains(read) {
            return Err(format!("the page does not read {read}"));
        }
    }
    // cornerstone3D 5.8.2 `ScaleOverlayTool`: the length list, the bounds
    // of 0.2 and 0.6 of the view width, and the detector words F-211's
    // length uses.
    for used in [
        "const SCALE_SIZES = [16000, 8000, 4000, 2000, 1000, 500, 250, 100, 50, 25, 10, 5, 2];",
        "SCALE_SIZES.find((mm) => mm < viewMm * 0.6 && mm > viewMm * 0.2)",
        "if (facts.calibrationSource === \"detector\") label += \" at the detector\";",
        "const perMm = show ? vp.scale * vp.aspectX / column : 0;",
    ] {
        if !page.contains(used) {
            return Err(format!("the scale bar lacks {used}"));
        }
    }
    if !page.contains(
        "\"toggle-letters\": \"The patient direction at each edge, read from Image Orientation (Patient), or from Patient Orientation on an image without Image Orientation (Patient). The letters are hidden while the picture is turned or flipped.\"",
    ) {
        return Err("the letters sentence does not name Patient Orientation".to_owned());
    }
    // F-213 AC12. The window rows come from the inspect reply, so the page
    // holds no preset token, no preset number and no modality test of its
    // own. Bare 1 to 9 choose a row and never the layout.
    // F-215 names the template, TID 1500, which is not a window, so the
    // number is looked for with every "TID 1500" taken out first.
    let windowless = page.replace("TID 1500", "TID");
    for held in ["data-preset", "-600", "1500", "\"CT\"", "soft_tissue"] {
        if windowless.contains(held) {
            return Err(format!("the page keeps a window of its own: {held}"));
        }
    }
    let keys = function_body(page, "onKey")?;
    if !keys.contains("event.key >= \"1\" && event.key <= \"9\"")
        || !keys.contains("chooseRow(vp, Number(event.key));")
        || keys.contains("setLayout(")
    {
        return Err("1 to 9 do not choose a window row".to_owned());
    }
    if !keys.contains("y: \"wlregion\"") || !page.contains("data-key=\"Y\"") {
        return Err("the region window has no key".to_owned());
    }
    for read in [
        "Array.isArray(info.presets) ? info.presets : []",
        "Array.isArray(info.windows) ? info.windows : []",
        "Array.isArray(info.voiLuts) ? info.voiLuts : []",
    ] {
        if !page.contains(read) {
            return Err(format!("the window list does not read {read}"));
        }
    }
    // Amendment 2. The file window index is in every viewport's cache key,
    // is sent as its header, and travels with a linked window.
    if !function_body(page, "cacheKey")?.contains("\"file\" + fileKeyIndex(image, view)")
        || !function_body(page, "fileKeyIndex")?
            .contains("const index = effectiveIndex(image, view);")
    {
        return Err("the cache key leaves out the file window index".to_owned());
    }
    // Review pass 1, D2. An index is sent only when the frame's own list
    // has confirmed it.
    if !function_body(page, "effectiveIndex")?.contains("if (!windowsKnown(image)) return null;") {
        return Err("an index is sent before the frame's list is known".to_owned());
    }
    if !page.contains(
        "const index = effectiveIndex(image, window);\n    if (index != null && index > 0) headers[\"X-Ocelli-Window-Index\"] = String(index);",
    ) {
        return Err("the file window index is not sent".to_owned());
    }
    if !function_body(page, "copyWindow")?.contains("vp.windowIndex = index;") {
        return Err("a linked window copy drops the file window index".to_owned());
    }
    // AC17. The region window imports `windowRegion` from the built library
    // through the import map, and says so when the build is missing.
    if !page.contains(
        "<script type=\"importmap\">{\"imports\":{\"@ocelli/core\":\"/lib/ocelli-core/index.js\",\"dcmjs\":\"/lib/dcmjs.js\"}}</script>",
    ) {
        return Err("the page has no import map for the library".to_owned());
    }
    if !page.contains("import(\"@ocelli/core\")")
        || !page.contains("core.windowRegion([extent.min, extent.max])")
    {
        return Err("the region window does not use windowRegion from the library".to_owned());
    }
    if !page.contains("\"The region window needs the library build. Run npm run build -w @ocelli/core in the repository, then start the viewer again.\"") {
        return Err("the page lacks the library sentence".to_owned());
    }
    if !page.contains("\"The region window needs dcmjs, which the library imports. Run npm ci in the repository, then start the viewer again.\"") {
        return Err("the page does not name dcmjs when it is what is missing".to_owned());
    }
    // The region and the per-frame windows are asked for by file id with
    // no body, so a frame change never posts the file again.
    for (route, name) in [
        ("fetch(\"/region\"", "region"),
        ("fetch(\"/inspect\"", "inspect"),
    ] {
        let at = page
            .find(route)
            .ok_or_else(|| format!("the page does not ask for the {name}"))?;
        let call = page
            .get(at..)
            .and_then(|rest| rest.find("});").and_then(|end| rest.get(..end)))
            .ok_or_else(|| format!("the {name} call does not close"))?;
        if !call.contains("\"X-Ocelli-File\": id") || call.contains("body") {
            return Err(format!(
                "the {name} call is not by id without a body: {call}"
            ));
        }
    }
    Ok(())
}

#[test]
fn the_page_has_viewport_cells() -> Result<(), String> {
    // F-208. The page text is read, not executed. The browser observations
    // B1 to B7 of the design are a separate record.
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    // AC1. A picker from 1 by 1 to 3 by 3, not a two-state toggle.
    for needle in [
        "id=\"layout\"",
        "id=\"layout-picker\"",
        "data-columns=\"3\"",
        "data-rows=\"3\"",
    ] {
        if !page.contains(needle) {
            return Err(format!("the page has no layout picker: {needle}"));
        }
    }
    // AC2. Every cell is cloned from one template.
    if !page.contains("id=\"cell-template\"") || !page.contains("class=\"cell\"") {
        return Err("the cells do not come from one template".to_owned());
    }
    // AC3. View state lives in one record per viewport. The script's top
    // level starts at column 0, so a global is a line that starts with it.
    for global in [
        "let scale =",
        "let panX =",
        "let rotation =",
        "let gridMode =",
        "let overrideWindow =",
        "let activePreset =",
        "let activeIndex =",
        "let viewIndex =",
    ] {
        if page.contains(&format!("\n{global}")) {
            return Err(format!("the page keeps a global view state: {global}"));
        }
    }
    if !page.contains("function blankViewport(") {
        return Err("the page has no viewport record".to_owned());
    }
    // AC4. A thumbnail drags onto a cell under the page's own type.
    for needle in [
        "application/x-ocelli-series",
        "draggable = true",
        "\"dragover\"",
        "\"drop\"",
    ] {
        if !page.contains(needle) {
            return Err(format!("a series cannot be dragged onto a cell: {needle}"));
        }
    }
    // AC5. The four-image view is kept as a tile mode with its sentences.
    for needle in [
        "id=\"tile-mode\"",
        "Four neighbouring images from the open series, each one decoded by the same preview. Double-click one to open it alone.",
        "Drawing works in the single view.",
        "Four images are not shown for a file larger than 32 MiB.",
    ] {
        if !page.contains(needle) {
            return Err(format!("the tile mode is not kept: {needle}"));
        }
    }
    // AC7. An empty cell says what to do.
    if !page.contains("Drag a series here.") {
        return Err("an empty cell does not say what to do".to_owned());
    }
    // AC13. `G` opens the layout picker and `Shift G` toggles tile mode,
    // through the two branch texts below and the two `data-key` attributes.
    // Nothing here is about any other key, so F-213 may bind the digits.
    for needle in [
        "data-key=\"G\"",
        "data-key=\"Shift G\"",
        "} else if (event.shiftKey && event.key.toLowerCase() === \"g\") {\n    toggleTileMode();",
        "} else if (event.key.toLowerCase() === \"g\") {\n    openLayoutPicker();",
    ] {
        if !page.contains(needle) {
            return Err(format!("the layout keys are not bound: {needle}"));
        }
    }
    // In the text of `onKey`, including its `clicks` and `map` literals,
    // `toggleTileMode(` and `openLayoutPicker(` each appear exactly once,
    // and each other token in this list appears zero times. That is the
    // whole check. A route that uses none of these tokens is not seen.
    let key_at = page
        .find("function onKey(event) {")
        .ok_or_else(|| "the page has no key handler".to_owned())?;
    let handler = page
        .get(key_at..)
        .and_then(|rest| rest.find("\n}\n").and_then(|end| rest.get(..end)))
        .ok_or_else(|| "the key handler does not close".to_owned())?;
    for (token, allowed) in [
        ("toggleTileMode(", 1),
        ("openLayoutPicker(", 1),
        ("setTileMode(", 0),
        ("setLayout(", 0),
        ("tileButton", 0),
        ("tile-mode", 0),
        ("layout\"", 0),
        ("layout'", 0),
        ("layout`", 0),
        ("#layout", 0),
        ("layoutButton.click", 0),
        ("layoutPicker.hidden = false", 0),
        ("data-columns", 0),
        ("dataset.columns", 0),
    ] {
        let found = handler.matches(token).count();
        if found != allowed {
            return Err(format!(
                "onKey reaches the layout or the tile mode outside G and Shift G: {token} appears {found} times, {allowed} allowed"
            ));
        }
    }
    // In the whole page, `setLayout(`, `openLayoutPicker(` and
    // `toggleTileMode` each appear exactly 3 times, which is their
    // definition and their two uses today.
    for (token, expected) in [
        ("setLayout(", 3),
        ("openLayoutPicker(", 3),
        ("toggleTileMode", 3),
    ] {
        let found = page.matches(token).count();
        if found != expected {
            return Err(format!(
                "{token} appears {found} times in the page, {expected} expected"
            ));
        }
    }
    // AC4. A cell accepts a drag only when it carries the series type, on
    // both the dragover and the drop.
    if !page.contains("return types.includes(SERIES_TYPE);")
        || page.matches("if (!carriesSeries(event)) return;").count() != 2
    {
        return Err("a cell accepts a drag without the series type".to_owned());
    }
    Ok(())
}

#[test]
fn every_page_carries_a_well_formed_icon() -> Result<(), String> {
    for path in ["/", "/more", "/use"] {
        let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = one_exchange(request.as_bytes())?;
        let (_, body) = split_http(&response)?;
        let page = std::str::from_utf8(body).map_err(|_| format!("{path} is not utf-8"))?;
        let start = "<link rel=\"icon\" type=\"image/svg+xml\" href=\"data:image/svg+xml,";
        let at = page
            .find(start)
            .ok_or_else(|| format!("{path} has no icon"))?;
        let rest = page
            .get(at + start.len()..)
            .ok_or_else(|| format!("{path} icon is cut short"))?;
        let end = rest
            .find('"')
            .ok_or_else(|| format!("{path} icon href never closes"))?;
        let href = rest.get(..end).unwrap_or_default();
        // A raw double quote inside the data URI ends the attribute early and
        // the browser reads a truncated href, so the whole mark must arrive
        // inside the attribute, closing tag included.
        if !href.ends_with("%3C/svg%3E") || rest.get(end..end + 2) != Some("\">") {
            return Err(format!("{path} icon href is truncated by a raw quote"));
        }
    }
    Ok(())
}

#[test]
fn more_and_use_are_static_pages() -> Result<(), String> {
    for path in ["/more", "/use"] {
        let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = one_exchange(request.as_bytes())?;
        let (header, body) = split_http(&response)?;
        if !header.contains("200") {
            return Err(format!("{path} was not 200"));
        }
        let page = std::str::from_utf8(body).map_err(|_| format!("{path} is not utf-8"))?;
        if page.contains("<canvas") {
            return Err(format!("{path} contains a viewer"));
        }
        // F-215 AC17. The key image flag now writes a Key Object Selection.
        if page.contains("It does not write a Key Object Selection and it is not saved") {
            return Err(format!("{path} still says the key image is not saved"));
        }
    }
    Ok(())
}

/// The quoted UIDs between `start` and the next `]` of `text`.
fn quoted_uids(text: &str, start: &str) -> Result<Vec<String>, String> {
    let at = text.find(start).ok_or_else(|| format!("no {start}"))?;
    let rest = text.get(at + start.len()..).unwrap_or_default();
    let end = rest
        .find(']')
        .ok_or_else(|| format!("{start} does not close"))?;
    Ok(rest
        .get(..end)
        .unwrap_or_default()
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect())
}

#[test]
fn the_page_and_the_module_read_the_same_document_classes() -> Result<(), String> {
    // F-215. The page leaves these classes out of the series list before
    // the module loads, so its own copy must be the module's list: Enhanced
    // SR, Comprehensive SR, Comprehensive 3D SR, Key Object Selection, PS3.4
    // B.5.
    let page = served_page("/")?;
    let module = include_str!("../src/mark-documents.mjs");
    let want = [
        "1.2.840.10008.5.1.4.1.1.88.22",
        "1.2.840.10008.5.1.4.1.1.88.33",
        "1.2.840.10008.5.1.4.1.1.88.34",
        "1.2.840.10008.5.1.4.1.1.88.59",
    ];
    for (name, text, start) in [
        ("the page", page.as_str(), "const DOCUMENT_CLASSES = ["),
        (
            "the module",
            module,
            "export const DOCUMENT_CLASSES = Object.freeze([",
        ),
    ] {
        let found = quoted_uids(text, start)?;
        if found != want {
            return Err(format!("{name} reads {found:?} as documents"));
        }
    }
    if !page.contains("const documents = found.filter((row) => DOCUMENT_CLASSES.includes((row.info || {}).sopClassUid));")
        || !page.contains("groupSeries(found.filter((row) => !documents.includes(row)));")
    {
        return Err("the page does not take documents out of the series list".to_owned());
    }
    Ok(())
}

#[test]
fn the_mark_documents_module_is_served() -> Result<(), String> {
    // F-215 AC16. The page imports `/mark-documents.mjs`, which is served as
    // JavaScript byte for byte from the tracked file and names its one bare
    // import, `@ocelli/core`, which the page's import map resolves. Its stem
    // is not `marks`, so it cannot be mistaken for `packages/core/src/marks.ts`.
    let response = one_exchange(b"GET /mark-documents.mjs HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200")
        || header_line(header, "Content-Type") != Some("text/javascript; charset=utf-8")
    {
        return Err(format!("the module answered {header}"));
    }
    let tracked = include_str!("../src/mark-documents.mjs");
    if body != tracked.as_bytes() {
        return Err("the served module is not the tracked file".to_owned());
    }
    let imports: Vec<&str> = tracked
        .lines()
        .filter(|line| line.starts_with("import "))
        .collect();
    if imports.len() != 1
        || imports
            .first()
            .is_none_or(|line| !line.ends_with("from \"@ocelli/core\";"))
    {
        return Err(format!("the module imports {imports:?}"));
    }
    Ok(())
}

fn served_page(path: &str) -> Result<String, String> {
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    let response = one_exchange(request.as_bytes())?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err(format!("{path} was not 200"));
    }
    std::str::from_utf8(body)
        .map(str::to_owned)
        .map_err(|_| format!("{path} is not utf-8"))
}

fn require(page: &str, needles: &[&str], what: &str) -> Result<(), String> {
    for needle in needles {
        if !page.contains(needle) {
            return Err(format!("{what}: the page lacks {needle}"));
        }
    }
    Ok(())
}

#[test]
fn the_page_offers_mouse_profiles() -> Result<(), String> {
    let page = served_page("/")?;
    // AC1. The three profiles of the F-214 plan, by id and by name.
    require(
        &page,
        &[
            "<select id=\"mouse-profile\"",
            "<option value=\"ocelli\">Ocelli</option>",
            "<option value=\"scroll-left\">Scroll on left</option>",
            "<option value=\"window-left\">Window on left</option>",
            "id=\"mouse-keys\"",
        ],
        "AC1",
    )?;
    // AC2. The plan's table. Left is the chosen tool in every profile, and
    // a profile names where left starts, middle and right.
    require(
        &page,
        &[
            "const PROFILES = Object.freeze({\n  \"ocelli\": Object.freeze({ name: \"Ocelli\", start: \"scroll\", middle: \"zoom\", right: \"pan\" }),\n  \"scroll-left\": Object.freeze({ name: \"Scroll on left\", start: \"scroll\", middle: \"wl\", right: \"zoom\" }),\n  \"window-left\": Object.freeze({ name: \"Window on left\", start: \"wl\", middle: \"pan\", right: \"zoom\" })\n});",
            // Ctrl with left pans before any profile is read, so a trackpad
            // with one button keeps pan.
            "function pointerMode(event) {\n  if (event.button === 0 && event.ctrlKey) return \"pan\";\n  const profile = PROFILES[activeProfile];\n  if (event.button === 1) return profile.middle;\n  if (event.button === 2) return profile.right;\n  return tool;\n}",
            "setTool(PROFILES[id].start);",
            "let tool = PROFILES[activeProfile].start;",
        ],
        "AC2",
    )?;
    // AC4. A stored value is used only when it is one of the allowed ids, so
    // `radiant` or `constructor` gives `ocelli` and is never used as a key.
    require(
        &page,
        &[
            "let activeProfile = readPreference(MOUSE_KEY, PROFILE_IDS, \"ocelli\");",
            "return allowed.includes(stored) ? stored : fallback;",
            "const MOUSE_KEY = \"ocelli.mouse\";",
            "const FPS_KEY = \"ocelli.fps\";",
            "fpsInput.value = readPreference(FPS_KEY, FPS_VALUES, \"10\");",
            "This browser did not keep the choice. It applies until you reload.",
        ],
        "AC4",
    )?;
    // F-214 amendment 5. A window drag on any button commits by the drag's
    // mode, which clears the preset and the file window index as a left
    // button drag does.
    require(
        &page,
        &["if (mode === \"wl\" && vp.draftWindow) {\n    vp.overrideWindow = {"],
        "amendment 5",
    )?;
    // Review pass 1, D1. Page keys do not run on the focused control, and a
    // pointer choice gives the focus back. D4. The active tool button is
    // synced from `tool` at start, so the markup marks no tool active.
    require(
        &page,
        &[
            "if (node.tagName === \"TEXTAREA\" || node.tagName === \"SELECT\") return true;",
            "  if (!profileKeyed) profileSelect.blur();",
            // Review pass 2, S1. A key on the control marks the next choice
            // as a keyboard one, and a press on it marks a pointer one.
            "profileSelect.addEventListener(\"keydown\", () => { profileKeyed = true; });",
            "profileSelect.addEventListener(\"pointerdown\", () => { profileKeyed = false; });",
            "setLayout(1, 1);\n// The kept profile may start left on W/L, so the active tool button comes\n// from `tool` and not from the markup.\nsyncToolButtons();",
            "<button type=\"button\" class=\"icon\" id=\"tool-scroll\"",
        ],
        "D1 and D4",
    )?;
    if page.contains(
        "if (event.button === 2 || (event.button === 0 && event.ctrlKey)) return \"pan\";",
    ) {
        return Err("pointerMode still has the one fixed binding".to_owned());
    }
    Ok(())
}

#[test]
fn every_storage_access_is_guarded() -> Result<(), String> {
    // AC3. A private window or blocked site data can make the accessor
    // throw, so each line that touches storage is inside `try`.
    let page = served_page("/")?;
    let lines: Vec<&str> = page.lines().collect();
    let mut seen = 0_usize;
    for (at, line) in lines.iter().enumerate() {
        if !line.contains("localStorage.") {
            continue;
        }
        seen += 1;
        let above = at.checked_sub(1).and_then(|before| lines.get(before));
        let guarded = line.contains("try {")
            || above.is_some_and(|before| before.trim_end().ends_with("try {"));
        if !guarded {
            return Err(format!(
                "page line {} reads storage outside try: {}",
                at + 1,
                line.trim()
            ));
        }
    }
    // One read and one write, both in the two helpers.
    if seen != 2 {
        return Err(format!(
            "the page touches storage on {seen} lines, not the two helpers"
        ));
    }
    Ok(())
}

#[test]
fn play_uses_the_file_frame_time() -> Result<(), String> {
    let page = served_page("/")?;
    // AC10 and F-214 amendment 1. PS3.3 C.7.6.5.1.1 gives Frame Time as the
    // interval between frames of one multi-frame object, so the file's
    // milliseconds apply only between two frames of the same file, and are
    // read before the slider. They are not turned into a whole slider rate.
    require(
        &page,
        &[
            "function fileInterval(from, to) {\n  if (useSlider || !sameFileNext(from, to)) return null;\n  const info = from.info;\n  if (!info || info.cine !== \"frame-time\") return null;\n  const ms = Number(info.frameTimeMs);\n  return Number.isFinite(ms) && ms > 0 ? ms : null;\n}",
            "function stepInterval(from, to) {\n  const fromFile = fileInterval(from, to);\n  if (fromFile !== null) return fromFile;\n  return 1000 / (Number(fpsInput.value) || 10);\n}",
            "const interval = stepInterval(currentImage(vp), upcomingImage(vp));",
            "const remain = interval - spent;",
            ">Use the slider</button>",
            "This file gives a Frame Time Vector, which this page does not play. The rate is the slider.",
            "This file gives a frame rate and no Frame Time. The rate is the slider.",
            "This file gives no Frame Time. The rate is the slider.",
            // Review pass 1, D3. A refusal that is not about the timing
            // makes no claim about Frame Time.
            "if (cine === \"unreadable\") return \"This page could not read the frames of this file. The rate is the slider.\";",
            // Review pass 1, S2. A file listed as one image, and the last
            // frame before another file, have no frame of the same file
            // next, so the line and the slider follow the next step.
            "function sameFileNext(from, to) {\n  return Boolean(from && to && from !== to && from.file === to.file);\n}",
            "const offered = Boolean(sameFileNext(image, next) && image.info && image.info.cine === \"frame-time\" && Number(image.info.frameTimeMs) > 0);",
            "if (!sameFileNext(image, next)) return \"Frame Time applies between frames of one file. The rate is the slider.\";",
            // Review pass 2, D2. A Cine refusal can be any timing attribute,
            // so the line names the cine timing and not Frame Time.
            "if (cine === \"unusable\") return \"The cine timing in this file is not usable. The rate is the slider.\";",
        ],
        "AC10",
    )?;
    // The per-second figure is display text, 1000 / 40 = 25.0 shown as 25.
    require(
        &page,
        &["(1000 / ms).toFixed(1).replace(/\\.0$/, \"\")"],
        "the per-second label",
    )?;
    if page.contains("const fps = Number(fpsInput.value) || 10;\n      const remain = 1000 / fps") {
        return Err("Play still reads the slider alone".to_owned());
    }
    Ok(())
}

#[test]
fn more_says_the_page_keeps_four_choices() -> Result<(), String> {
    // AC11 keeps four choices. The operator now defines status by the viewer,
    // so those persisted settings make Preferences partial.
    let more = served_page("/more")?;
    require(
        &more,
        &[
            "The image page keeps four choices in this browser, the mouse profile, the play rate and whether each of the two side columns is hidden. There is still no preferences module in the crates or packages.",
            "<li><a href=\"#preferences\">Preferences</a><span class=\"mark\">partial</span></li>",
        ],
        "AC11",
    )?;
    let use_book = served_page("/use")?;
    require(
        &use_book,
        &["<code>cine_timing(&amp;metadata)</code>"],
        "the Rust parse chapter",
    )
}

#[test]
fn a_post_without_a_length_is_refused() -> Result<(), String> {
    let response = one_exchange(b"POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: length") {
        return Err("a post without a length was not refused".to_owned());
    }
    Ok(())
}

#[test]
fn a_jpegxl_lossless_frame_draws() -> Result<(), String> {
    // PS3.5 A.4.12: one codestream in one fragment of an OB Pixel Data whose
    // File Meta names JPEG XL Lossless. The codestream is the tracked 12-bit
    // lossless fixture beside the adapter's own tests, 87 bytes, so the
    // fragment carries the one trailing NULL pad of an odd item value. Its
    // stored values are measured from the fixture's reference ramp
    // jpegxl_reference_mono12_u16le.raw: 64 by 96 samples, minimum 0 first and
    // maximum 4095 last. No window is declared, so the automatic window
    // draws the minimum, stored 0 and left out, at grey 0 and the maximum,
    // above the 99th percentile, at 255.
    let file = jpegxl_frame(
        "jpegxl_lossless_12.jxl",
        "1.2.840.10008.1.2.4.110",
        1,
        "MONOCHROME2",
        12,
    )?;
    let preview = preview_first_frame(&file)
        .map_err(|error| format!("the JPEG XL lossless frame was refused: {error}"))?;
    assert_eq!(preview.width, 96);
    assert_eq!(preview.height, 64);
    assert_eq!(preview.grey.len(), 64 * 96);
    assert_eq!(preview.grey.first().copied(), Some(0));
    assert_eq!(preview.grey.last().copied(), Some(255));
    Ok(())
}

#[test]
fn a_jpegxl_colour_frame_draws_rgb_and_mismatched_sample_shapes_refuse() -> Result<(), String> {
    // The codec fixture's original RGB bytes are the independent source
    // encoded by generate_jpegxl.py. A decoder reporting RGB must not enter
    // a second colour conversion. PS3.3 C.7.6.3.1.2.
    let file = jpegxl_frame(
        "jpegxl_lossless_rgb8.jxl",
        "1.2.840.10008.1.2.4.110",
        3,
        "RGB",
        8,
    )?;
    let preview = preview_first_frame(&file).map_err(|error| error.to_string())?;
    assert_eq!(preview.channels, 3);
    assert_eq!(
        preview.grey.as_slice(),
        include_bytes!("../../../crates/ocelli-codec/tests/fixtures/jpegxl_reference_rgb8.raw")
            .as_slice()
    );
    for (samples, space) in [(3, "MONOCHROME2"), (1, "RGB")] {
        let file = jpegxl_frame(
            "jpegxl_lossless_rgb8.jxl",
            "1.2.840.10008.1.2.4.110",
            samples,
            space,
            8,
        )?;
        match preview_first_frame(&file) {
            Err(PreviewError::ColourUnavailable(_)) => {}
            other => {
                return Err(format!(
                    "mismatched {samples}-sample {space} shape: {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn every_registration_in_codecs_decodes_a_monochrome_frame() -> Result<(), String> {
    // One file per registration function in codecs(), so dropping any one
    // register_* call leaves that call's UIDs known but unavailable and its
    // file refused with PreviewError::Decode, which turns this test red.
    // Expected values are hand-computed or read from the fixture reference,
    // none from a previous run of this preview.
    //
    // RLE Lossless, PS3.5 G.2 and G.3.1: a 64 byte header naming one segment
    // at offset 64, then one literal run of count 3 carrying the samples
    // 10, 20, 30 and 40, padded to even length with one zero byte. The
    // automatic window's nearest ranks of four values are the first and the
    // fourth, 10 and 40, so they map to grey 0, 85, 170 and 255.
    let mut rle = Vec::new();
    rle.extend_from_slice(&1_u32.to_le_bytes());
    rle.extend_from_slice(&64_u32.to_le_bytes());
    rle.resize(64, 0);
    rle.extend_from_slice(&[3, 10, 20, 30, 40, 0]);
    let preview = preview_first_frame(&encapsulated_frame(
        &rle,
        "1.2.840.10008.1.2.5",
        1,
        4,
        1,
        "MONOCHROME2",
        8,
    )?)
    .map_err(|error| format!("the RLE Lossless frame was refused: {error}"))?;
    assert_eq!(preview.grey, vec![0, 85, 170, 255]);

    // JPEG lossless process 14, UID .57. The codec suite hand-computes this
    // fixture's four samples as 0, 2047, 2048 and 4095 from PS3.5 Annex F,
    // so they come from the standard and not from the decoder. The automatic
    // window leaves the stored 0 out and runs from 2047 to 4095, so the first
    // is grey 0 and the last grey 255.
    let preview = preview_first_frame(&encapsulated_frame(
        include_bytes!(
            "../../../crates/ocelli-codec/tests/fixtures/jpeg_lossless_process14_12bit.jpg"
        ),
        "1.2.840.10008.1.2.4.57",
        1,
        4,
        1,
        "MONOCHROME2",
        12,
    )?)
    .map_err(|error| format!("the JPEG lossless frame was refused: {error}"))?;
    assert_eq!(preview.grey.first(), Some(&0));
    assert_eq!(preview.grey.last(), Some(&255));

    // JPEG 2000 lossless, UID .90. The codec suite states that this fixture
    // repeats the boundary values 0, 2047, 2048 and 4095 across a 64 by 64
    // frame, so its grey minimum is 0 and its grey maximum is 255.
    let preview = preview_first_frame(&encapsulated_frame(
        include_bytes!("../../../crates/ocelli-codec/tests/fixtures/jpeg2000_lossless_u12.j2k"),
        "1.2.840.10008.1.2.4.90",
        64,
        64,
        1,
        "MONOCHROME2",
        12,
    )?)
    .map_err(|error| format!("the JPEG 2000 frame was refused: {error}"))?;
    assert_eq!(preview.grey.len(), 64 * 64);
    assert_eq!(preview.grey.iter().min(), Some(&0));
    assert_eq!(preview.grey.iter().max(), Some(&255));

    // JPEG-LS lossless and HTJ2K lossless, UIDs .80 and .201. The codec suite
    // reproduces the uncompressed corpus ramp from both fixtures byte for
    // byte, a 64 by 96 unsigned 16-bit ramp whose first sample is 0 and whose
    // last is 65535. The automatic window leaves the stored 0 out and draws it
    // at grey 0, and 65535 is above the 99th percentile, grey 255.
    for (uid, codestream, name) in [
        (
            "1.2.840.10008.1.2.4.80",
            &include_bytes!(
                "../../../crates/ocelli-codec/tests/fixtures/jpegls_corpus_lossless.jls"
            )[..],
            "JPEG-LS",
        ),
        (
            "1.2.840.10008.1.2.4.201",
            &include_bytes!(
                "../../../crates/ocelli-codec/tests/fixtures/htj2k_corpus_lossless.j2c"
            )[..],
            "HTJ2K",
        ),
    ] {
        let preview = preview_first_frame(&encapsulated_frame(
            codestream,
            uid,
            64,
            96,
            1,
            "MONOCHROME2",
            16,
        )?)
        .map_err(|error| format!("the {name} frame was refused: {error}"))?;
        assert_eq!(preview.width, 96);
        assert_eq!(preview.height, 64);
        assert_eq!(preview.grey.len(), 64 * 96);
        assert_eq!(preview.grey.first(), Some(&0));
        assert_eq!(preview.grey.last(), Some(&255));
    }

    // JPEG XL Lossless, UID .110, the same file `a_jpegxl_lossless_frame_draws`
    // previews. It is listed here so all six registrations have a file.
    let preview = preview_first_frame(&jpegxl_frame(
        "jpegxl_lossless_12.jxl",
        "1.2.840.10008.1.2.4.110",
        1,
        "MONOCHROME2",
        12,
    )?)
    .map_err(|error| format!("the JPEG XL frame was refused: {error}"))?;
    assert_eq!(preview.grey.first(), Some(&0));
    assert_eq!(preview.grey.last(), Some(&255));
    Ok(())
}

fn only_byte(preview: &Preview) -> Result<u8, String> {
    if preview.grey.len() != 1 {
        return Err("the frame is not one pixel".to_owned());
    }
    preview
        .grey
        .first()
        .copied()
        .ok_or_else(|| "the frame is empty".to_owned())
}

fn one_exchange(request: &[u8]) -> Result<Vec<u8>, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let server = std::thread::spawn(move || -> Result<(), String> {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        handle_connection(stream).map_err(|error| error.to_string())
    });
    let response = exchange(addr, request)?;
    match server.join() {
        Ok(Ok(())) => Ok(response),
        Ok(Err(message)) => Err(message),
        Err(_) => Err("the preview server stopped".to_owned()),
    }
}

fn exchange(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(request)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    Ok(response)
}

fn split_http(response: &[u8]) -> Result<(&str, &[u8]), String> {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
        .ok_or_else(|| "the preview response has no header".to_owned())?;
    let header = response
        .get(..end)
        .ok_or_else(|| "the preview response header is short".to_owned())?;
    let body = response
        .get(end..)
        .ok_or_else(|| "the preview response body is short".to_owned())?;
    let text = std::str::from_utf8(header)
        .map_err(|_| "the preview response header is not text".to_owned())?;
    Ok((text, body))
}

fn http_post(body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
        len = body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn enhanced_frames() -> Result<Vec<u8>, String> {
    let rescale = item(&concat_bytes(&[
        &short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?,
        &short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
    ]))?;
    let shared = item(&long_le((0x0028, 0x9145), *b"SQ", &rescale)?)?;
    let frame_window = |center: &str, width: &str| -> Result<Vec<u8>, String> {
        let body = concat_bytes(&[
            &short_le((0x0028, 0x1050), *b"DS", &text_pad(center))?,
            &short_le((0x0028, 0x1051), *b"DS", &text_pad(width))?,
            &short_le((0x0028, 0x1056), *b"CS", &text_pad("LINEAR_EXACT"))?,
        ]);
        item(&long_le((0x0028, 0x9132), *b"SQ", &item(&body)?)?)
    };
    let per_frame = concat_bytes(&[&frame_window("100", "200")?, &frame_window("50", "1")?]);
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&50_u16.to_le_bytes());
    pixels.extend_from_slice(&50_u16.to_le_bytes());
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0008), *b"IS", &text_pad("2"))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(long_le((0x5200, 0x9229), *b"SQ", &shared)?);
    data.extend(long_le((0x5200, 0x9230), *b"SQ", &per_frame)?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn item(content: &[u8]) -> Result<Vec<u8>, String> {
    let length = u32::try_from(content.len()).map_err(|_| "item length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(content);
    Ok(bytes)
}

fn concat_bytes(parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes
}

fn mono_frame(window: bool) -> Result<Vec<u8>, String> {
    mono_with(window, &[])
}

fn mono_zero_window() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &[])?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &[])?);
    data.extend(rescale_one()?);
    data.extend(pixels_50()?);
    part10(data)
}

fn mono_zero_rescale() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(window_exact()?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &[])?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &[])?);
    data.extend(pixels_50()?);
    part10(data)
}

fn mono_zero_function() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(rescale_one()?);
    data.extend(short_le((0x0028, 0x1056), *b"CS", &[])?);
    data.extend(pixels_50()?);
    part10(data)
}

fn image_module() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    Ok(data)
}

fn rescale_one() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    Ok(data)
}

fn window_exact() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("LINEAR_EXACT"),
    )?);
    Ok(data)
}

fn pixels_50() -> Result<Vec<u8>, String> {
    long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())
}

fn part10(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn part10_as(data: Vec<u8>, transfer_syntax: &str) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(transfer_syntax)?);
    bytes.extend(data);
    Ok(bytes)
}

/// One encapsulated frame around a tracked JPEG XL fixture.
///
/// The wrapper has the measured shape of the corpus JPEG XL rows: Pixel Data
/// OB of undefined length, a Basic Offset Table item carrying its one offset,
/// one Fragment Item holding the codestream padded to even length, and the
/// Sequence Delimitation Item.
fn jpegxl_frame(
    fixture: &str,
    uid: &str,
    samples: u16,
    photometric: &str,
    bits_stored: u16,
) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/ocelli-codec/tests/fixtures")
        .join(fixture);
    let codestream =
        std::fs::read(&path).map_err(|_| format!("{} is not readable", path.display()))?;
    encapsulated_frame(&codestream, uid, 64, 96, samples, photometric, bits_stored)
}

/// One encapsulated frame around a codestream, in the wrapper shape described
/// on `jpegxl_frame`.
#[test]
fn an_empty_table_rle_frame_draws() -> Result<(), String> {
    // F-244 AC7. The hand RLE frame of the test above, PS3.5 G.2 and G.3.1:
    // a 64 byte header naming one segment at offset 64, one literal run of
    // count 3 carrying 10, 20, 30 and 40, and one pad byte, 70 bytes in all.
    // PS3.5 A.4 lets the Basic Offset Table Item be empty, length zero, and
    // decoders "need to accept" it. Full range over 10 to 40 maps the
    // samples to grey 0, 85, 170 and 255, PS3.3 C.11.2.1.3.2.
    let mut rle = Vec::new();
    rle.extend_from_slice(&1_u32.to_le_bytes());
    rle.extend_from_slice(&64_u32.to_le_bytes());
    rle.resize(64, 0);
    rle.extend_from_slice(&[3, 10, 20, 30, 40, 0]);
    let file = with_empty_basic_offset_table(encapsulated_frame(
        &rle,
        "1.2.840.10008.1.2.5",
        1,
        4,
        1,
        "MONOCHROME2",
        8,
    )?)?;
    let preview = preview_first_frame(&file)
        .map_err(|error| format!("the empty-table RLE frame was refused: {error}"))?;
    assert_eq!(preview.grey, vec![0, 85, 170, 255]);
    Ok(())
}

/// Rewrite the one-value Basic Offset Table `[0]` that `encapsulated_frame`
/// writes as the empty table PS3.5 A.4 Table A.4-1 shows, Item Tag
/// `FE FF 00 E0` and length `00 00 00 00` with no value. Pixel Data has
/// undefined length, so no enclosing length changes.
fn with_empty_basic_offset_table(file: Vec<u8>) -> Result<Vec<u8>, String> {
    let populated: [u8; 24] = [
        0xE0, 0x7F, 0x10, 0x00, b'O', b'B', 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0x00, 0xE0,
        4, 0, 0, 0, 0, 0, 0, 0,
    ];
    let at = file
        .windows(populated.len())
        .position(|window| window == populated)
        .ok_or("no one-value Basic Offset Table to empty")?;
    let mut out = Vec::with_capacity(file.len());
    out.extend_from_slice(file.get(..at + 16).ok_or("short file")?);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(file.get(at + populated.len()..).ok_or("short file")?);
    Ok(out)
}

fn encapsulated_frame(
    codestream: &[u8],
    uid: &str,
    rows: u16,
    columns: u16,
    samples: u16,
    photometric: &str,
    bits_stored: u16,
) -> Result<Vec<u8>, String> {
    let bits_allocated: u16 = if bits_stored > 8 { 16 } else { 8 };
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &samples.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0004), *b"CS", &text_pad(photometric))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &rows.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &columns.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0100),
        *b"US",
        &bits_allocated.to_le_bytes(),
    )?);
    data.extend(short_le(
        (0x0028, 0x0101),
        *b"US",
        &bits_stored.to_le_bytes(),
    )?);
    data.extend(short_le(
        (0x0028, 0x0102),
        *b"US",
        &(bits_stored - 1).to_le_bytes(),
    )?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    let mut fragment = codestream.to_vec();
    if fragment.len() % 2 == 1 {
        fragment.push(0);
    }
    let mut pixels = item(&0_u32.to_le_bytes())?;
    pixels.extend_from_slice(&item(&fragment)?);
    pixels.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    pixels.extend_from_slice(&0xE0DD_u16.to_le_bytes());
    pixels.extend_from_slice(&0_u32.to_le_bytes());
    let mut element = Vec::new();
    element.extend_from_slice(&0x7FE0_u16.to_le_bytes());
    element.extend_from_slice(&0x0010_u16.to_le_bytes());
    element.extend_from_slice(b"OB");
    element.extend_from_slice(&[0, 0]);
    element.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes());
    element.extend_from_slice(&pixels);
    data.extend(element);
    part10_as(data, uid)
}

fn mono_unusable_window() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100\\50"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("NOT_A_FUNCTION"),
    )?);
    data.extend(long_le((0x0028, 0x3010), *b"SQ", &[])?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn mono_with(window: bool, sequences: &[(u16, u16)]) -> Result<Vec<u8>, String> {
    mono_modality(window, sequences, None)
}

/// [`mono_with`] with Modality CT, so a preset passes the F-213 modality
/// check and the refusal a test sees is the one it names.
fn mono_ct_with(window: bool, sequences: &[(u16, u16)]) -> Result<Vec<u8>, String> {
    mono_modality(window, sequences, Some("CT"))
}

fn mono_modality(
    window: bool,
    sequences: &[(u16, u16)],
    modality: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    if let Some(code) = modality {
        data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad(code))?);
    }
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    if window {
        data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
        data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    }
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    if window {
        data.extend(short_le(
            (0x0028, 0x1056),
            *b"CS",
            &text_pad("LINEAR_EXACT"),
        )?);
    }
    for tag in sequences {
        data.extend(long_le(*tag, *b"SQ", &[])?);
    }
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn file_meta(transfer_syntax: &str) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(transfer_syntax.as_bytes()),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui(b"2.25.2"))?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length".to_owned())?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui(text: &[u8]) -> Vec<u8> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    value
}

fn text_pad(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    value
}

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

#[test]
fn an_invert_flag_flips_grey_64_to_191() -> Result<(), String> {
    // The windowed frame is grey 64. Invert is a display flip after the LUT,
    // 255 - 64 = 191. It is not a second photometric rule.
    let file = mono_frame(true)?;
    let request = http_post_to("/preview", "X-Ocelli-Invert: 1\r\n", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") || !header.contains("X-Ocelli-Invert: 1") {
        return Err("the inverted preview was refused".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the inverted preview was empty".to_owned())?;
    if grey != 191 {
        return Err("the inverted preview was not grey 191".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_override_maps_stored_50_to_grey_128() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. Override LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_frame(
        &mono_frame(true)?,
        FrameRequest {
            frame: 0,
            invert: false,
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            window_preset: None,
            window_index: None,
            series_window: None,
        },
    )
    .map_err(|_| "the window override was refused".to_owned())?;
    if preview.auto != AutoSource::None {
        return Err("a window override was labelled auto".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the window override was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn the_second_frame_uses_its_own_stored_value() -> Result<(), String> {
    // Frame 1 stores 0. ((0 - 100) / 200 + 0.5) * 255 = 0.
    // display_index of a non-positive display value is 0.
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&50_u16.to_le_bytes());
    pixels.extend_from_slice(&0_u16.to_le_bytes());
    let file = tagged_frame(2, &pixels)?;
    let request = http_post_to("/preview", "X-Ocelli-Frame: 1\r\n", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") || !header.contains("X-Ocelli-Frame: 1") {
        return Err("the second frame was refused".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the second frame was empty".to_owned())?;
    if grey != 0 {
        return Err("the second frame was not grey 0".to_owned());
    }
    let first = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the first frame of the pair was refused".to_owned())?;
    if only_byte(&first)? != 64 {
        return Err("the first frame of the pair was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn inspect_json_reports_the_series_fields() -> Result<(), String> {
    let file = tagged_frame(1, &50_u16.to_le_bytes())?;
    let request = http_post_to("/inspect", "", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("inspect was refused".to_owned());
    }
    let json = std::str::from_utf8(body).map_err(|_| "inspect is not utf-8".to_owned())?;
    for needle in [
        "\"seriesUid\":\"2.25.9\"",
        "\"modality\":\"CT\"",
        "\"seriesNumber\":\"3\"",
        "\"instanceNumber\":\"2\"",
        "\"patientName\":\"\"",
        "\"monochrome\":true",
        "\"windowFunction\":\"LINEAR_EXACT\"",
        "\"rowSpacing\":null",
        "\"columnSpacing\":null",
        "\"spacingSource\":\"\"",
        "\"edgeLeft\":\"\"",
        "\"edgeRight\":\"\"",
        "\"edgeTop\":\"\"",
        "\"edgeBottom\":\"\"",
    ] {
        if !json.contains(needle) {
            return Err("inspect omitted a series field".to_owned());
        }
    }
    let direct = inspect_json(&file, 0).map_err(|_| "inspect_json refused the file".to_owned())?;
    if direct != json {
        return Err("inspect_json and the page disagree".to_owned());
    }
    Ok(())
}

#[test]
fn inspect_reports_sop_class_and_study() -> Result<(), String> {
    // F-215 AC14. SOP Class UID (0008,0016), PS3.3 C.12.1, and Study
    // Instance UID (0020,000D), PS3.3 C.7.2.1, so the page recognises an SR
    // or Key Object Selection by its class and files it under its study,
    // never by its file name. A study UID that is not a UID is sent empty.
    let study = short_le((0x0020, 0x000D), *b"UI", &ui(b"1.2.3.4.5"))?;
    for (extra, want) in [
        (vec![study], "\"studyUid\":\"1.2.3.4.5\""),
        (Vec::new(), "\"studyUid\":\"\""),
        (
            vec![short_le((0x0020, 0x000D), *b"UI", &ui(b"1.2.x"))?],
            "\"studyUid\":\"\"",
        ),
    ] {
        let file = tagged_frame_with(1, &50_u16.to_le_bytes(), &extra)?;
        let json =
            inspect_json(&file, 0).map_err(|_| "inspect_json refused the file".to_owned())?;
        if !json.contains("\"sopClassUid\":\"1.2.840.10008.5.1.4.1.1.2\"") {
            return Err(format!("inspect did not name the SOP class: {json}"));
        }
        if !json.contains(want) {
            return Err(format!("inspect did not give {want}: {json}"));
        }
    }
    Ok(())
}

#[test]
fn inspect_reports_the_stored_study_time() -> Result<(), String> {
    // F-215 review pass 1, D3. Study Time (0008,0030) is an attribute of the
    // study, PS3.3 C.7.2.1, so a document about the study copies the stored
    // TM, PS3.5 6.2, fraction included. `studyTime` stays F-212's display
    // text, which drops the fraction.
    let file = corner_object(&every_corner_attribute())?;
    let json = inspect_json(&file, 0).map_err(|_| "inspect refused the CT".to_owned())?;
    for needle in [
        "\"studyTimeValue\":\"143012.5\"",
        "\"studyTime\":\"14:30:12\"",
    ] {
        if !json.contains(needle) {
            return Err(format!("inspect lacks {needle}: {json}"));
        }
    }
    let bare = tagged_frame(1, &50_u16.to_le_bytes())?;
    let json = inspect_json(&bare, 0).map_err(|_| "inspect refused the file".to_owned())?;
    if !json.contains("\"studyTimeValue\":\"\"") {
        return Err(format!("an absent Study Time is not empty: {json}"));
    }
    Ok(())
}

#[test]
fn headers_json_lists_the_window_and_hides_pixels() -> Result<(), String> {
    let file = tagged_frame(1, &50_u16.to_le_bytes())?;
    let json = headers_json(&file).map_err(|_| "headers_json refused the file".to_owned())?;
    if !json.contains("\"tag\":\"(0028,1056)\"") || !json.contains("\"value\":\"LINEAR_EXACT\"") {
        return Err("the headers omitted the window function".to_owned());
    }
    if !json.contains("\"tag\":\"(7FE0,0010)\"") || !json.contains("\"value\":\"2 bytes\"") {
        return Err("pixel data was not reported as a byte count".to_owned());
    }
    if json.contains("\"value\":\"50\"") {
        return Err("pixel data was copied into the header table".to_owned());
    }
    Ok(())
}

#[test]
fn a_modality_lut_sequence_is_refused() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 100 width 200
    // would address grey 64 if the Modality LUT Sequence were ignored.
    // PS3.3 C.11.1 says that sequence replaces the rescale. This preview
    // does not apply a sequence, so the file is refused.
    let file = mono_with(true, &[(0x0028, 0x3000)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => {
            if PreviewError::Window.code() != "window" {
                return Err("the window refusal has the wrong code".to_owned());
            }
            if PreviewError::Window.to_string() != "the window or presentation could not be applied"
            {
                return Err("the window refusal has the wrong sentence".to_owned());
            }
            Ok(())
        }
        Ok(_) => Err("a modality LUT sequence was drawn".to_owned()),
        Err(error) => Err(format!("a modality LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_window_override_does_not_replace_a_modality_lut() -> Result<(), String> {
    // The same stored 50 with a Modality LUT Sequence. Center 50 and width 1
    // replace a VOI stage only. The modality sequence is still unapplied.
    let file = mono_with(true, &[(0x0028, 0x3000)])?;
    match preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a modality LUT sequence was drawn under a window override".to_owned()),
        Err(error) => Err(format!("a modality LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_voi_lut_sequence_without_an_override_is_refused() -> Result<(), String> {
    // No Window Center, and a VOI LUT Sequence is present. The automatic
    // window would address grey 128 for stored 50. PS3.3 C.11.2 says
    // the sequence is the VOI transform. This preview refuses it.
    let file = mono_with(false, &[(0x0028, 0x3010)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a VOI LUT sequence with no window was drawn".to_owned()),
        Err(error) => Err(format!("a VOI LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_file_window_does_not_hide_a_voi_lut_sequence() -> Result<(), String> {
    // The file also has LINEAR_EXACT center 100 width 200. Ignoring the
    // VOI LUT Sequence would address grey 64. The sequence takes precedence
    // until the caller replaces the window.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a VOI LUT sequence was drawn with the file window".to_owned()),
        Err(error) => Err(format!("a VOI LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_window_override_replaces_a_voi_lut_sequence() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. Override LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    let preview = preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "a window override did not replace the VOI LUT sequence".to_owned())?;
    if preview.auto != AutoSource::None {
        return Err("a window override was labelled auto".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the VOI LUT override was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_override_ignores_an_unusable_file_window() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. The file has a VOI LUT Sequence, two
    // centers with one width, and a function token this preview does not know.
    // Without an override that file is refused. An override of LINEAR_EXACT
    // center 50 width 1 does not read those attributes.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_unusable_window()?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("an unusable file window was drawn".to_owned()),
        Err(error) => return Err(format!("an unusable file window was refused as {error}")),
    }
    let preview = preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "a window override still read the unusable file window".to_owned())?;
    if preview.auto != AutoSource::None {
        return Err("a window override was labelled auto".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the override of an unusable window was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn synthetic_study_has_no_patient_name() -> Result<(), String> {
    // The study is a phantom. (0010,0010) and (0010,0020) must not be written.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if contains_tag(&file, 0x0010, 0x0010) || contains_tag(&file, 0x0010, 0x0020) {
        return Err("the synthetic study wrote a patient tag".to_owned());
    }
    if !file
        .windows(b"Synthetic phantom".len())
        .any(|window| window == b"Synthetic phantom")
    {
        return Err("the series description is not Synthetic phantom".to_owned());
    }
    if !file
        .windows(b"2.25.".len())
        .any(|window| window == b"2.25.")
    {
        return Err("the synthetic study has no 2.25 uid".to_owned());
    }
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    if !json.contains("\"patientName\":\"\"") || !json.contains("\"patientId\":\"\"") {
        return Err("inspect reported a patient name or id".to_owned());
    }
    if !json.contains("\"seriesDescription\":\"Synthetic phantom\"") {
        return Err("inspect omitted the series description".to_owned());
    }
    Ok(())
}

#[test]
fn every_synthetic_slice_previews() -> Result<(), String> {
    if SLICES != 12 {
        return Err("the study is not 12 slices".to_owned());
    }
    if slice(SLICES).is_some() {
        return Err("an index past the study returned bytes".to_owned());
    }
    for index in 0..SLICES {
        let file = slice(index).ok_or_else(|| format!("slice {index} was not built"))?;
        let preview = preview_frame(&file, FrameRequest::default())
            .map_err(|_| format!("slice {index} was refused"))?;
        if preview.width != 128 || preview.height != 96 || preview.frames != 1 {
            return Err(format!("slice {index} is not 128 by 96"));
        }
        if preview.grey.len() != 128 * 96 {
            return Err(format!("slice {index} did not return one byte per pixel"));
        }
        // Corner is air. Centre of slice 0 is body and stays body as the
        // ellipse grows. Stored values are HU + 1024.
        if pixel_i16(&file, 0, 0)? != 24 {
            return Err(format!("slice {index} corner is not air"));
        }
        if pixel_i16(&file, 64, 48)? != 1064 {
            return Err(format!("slice {index} centre is not body"));
        }
    }
    Ok(())
}

#[test]
fn soft_tissue_matches_the_file_window() -> Result<(), String> {
    // File window is centre 40 width 400 LINEAR. Soft tissue is the same
    // pair from WindowPreset, so the grey bytes match. A second formula
    // would move the body pixel off 128.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let file_window = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the file window was refused".to_owned())?;
    let preset = preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::SoftTissue),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "the soft tissue preset was refused".to_owned())?;
    if file_window.grey != preset.grey {
        return Err("soft tissue disagreed with the file window".to_owned());
    }
    if preset.window_function != "LINEAR" {
        return Err("soft tissue was not LINEAR".to_owned());
    }
    if preset.window_center.to_bits() != WindowPreset::SoftTissue.center().to_bits()
        || preset.window_width.to_bits() != WindowPreset::SoftTissue.width().to_bits()
    {
        return Err("soft tissue did not use the library centre and width".to_owned());
    }
    // Body at column 64 row 48. PS3.3 C.11.2.1.2 LINEAR.
    // x = 40, c' = 39.5, w' = 399.
    // y = ((40 - 39.5) / 399 + 0.5) * 255 = 127.819550 in f32.
    // display_index adds 0.5 in f32 and addresses 128.
    if grey_at(&preset, 64, 48)? != 128 {
        return Err("the body pixel was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn preset_grey_matches_ps3_3_c_11() -> Result<(), String> {
    // Named pixels on slice 0, stored = HU + 1024 with intercept -1024.
    // Air (0, 0) is HU -1000. Body (64, 48) is HU 40.
    // Lung (46, 48) is HU -800. Bone (72, 48) is HU 700.
    // PS3.3 C.11.2.1.2 LINEAR. ymin 0, ymax 255.
    // c' = c - 0.5, w' = w - 1.
    // y = ((x - c') / w' + 0.5) * 255 inside the window, else 0 or 255.
    // display_index is the largest n in 0..=255 with f32(n) <= y + 0.5.
    //
    // Soft tissue c = 40, w = 400. Air 0, body 128, lung 0, bone 255.
    // Lung c = -600, w = 1500.
    // Air y = 59.539696 grey 60. Body y = 236.457626 grey 236.
    // Lung y = 93.562378 grey 94. Bone y = 255 grey 255.
    // Bone c = 300, w = 1500.
    // Air 0, body y = 83.355568 grey 83, lung 0, bone y = 195.630417 grey 196.
    // Changing the intercept from -1024 moves body off these codes.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if pixel_i16(&file, 0, 0)? != 24
        || pixel_i16(&file, 64, 48)? != 1064
        || pixel_i16(&file, 46, 48)? != 224
        || pixel_i16(&file, 72, 48)? != 1724
    {
        return Err("a named pixel is not the planned Hounsfield value".to_owned());
    }
    let cases = [
        (WindowPreset::SoftTissue, [0_u8, 128, 0, 255]),
        (WindowPreset::Lung, [60, 236, 94, 255]),
        (WindowPreset::Bone, [0, 83, 0, 196]),
    ];
    let points = [(0_usize, 0_usize), (64, 48), (46, 48), (72, 48)];
    for (preset, expected) in cases {
        let preview = preview_frame(
            &file,
            FrameRequest {
                window_preset: Some(preset),
                ..FrameRequest::default()
            },
        )
        .map_err(|_| "a preset was refused".to_owned())?;
        if preview.window_center.to_bits() != preset.center().to_bits()
            || preview.window_width.to_bits() != preset.width().to_bits()
        {
            return Err("a preset did not use the library centre and width".to_owned());
        }
        for (point, grey) in points.iter().zip(expected) {
            if grey_at(&preview, point.0, point.1)? != grey {
                return Err("a named pixel missed the hand-computed grey".to_owned());
            }
        }
    }
    Ok(())
}

#[test]
fn spacing_keeps_row_then_column() -> Result<(), String> {
    // PS3.3 C.7.6.2.1.1 stores adjacent row spacing, then adjacent column
    // spacing. The phantom writes 0.7 then 0.5. The tokens stay decimal text.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    if !json.contains("\"rowSpacing\":0.7") || !json.contains("\"columnSpacing\":0.5") {
        return Err("spacing was not row 0.7 then column 0.5".to_owned());
    }
    if !json.contains("\"spacingSource\":\"PixelSpacing\"") {
        return Err("pixel spacing was not named".to_owned());
    }
    let preview = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the spaced slice was refused".to_owned())?;
    if preview.facts.row_spacing.as_deref() != Some("0.7")
        || preview.facts.column_spacing.as_deref() != Some("0.5")
        || preview.facts.spacing_source != "PixelSpacing"
    {
        return Err("the preview spacing disagreed with inspect".to_owned());
    }
    let both = spacing_object(Some("0.7\\0.5"), Some("1.5\\1.5"))?;
    let both_json =
        inspect_json(&both, 0).map_err(|_| "inspect refused both spacings".to_owned())?;
    if !both_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || !both_json.contains("\"rowSpacing\":0.7")
    {
        return Err("imager spacing replaced pixel spacing".to_owned());
    }
    let imager = spacing_object(None, Some("0.3\\0.2"))?;
    let imager_json =
        inspect_json(&imager, 0).map_err(|_| "inspect refused imager spacing".to_owned())?;
    if !imager_json.contains("\"spacingSource\":\"ImagerPixelSpacing\"")
        || !imager_json.contains("\"rowSpacing\":0.3")
        || !imager_json.contains("\"columnSpacing\":0.2")
    {
        return Err("imager spacing was not labelled".to_owned());
    }
    // 00.5 is legal DS and not a JSON number. The reply uses 0.5.
    let padded = spacing_object(Some("00.5\\00.5"), None)?;
    let padded_json =
        inspect_json(&padded, 0).map_err(|_| "inspect refused a padded spacing".to_owned())?;
    if !padded_json.contains("\"rowSpacing\":0.5")
        || !padded_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || padded_json.contains("00.5")
    {
        return Err("a padded spacing was not a JSON number".to_owned());
    }
    // -.5 is legal DS. The reply uses -0.5, which is a JSON number.
    let dotted = spacing_object(Some("-.5\\-.5"), None)?;
    let dotted_json =
        inspect_json(&dotted, 0).map_err(|_| "inspect refused a dotted spacing".to_owned())?;
    if !dotted_json.contains("\"rowSpacing\":-0.5")
        || !dotted_json.contains("\"columnSpacing\":-0.5")
    {
        return Err("a leading dot was not rewritten as a JSON number".to_owned());
    }
    // 5E-1 is legal DS. It must stay Pixel Spacing and must not be replaced
    // by the imager pair 0.6.
    let exponent = spacing_object(Some("5E-1\\5E-1"), Some("0.6\\0.6"))?;
    let exponent_json =
        inspect_json(&exponent, 0).map_err(|_| "inspect refused an exponent spacing".to_owned())?;
    if !exponent_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || !exponent_json.contains("\"rowSpacing\":5E-1")
        || exponent_json.contains("0.6")
    {
        return Err("an exponent spacing fell through to the imager pair".to_owned());
    }
    // Three values are not a pair. The imager value must not be substituted.
    let triple = spacing_object(Some("1\\2\\3"), Some("0.6\\0.6"))?;
    let triple_json =
        inspect_json(&triple, 0).map_err(|_| "inspect refused a triple spacing".to_owned())?;
    if !triple_json.contains("\"rowSpacing\":null")
        || !triple_json.contains("\"spacingSource\":\"\"")
        || triple_json.contains("0.6")
    {
        return Err("a triple pixel spacing used the imager pair".to_owned());
    }
    Ok(())
}

#[test]
fn axial_edge_letters() -> Result<(), String> {
    // PS3.3 C.7.6.1.1.1. Row cosine 1\\0\\0 points to the patient's left,
    // which is the right edge of an axial image. Column cosine 0\\1\\0
    // points posterior, which is the bottom edge. The opposite edges are
    // R and A. Left R, right L, top A, bottom P.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    for needle in [
        "\"edgeLeft\":\"R\"",
        "\"edgeRight\":\"L\"",
        "\"edgeTop\":\"A\"",
        "\"edgeBottom\":\"P\"",
    ] {
        if !json.contains(needle) {
            return Err("the axial letters were not R L A P".to_owned());
        }
    }
    let preview = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the oriented slice was refused".to_owned())?;
    if preview.facts.edge_left != "R"
        || preview.facts.edge_right != "L"
        || preview.facts.edge_top != "A"
        || preview.facts.edge_bottom != "P"
    {
        return Err("the preview letters disagreed with inspect".to_owned());
    }
    // Row cosine 2\\0\\0 is not a unit vector. Letters stay empty and the
    // frame still previews.
    let stretched = oriented_frame("2\\0\\0\\0\\1\\0")?;
    let stretched_json = inspect_json(&stretched, 0)
        .map_err(|_| "inspect refused a non-unit orientation".to_owned())?;
    if !stretched_json.contains("\"edgeLeft\":\"\"")
        || !stretched_json.contains("\"edgeRight\":\"\"")
    {
        return Err("a non-unit orientation still produced letters".to_owned());
    }
    preview_frame(&stretched, FrameRequest::default())
        .map_err(|_| "a non-unit orientation was refused".to_owned())?;
    Ok(())
}

#[test]
fn a_preset_refuses_a_voi_lut_sequence() -> Result<(), String> {
    // The file has a VOI LUT Sequence. A numeric override may replace it.
    // A named preset must not. Lung would otherwise draw a window. The file
    // is a CT, so the refusal is the LUT one and not F-213's modality one.
    let file = mono_ct_with(true, &[(0x0028, 0x3010)])?;
    match preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset replaced a VOI LUT sequence".to_owned()),
        Err(error) => {
            return Err(format!(
                "a preset on a VOI LUT sequence was refused as {error}"
            ));
        }
    }
    // A Modality LUT Sequence is also refused. The preset must not draw
    // the identity slope in its place.
    let modality = mono_ct_with(true, &[(0x0028, 0x3000)])?;
    match preview_frame(
        &modality,
        FrameRequest {
            window_preset: Some(WindowPreset::SoftTissue),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a preset replaced a modality LUT sequence".to_owned()),
        Err(error) => Err(format!(
            "a preset on a modality LUT sequence was refused as {error}"
        )),
    }
}

#[test]
fn a_preset_with_a_numeric_window_is_refused() -> Result<(), String> {
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    match preview_frame(
        &file,
        FrameRequest {
            window_center: Some(40.0),
            window_width: Some(400.0),
            window_preset: Some(WindowPreset::Bone),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset was combined with a numeric window".to_owned()),
        Err(error) => {
            return Err(format!("a combined window was refused as {error}"));
        }
    }
    let request = http_post_to(
        "/preview",
        "X-Ocelli-Window-Preset: lung\r\nX-Ocelli-Window-Center: 40\r\n",
        &file,
    );
    let response = one_exchange(&request)?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: window") {
        return Err("the preset header was combined with a centre".to_owned());
    }
    let lung = http_post_to("/preview", "X-Ocelli-Window-Preset: lung\r\n", &file);
    let drawn = one_exchange(&lung)?;
    let (header, body) = split_http(&drawn)?;
    if !header.contains("200")
        || !header.contains("X-Ocelli-Window-Center: -600")
        || !header.contains("X-Ocelli-Window-Width: 1500")
        || !header.contains("X-Ocelli-Window-Function: LINEAR")
        || !header.contains("X-Ocelli-Row-Spacing: 0.7")
        || !header.contains("X-Ocelli-Column-Spacing: 0.5")
        || !header.contains("X-Ocelli-Spacing-Source: PixelSpacing")
        || !header.contains("X-Ocelli-Edge-Left: R")
        || !header.contains("X-Ocelli-Edge-Right: L")
        || !header.contains("X-Ocelli-Edge-Top: A")
        || !header.contains("X-Ocelli-Edge-Bottom: P")
    {
        return Err("the lung preset response omitted the window or the geometry".to_owned());
    }
    // Lung window, body pixel. y = 236.457626, grey 236.
    let index = 48 * 128 + 64;
    let grey = body
        .get(index)
        .copied()
        .ok_or_else(|| "the lung preview was short".to_owned())?;
    if grey != 236 {
        return Err("the lung preset body pixel was not grey 236".to_owned());
    }
    match preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset kept a different window function".to_owned()),
        Err(error) => return Err(format!("a preset function was refused as {error}")),
    }
    // The body is empty so the window error is written with nothing left
    // unread on the socket.
    let bad = http_post_to("/preview", "X-Ocelli-Window-Preset: soft\r\n", b"");
    let refused = one_exchange(&bad)?;
    let (header, _) = split_http(&refused)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: window") {
        return Err("a bad preset token was not the window error".to_owned());
    }
    Ok(())
}

#[test]
fn synthetic_routes_serve_part10() -> Result<(), String> {
    let count = one_exchange(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&count)?;
    if !header.contains("200") || body != br#"{"slices":12}"# {
        return Err("the study count was not 12".to_owned());
    }
    let first = one_exchange(b"GET /synthetic/0 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&first)?;
    let expected = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if !header.contains("200") || !header.contains("application/octet-stream") || body != expected {
        return Err("GET /synthetic/0 was not the first slice".to_owned());
    }
    let missing = one_exchange(b"GET /synthetic/12 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&missing)?;
    if !header.contains("404") {
        return Err("an index past the study was not 404".to_owned());
    }
    let word = one_exchange(b"GET /synthetic/no HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&word)?;
    if !header.contains("404") {
        return Err("a non-numeric slice was not 404".to_owned());
    }
    Ok(())
}

fn grey_at(preview: &Preview, column: usize, row: usize) -> Result<u8, String> {
    let index = row
        .checked_mul(128)
        .and_then(|row_start| row_start.checked_add(column))
        .ok_or_else(|| "the pixel index overflowed".to_owned())?;
    preview
        .grey
        .get(index)
        .copied()
        .ok_or_else(|| "the pixel is outside the frame".to_owned())
}

fn pixel_i16(file: &[u8], column: usize, row: usize) -> Result<i16, String> {
    let marker = [0xE0, 0x7F, 0x10, 0x00, b'O', b'W', 0, 0];
    let start = file
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| "the slice has no OW pixel data".to_owned())?;
    let length_at = start + marker.len();
    let length_bytes = file
        .get(length_at..length_at + 4)
        .ok_or_else(|| "the pixel length is short".to_owned())?;
    let length_bytes: [u8; 4] = length_bytes
        .try_into()
        .map_err(|_| "the pixel length is short".to_owned())?;
    let length = u32::from_le_bytes(length_bytes);
    let pixels_at = length_at + 4;
    let offset = row
        .checked_mul(128)
        .and_then(|row_start| row_start.checked_add(column))
        .and_then(|index| index.checked_mul(2))
        .ok_or_else(|| "the stored index overflowed".to_owned())?;
    if u32::try_from(offset + 2).map_err(|_| "the stored index overflowed".to_owned())? > length {
        return Err("the stored pixel is outside pixel data".to_owned());
    }
    let at = pixels_at + offset;
    let bytes = file
        .get(at..at + 2)
        .ok_or_else(|| "the stored pixel is short".to_owned())?;
    let bytes: [u8; 2] = bytes
        .try_into()
        .map_err(|_| "the stored pixel is short".to_owned())?;
    Ok(i16::from_le_bytes(bytes))
}

fn contains_tag(bytes: &[u8], group: u16, element: u16) -> bool {
    let mut needle = [0_u8; 4];
    needle[..2].copy_from_slice(&group.to_le_bytes());
    needle[2..].copy_from_slice(&element.to_le_bytes());
    bytes.windows(4).any(|window| window == needle)
}

fn oriented_frame(orientation: &str) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0020, 0x0037), *b"DS", &text_pad(orientation))?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &0_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn spacing_object(pixel: Option<&str>, imager: Option<&str>) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    if let Some(pair) = imager {
        data.extend(short_le((0x0018, 0x1164), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    if let Some(pair) = pixel {
        data.extend(short_le((0x0028, 0x0030), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("1"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &0_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn http_post_to(path: &str, extra: &str, body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{extra}Content-Length: {len}\r\n\r\n",
        len = body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn tagged_frame(frames: u16, pixels: &[u8]) -> Result<Vec<u8>, String> {
    tagged_frame_with(frames, pixels, &[])
}

/// [`tagged_frame`] with `extra` encoded elements placed after Series
/// Description `(0008,103E)` and before Series Instance UID `(0020,000E)`,
/// which keeps the data set in tag order for every cine attribute.
fn tagged_frame_with(frames: u16, pixels: &[u8], extra: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0018), *b"UI", &ui(b"2.25.1"))?);
    data.extend(short_le((0x0008, 0x0020), *b"DA", &text_pad("20200101"))?);
    data.extend(short_le((0x0008, 0x0050), *b"SH", &text_pad("A1"))?);
    data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad("CT"))?);
    data.extend(short_le((0x0008, 0x1030), *b"LO", &text_pad("SYNTH"))?);
    data.extend(short_le((0x0008, 0x103E), *b"LO", &text_pad("SYNTH"))?);
    for element in extra {
        data.extend_from_slice(element);
    }
    data.extend(short_le((0x0020, 0x000E), *b"UI", &ui(b"2.25.9"))?);
    data.extend(short_le((0x0020, 0x0011), *b"IS", &text_pad("3"))?);
    data.extend(short_le((0x0020, 0x0013), *b"IS", &text_pad("2"))?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    if frames > 1 {
        data.extend(short_le(
            (0x0028, 0x0008),
            *b"IS",
            &text_pad(&frames.to_string()),
        )?);
    }
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("LINEAR_EXACT"),
    )?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

/// Two frames of one 1 by 1 sample each, with `extra` cine elements.
fn cine_frames(frames: u16, extra: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let pixels: Vec<u8> = (0..frames).flat_map(|_| 50_u16.to_le_bytes()).collect();
    tagged_frame_with(frames, &pixels, extra)
}

fn ds_element(tag: (u16, u16), text: &str) -> Result<Vec<u8>, String> {
    short_le(tag, *b"DS", &text_pad(text))
}

fn is_element(tag: (u16, u16), text: &str) -> Result<Vec<u8>, String> {
    short_le(tag, *b"IS", &text_pad(text))
}

const FRAME_TIME: (u16, u16) = (0x0018, 0x1063);
const FRAME_TIME_VECTOR: (u16, u16) = (0x0018, 0x1065);
const RECOMMENDED_RATE: (u16, u16) = (0x0008, 0x2144);
const CINE_RATE: (u16, u16) = (0x0018, 0x0040);
const PLAYBACK: (u16, u16) = (0x0018, 0x1244);

fn cine_fields(file: &[u8]) -> Result<String, String> {
    inspect_json(file, 0).map_err(|_| "inspect_json refused the file".to_owned())
}

fn expect_cine(json: &str, cine: &str, frame_time: &str, case: &str) -> Result<(), String> {
    let wanted = format!("\"cine\":\"{cine}\",\"frameTimeMs\":{frame_time}");
    if json.contains(&wanted) {
        Ok(())
    } else {
        Err(format!("{case}: inspect did not report {wanted}"))
    }
}

#[test]
fn inspect_reports_frame_time() -> Result<(), String> {
    // AC5. PS3.3 C.7.6.5 gives Frame Time (0018,1063) as DS in milliseconds,
    // so DS 40 is 40 ms, which is 1000 / 40 = 25 frames per second. The
    // reply carries the milliseconds, not seconds and not a rate.
    let forty = cine_frames(
        2,
        &[is_element(CINE_RATE, "30")?, ds_element(FRAME_TIME, "40")?],
    )?;
    let request = http_post_to("/inspect", "", &forty);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("inspect was refused".to_owned());
    }
    let json = std::str::from_utf8(body).map_err(|_| "inspect is not utf-8".to_owned())?;
    expect_cine(json, "frame-time", "40", "Frame Time 40")?;
    if cine_fields(&forty)? != json {
        return Err("inspect_json and the page disagree".to_owned());
    }
    // AC6. No timing attribute gives none and null, never 0.
    expect_cine(
        &cine_fields(&cine_frames(2, &[])?)?,
        "none",
        "null",
        "no timing",
    )?;
    // AC7. PS3.3 C.7.6.5.1.1 gives Frame Time as the interval between frames
    // of a multi-frame image, so 0 on two frames is not usable, and it is not
    // replaced by the Recommended Display Frame Rate (0008,2144) of 10.
    let zero = cine_frames(
        2,
        &[
            is_element(RECOMMENDED_RATE, "10")?,
            ds_element(FRAME_TIME, "0")?,
        ],
    )?;
    expect_cine(
        &cine_fields(&zero)?,
        "unusable",
        "null",
        "Frame Time 0 on two frames",
    )?;
    // F-214 amendment 4. One frame has no interval, so Frame Time 0 there is
    // legal and reports none.
    let single = cine_frames(1, &[ds_element(FRAME_TIME, "0")?])?;
    expect_cine(
        &cine_fields(&single)?,
        "none",
        "null",
        "Frame Time 0 on one frame",
    )?;
    // AC8. PS3.3 C.7.6.5.1.2. A Frame Time Vector (0018,1065) of one value per
    // frame comes before Frame Time. Its second value, 40, is not reported as
    // one interval.
    let vector = cine_frames(
        2,
        &[
            ds_element(FRAME_TIME, "100")?,
            ds_element(FRAME_TIME_VECTOR, "0\\40")?,
        ],
    )?;
    expect_cine(
        &cine_fields(&vector)?,
        "vector",
        "null",
        "a matching vector",
    )?;
    // A rate and no Frame Time is a rate, which Play does not use.
    let rate = cine_frames(2, &[is_element(RECOMMENDED_RATE, "25")?])?;
    expect_cine(&cine_fields(&rate)?, "rate", "null", "a recommended rate")?;
    let cine = cine_frames(2, &[is_element(CINE_RATE, "30")?])?;
    expect_cine(&cine_fields(&cine)?, "rate", "null", "a cine rate")?;
    // Review pass 2, D2. Cine Rate (0018,0040) is IS in frames per second,
    // PS3.3 C.7.6.5, so 0 is not a rate. With no Frame Time the refusal is
    // still the cine timing's, and the reply says unusable.
    let zero_rate = cine_frames(2, &[is_element(CINE_RATE, "0")?])?;
    expect_cine(
        &cine_fields(&zero_rate)?,
        "unusable",
        "null",
        "Cine Rate 0 and no Frame Time",
    )?;
    // F-214 amendment 2. Preferred Playback Sequencing (0018,1244) is US 0 or
    // 1, PS3.3 C.7.6.5. A value of 2 does not hide a good Frame Time of 40.
    let playback = cine_frames(
        2,
        &[
            ds_element(FRAME_TIME, "40")?,
            short_le(PLAYBACK, *b"US", &2_u16.to_le_bytes())?,
        ],
    )?;
    expect_cine(
        &cine_fields(&playback)?,
        "frame-time",
        "40",
        "a bad playback value",
    )?;
    Ok(())
}

#[test]
fn a_frame_declaration_refusal_makes_no_frame_time_claim() -> Result<(), String> {
    // F-214 review pass 1, D3. Two frames, no timing attribute, and a
    // Per-frame Functional Groups Sequence (5200,9230) of one item, where
    // PS3.3 C.7.6.16 asks for one item per frame. The refusal is about the
    // frame declaration, so the reply must not say the Frame Time is not
    // usable, because the file has none.
    let plain = cine_frames(2, &[])?;
    let pixel_at = plain
        .windows(4)
        .rposition(|window| window == [0xE0, 0x7F, 0x10, 0x00])
        .ok_or_else(|| "the fixture has no pixel data".to_owned())?;
    let one_item = long_le((0x5200, 0x9230), *b"SQ", &item(&[])?)?;
    let mut file = plain
        .get(..pixel_at)
        .ok_or_else(|| "the fixture is short".to_owned())?
        .to_vec();
    file.extend(one_item);
    file.extend_from_slice(plain.get(pixel_at..).unwrap_or_default());
    expect_cine(
        &cine_fields(&file)?,
        "unreadable",
        "null",
        "a bad frame declaration",
    )
}

#[test]
fn each_file_reports_its_own_frame_time() -> Result<(), String> {
    // F-214 amendment 1. Frame Time is a property of one multi-frame object,
    // PS3.3 C.7.6.5.1.1, so two files of one series report their own. DS 33.3
    // is 33.3 ms, 1000 / 33.3 = 30.03 per second, and DS 40 is 40 ms.
    let first = cine_frames(2, &[ds_element(FRAME_TIME, "40")?])?;
    let second = cine_frames(3, &[ds_element(FRAME_TIME, "33.3")?])?;
    expect_cine(&cine_fields(&first)?, "frame-time", "40", "the first file")?;
    expect_cine(
        &cine_fields(&second)?,
        "frame-time",
        "33.3",
        "the second file",
    )?;
    // Every frame of one file carries the same object timing.
    let later = inspect_json(&second, 2).map_err(|_| "inspect_json refused frame 3".to_owned())?;
    expect_cine(&later, "frame-time", "33.3", "the third frame")
}

fn viewer_on(bytes: usize) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::new(ViewerConfig { cache_bytes: bytes }));
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

fn viewer_capped(bytes: usize, cap: usize) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::with_connection_cap(
        ViewerConfig { cache_bytes: bytes },
        cap,
    ));
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

fn header_line<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split("\r\n").find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim())
        } else {
            None
        }
    })
}

fn json_number(body: &str, key: &str) -> Result<usize, String> {
    let needle = format!("\"{key}\":");
    let rest = body
        .split_once(&needle)
        .ok_or_else(|| format!("the census omitted {key}"))?
        .1;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits
        .parse::<usize>()
        .map_err(|_| format!("the census {key} was not a number"))
}

fn held_census(addr: SocketAddr) -> Result<String, String> {
    let response = exchange(addr, b"GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.starts_with("HTTP/1.1 200 ") {
        return Err(header.to_owned());
    }
    std::str::from_utf8(body)
        .map(str::to_owned)
        .map_err(|_| "the census was not text".to_owned())
}

fn preview_id(id: &str, extra: &str) -> Vec<u8> {
    format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\n{extra}Content-Length: 0\r\n\r\n"
    )
    .into_bytes()
}

fn read_one(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        if let Some(end) = buf
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|start| start + 4)
        {
            break end;
        }
        let n = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if n == 0 {
            return Err("the response closed before the header".to_owned());
        }
        let Some(got) = chunk.get(..n) else {
            return Err("the response read was short".to_owned());
        };
        buf.extend_from_slice(got);
    };
    let header = {
        let bytes = buf
            .get(..header_end)
            .ok_or_else(|| "the response header is short".to_owned())?;
        std::str::from_utf8(bytes).map_err(|_| "the response header is not text".to_owned())?
    };
    let length = header_line(header, "Content-Length")
        .ok_or_else(|| "the response has no length".to_owned())?
        .parse::<usize>()
        .map_err(|_| "the response length is not a number".to_owned())?;
    while buf.len() - header_end < length {
        let n = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if n == 0 {
            return Err("the response ended early".to_owned());
        }
        let have = buf.len() - header_end;
        let need = length - have;
        let take = n.min(need);
        let Some(got) = chunk.get(..take) else {
            return Err("the response read was short".to_owned());
        };
        buf.extend_from_slice(got);
    }
    Ok(buf)
}

fn response_parts(bytes: &[u8]) -> Result<Vec<(&str, &[u8])>, String> {
    let mut rest = bytes;
    let mut found = Vec::new();
    while !rest.is_empty() {
        let (header, body) = split_http(rest)?;
        let length = header_line(header, "Content-Length")
            .ok_or_else(|| "a pipelined response has no length".to_owned())?
            .parse::<usize>()
            .map_err(|_| "a pipelined response length is not a number".to_owned())?;
        let taken = body
            .get(..length)
            .ok_or_else(|| "a pipelined response body is short".to_owned())?;
        found.push((header, taken));
        let skip = header
            .len()
            .checked_add(length)
            .ok_or_else(|| "a pipelined response is too long".to_owned())?;
        rest = rest
            .get(skip..)
            .ok_or_else(|| "a pipelined response overran".to_owned())?;
    }
    Ok(found)
}

#[test]
fn held_frames_keep_the_hand_computed_grey() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, file LINEAR_EXACT center 100 width 200.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    // Override center 50 width 1: ((50 - 50) / 1 + 0.5) * 255 = 127.5, grey 128.
    // Invert of 64 is 255 - 64 = 191.
    // PS3.3 C.11.2.1.2 and C.11.2.1.3.2. Invert is after the chain.
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    if !header.contains("X-Ocelli-Held: 1") {
        return Err("the file was not held".to_owned());
    }
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (first_header, first_body) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Source: decoded") {
        return Err("the first preview was not decoded".to_owned());
    }
    if first_body != [64] {
        return Err(format!(
            "the held file window was not grey 64: {first_body:?}"
        ));
    }
    let second = exchange(addr, &preview_id(id, ""))?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: held") || second_body != [64] {
        return Err("the second preview did not reuse the held frame".to_owned());
    }
    let narrow = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (narrow_header, narrow_body) = split_http(&narrow)?;
    if !narrow_header.contains("X-Ocelli-Source: held") || narrow_body != [128] {
        return Err("the held override was not grey 128".to_owned());
    }
    let flipped = exchange(addr, &preview_id(id, "X-Ocelli-Invert: 1\r\n"))?;
    let (_, flipped_body) = split_http(&flipped)?;
    if flipped_body != [191] {
        return Err(format!(
            "the held invert was not grey 191: {flipped_body:?}"
        ));
    }
    Ok(())
}

#[test]
fn a_held_missing_window_stays_auto() -> Result<(), String> {
    // Stored 50. Absent rescale is slope 1 and intercept 0. Absent center
    // and width are the constant automatic window, LINEAR_EXACT center 50
    // width 1. ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    // A later request with no window must keep that window. The first
    // request's center 100 width 200 must not stick.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    let file = mono_frame(false)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let overridden = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 100\r\nX-Ocelli-Window-Width: 200\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (over_header, over_body) = split_http(&overridden)?;
    if !over_header.contains("X-Ocelli-Auto-Window: none") || over_body != [64] {
        return Err("the first held window was not grey 64".to_owned());
    }
    let plain = exchange(addr, &preview_id(id, ""))?;
    let (plain_header, plain_body) = split_http(&plain)?;
    if !plain_header.contains("X-Ocelli-Auto-Window: image") || plain_body != [128] {
        return Err("the held file lost its automatic window".to_owned());
    }
    if !plain_header.contains("X-Ocelli-Source: held") {
        return Err("the automatic window preview decoded again".to_owned());
    }
    Ok(())
}

#[test]
fn a_file_post_matches_inspect_and_holds_once() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let inspected = exchange(addr, &http_post_to("/inspect", "", &file))?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (inspect_header, inspect_body) = split_http(&inspected)?;
    let (file_header, file_body) = split_http(&posted)?;
    if inspect_body != file_body {
        return Err("the file response body differed from inspect".to_owned());
    }
    if !file_header.contains("200") || !inspect_header.contains("200") {
        return Err("inspect and file did not both succeed".to_owned());
    }
    let id = header_line(file_header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?;
    let again = exchange(addr, &http_post_to("/file", "", &file))?;
    let (again_header, _) = split_http(&again)?;
    let other = header_line(again_header, "X-Ocelli-File")
        .ok_or_else(|| "the second id was missing".to_owned())?;
    if id == other {
        return Err("a second post reused an id".to_owned());
    }
    let junk = b"not a dicom file";
    let refused = exchange(addr, &http_post_to("/file", "", junk))?;
    let direct = exchange(addr, &http_post_to("/inspect", "", junk))?;
    let (refused_header, refused_body) = split_http(&refused)?;
    let (direct_header, direct_body) = split_http(&direct)?;
    if refused_body != direct_body
        || !refused_header.contains("X-Ocelli-Error: not-part10")
        || !direct_header.contains("X-Ocelli-Error: not-part10")
    {
        return Err("a non Part 10 file was not the same refusal".to_owned());
    }
    if refused_header.contains("X-Ocelli-File:") {
        return Err("a non Part 10 file was given an id".to_owned());
    }
    Ok(())
}

#[test]
fn an_unknown_id_is_gone_and_a_bad_id_is_a_header_error() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let unknown = exchange(addr, &preview_id("1.1", ""))?;
    let (header, body) = split_http(&unknown)?;
    if !header.contains("410") || !header.contains("X-Ocelli-Error: file-gone") {
        return Err("an unknown id was not gone".to_owned());
    }
    if body != b"the file is no longer held by this process" {
        return Err("the gone sentence differed".to_owned());
    }
    let bad = exchange(addr, &preview_id("A.1", ""))?;
    let (bad_header, _) = split_http(&bad)?;
    if !bad_header.contains("400") || !bad_header.contains("X-Ocelli-Error: header") {
        return Err("an uppercase id was not a header error".to_owned());
    }
    let mixed = "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: 1.1\r\nContent-Length: 4\r\n\r\nnope";
    let both = exchange(addr, mixed.as_bytes())?;
    let (both_header, both_body) = split_http(&both)?;
    if !both_header.contains("400") || both_body != b"a file id and a body cannot be sent together"
    {
        return Err("an id with a body was accepted".to_owned());
    }
    Ok(())
}

#[test]
fn two_viewers_do_not_serve_each_others_ids() -> Result<(), String> {
    let left = viewer_on(8 * 1024 * 1024)?;
    let right = viewer_on(8 * 1024 * 1024)?;
    let file = mono_frame(true)?;
    let left_post = exchange(left, &http_post_to("/file", "", &file))?;
    let right_post = exchange(right, &http_post_to("/file", "", &file))?;
    let (left_header, _) = split_http(&left_post)?;
    let (right_header, _) = split_http(&right_post)?;
    let left_id = header_line(left_header, "X-Ocelli-File")
        .ok_or_else(|| "the left id was missing".to_owned())?;
    let right_id = header_line(right_header, "X-Ocelli-File")
        .ok_or_else(|| "the right id was missing".to_owned())?;
    if left_id == right_id {
        return Err("two viewers issued the same id".to_owned());
    }
    let crossed = exchange(right, &preview_id(left_id, ""))?;
    let (crossed_header, _) = split_http(&crossed)?;
    if !crossed_header.contains("410") {
        return Err("another viewer served a foreign id".to_owned());
    }
    let kept = exchange(left, &preview_id(left_id, ""))?;
    let (kept_header, kept_body) = split_http(&kept)?;
    if !kept_header.contains("200") || kept_body != [64] {
        return Err("the owning viewer lost its file".to_owned());
    }
    Ok(())
}

#[test]
fn a_voi_lut_preset_still_holds_the_frame() -> Result<(), String> {
    // The file has a VOI LUT Sequence. A preset does not replace it, so the
    // first answer is 400 window. The frame is still held. A numeric override
    // then draws grey 128 from the held frame. The file is a CT, so the
    // refusal is the LUT one and not F-213's modality one.
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_ct_with(true, &[(0x0028, 0x3010)])?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let preset = exchange(
        addr,
        &preview_id(id, "X-Ocelli-Window-Preset: soft_tissue\r\n"),
    )?;
    let (preset_header, _) = split_http(&preset)?;
    if !preset_header.contains("400") || !preset_header.contains("X-Ocelli-Error: window") {
        return Err("the preset did not refuse the VOI LUT".to_owned());
    }
    let drawn = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (drawn_header, drawn_body) = split_http(&drawn)?;
    if !drawn_header.contains("X-Ocelli-Source: held") || drawn_body != [128] {
        return Err("the override after a preset did not reuse the held frame".to_owned());
    }
    Ok(())
}

/// Insert Image Comments (0020,4000), `len` spaces of LT, before (0028,0002).
///
/// The element changes no pixel and no LUT chain input. It makes the file
/// longer than the held frame's fixed charge, which is what a check that
/// the held bytes are smaller than the file needs.
fn with_image_comments(file: &[u8], len: u16) -> Result<Vec<u8>, String> {
    let marker = [0x28, 0x00, 0x02, 0x00, b'U', b'S'];
    let at = file
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| "the file has no Samples per Pixel element".to_owned())?;
    let comment = short_le((0x0020, 0x4000), *b"LT", &vec![b' '; usize::from(len)])?;
    let mut out = Vec::with_capacity(file.len() + comment.len());
    out.extend_from_slice(file.get(..at).ok_or("the split is outside the file")?);
    out.extend(comment);
    out.extend_from_slice(file.get(at..).ok_or("the split is outside the file")?);
    Ok(out)
}

#[test]
fn single_frame_bytes_drop_and_preview_stays() -> Result<(), String> {
    // The held frame is charged its samples, the struct and its value facts
    // (F-210). The one-pixel file alone is shorter than that charge, so
    // 1024 bytes of Image Comments keep the file longer than the frame and
    // `used < file.len()` still tells the held frame from the encoded file.
    let file = with_image_comments(&mono_frame(true)?, 1024)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let headers = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (headers_header, _) = split_http(&headers)?;
    if !headers_header.contains("200") {
        return Err("headers by id failed before the frame was held".to_owned());
    }
    let _ = exchange(addr, &preview_id(id, ""))?;
    let after = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (after_header, _) = split_http(&after)?;
    if !after_header.contains("410") {
        return Err("single-frame bytes were still held".to_owned());
    }
    let census = held_census(addr)?;
    let used = json_number(&census, "used")?;
    if json_number(&census, "files")? != 0
        || json_number(&census, "frames")? != 1
        || used == 0
        || used >= file.len()
    {
        return Err(format!(
            "held bytes {used} were not the decoded frame alone, file {} census {census}",
            file.len()
        ));
    }
    let again = exchange(addr, &preview_id(id, ""))?;
    let (again_header, again_body) = split_http(&again)?;
    if !again_header.contains("X-Ocelli-Source: held") || again_body != [64] {
        return Err("the held frame was dropped with the bytes".to_owned());
    }
    Ok(())
}

#[test]
fn multi_frame_bytes_drop_only_after_every_frame() -> Result<(), String> {
    let file = enhanced_frames()?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (_, first_body) = split_http(&first)?;
    if first_body != [64] {
        return Err("held frame 0 was not grey 64".to_owned());
    }
    let mid = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (mid_header, _) = split_http(&mid)?;
    if !mid_header.contains("200") {
        return Err("multi-frame bytes dropped before every frame was held".to_owned());
    }
    let second = exchange(addr, &preview_id(id, "X-Ocelli-Frame: 1\r\n"))?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: decoded") || second_body != [128] {
        return Err("held frame 1 was not grey 128".to_owned());
    }
    let end = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (end_header, _) = split_http(&end)?;
    if !end_header.contains("410") {
        return Err("multi-frame bytes stayed after every frame was held".to_owned());
    }
    Ok(())
}

#[test]
fn pipelined_bytes_are_the_next_request() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let file = mono_frame(true)?;
    let mut request = http_post_to("/inspect", "", &file);
    request.extend_from_slice(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    let response = exchange(addr, &request)?;
    let parts = response_parts(&response)?;
    let second = parts
        .get(1)
        .ok_or_else(|| "the second response was missing".to_owned())?;
    if !second.0.contains("200") || !second.1.windows(8).any(|window| window == b"\"slices\"") {
        return Err("the pipelined request was not served".to_owned());
    }
    Ok(())
}

#[test]
fn a_full_connection_is_busy_without_taking_the_slot() -> Result<(), String> {
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") || !opened_header.contains("Connection: keep-alive") {
        return Err("the first connection was not kept".to_owned());
    }
    let busy = exchange(addr, b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (busy_header, busy_body) = split_http(&busy)?;
    if !busy_header.contains("503")
        || !busy_header.contains("X-Ocelli-Error: busy")
        || busy_body != b"this process is already serving its connection limit"
    {
        return Err("a second connection was not busy".to_owned());
    }
    drop(first);
    let mut served = false;
    for _ in 0..20 {
        let again = exchange(addr, b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
        let (again_header, _) = split_http(&again)?;
        if again_header.contains("200") {
            served = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !served {
        return Err("the slot was not released".to_owned());
    }
    Ok(())
}

#[test]
fn held_census_counts_files_and_frames() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let empty = held_census(addr)?;
    if json_number(&empty, "budget")? != 8 * 1024 * 1024 || json_number(&empty, "used")? != 0 {
        return Err(format!("the empty census was {empty}"));
    }
    if json_number(&empty, "inflight")? != 0
        || json_number(&empty, "workspace")? != 0
        || json_number(&empty, "free")? != 0
    {
        return Err(format!("an idle viewer reported live bytes: {empty}"));
    }
    if empty.contains("\"pool\":") {
        return Err(format!("the census named a pool: {empty}"));
    }
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    if !empty.contains(&format!("\"build\":\"{build}\"")) {
        return Err(format!("the build name was not {build}: {empty}"));
    }
    let file = mono_frame(true)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let files = held_census(addr)?;
    if json_number(&files, "files")? != 1 || json_number(&files, "frames")? != 0 {
        return Err(format!("the file census was {files}"));
    }
    let _ = exchange(addr, &preview_id(id, ""))?;
    let frames = held_census(addr)?;
    if json_number(&frames, "files")? != 0 || json_number(&frames, "frames")? != 1 {
        return Err(format!("the frame census was {frames}"));
    }
    if json_number(&frames, "inflight")? != 0 || json_number(&frames, "workspace")? == 0 {
        return Err(format!("the preview census was {frames}"));
    }
    let used = json_number(&frames, "used")?;
    let again = exchange(addr, &preview_id(id, ""))?;
    let _ = split_http(&again)?;
    let census = held_census(addr)?;
    let stable = json_number(&census, "used")?;
    if used != stable || used > 8 * 1024 * 1024 {
        return Err("a second window changed the held bytes".to_owned());
    }
    let tiny = viewer_on(1)?;
    let refused = exchange(tiny, &http_post_to("/file", "", &file))?;
    let (refused_header, _) = split_http(&refused)?;
    if !refused_header.contains("X-Ocelli-Held: 0") || refused_header.contains("X-Ocelli-File:") {
        return Err("a file larger than the budget was held".to_owned());
    }
    let census = held_census(tiny)?;
    if json_number(&census, "used")? != 0
        || json_number(&census, "files")? != 0
        || json_number(&census, "inflight")? != 0
        || json_number(&census, "workspace")? != 0
    {
        return Err(format!("a refused file changed the census: {census}"));
    }
    Ok(())
}

#[test]
fn a_bad_window_does_not_drop_a_held_file() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let bad = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Preset: lung\r\nX-Ocelli-Window-Center: 40\r\n",
        ),
    )?;
    let (bad_header, _) = split_http(&bad)?;
    if !bad_header.contains("400") {
        return Err("a mixed preset was accepted".to_owned());
    }
    let drawn = exchange(addr, &preview_id(id, ""))?;
    let (drawn_header, drawn_body) = split_http(&drawn)?;
    if !drawn_header.contains("200") || drawn_body != [64] {
        return Err("a bad window dropped the file".to_owned());
    }
    Ok(())
}

fn file_id_form(id: &str) -> bool {
    let Some((generation, serial)) = id.split_once('.') else {
        return false;
    };
    let hex = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    };
    hex(generation) && hex(serial) && !id.contains('/')
}

fn status_token(header: &str) -> Result<&str, String> {
    header
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "the status line was short".to_owned())
}

fn ocelli_headers(header: &str) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for line in header.split("\r\n") {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if !key.to_ascii_lowercase().starts_with("x-ocelli-") {
            continue;
        }
        if key.eq_ignore_ascii_case("X-Ocelli-Source") {
            continue;
        }
        rows.push((key.to_ascii_lowercase(), value.trim().to_owned()));
    }
    rows.sort();
    rows
}

fn same_preview(left: &[u8], right: &[u8]) -> Result<(), String> {
    let (left_header, left_body) = split_http(left)?;
    let (right_header, right_body) = split_http(right)?;
    if status_token(left_header)? != status_token(right_header)? || left_body != right_body {
        return Err(format!(
            "status or body differed: {} and {}",
            status_token(left_header)?,
            status_token(right_header)?
        ));
    }
    if ocelli_headers(left_header) != ocelli_headers(right_header) {
        return Err("the X-Ocelli headers differed apart from source".to_owned());
    }
    Ok(())
}

fn same_held_and_body(addr: SocketAddr, file: &[u8], extra: &str) -> Result<(), String> {
    let posted = exchange(addr, &http_post_to("/file", "", file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    if !file_id_form(id) {
        return Err(format!("the file id {id} is not generation.serial"));
    }
    let by_id = exchange(addr, &preview_id(id, extra))?;
    let by_body = exchange(addr, &http_post_to("/preview", extra, file))?;
    same_preview(&by_id, &by_body)
}

#[test]
fn a_posted_file_returns_its_inspect_json_and_an_id() -> Result<(), String> {
    a_file_post_matches_inspect_and_holds_once()?;
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    if !file_id_form(id) {
        return Err(format!("the file id {id} is not generation.serial"));
    }
    Ok(())
}

#[test]
fn a_held_frame_matches_the_posted_frame() -> Result<(), String> {
    let addr = viewer_on(16 * 1024 * 1024)?;
    let mono = mono_frame(true)?;
    same_held_and_body(addr, &mono, "")?;
    same_held_and_body(
        addr,
        &mono,
        "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
    )?;
    same_held_and_body(addr, &mono, "X-Ocelli-Invert: 1\r\n")?;
    let enhanced = enhanced_frames()?;
    same_held_and_body(addr, &enhanced, "")?;
    same_held_and_body(addr, &enhanced, "X-Ocelli-Frame: 1\r\n")?;
    let study = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    same_held_and_body(addr, &study, "X-Ocelli-Window-Preset: lung\r\n")?;
    let modality = mono_with(true, &[(0x0028, 0x3000)])?;
    same_held_and_body(addr, &modality, "")?;
    let voi = mono_with(true, &[(0x0028, 0x3010)])?;
    same_held_and_body(addr, &voi, "")?;
    same_held_and_body(addr, &enhanced, "X-Ocelli-Frame: 2\r\n")?;
    // A file that is not Part 10 is not held, so it has no id form.
    let junk = b"not a dicom file";
    let posted = exchange(addr, &http_post_to("/file", "", junk))?;
    let (posted_header, _) = split_http(&posted)?;
    if posted_header.contains("X-Ocelli-File:") {
        return Err("a non Part 10 file was given an id".to_owned());
    }
    let left = exchange(addr, &http_post_to("/preview", "", junk))?;
    let right = exchange(addr, &http_post_to("/preview", "", junk))?;
    same_preview(&left, &right)?;
    Ok(())
}

#[test]
fn a_window_change_on_a_held_frame_does_not_decode_again() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (first_header, first_body) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Source: decoded") || first_body != [64] {
        return Err("the first held preview did not decode".to_owned());
    }
    let second = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: held") || second_body != [128] {
        return Err("the window change decoded again".to_owned());
    }
    Ok(())
}

#[test]
fn an_evicted_file_answers_gone() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024)?;
    let first = exchange(addr, &http_post_to("/file", "", &file))?;
    let (first_header, _) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Held: 1") {
        return Err("the first file was not held".to_owned());
    }
    let id = header_line(first_header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?;
    let census = held_census(addr)?;
    let used = json_number(&census, "used")?;
    if used == 0 || used > 8 * 1024 {
        return Err(format!("the first file used {used}"));
    }
    // One more file than the budget can hold. Asking early would refresh the
    // first id and keep it.
    let room = (8 * 1024) / used;
    let mut survivor = String::new();
    for _ in 0..room + 2 {
        let posted = exchange(addr, &http_post_to("/file", "", &file))?;
        let (posted_header, _) = split_http(&posted)?;
        if let Some(next) = header_line(posted_header, "X-Ocelli-File") {
            survivor = next.to_owned();
        }
    }
    let answer = exchange(addr, &preview_id(id, ""))?;
    let (answer_header, answer_body) = split_http(&answer)?;
    if !answer_header.contains("410")
        || !answer_header.contains("X-Ocelli-Error: file-gone")
        || answer_body != b"the file is no longer held by this process"
    {
        return Err("the first id was not evicted".to_owned());
    }
    if survivor.is_empty() {
        return Err("no later file was held".to_owned());
    }
    let kept = exchange(addr, &preview_id(&survivor, ""))?;
    let (kept_header, kept_body) = split_http(&kept)?;
    if !kept_header.contains("200") || kept_body != [64] {
        return Err("a later held id stopped answering".to_owned());
    }
    Ok(())
}

#[test]
fn an_id_from_another_process_answers_gone() -> Result<(), String> {
    two_viewers_do_not_serve_each_others_ids()
}

#[test]
fn a_file_larger_than_the_budget_is_not_held() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(1)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    if !header.contains("X-Ocelli-Held: 0") || header.contains("X-Ocelli-File:") {
        return Err("a file larger than the budget was held".to_owned());
    }
    let census = held_census(addr)?;
    if json_number(&census, "used")? != 0
        || json_number(&census, "files")? != 0
        || json_number(&census, "inflight")? != 0
        || json_number(&census, "workspace")? != 0
    {
        return Err(format!("a refused file changed the census: {census}"));
    }
    Ok(())
}

#[test]
fn an_id_with_a_body_is_refused() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let mixed = "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: 1.1\r\nContent-Length: 4\r\n\r\nnope";
    let both = exchange(addr, mixed.as_bytes())?;
    let (header, body) = split_http(&both)?;
    if !header.contains("400")
        || !header.contains("X-Ocelli-Error: header")
        || body != b"a file id and a body cannot be sent together"
    {
        return Err("an id with a body was accepted".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_refusal_keeps_the_held_frame() -> Result<(), String> {
    a_voi_lut_preset_still_holds_the_frame()?;
    a_bad_window_does_not_drop_a_held_file()
}

#[test]
fn keep_alive_serves_two_requests() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(b"GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let first = read_one(&mut stream)?;
    let (first_header, _) = split_http(&first)?;
    if !first_header.contains("200") || !first_header.contains("Connection: keep-alive") {
        return Err("the first request was not kept".to_owned());
    }
    stream
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let second = read_one(&mut stream)?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("200")
        || !second_body.windows(8).any(|window| window == b"\"slices\"")
    {
        return Err("the second request on the same connection was not served".to_owned());
    }
    Ok(())
}

#[test]
fn pipelined_requests_get_two_replies() -> Result<(), String> {
    pipelined_bytes_are_the_next_request()
}

#[test]
fn a_connection_over_the_cap_is_busy() -> Result<(), String> {
    a_full_connection_is_busy_without_taking_the_slot()
}

fn span_frame() -> Result<Vec<u8>, String> {
    // 1 by 4, stored 0, 27, 199 and 200. No Window Center, Window Width or function.
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&0_u16.to_le_bytes());
    pixels.extend_from_slice(&27_u16.to_le_bytes());
    pixels.extend_from_slice(&199_u16.to_le_bytes());
    pixels.extend_from_slice(&200_u16.to_le_bytes());
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

#[test]
fn a_function_sent_alone_replaces_the_automatic_default() -> Result<(), String> {
    // Stored 0, 27, 199 and 200. Rescale slope 1, intercept 0. F-227 leaves
    // stored 0 out, so the counted values are 27, 199 and 200, n = 3, and the
    // nearest ranks ceil(3 / 100) = 1 and ceil(297 / 100) = 3 are 27 and 200.
    // The automatic window is center 113.5 and width 173, moved inside by at
    // most 16 f32 units at each edge.
    // PS3.3 C.11.2.1.3.2 LINEAR_EXACT, the default. Stored 0 and 27 are at or
    // below the lower edge 27, grey 0. Stored 199:
    // ((199 - 113.5) / 173 + 0.5) * 255 = 253.53, grey 254. Stored 200 is the
    // upper edge, grey 255.
    // PS3.3 C.11.2.1.2 LINEAR, sent alone, uses c' = 113 and w' = 172, so its
    // upper edge is c' + w' / 2 = 199. Stored 199 is not above it, and
    // ((199 - 113) / 172 + 0.5) * 255 = 255, grey 255. Its lower edge is
    // c' - w' / 2 = 27, so stored 27 is grey 0.
    let file = span_frame()?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let linear = exchange(
        addr,
        &preview_id(id, "X-Ocelli-Window-Function: LINEAR\r\n"),
    )?;
    let (linear_header, linear_body) = split_http(&linear)?;
    if header_line(linear_header, "X-Ocelli-Window-Function") != Some("LINEAR")
        || header_line(linear_header, "X-Ocelli-Auto-Window") != Some("image")
        || linear_body != [0, 0, 255, 255]
    {
        return Err(format!(
            "LINEAR alone was not grey 0, 0, 255, 255: {linear_header} body {linear_body:?}"
        ));
    }
    let exact = exchange(addr, &preview_id(id, ""))?;
    let (exact_header, exact_body) = split_http(&exact)?;
    if header_line(exact_header, "X-Ocelli-Window-Function") != Some("LINEAR_EXACT")
        || header_line(exact_header, "X-Ocelli-Auto-Window") != Some("image")
        || exact_body != [0, 0, 254, 255]
    {
        return Err(format!(
            "the automatic default was not grey 0, 0, 254, 255: {exact_header} body {exact_body:?}"
        ));
    }
    for (name, want) in [("Center", 113.5), ("Width", 173.0)] {
        let text = header_line(exact_header, &format!("X-Ocelli-Window-{name}"))
            .ok_or_else(|| format!("no window {name}"))?;
        close_to(text, want)?;
    }
    Ok(())
}

#[test]
fn a_busy_post_is_refused_before_its_body() -> Result<(), String> {
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") {
        return Err("the first connection was not served".to_owned());
    }
    let mut second = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    second
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let started = std::time::Instant::now();
    second
        .write_all(b"POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2000000\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let response = read_one(&mut second)?;
    if started.elapsed() > Duration::from_secs(2) {
        return Err("the busy reply waited for the body".to_owned());
    }
    let (header, body) = split_http(&response)?;
    if !header.contains("503")
        || !header.contains("X-Ocelli-Error: busy")
        || !header.contains("Connection: close")
        || body != b"this process is already serving its connection limit"
    {
        return Err("a header-only post over the cap was not busy".to_owned());
    }
    Ok(())
}

#[test]
fn a_busy_post_with_a_body_is_still_readable() -> Result<(), String> {
    // requestGrey retries only when the status is 503, X-Ocelli-Error is
    // busy, and the body can be read. A reset never reaches that branch.
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") {
        return Err("the first connection was not served".to_owned());
    }
    let mut second = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    second
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let payload = vec![0xA5_u8; 32 * 1024];
    let mut request = format!(
        "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    request.extend_from_slice(&payload);
    let started = std::time::Instant::now();
    second
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    let response = read_one(&mut second)?;
    if started.elapsed() > Duration::from_secs(2) {
        return Err("the busy reply waited on the whole declared body".to_owned());
    }
    let (header, body) = split_http(&response)?;
    if !header.contains("503")
        || !header.contains("X-Ocelli-Error: busy")
        || !header.contains("Connection: close")
        || body != b"this process is already serving its connection limit"
    {
        return Err(format!(
            "a posted body over the cap was not a readable 503: {header} body {body:?}"
        ));
    }
    // The payload is 32 KiB, under BUSY_DRAIN_CAP. The 503 must be followed by a clean close.
    // A body above the cap is outside this assertion.
    let mut extra = [0_u8; 1];
    match second.read(&mut extra) {
        Ok(0) => {}
        Ok(n) => return Err(format!("bytes followed the 503: {n}")),
        Err(error) => return Err(format!("the 503 was reset: {error}")),
    }
    Ok(())
}

#[test]
fn the_page_names_the_release_build() -> Result<(), String> {
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    if !page.contains("cargo run --release -p ocelli-folder-viewer") {
        return Err("the page does not name the release build".to_owned());
    }
    if !page.contains("This is the slow debug build.") {
        return Err("the page does not name the debug build".to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// F-209. Plane tokens and `POST /link`. Every expected number is worked by
// hand from PS3.3 C.7.6.2.1.1, Equation C.7.6.2.1-1, with Pixel Spacing in
// PS3.3 10.7.1.3's order, row spacing first, and from the cornerstone3D
// 5.8.2 rules transcribed in `.claude/plans/F-209-design.md`. None is read
// from a previous run of the route.

/// A one-frame object with the plane attributes given as DS text.
fn plane_object(
    position: &str,
    orientation: &str,
    pixel: Option<&str>,
    imager: Option<&str>,
    frame_of_reference: Option<&str>,
    rows: u16,
    columns: u16,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad("CT"))?);
    if let Some(pair) = imager {
        data.extend(short_le((0x0018, 0x1164), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0020, 0x000E), *b"UI", &ui(b"2.25.9"))?);
    data.extend(short_le((0x0020, 0x0032), *b"DS", &text_pad(position))?);
    data.extend(short_le((0x0020, 0x0037), *b"DS", &text_pad(orientation))?);
    if let Some(uid) = frame_of_reference {
        data.extend(short_le((0x0020, 0x0052), *b"UI", &ui(uid.as_bytes()))?);
    }
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &rows.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &columns.to_le_bytes())?);
    if let Some(pair) = pixel {
        data.extend(short_le((0x0028, 0x0030), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("1"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    let samples = usize::from(rows) * usize::from(columns);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &vec![0_u8; samples * 2])?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

/// The text between `"planes":` and the closing `]`, inclusive.
fn planes_field(json: &str) -> Result<&str, String> {
    let at = json
        .find("\"planes\":")
        .ok_or_else(|| "no planes field".to_owned())?;
    let rest = json.get(at..).ok_or_else(|| "planes".to_owned())?;
    let end = rest
        .find(']')
        .ok_or_else(|| "planes do not close".to_owned())?;
    rest.get(..=end).ok_or_else(|| "planes".to_owned())
}

/// The first token in `"planes":["..."]`.
fn first_token(json: &str) -> Result<String, String> {
    let field = planes_field(json)?;
    let token = field
        .strip_prefix("\"planes\":[\"")
        .and_then(|rest| rest.split('"').next())
        .ok_or_else(|| format!("no token in {field}"))?;
    Ok(token.to_owned())
}

#[test]
fn inspect_reports_the_plane_token() -> Result<(), String> {
    // AC8. Synthetic slice 3 is IPP `0\0\7.5`, IOP `1\0\0\0\1\0`, Pixel
    // Spacing `0.7\0.5` (row 0.7, column 0.5, PS3.3 10.7.1.3), 96 rows and
    // 128 columns. `{:e}` writes 0 as `0e0`, 7.5 as `7.5e0`, 1 as `1e0`,
    // 0.7 as `7e-1` and 0.5 as `5e-1`.
    let want = "\"planes\":[\"0e0,0e0,7.5e0,1e0,0e0,0e0,0e0,1e0,0e0,7e-1,5e-1,96,128\"]";
    let file = slice(3).ok_or_else(|| "slice 3 was not built".to_owned())?;
    let json = inspect_json(&file, 0).map_err(|_| "inspect refused slice 3".to_owned())?;
    if !json.contains("\"frameOfReferenceUid\":\"2.25.44000\"") || !json.contains(want) {
        return Err(format!("slice 3 reported {json}"));
    }
    // The same fields reach the page through `POST /file`.
    let response = one_exchange(&http_post_to("/file", "", &file))?;
    let (header, body) = split_http(&response)?;
    let body = std::str::from_utf8(body).map_err(|_| "the reply is not text".to_owned())?;
    if !header.contains("200") || body != json {
        return Err(format!("POST /file did not carry the plane: {header}"));
    }
    // AC9. Imager Pixel Spacing alone is a detector-plane spacing, PS3.3
    // C.8.11.4, so `FrameGeometry::derive` refuses it and there is no plane.
    // The control, the same object with Pixel Spacing, has one.
    let imager = plane_object(
        "0\\0\\0",
        "1\\0\\0\\0\\1\\0",
        None,
        Some("0.3\\0.2"),
        None,
        1,
        1,
    )?;
    let imager_json =
        inspect_json(&imager, 0).map_err(|_| "inspect refused imager spacing".to_owned())?;
    if planes_field(&imager_json)? != "\"planes\":[null]" {
        return Err(format!("imager spacing made a plane: {imager_json}"));
    }
    let pixel = plane_object(
        "0\\0\\0",
        "1\\0\\0\\0\\1\\0",
        Some("0.3\\0.2"),
        None,
        None,
        1,
        1,
    )?;
    let pixel_json = inspect_json(&pixel, 0).map_err(|_| "inspect refused".to_owned())?;
    if planes_field(&pixel_json)?
        != "\"planes\":[\"0e0,0e0,0e0,1e0,0e0,0e0,0e0,1e0,0e0,3e-1,2e-1,1,1\"]"
    {
        return Err(format!("pixel spacing made no plane: {pixel_json}"));
    }
    // An absent UID and one the page could not send back are both "".
    // PS3.5 9.1: a UID is digits and `.`, at most 64 characters.
    if !pixel_json.contains("\"frameOfReferenceUid\":\"\"") {
        return Err("an absent UID was not empty".to_owned());
    }
    for (uid, want) in [
        ("1.2.840.99", "\"frameOfReferenceUid\":\"1.2.840.99\""),
        ("1.2.x4", "\"frameOfReferenceUid\":\"\""),
        (&"1".repeat(65), "\"frameOfReferenceUid\":\"\""),
    ] {
        let file = plane_object(
            "0\\0\\0",
            "1\\0\\0\\0\\1\\0",
            Some("1\\1"),
            None,
            Some(uid),
            1,
            1,
        )?;
        let json = inspect_json(&file, 0).map_err(|_| "inspect refused a UID".to_owned())?;
        if !json.contains(want) {
            return Err(format!("UID {uid} reported {json}"));
        }
    }
    // One entry per declared frame. This object has no plane attributes.
    let two = tagged_frame(2, &[0, 0, 0, 0])?;
    let two_json = inspect_json(&two, 0).map_err(|_| "inspect refused two frames".to_owned())?;
    if planes_field(&two_json)? != "\"planes\":[null,null]" {
        return Err(format!("two frames reported {two_json}"));
    }
    Ok(())
}

#[test]
fn the_frame_list_caps_agree() -> Result<(), String> {
    // AC15. The page lists at most this many images per file and the
    // process writes at most this many planes for it.
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (_, body) = split_http(&response)?;
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    if !page.contains(&format!("\nconst FRAME_LIST_CAP = {FRAME_LIST_CAP};\n")) {
        return Err("the page and the process disagree on FRAME_LIST_CAP".to_owned());
    }
    // The link body cap counts MAX_SIDE squared cells.
    if !page.contains("\nconst MAX_SIDE = 3;\n") {
        return Err("the page's MAX_SIDE is not the 3 the link cap counts".to_owned());
    }
    Ok(())
}

#[test]
fn a_plane_field_is_at_most_24_characters() -> Result<(), String> {
    // `{:e}` writes the shortest digits that read back to the same bits:
    // at most 17 significant digits, so a sign, one digit, a point, 16
    // digits and a three-digit negative exponent, 1 + 1 + 1 + 16 + 5 = 24.
    let extremes = [
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        -2.225_073_858_507_201_4e-308,
        -1.234_567_890_123_456_7e-300,
        5e-324,
        -5e-324,
        0.1 + 0.2,
        -0.0,
        f64::EPSILON,
    ];
    let mut longest = 0;
    for value in extremes {
        let text = format!("{value:e}");
        let back: f64 = text
            .parse()
            .map_err(|_| format!("{text} did not read back"))?;
        if back.to_bits() != value.to_bits() {
            return Err(format!("{text} read back as another value"));
        }
        longest = longest.max(text.len());
    }
    if longest != 24 {
        return Err(format!("the longest field was {longest} characters"));
    }
    // A field one character longer is not a token field.
    let wide = format!(
        "{},0e0,0e0,1e0,0e0,0e0,0e0,1e0,0e0,1e0,1e0,1,1",
        "0".repeat(25)
    );
    if plane_from_token(&wide).is_some() {
        return Err("a 25 character field was read".to_owned());
    }
    let narrow = format!(
        "{},0e0,0e0,1e0,0e0,0e0,0e0,1e0,0e0,1e0,1e0,1,1",
        "0".repeat(24)
    );
    if plane_from_token(&narrow).is_none() {
        return Err("a 24 character field was refused".to_owned());
    }
    Ok(())
}

#[test]
fn a_plane_token_round_trips() -> Result<(), String> {
    // AC14. An oblique plane whose numbers need every digit. Row (0.6, 0.48,
    // 0.64) and column (0.8, -0.36, -0.48) are unit and orthogonal:
    // 0.36 + 0.2304 + 0.4096 = 1, 0.64 + 0.1296 + 0.2304 = 1 and
    // 0.48 - 0.1728 - 0.3072 = 0. The four corners of a 2 by 3 image are
    // compared, PS3.3 C.7.6.2.1.1 with i to 2 and j to 1.
    let file = plane_object(
        "-123.456789\\87.654321\\-0.000123",
        "0.6\\0.48\\0.64\\0.8\\-0.36\\-0.48",
        Some("0.488281\\0.3125"),
        None,
        Some("2.25.7"),
        2,
        3,
    )?;
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the oblique plane".to_owned())?;
    let token = first_token(&json)?;
    let read = plane_from_token(&token).ok_or_else(|| format!("{token} was not read back"))?;
    let parsed = parse_part10(&file).map_err(|_| "the object did not parse".to_owned())?;
    let metadata = MetadataSet::from_object(parsed.object())
        .map_err(|_| "the metadata did not project".to_owned())?;
    let multi = MultiframeMetadata::new(&metadata).map_err(|_| "no multiframe view".to_owned())?;
    let derived = FrameGeometry::derive(&multi, 0)
        .map_err(|error| format!("derive refused: {error:?}"))?
        .plane();
    if plane_token(&derived) != token {
        return Err("the token is not the derived plane's".to_owned());
    }
    for (column, row) in [(0.0, 0.0), (2.0, 0.0), (0.0, 1.0), (2.0, 1.0)] {
        let corner = Pt::<Index>::new(column, row, 0.0);
        let want = derived.index_to_world().apply(corner);
        let got = read.index_to_world().apply(corner);
        let gap = (want.x - got.x)
            .abs()
            .max((want.y - got.y).abs())
            .max((want.z - got.z).abs());
        // A NaN gap compares as nothing and is refused too.
        if gap.partial_cmp(&1e-9).is_none_or(|order| order.is_gt()) {
            return Err(format!("corner ({column}, {row}) moved {gap} mm"));
        }
    }
    Ok(())
}

/// Synthetic slice `index`'s token, from `inspect_json`.
fn synthetic_token(index: u32) -> Result<String, String> {
    let file = slice(index).ok_or_else(|| format!("slice {index} was not built"))?;
    first_token(&inspect_json(&file, 0).map_err(|_| "inspect refused".to_owned())?)
}

/// The coronal plane `C` of the design: IPP `0\20\30`, IOP `1\0\0\0\0\-1`,
/// Pixel Spacing `2.5\0.5`, 13 rows and 128 columns, written by hand.
const CORONAL: &str = "0e0,2e1,3e1,1e0,0e0,0e0,0e0,0e0,-1e0,2.5e0,5e-1,13,128";

/// An axial plane of the synthetic geometry at height `z`, written by hand.
fn axial_token(z: &str) -> String {
    format!("0e0,0e0,{z},1e0,0e0,0e0,0e0,1e0,0e0,7e-1,5e-1,96,128")
}

fn link_viewers() -> Result<[(&'static str, SocketAddr); 2], String> {
    let start = |viewer: Viewer| -> Result<SocketAddr, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let addr = listener.local_addr().map_err(|error| error.to_string())?;
        let viewer = Arc::new(viewer);
        std::thread::spawn(move || {
            let _ = serve_on(listener, viewer);
        });
        Ok(addr)
    };
    let config = || ViewerConfig {
        cache_bytes: 8 * 1024 * 1024,
    };
    Ok([
        ("direct", start(Viewer::new(config()))?),
        ("server", start(Viewer::server(config()))?),
    ])
}

fn post_link(addr: SocketAddr, body: &[u8]) -> Result<(String, String), String> {
    let response = exchange(addr, &http_post_to("/link", "", body))?;
    let (header, body) = split_http(&response)?;
    Ok((
        header.to_owned(),
        String::from_utf8_lossy(body).into_owned(),
    ))
}

fn expect_link(addr: SocketAddr, name: &str, body: &str, want: &str) -> Result<(), String> {
    let (header, reply) = post_link(addr, body.as_bytes())?;
    if !header.starts_with("HTTP/1.1 200 ") || reply != want {
        return Err(format!("{name}: {header} {reply}"));
    }
    Ok(())
}

#[test]
fn link_route_draws_the_axial_level() -> Result<(), String> {
    // AC10. Source synthetic slice 2, IPP z = 5, against `C` in cell 1.
    // Corners TL (0,0,5), TR (127 * 0.5, 0, 5) = (63.5,0,5), BL (0, 95 *
    // 0.7, 5) = (0,66.5,5). TL - BL normalised is (0,-1,0), whose dot with
    // `C`'s normal (0,1,0) is -1, so the edges are TL to BL and TR to BR,
    // cut by y = 20 at (0,20,5) and (63.5,20,5). In `C`'s index space
    // (P - S) = (0,0,-25) and (63.5,0,-25): column 0 / 0.5 = 0 and
    // 63.5 / 0.5 = 127, row 25 / 2.5 = 10. The page puts a pixel centre at
    // index + 0.5, so (0.5,10.5) and (127.5,10.5). The normals have dot 0,
    // so scroll is refused as not coplanar and the cell does not move.
    let source = synthetic_token(2)?;
    if source != axial_token("5e0") {
        return Err(format!("slice 2 is {source}"));
    }
    let body = format!("source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {CORONAL}");
    let want = r#"{"targets":[{"cell":1,"index":null,"line":[0.5,10.5,127.5,10.5],"reason":"not-coplanar"}]}"#;
    for (name, addr) in link_viewers()? {
        expect_link(addr, name, &body, want)?;
        // An identical repeat is the same answer.
        expect_link(addr, name, &body, want)?;
        // With scroll off the line is drawn and no reason is given.
        let off = format!("source 2.25.44000 {source}\ntarget 1 0 0 2.25.44000 {CORONAL}\n");
        expect_link(
            addr,
            name,
            &off,
            r#"{"targets":[{"cell":1,"index":null,"line":[0.5,10.5,127.5,10.5],"reason":""}]}"#,
        )?;
    }
    Ok(())
}

#[test]
fn link_route_scrolls_to_the_nearest_position() -> Result<(), String> {
    // A target stack at z = 0, 2.5 and 5 showing image 0. The source is at
    // z = 5, distances 5, 2.5 and 0, so it moves to image 2. Both stacks
    // are axial, dot 1, so the line is refused as parallel.
    let source = axial_token("5e0");
    let stack = [axial_token("0e0"), axial_token("2.5e0"), axial_token("5e0")];
    let body = format!(
        "source 2.25.44000 {source}\ntarget 4 0 1 2.25.44000 {} 2.25.44000 {} 2.25.44000 {}",
        stack[0], stack[1], stack[2]
    );
    for (name, addr) in link_viewers()? {
        expect_link(
            addr,
            name,
            &body,
            r#"{"targets":[{"cell":4,"index":2,"line":null,"reason":"parallel"}]}"#,
        )?;
        // Already on the nearest image: it does not move.
        let still = body.replacen("target 4 0 1", "target 4 2 1", 1);
        expect_link(
            addr,
            name,
            &still,
            r#"{"targets":[{"cell":4,"index":null,"line":null,"reason":"parallel"}]}"#,
        )?;
    }
    Ok(())
}

#[test]
fn link_route_refuses_another_frame() -> Result<(), String> {
    // AC11. Different UIDs are different frames of reference, PS3.3
    // C.7.4.1.1.1, and no registration is done. A missing source UID links
    // nothing either.
    let source = axial_token("5e0");
    let want = r#"{"targets":[{"cell":1,"index":null,"line":null,"reason":"frame-of-reference"}]}"#;
    for (name, addr) in link_viewers()? {
        let other = format!("source 2.25.44000 {source}\ntarget 1 0 1 2.25.44001 {CORONAL}");
        expect_link(addr, name, &other, want)?;
        let missing = format!("source - {source}\ntarget 1 0 1 2.25.44000 {CORONAL}");
        expect_link(addr, name, &missing, want)?;
        // One listed image in another frame refuses the whole target.
        let mixed = format!(
            "source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {CORONAL} 2.25.44001 {CORONAL}"
        );
        expect_link(addr, name, &mixed, want)?;
    }
    Ok(())
}

#[test]
fn link_route_refuses_a_missing_plane() -> Result<(), String> {
    // AC12. Without the gap the target at z = 0, 2.5 and 5 moves to image 2,
    // as above. With the middle plane missing it moves nowhere, because
    // skipping the gap would move it on a list it cannot place.
    let source = axial_token("5e0");
    let body = format!(
        "source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {} 2.25.44000 - 2.25.44000 {}",
        axial_token("0e0"),
        axial_token("5e0")
    );
    let want = r#"{"targets":[{"cell":1,"index":null,"line":null,"reason":"no-plane"}]}"#;
    for (name, addr) in link_viewers()? {
        expect_link(addr, name, &body, want)?;
        let sourceless = format!("source 2.25.44000 -\ntarget 1 0 1 2.25.44000 {CORONAL}");
        expect_link(addr, name, &sourceless, want)?;
    }
    Ok(())
}

#[test]
fn link_route_answers_each_target() -> Result<(), String> {
    // Two targets in one body: cell 2 another frame, cell 5 `C`.
    let source = axial_token("5e0");
    let body = format!(
        "source 2.25.44000 {source}\ntarget 2 0 0 2.25.1 {CORONAL}\ntarget 5 0 0 2.25.44000 {CORONAL}"
    );
    let want = concat!(
        r#"{"targets":[{"cell":2,"index":null,"line":null,"reason":"frame-of-reference"},"#,
        r#"{"cell":5,"index":null,"line":[0.5,10.5,127.5,10.5],"reason":""}]}"#
    );
    for (name, addr) in link_viewers()? {
        expect_link(addr, name, &body, want)?;
        // A source with no target is an empty answer.
        expect_link(
            addr,
            name,
            &format!("source - {source}"),
            r#"{"targets":[]}"#,
        )?;
    }
    Ok(())
}

#[test]
fn link_route_refuses_a_malformed_body() -> Result<(), String> {
    // AC13. The page sends only text this process wrote, so anything else
    // is 400 `link` and links nothing.
    let source = axial_token("5e0");
    let good = format!("source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {CORONAL}");
    let non_unit = "0e0,0e0,0e0,2e0,0e0,0e0,0e0,1e0,0e0,7e-1,5e-1,96,128";
    let wide_field = format!(
        "{},0e0,0e0,1e0,0e0,0e0,0e0,1e0,0e0,7e-1,5e-1,96,128",
        "0".repeat(25)
    );
    let cases: Vec<(&str, String)> = vec![
        ("empty", String::new()),
        ("no source", format!("target 1 0 1 2.25.44000 {CORONAL}")),
        (
            "two sources",
            format!("source - {source}\nsource - {source}"),
        ),
        (
            "an unknown word",
            format!("{good}\nmove 1 0 1 2.25.44000 {CORONAL}"),
        ),
        (
            "a source field more",
            format!("source 2.25.44000 {source} 1\ntarget 1 0 1 - -"),
        ),
        ("a source field less", "source 2.25.44000".to_owned()),
        ("a double space", good.replacen("target 1", "target  1", 1)),
        (
            "a blank line",
            format!("source - {source}\n\ntarget 1 0 1 - -"),
        ),
        ("two trailing newlines", format!("{good}\n\n")),
        ("a carriage return", good.replace('\n', "\r\n")),
        (
            "a non-ASCII byte",
            good.replacen("2.25.44000", "2.25.4400\u{00e9}", 1),
        ),
        (
            "a UID with a letter",
            good.replacen("2.25.44000", "2.25.4400a", 1),
        ),
        (
            "a 65 character UID",
            good.replacen("2.25.44000", &"1".repeat(65), 1),
        ),
        ("a non-unit cosine", good.replacen(CORONAL, non_unit, 1)),
        (
            "a 25 character field",
            good.replacen(CORONAL, &wide_field, 1),
        ),
        ("twelve fields", good.replacen(",13,128", ",13", 1)),
        ("fourteen fields", good.replacen(",13,128", ",13,128,1", 1)),
        ("not a number", good.replacen("2.5e0", "x", 1)),
        ("infinity", good.replacen("2e1", "inf", 1)),
        ("zero rows", good.replacen(",13,128", ",0,128", 1)),
        (
            "a zero spacing on 13 rows",
            good.replacen("2.5e0", "0e0", 1),
        ),
        (
            "a uid without a plane",
            format!("source - {source}\ntarget 1 0 1 2.25.44000"),
        ),
        ("no image", format!("source - {source}\ntarget 1 0 1")),
        (
            "shown out of range",
            good.replacen("target 1 0 1", "target 1 1 1", 1),
        ),
        (
            "a negative shown",
            good.replacen("target 1 0 1", "target 1 -1 1", 1),
        ),
        ("cell 9", good.replacen("target 1 0 1", "target 9 0 1", 1)),
        (
            "a repeated cell",
            format!("{good}\ntarget 1 0 1 2.25.44000 {CORONAL}"),
        ),
        ("scroll 2", good.replacen("target 1 0 1", "target 1 0 2", 1)),
        (
            "a file",
            String::from_utf8_lossy(&slice(2).unwrap_or_default()).into_owned(),
        ),
    ];
    for (name, addr) in link_viewers()? {
        for (case, body) in &cases {
            let (header, _) = post_link(addr, body.as_bytes())?;
            if !header.starts_with("HTTP/1.1 400 ")
                || header_line(&header, "X-Ocelli-Error") != Some("link")
            {
                return Err(format!("{name} {case}: {header}"));
            }
        }
        // The control: the well-formed body, with one trailing newline.
        let (header, _) = post_link(addr, format!("{good}\n").as_bytes())?;
        if !header.starts_with("HTTP/1.1 200 ") {
            return Err(format!("{name}: the good body was refused: {header}"));
        }
    }
    Ok(())
}

/// One `f64` field at its longest, `{:e}`'s 24 characters, written with
/// padding zeros so it still reads back to `digit` times ten to `exponent`.
fn widest(digit: char, exponent: &str) -> String {
    let fixed = 1 + 1 + 1 + exponent.len();
    format!("{digit}.{}e{exponent}", "0".repeat(24 - fixed))
}

#[test]
fn link_body_cap_admits_the_largest_body() -> Result<(), String> {
    // AC13, the declared-length cap. The largest body the page can send:
    // a 64 character UID on every line, PS3.5 Table 6.2-1, a token whose
    // eleven `f64` fields are 24 characters and whose two `u32` fields are
    // 10, eight targets, which is MAX_SIDE 3 squared less the source, each
    // listing FRAME_LIST_CAP images and showing the last, and a newline after
    // every line. It is accepted whole, and one byte more is refused from
    // the declared length before any body is sent.
    let uid = format!("2.25.{}", "1".repeat(59));
    let zero = widest('0', "0");
    let one = widest('1', "0");
    let token = [
        zero.clone(),
        zero.clone(),
        zero.clone(),
        one.clone(),
        zero.clone(),
        zero.clone(),
        zero.clone(),
        one,
        zero,
        widest('7', "-1"),
        widest('5', "-1"),
        "4294967295".to_owned(),
        "4294967295".to_owned(),
    ]
    .join(",");
    if uid.len() != 64
        || token
            .split(',')
            .any(|field| field.len() != 24 && field.len() != 10)
    {
        return Err("the largest fields are not their longest".to_owned());
    }
    let image = format!(" {uid} {token}");
    let shown = FRAME_LIST_CAP - 1;
    let mut body = format!("source {uid} {token}\n");
    for cell in 1..=8 {
        body.push_str(&format!("target {cell} {shown} 1"));
        for _ in 0..FRAME_LIST_CAP {
            body.push_str(&image);
        }
        body.push('\n');
    }
    for (name, addr) in link_viewers()? {
        let mut request = format!(
            "POST /link HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
            len = body.len()
        )
        .into_bytes();
        request.extend_from_slice(body.as_bytes());
        let response = long_exchange(addr, &request)?;
        let (header, reply) = split_http(&response)?;
        // Every image is at the same position, so each target moves to the
        // first, index 0, and the axial planes are parallel.
        let reply = std::str::from_utf8(reply).map_err(|_| "the reply is not text".to_owned())?;
        if !header.starts_with("HTTP/1.1 200 ")
            || !reply
                .starts_with(r#"{"targets":[{"cell":1,"index":0,"line":null,"reason":"parallel"}"#)
        {
            return Err(format!("{name}: the largest body was refused: {header}"));
        }
        let over = format!(
            "POST /link HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
            len = body.len() + 1
        );
        let response = exchange(addr, over.as_bytes())?;
        let (header, reply) = split_http(&response)?;
        if !header.starts_with("HTTP/1.1 400 ")
            || header_line(header, "X-Ocelli-Error") != Some("link")
            || reply != b"the link request is too large"
        {
            return Err(format!("{name}: one byte over was not refused: {header}"));
        }
    }
    Ok(())
}

/// `exchange` with time for a body of a few hundred megabytes.
fn long_exchange(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(300)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(request)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    Ok(response)
}

/// The text of `function <name>(` up to the first line that is `}`.
fn function_body<'a>(page: &'a str, name: &str) -> Result<&'a str, String> {
    let at = page
        .find(&format!("\nfunction {name}("))
        .ok_or_else(|| format!("the page has no {name}"))?;
    page.get(at..)
        .and_then(|rest| rest.find("\n}\n").and_then(|end| rest.get(..end)))
        .ok_or_else(|| format!("{name} does not close"))
}

#[test]
fn the_page_links_viewports() -> Result<(), String> {
    // AC15. The page text is read, not executed. B1 to B3 are separate.
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    for needle in [
        "fetch(\"/link\"",
        "id=\"link\"",
        "id=\"link-menu\"",
        "id=\"link-scroll\"",
        "id=\"link-window\"",
        "id=\"link-zoom\"",
        "id=\"toggle-reference\"",
        "class=\"ref-lines\"",
        "Window not applicable",
        "This image is colour, so the file's windows and the presets do not apply. Drag to window each channel.",
    ] {
        if !page.contains(needle) {
            return Err(format!("the page does not link viewports: {needle}"));
        }
    }
    // The line layer is a sibling of ink inside the stage, before it, so
    // marks draw over the line and nothing that rebuilds ink erases it.
    let template_at = page
        .find("<template id=\"cell-template\">")
        .ok_or_else(|| "no cell template".to_owned())?;
    let template = page
        .get(template_at..)
        .and_then(|rest| rest.find("</template>").and_then(|end| rest.get(..end)))
        .ok_or_else(|| "the template does not close".to_owned())?;
    let refs = template
        .find("<svg class=\"ref-lines\"")
        .ok_or_else(|| "the template has no line layer".to_owned())?;
    let ink = template
        .find("<svg class=\"ink\"")
        .ok_or_else(|| "the template has no ink".to_owned())?;
    let ink_close = template
        .get(refs..)
        .and_then(|rest| rest.find("</svg>"))
        .map(|end| refs + end);
    if refs > ink || ink_close.is_none_or(|end| end > ink) {
        return Err("the line layer is not a sibling before ink".to_owned());
    }
    if function_body(page, "renderInk")?.contains("ref-lines")
        || function_body(page, "renderInk")?.contains(".refs")
    {
        return Err("renderInk writes the line layer".to_owned());
    }
    // The page never reads a plane token: no `.planes[` is followed by a
    // `.split(` on the same line, and no token is split anywhere.
    for line in page.lines() {
        if let Some(at) = line.find(".planes[")
            && line.get(at..).is_some_and(|rest| rest.contains(".split("))
        {
            return Err(format!("the page splits a plane token: {line}"));
        }
    }
    if page.contains("token.split(") || page.contains("planeToken(image).split(") {
        return Err("the page splits a plane token".to_owned());
    }
    // No key is bound for the link, so F-213 keeps the digits.
    if function_body(page, "onKey")?.contains("setLink(")
        || function_body(page, "onKey")?.contains("setReference(")
    {
        return Err("onKey binds a link key".to_owned());
    }
    Ok(())
}

/// A tilted plane at height `z`: row (1,0,0), column (0,0.96,0.28), Pixel
/// Spacing `1\1`, 13 rows and 128 columns. 0.96^2 + 0.28^2 = 1.
fn tilted_token(z: &str) -> String {
    format!("0e0,0e0,{z},1e0,0e0,0e0,0e0,9.6e-1,2.8e-1,1e0,1e0,13,128")
}

#[test]
fn link_route_draws_the_line_on_the_moved_image() -> Result<(), String> {
    // Plan row 297: the line is computed against the image the target will
    // show. The tilted normal is (1,0,0) x (0,0.96,0.28) = (0,-0.28,0.96),
    // dot 0.96 with the axial source's (0,0,1), which is above 0.9, so the
    // stack at z = 0, 5 and 10 scrolls. The source is at z = 5, distances
    // 5, 0 and 5, so it moves from image 0 to image 1.
    // On image 1, S = (0,0,5), the plane is -0.28 y + 0.96 z = 4.8. The
    // source corners at z = 5 meet it at y = 0. TL - BL is (0,-1,0) after
    // normalising, dot 0.28 with the normal, so the edges are TL to BL and
    // TR to BR, cut at (0,0,5) and (63.5,0,5). P - S is (0,0,0) and
    // (63.5,0,0). The target's column spacing is 1 mm, so column 0 and
    // 63.5 on row 0, which the page draws at (0.5,0.5) and (64,0.5).
    // On the shown image 0 the plane is -0.28 y + 0.96 z = 0, cut at
    // y = 4.8 / 0.28 = 17.142857..., row 17.142857 * 0.96 + 5 * 0.28 =
    // 17.857142..., so a line left on image 0 would be at 18.357....
    let source = axial_token("5e0");
    let body = format!(
        "source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {} 2.25.44000 {} 2.25.44000 {}",
        tilted_token("0e0"),
        tilted_token("5e0"),
        tilted_token("1e1")
    );
    for (name, addr) in link_viewers()? {
        expect_link(
            addr,
            name,
            &body,
            r#"{"targets":[{"cell":1,"index":1,"line":[0.5,0.5,64,0.5],"reason":""}]}"#,
        )?;
    }
    Ok(())
}

#[test]
fn link_route_names_the_scroll_refusal_first() -> Result<(), String> {
    // `C` with one row at row spacing 0, PS3.3 10.7.1.3's single-row case.
    // Scroll is refused as not coplanar, dot 0, and the line as degenerate.
    // The scroll refusal is named.
    let source = axial_token("5e0");
    let flat = "0e0,2e1,3e1,1e0,0e0,0e0,0e0,0e0,-1e0,0e0,5e-1,1,128";
    let body = format!("source 2.25.44000 {source}\ntarget 1 0 1 2.25.44000 {flat}");
    for (name, addr) in link_viewers()? {
        expect_link(
            addr,
            name,
            &body,
            r#"{"targets":[{"cell":1,"index":null,"line":null,"reason":"not-coplanar"}]}"#,
        )?;
    }
    Ok(())
}

#[test]
fn link_route_refuses_a_target_past_the_frame_list() -> Result<(), String> {
    // A target lists at most FRAME_LIST_CAP images, as the page does. One
    // more is refused whole, although it is far inside LINK_BODY_CAP.
    let source = axial_token("5e0");
    let at_cap = format!(
        "source - {source}\ntarget 1 0 0{}",
        " - -".repeat(FRAME_LIST_CAP)
    );
    let over = format!("{at_cap} - -");
    for (name, addr) in link_viewers()? {
        let (header, _) = post_link(addr, at_cap.as_bytes())?;
        if !header.starts_with("HTTP/1.1 200 ") {
            return Err(format!(
                "{name}: FRAME_LIST_CAP images were refused: {header}"
            ));
        }
        let (header, _) = post_link(addr, over.as_bytes())?;
        if !header.starts_with("HTTP/1.1 400 ")
            || header_line(&header, "X-Ocelli-Error") != Some("link")
        {
            return Err(format!("{name}: one image past the cap was read: {header}"));
        }
    }
    Ok(())
}

#[test]
fn inspect_writes_at_most_the_frame_list_of_planes() -> Result<(), String> {
    // An object declaring 65537 frames. The page lists 65536 of them, and
    // `inspect_json` writes one plane entry for each of those and no more.
    let two = tagged_frame(2, &[0, 0, 0, 0])?;
    let declared: &[u8] = &[0x28, 0x00, 0x08, 0x00, b'I', b'S', 2, 0, b'2', b' '];
    let wider: &[u8] = &[
        0x28, 0x00, 0x08, 0x00, b'I', b'S', 6, 0, b'6', b'5', b'5', b'3', b'7', b' ',
    ];
    let at = two
        .windows(declared.len())
        .position(|window| window == declared)
        .ok_or_else(|| "no Number of Frames element".to_owned())?;
    let mut file = two.get(..at).ok_or_else(|| "prefix".to_owned())?.to_vec();
    file.extend_from_slice(wider);
    file.extend_from_slice(
        two.get(at + declared.len()..)
            .ok_or_else(|| "suffix".to_owned())?,
    );
    let json = inspect_json(&file, 0).map_err(|_| "inspect refused 65537 frames".to_owned())?;
    if !json.contains("\"frames\":65537") {
        return Err("the object does not declare 65537 frames".to_owned());
    }
    let planes = planes_field(&json)?;
    let entries = planes.matches("null").count();
    if entries != FRAME_LIST_CAP {
        return Err(format!("{entries} plane entries for 65537 frames"));
    }
    Ok(())
}

/// F-226. Post `file` and return the id the process gave it.
fn forget_post(addr: SocketAddr, file: &[u8]) -> Result<String, String> {
    let posted = exchange(addr, &http_post_to("/file", "", file))?;
    let (header, _) = split_http(&posted)?;
    header_line(header, "X-Ocelli-File")
        .map(str::to_owned)
        .ok_or_else(|| "the file id was missing".to_owned())
}

/// The same serial under the next generation, which this process never issued.
fn other_generation(id: &str) -> Result<String, String> {
    let (generation, serial) = id
        .split_once('.')
        .ok_or_else(|| "the file id had no dot".to_owned())?;
    let generation =
        u64::from_str_radix(generation, 16).map_err(|_| "the generation was not hex".to_owned())?;
    Ok(format!("{:x}.{serial}", generation.wrapping_add(1)))
}

/// `files`, `frames` and `used` from `GET /held`.
fn held_counts(addr: SocketAddr) -> Result<(usize, usize, usize), String> {
    let census = held_census(addr)?;
    Ok((
        json_number(&census, "files")?,
        json_number(&census, "frames")?,
        json_number(&census, "used")?,
    ))
}

fn forget_exchange(addr: SocketAddr, extra: &str, body: &[u8]) -> Result<(String, String), String> {
    let reply = exchange(addr, &http_post_to("/forget", extra, body))?;
    let (header, body) = split_http(&reply)?;
    let body = std::str::from_utf8(body).map_err(|_| "the forget reply was not text".to_owned())?;
    Ok((header.to_owned(), body.to_owned()))
}

#[test]
fn forget_drops_files_and_frames() -> Result<(), String> {
    // F-226 AC10. Held before the forget, on one viewer of 8 MiB:
    //   two  a three-frame object, posted, and previewed at frames 0 and 1.
    //        Frame 2 is not decoded, so the posted bytes stay beside them.
    //   one  a one-frame object, posted and never previewed.
    // The body names two, one, the next generation's id for two's serial and
    // two again. Two posted bodies and two frames were held, so the reply is
    // files 2, frames 2, and unknown 2 for the other generation and the
    // repeat. Each posted body is held at its length, which the census shows
    // after each post, and the two frames' charges are what the census rose
    // by when they were stored. Bytes is the sum of the four. Frame 1 is
    // held so that a forget keeping frame 1 fails here.
    let addr = viewer_on(8 * 1024 * 1024)?;
    let two = tagged_frame(3, &[50, 0, 60, 0, 70, 0])?;
    let one = mono_frame(true)?;
    let two_id = forget_post(addr, &two)?;
    if held_counts(addr)? != (1, 0, two.len()) {
        return Err(format!(
            "after one post the census was {:?}",
            held_counts(addr)?
        ));
    }
    let one_id = forget_post(addr, &one)?;
    let posted = two.len() + one.len();
    if held_counts(addr)? != (2, 0, posted) {
        return Err(format!(
            "after two posts the census was {:?}",
            held_counts(addr)?
        ));
    }
    for frame in ["0", "1"] {
        let drawn = exchange(
            addr,
            &preview_id(&two_id, &format!("X-Ocelli-Frame: {frame}\r\n")),
        )?;
        if !split_http(&drawn)?.0.starts_with("HTTP/1.1 200 ") {
            return Err(format!(
                "frame {frame} of the three-frame object did not draw"
            ));
        }
    }
    let (files, frames, used) = held_counts(addr)?;
    if files != 2 || frames != 2 || used <= posted {
        return Err(format!(
            "after the preview the census was {files} files, {frames} frames, {used} used"
        ));
    }
    let frame_charges = used - posted;
    let expected_bytes = two.len() + one.len() + frame_charges;
    let other = other_generation(&two_id)?;
    let body = format!("{two_id}\n{one_id}\n{other}\n{two_id}");
    let (header, reply) = forget_exchange(addr, "", body.as_bytes())?;
    if !header.starts_with("HTTP/1.1 200 ")
        || header_line(&header, "Content-Type") != Some("application/json; charset=utf-8")
    {
        return Err(format!("the forget was not a JSON 200: {header}"));
    }
    let expected = format!("{{\"files\":2,\"frames\":2,\"bytes\":{expected_bytes},\"unknown\":2}}");
    if reply != expected {
        return Err(format!("the forget answered {reply}, not {expected}"));
    }
    let after = held_counts(addr)?;
    if after != (files - 2, frames - 2, used - expected_bytes) {
        return Err(format!(
            "the census went from {:?} to {after:?}, not down by the reply",
            (files, frames, used)
        ));
    }
    for (name, id) in [("two", &two_id), ("one", &one_id)] {
        for frame in ["0", "1", "2"] {
            let gone = exchange(
                addr,
                &preview_id(id, &format!("X-Ocelli-Frame: {frame}\r\n")),
            )?;
            if !split_http(&gone)?.0.starts_with("HTTP/1.1 410 ") {
                return Err(format!(
                    "{name} frame {frame} was still served after the forget"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn forget_refusals() -> Result<(), String> {
    // F-226 AC11. Every line is parsed before anything is removed, so a held
    // id before a bad line stays held. The census is read after each refusal.
    let addr = viewer_on(8 * 1024 * 1024)?;
    let file = mono_frame(true)?;
    let id = forget_post(addr, &file)?;
    let held = held_counts(addr)?;
    if held != (1, 0, file.len()) {
        return Err(format!("the posted file was held as {held:?}"));
    }
    let refusals: [(&str, String, &str, &str); 4] = [
        ("a malformed line", format!("{id}\nnot-an-id"), "", "forget"),
        ("an empty line", format!("{id}\n"), "", "forget"),
        ("an uppercase id", id.to_uppercase(), "", "forget"),
        (
            "a file id header",
            id.clone(),
            "X-Ocelli-File: 0.0\r\n",
            "header",
        ),
    ];
    for (name, body, extra, code) in &refusals {
        let (header, _) = forget_exchange(addr, extra, body.as_bytes())?;
        if !header.starts_with("HTTP/1.1 400 ")
            || header_line(&header, "X-Ocelli-Error") != Some(code)
        {
            return Err(format!("{name} was not 400 {code}: {header}"));
        }
        if held_counts(addr)? != held {
            return Err(format!(
                "{name} changed the census to {:?}",
                held_counts(addr)?
            ));
        }
    }
    // One line past 65,536, every one of them the held id, is refused from
    // the count and forgets nothing. At 65,536 the id is forgotten once and
    // the other 65,535 lines are repeats.
    let over = vec![id.as_str(); 65_537].join("\n");
    let (header, _) = forget_exchange(addr, "", over.as_bytes())?;
    if !header.starts_with("HTTP/1.1 400 ")
        || header_line(&header, "X-Ocelli-Error") != Some("forget")
    {
        return Err(format!("65,537 lines were not 400 forget: {header}"));
    }
    if held_counts(addr)? != held {
        return Err("65,537 lines changed the census".to_owned());
    }
    // A declared length one byte over 4 MiB is refused before any body is read.
    let long = exchange(
        addr,
        b"POST /forget HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 4194305\r\n\r\n",
    )?;
    let (long_header, _) = split_http(&long)?;
    if !long_header.starts_with("HTTP/1.1 400 ")
        || header_line(long_header, "X-Ocelli-Error") != Some("forget")
    {
        return Err(format!(
            "a 4 MiB and one byte forget was not 400 forget: {long_header}"
        ));
    }
    let (header, reply) = forget_exchange(addr, "", b"")?;
    if !header.starts_with("HTTP/1.1 200 ")
        || reply != "{\"files\":0,\"frames\":0,\"bytes\":0,\"unknown\":0}"
    {
        return Err(format!("an empty forget answered {header} {reply}"));
    }
    if held_counts(addr)? != held {
        return Err("an empty forget changed the census".to_owned());
    }
    let at_cap = vec![id.as_str(); 65_536].join("\n");
    let (header, reply) = forget_exchange(addr, "", at_cap.as_bytes())?;
    let expected = format!(
        "{{\"files\":1,\"frames\":0,\"bytes\":{},\"unknown\":65535}}",
        file.len()
    );
    if !header.starts_with("HTTP/1.1 200 ") || reply != expected {
        return Err(format!(
            "65,536 lines answered {header} {reply}, not {expected}"
        ));
    }
    if held_counts(addr)? != (0, 0, 0) {
        return Err(format!(
            "the census after the forget was {:?}",
            held_counts(addr)?
        ));
    }
    Ok(())
}

/// Post `file` under the page token `page` and return its id.
fn page_post(addr: SocketAddr, file: &[u8], page: &str) -> Result<String, String> {
    let posted = exchange(
        addr,
        &http_post_to("/file", &format!("X-Ocelli-Page: {page}\r\n"), file),
    )?;
    let (header, _) = split_http(&posted)?;
    header_line(header, "X-Ocelli-File")
        .map(str::to_owned)
        .ok_or_else(|| format!("the file posted under {page} had no id: {header}"))
}

#[test]
fn forget_by_page_drops_only_that_pages_files() -> Result<(), String> {
    // F-226 AC15 as the lead restated it, on the process side. Page `left`
    // posts a one-frame object and previews it, so only its frame is held,
    // and posts a second one-frame object that is never previewed, so its
    // bytes are held. Page `stays` posts a third. `page <left>` forgets the
    // frame and the bytes, files 1 and frames 1, and leaves the third
    // object exactly as held. The tokens are 32 lowercase hex digits.
    let left = "0123456789abcdef0123456789abcdef";
    let stays = "fedcba9876543210fedcba9876543210";
    let unknown = "00000000000000000000000000000001";
    let addr = viewer_on(8 * 1024 * 1024)?;
    let drawn_file = mono_frame(true)?;
    let posted_file = mono_frame(false)?;
    let kept_file = tagged_frame(1, &50_u16.to_le_bytes())?;
    let drawn = page_post(addr, &drawn_file, left)?;
    let preview = exchange(addr, &preview_id(&drawn, ""))?;
    if !split_http(&preview)?.0.starts_with("HTTP/1.1 200 ") {
        return Err("the left page's file did not draw".to_owned());
    }
    let (_, _, drawn_used) = held_counts(addr)?;
    let posted = page_post(addr, &posted_file, left)?;
    let kept = page_post(addr, &kept_file, stays)?;
    let before = held_counts(addr)?;
    if before != (2, 1, drawn_used + posted_file.len() + kept_file.len()) {
        return Err(format!("before the forget the census was {before:?}"));
    }
    let refusals = [
        (
            "a short token",
            "page 0123456789abcdef0123456789abcde".to_owned(),
        ),
        ("a long token", format!("page {left}0")),
        (
            "an uppercase token",
            format!("page {}", left.to_uppercase()),
        ),
        ("a token and an id", format!("page {left}\n{kept}")),
        ("no space", format!("page{left}")),
    ];
    for (name, body) in &refusals {
        let (header, _) = forget_exchange(addr, "", body.as_bytes())?;
        if !header.starts_with("HTTP/1.1 400 ")
            || header_line(&header, "X-Ocelli-Error") != Some("forget")
        {
            return Err(format!("{name} was not 400 forget: {header}"));
        }
        if held_counts(addr)? != before {
            return Err(format!("{name} changed the census"));
        }
    }
    let zeros = "{\"files\":0,\"frames\":0,\"bytes\":0,\"unknown\":0}";
    let (header, reply) = forget_exchange(addr, "", format!("page {unknown}").as_bytes())?;
    if !header.starts_with("HTTP/1.1 200 ") || reply != zeros || held_counts(addr)? != before {
        return Err(format!("an unknown token answered {header} {reply}"));
    }
    let (header, reply) = forget_exchange(addr, "", format!("page {left}").as_bytes())?;
    let expected = format!(
        "{{\"files\":1,\"frames\":1,\"bytes\":{},\"unknown\":0}}",
        drawn_used + posted_file.len()
    );
    if !header.starts_with("HTTP/1.1 200 ") || reply != expected {
        return Err(format!(
            "the page forget answered {header} {reply}, not {expected}"
        ));
    }
    if held_counts(addr)? != (1, 0, kept_file.len()) {
        return Err(format!(
            "after the page forget the census was {:?}",
            held_counts(addr)?
        ));
    }
    for (name, id) in [("drawn", &drawn), ("posted", &posted)] {
        let gone = exchange(addr, &preview_id(id, ""))?;
        if !split_http(&gone)?.0.starts_with("HTTP/1.1 410 ") {
            return Err(format!("the {name} file of the left page was still served"));
        }
    }
    let again = forget_exchange(addr, "", format!("page {left}").as_bytes())?;
    if again.1 != zeros {
        return Err(format!("a second page forget answered {}", again.1));
    }
    let bad_post = exchange(
        addr,
        &http_post_to("/file", "X-Ocelli-Page: not-a-token\r\n", &kept_file),
    )?;
    let (bad_header, _) = split_http(&bad_post)?;
    if !bad_header.starts_with("HTTP/1.1 400 ")
        || header_line(bad_header, "X-Ocelli-Error") != Some("header")
    {
        return Err(format!(
            "a malformed page token on /file was accepted: {bad_header}"
        ));
    }
    if held_counts(addr)? != (1, 0, kept_file.len()) {
        return Err("a refused post changed the census".to_owned());
    }
    let survivor = exchange(addr, &preview_id(&kept, ""))?;
    if !split_http(&survivor)?.0.starts_with("HTTP/1.1 200 ") {
        return Err("the staying page's file was not served".to_owned());
    }
    Ok(())
}

/// The text from `start` up to the first line that is `});` or `}`.
fn block_from<'a>(page: &'a str, start: &str) -> Result<&'a str, String> {
    let at = page
        .find(start)
        .ok_or_else(|| format!("the page has no {start}"))?;
    page.get(at..)
        .and_then(|rest| {
            let end = [rest.find("\n});\n"), rest.find("\n}\n")]
                .into_iter()
                .flatten()
                .min()?;
            rest.get(..end)
        })
        .ok_or_else(|| format!("{start} does not close"))
}

#[test]
fn the_page_keys_pixels_by_file_and_probes_for_changes() -> Result<(), String> {
    // F-224 AC16, and the page invariant that no pixel cache key is derived
    // from a file name, size or time. The page text is read, not executed.
    // changed.mjs drives the same rules in Chromium.
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    // The change handler copies the list before the reset, because the reset
    // empties it, and opens the copy.
    let change = block_from(page, "folderInput.addEventListener(\"change\", () => {")?;
    let copy = change
        .find("const files = Array.from(folderInput.files")
        .ok_or_else(|| "the change handler does not copy the files".to_owned())?;
    let reset = change
        .find("folderInput.value = \"\";")
        .ok_or_else(|| "the change handler does not reset the input".to_owned())?;
    let open = change
        .find("readFolder(files);")
        .ok_or_else(|| "the change handler does not open the copy".to_owned())?;
    if !(copy < reset && reset < open) {
        return Err("the change handler resets the input before it copies the files".to_owned());
    }
    // A series selection in the active viewport probes, and so do focus and
    // a return to a visible page.
    if !function_body(page, "loadSeries")?.contains("if (vp.cell === activeCell) probeActive();") {
        return Err("a series selection does not probe".to_owned());
    }
    // Activating another viewport counts and probes its series, as a series
    // selection does.
    let activate = function_body(page, "setActive")?;
    if !activate.contains("noteChanged();") || !activate.contains("probeActive();") {
        return Err("activating a viewport does not count and probe its series".to_owned());
    }
    if !page.contains("window.addEventListener(\"focus\", probeActive);") {
        return Err("a window focus does not probe".to_owned());
    }
    let visible = block_from(
        page,
        "document.addEventListener(\"visibilitychange\", () => {",
    )?;
    if !visible.contains("if (document.visibilityState === \"visible\") probeActive();") {
        return Err("a return to a visible page does not probe".to_owned());
    }
    // One byte a file, never the whole file.
    let probe = block_from(page, "async function probeFiles(")?;
    if !probe.contains("await file.slice(0, 1).arrayBuffer();")
        || probe.contains("await file.arrayBuffer()")
    {
        return Err("the probe does not read one byte a file".to_owned());
    }
    // The grey cache key starts with the picked file's serial.
    let key = function_body(page, "cacheKey")?;
    if !key.contains("return fileSerial(image.file) + ") || key.contains("fileIdentity(") {
        return Err("the grey cache is not keyed by the picked file".to_owned());
    }
    // A changed file is refused before any held id is used.
    let ensure = block_from(page, "async function ensureFile(")?;
    let refused = ensure
        .find("if (changedFiles.has(file)) throw changedError();")
        .ok_or_else(|| "ensureFile does not refuse a changed file".to_owned())?;
    let held = ensure
        .find("if (filePosts.has(file))")
        .ok_or_else(|| "ensureFile no longer reads its held id".to_owned())?;
    if refused > held {
        return Err("ensureFile serves a held id before it refuses a changed file".to_owned());
    }
    if !page.contains("\"This file changed or was removed on disk after the folder was opened. Open the folder again to see it.\"") {
        return Err("the page does not carry the changed-file sentence".to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// F-212. Corner text, edge letters from Patient Orientation, and the scale
// bar's calibration. Every expected value below is read from the PS3.5 6.2
// Table 6.2-1 row or the PS3.3 section named beside it, or worked by hand in
// the comment, never from a run of `corners.rs`.

#[test]
fn person_name_text_follows_ps3_5_pn() -> Result<(), String> {
    // PS3.5 6.2 PN. Five `^` components in the order family, given, middle,
    // prefix, suffix. Groups separated by `=` are alphabetic, ideographic,
    // phonetic. The plan's display is `family, prefix given middle, suffix`
    // with empty parts and their separators dropped.
    for (stored, want) in [
        ("Doe^John^Quincy^Dr^Jr", "Doe, Dr John Quincy, Jr"),
        ("Doe^John", "Doe, John"),
        ("^John", "John"),
        // The alphabetic group exists, so the ideographic one is not used.
        ("Yamada^Tarou=山田^太郎=やまだ^たろう", "Yamada, Tarou"),
        // The alphabetic group is empty, so the ideographic one is used.
        ("=山田^太郎", "山田, 太郎"),
        // VM 2. Only the first value is a name.
        ("A\\B", "A"),
        ("", ""),
    ] {
        let got = person_name_text(stored);
        if got != want {
            return Err(format!("{stored:?} gave {got:?}, want {want:?}"));
        }
    }
    Ok(())
}

#[test]
fn dicom_date_text_follows_ps3_5_da() -> Result<(), String> {
    // PS3.5 6.2 DA, YYYYMMDD on the Gregorian calendar. 2000 and 1600 are
    // divisible by 400, so they are leap years and 29 February exists. 2020
    // is divisible by 4 and not by 100, so it is one too. 1900 and 2100 are
    // divisible by 100 and not by 400, so they are not, and their 29
    // February names no day. Month 13 and day 32 name no day. `1970.01.02`
    // is the retired ACR-NEMA form and `197001` is not eight digits. Each of
    // those stays as stored.
    for (stored, want) in [
        ("19700102", "1970-01-02"),
        ("20000229", "2000-02-29"),
        ("16000229", "1600-02-29"),
        ("20200229", "2020-02-29"),
        ("19000229", "19000229"),
        ("21000229", "21000229"),
        ("19701302", "19701302"),
        ("19700132", "19700132"),
        ("1970.01.02", "1970.01.02"),
        ("197001", "197001"),
        ("", ""),
    ] {
        let got = dicom_date_text(stored);
        if got != want {
            return Err(format!("{stored:?} gave {got:?}, want {want:?}"));
        }
    }
    Ok(())
}

#[test]
fn dicom_time_text_follows_ps3_5_tm() -> Result<(), String> {
    // PS3.5 6.2 TM, HHMMSS.FFFFFF with HH 00 to 23, MM 00 to 59, SS 00 to
    // 60 and a fraction of 1 to 6 digits. A component may be left out only
    // with every component to its right. The fraction is dropped, never
    // rounded, and no component is invented. `2400` is outside the hour
    // range, `143` is not a whole component, `14:30:12` is the retired
    // ACR-NEMA form, `146012` has minute 60, and `143012.` has a separator
    // with no fraction digit. Each of those stays as stored.
    for (stored, want) in [
        ("143012", "14:30:12"),
        ("143012.123456", "14:30:12"),
        ("1430", "14:30"),
        ("14", "14"),
        ("235959.999999", "23:59:59"),
        ("235960", "23:59:60"),
        ("2400", "2400"),
        ("143", "143"),
        ("14:30:12", "14:30:12"),
        ("146012", "146012"),
        ("143012.", "143012."),
        ("143012.1234567", "143012.1234567"),
        ("", ""),
    ] {
        let got = dicom_time_text(stored);
        if got != want {
            return Err(format!("{stored:?} gave {got:?}, want {want:?}"));
        }
    }
    Ok(())
}

#[test]
fn patient_age_follows_ps3_5_as() -> Result<(), String> {
    // PS3.5 6.2 AS, nnnD, nnnW, nnnM or nnnY, 4 bytes fixed. The number is
    // shown without its leading zeros and the unit as a word, singular for
    // 1. `45Y` is three bytes and `045y` has a lower-case unit, so neither
    // is AS. Neither falls through to the age computed from the two dates,
    // which here would be 54 years.
    for (stored, want, source) in [
        ("045Y", "45 years", "PatientAge"),
        ("001Y", "1 year", "PatientAge"),
        ("018M", "18 months", "PatientAge"),
        ("003W", "3 weeks", "PatientAge"),
        ("012D", "12 days", "PatientAge"),
        ("000D", "0 days", "PatientAge"),
        ("45Y", "", ""),
        ("045y", "", ""),
    ] {
        let (text, from) = age_text(Some(stored), Some("19700102"), Some("20240305"));
        if text != want || from != source {
            return Err(format!(
                "{stored:?} gave {text:?} from {from:?}, want {want:?} from {source:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn computed_age_is_the_age_at_the_study() -> Result<(), String> {
    // Whole years are `sy - by`, less one while `(sm, sd) < (bm, bd)`.
    // 1970-01-02 to 2024-03-05: 54, and (3, 5) is not before (1, 2).
    // 1970-03-06 to 2024-03-05: 54 less one, (3, 5) is before (3, 6).
    // 1970-03-05 to 2024-03-05: the birthday itself, 54.
    // 2000-02-29 to 2001-02-28: 1 less one is 0 years. Months are
    // 1 * 12 + (2 - 2) = 12, less one because day 28 is before day 29: 11.
    // 2000-02-29 to 2001-03-01: (3, 1) is not before (2, 29), so 1 year.
    // 2024-01-15 to 2024-03-05: 0 years, 2 months less one for day 5: 1.
    // 2024-01-05 to 2024-03-05: day 5 is not before day 5, so 2 months.
    // 2024-03-01 to 2024-03-15: 0 years, 0 months, 14 Gregorian days.
    // 2024-02-20 to 2024-03-05: 0 months, and 2024 is a leap year, so
    // 9 days to 29 February and 5 more, 14 days. 2020 is a leap year as
    // well, so 2020-02-20 to 2020-03-05 is also 14 days. 2023 is not, so
    // 2023-02-20 to 2023-03-05 is 8 days to 28 February and 5 more, 13.
    // 2000-12-15 to 2001-01-05 crosses the end of the leap year 2000: 0
    // years, 1 * 12 + 1 - 12 = 1 month less one for day 5, so 0 months, and
    // 16 days to 31 December and 5 more, 21 days.
    // A study before the birth, or no study date, is empty.
    for (birth, study, want, source) in [
        (Some("19700102"), Some("20240305"), "54 years", "computed"),
        (Some("19700306"), Some("20240305"), "53 years", "computed"),
        (Some("19700305"), Some("20240305"), "54 years", "computed"),
        (Some("20000229"), Some("20010228"), "11 months", "computed"),
        (Some("20000229"), Some("20010301"), "1 year", "computed"),
        (Some("20240115"), Some("20240305"), "1 month", "computed"),
        (Some("20240105"), Some("20240305"), "2 months", "computed"),
        (Some("20240301"), Some("20240315"), "14 days", "computed"),
        (Some("20240220"), Some("20240305"), "14 days", "computed"),
        (Some("20200220"), Some("20200305"), "14 days", "computed"),
        (Some("20230220"), Some("20230305"), "13 days", "computed"),
        (Some("20001215"), Some("20010105"), "21 days", "computed"),
        (Some("20240305"), Some("20240301"), "", ""),
        (Some("19700102"), None, "", ""),
        (Some("19000229"), Some("20240305"), "", ""),
    ] {
        let (text, from) = age_text(None, birth, study);
        if text != want || from != source {
            return Err(format!(
                "{birth:?} to {study:?} gave {text:?} from {from:?}, want {want:?} from {source:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn exposure_mas_prefers_exposure_then_micro() -> Result<(), String> {
    // PS3.3 C.8.2.1 and C.8.7.8. Exposure (0018,1152) is IS in mAs.
    // Exposure in µAs (0018,1153) is IS in µAs, so mAs is the value over
    // 1000, written exactly: 200000 is 200, 500 is 0.5, 1234567 is
    // 1234.567, 2147483647 is 2147483.647 and -500 is -0.5. PS3.5 6.2 IS
    // ranges from -2^31 to 2^31 - 1, so 2147483648 is not an IS and the
    // next source is read.
    for (exposure, micro, want, source) in [
        (Some("200"), None, Some("200"), "Exposure"),
        (Some("200"), Some("500"), Some("200"), "Exposure"),
        (None, Some("200000"), Some("200"), "ExposureInuAs"),
        (None, Some("500"), Some("0.5"), "ExposureInuAs"),
        (None, Some("1234567"), Some("1234.567"), "ExposureInuAs"),
        (
            None,
            Some("2147483647"),
            Some("2147483.647"),
            "ExposureInuAs",
        ),
        (None, Some("-500"), Some("-0.5"), "ExposureInuAs"),
        (
            Some("2147483648"),
            Some("500"),
            Some("0.5"),
            "ExposureInuAs",
        ),
        (Some("2147483647"), None, Some("2147483647"), "Exposure"),
        (Some("2.5"), None, None, ""),
        (None, None, None, ""),
    ] {
        let (text, from) = exposure_mas(exposure, micro);
        if text.as_deref() != want || from != source {
            return Err(format!(
                "{exposure:?} and {micro:?} gave {text:?} from {from:?}, want {want:?} from {source:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn patient_orientation_follows_ps3_3_c_7_6_1_1_1() -> Result<(), String> {
    // PS3.3 C.7.6.1.1.1. The first value is the direction of the positive
    // row axis, left to right, so it names the right edge, and the left edge
    // is its opposite. The second is the positive column axis, top to
    // bottom, so it names the bottom edge, and the top is its opposite.
    // Opposites are A and P, R and L, H and F.
    for (values, want) in [
        (&["L", "F"][..], ["R", "L", "H", "F"]),
        (&["A", "F"][..], ["P", "A", "H", "F"]),
        (&["A", "FR"][..], ["P", "A", "HL", "FR"]),
    ] {
        let got =
            patient_orientation_letters(values).ok_or_else(|| format!("{values:?} was refused"))?;
        if got != want.map(str::to_owned) {
            return Err(format!("{values:?} gave {got:?}, want {want:?}"));
        }
    }
    // A letter with its opposite, a letter outside A P R L H F, one value,
    // three values, four letters, and the quadruped terms are refused.
    // `LAHL` has four letters of the set and no opposite pair, so only the
    // length refuses it.
    for values in [
        &["LR", "F"][..],
        &["F", "PA"][..],
        &["X", "F"][..],
        &["A"][..],
        &["L", "F", "A"][..],
        &["ABCD", "F"][..],
        &["LAHL", "F"][..],
        &["RT", "V"][..],
    ] {
        if let Some(got) = patient_orientation_letters(values) {
            return Err(format!("{values:?} gave {got:?}, want refused"));
        }
    }
    Ok(())
}

/// One element a `corner_object` adds: tag, VR and the padded value.
type Planted = ((u16, u16), [u8; 2], Vec<u8>);

fn planted_text(tag: (u16, u16), vr: [u8; 2], text: &str) -> Planted {
    (tag, vr, text_pad(text))
}

/// A one-pixel CT image with the planted elements, in ascending tag order.
fn corner_object(planted: &[Planted]) -> Result<Vec<u8>, String> {
    let mut elements = vec![
        (
            (0x0008, 0x0016),
            short_le((0x0008, 0x0016), *b"UI", &ui(b"1.2.840.10008.5.1.4.1.1.2"))?,
        ),
        (
            (0x0008, 0x0060),
            short_le((0x0008, 0x0060), *b"CS", &text_pad("CT"))?,
        ),
        (
            (0x0028, 0x0002),
            short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0004),
            short_le((0x0028, 0x0004), *b"CS", &text_pad("MONOCHROME2"))?,
        ),
        (
            (0x0028, 0x0010),
            short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0011),
            short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0100),
            short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0101),
            short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0102),
            short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x0103),
            short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?,
        ),
        (
            (0x0028, 0x1050),
            short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?,
        ),
        (
            (0x0028, 0x1051),
            short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?,
        ),
        (
            (0x0028, 0x1052),
            short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?,
        ),
        (
            (0x0028, 0x1053),
            short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
        ),
        (
            (0x7FE0, 0x0010),
            long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())?,
        ),
    ];
    for (tag, vr, value) in planted {
        let encoded = if vr == b"SQ" {
            long_le(*tag, *vr, value)?
        } else {
            short_le(*tag, *vr, value)?
        };
        elements.push((*tag, encoded));
    }
    elements.sort_by_key(|(tag, _)| *tag);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    for (_, encoded) in elements {
        bytes.extend(encoded);
    }
    Ok(bytes)
}

const PATIENT_ORIENTATION: (u16, u16) = (0x0020, 0x0020);
const IMAGE_ORIENTATION: (u16, u16) = (0x0020, 0x0037);
const ANATOMICAL_ORIENTATION_TYPE: (u16, u16) = (0x0010, 0x2210);

fn edges_of(file: &[u8]) -> Result<[String; 5], String> {
    let preview = preview_frame(file, FrameRequest::default())
        .map_err(|error| format!("the frame was refused with {}", error.code()))?;
    let facts = preview.facts;
    Ok([
        facts.edge_left,
        facts.edge_right,
        facts.edge_top,
        facts.edge_bottom,
        facts.edge_source.to_owned(),
    ])
}

#[test]
fn patient_orientation_is_used_only_without_image_orientation() -> Result<(), String> {
    // PS3.3 C.7.6.1.1.1. Patient Orientation `L\F`: rows run to the
    // patient's left, so the right edge is L and the left edge R. Columns
    // run to the feet, so the bottom edge is F and the top H.
    let po = planted_text(PATIENT_ORIENTATION, *b"CS", "L\\F");
    let projection = corner_object(std::slice::from_ref(&po))?;
    let want = ["R", "L", "H", "F", "PatientOrientation"].map(str::to_owned);
    if edges_of(&projection)? != want {
        return Err(format!("L\\F gave {:?}", edges_of(&projection)?));
    }
    let response = one_exchange(&http_post(&projection))?;
    let (header, _) = split_http(&response)?;
    if header_line(header, "X-Ocelli-Edge-Left") != Some("R")
        || header_line(header, "X-Ocelli-Edge-Source") != Some("PatientOrientation")
    {
        return Err(format!("the reply did not carry the letters: {header}"));
    }
    // With the axial cosines 1\0\0\0\1\0, F-206 decides: row +x is the
    // patient's left, column +y is posterior, so left R, right L, top A,
    // bottom P, and Patient Orientation is not read.
    let axial = corner_object(&[
        po.clone(),
        planted_text(IMAGE_ORIENTATION, *b"DS", "1\\0\\0\\0\\1\\0"),
    ])?;
    let want = ["R", "L", "A", "P", "ImageOrientationPatient"].map(str::to_owned);
    if edges_of(&axial)? != want {
        return Err(format!("the axial cosines gave {:?}", edges_of(&axial)?));
    }
    // A present Image Orientation (Patient) that cannot be used, a non-unit
    // row cosine or a value that is not text, is not rescued by Patient
    // Orientation.
    let empty = [""; 5].map(str::to_owned);
    for (name, orientation) in [
        (
            "non-unit",
            planted_text(IMAGE_ORIENTATION, *b"DS", "2\\0\\0\\0\\1\\0"),
        ),
        (
            "binary",
            (IMAGE_ORIENTATION, *b"US", 1_u16.to_le_bytes().to_vec()),
        ),
    ] {
        let file = corner_object(&[po.clone(), orientation])?;
        if edges_of(&file)? != empty {
            return Err(format!("a {name} orientation gave {:?}", edges_of(&file)?));
        }
    }
    // A zero-length Image Orientation (Patient) is the same as an absent one.
    let zero = corner_object(&[po.clone(), (IMAGE_ORIENTATION, *b"DS", Vec::new())])?;
    let want = ["R", "L", "H", "F", "PatientOrientation"].map(str::to_owned);
    if edges_of(&zero)? != want {
        return Err(format!(
            "a zero-length orientation gave {:?}",
            edges_of(&zero)?
        ));
    }
    // A refused Patient Orientation gives no letters, and the frame draws.
    let refused = corner_object(&[planted_text(PATIENT_ORIENTATION, *b"CS", "LR\\F")])?;
    if edges_of(&refused)? != empty {
        return Err(format!("LR\\F gave {:?}", edges_of(&refused)?));
    }
    // PS3.3 C.7.6.1.1.1. In a QUADRUPED object `R` is rostral, so the human
    // letters would be wrong. Neither attribute gives letters there.
    let quadruped = planted_text(ANATOMICAL_ORIENTATION_TYPE, *b"CS", "QUADRUPED");
    for (name, orientation) in [
        (
            "Patient Orientation",
            planted_text(PATIENT_ORIENTATION, *b"CS", "R\\PL"),
        ),
        (
            "Image Orientation (Patient)",
            planted_text(IMAGE_ORIENTATION, *b"DS", "1\\0\\0\\0\\1\\0"),
        ),
    ] {
        let file = corner_object(&[quadruped.clone(), orientation])?;
        if edges_of(&file)? != empty {
            return Err(format!("a quadruped {name} gave {:?}", edges_of(&file)?));
        }
    }
    let biped = corner_object(&[
        planted_text(ANATOMICAL_ORIENTATION_TYPE, *b"CS", "BIPED"),
        planted_text(PATIENT_ORIENTATION, *b"CS", "R\\PL"),
    ])?;
    // `R\PL`: right edge R, left L, bottom PL, top AR.
    let want = ["L", "R", "AR", "PL", "PatientOrientation"].map(str::to_owned);
    if edges_of(&biped)? != want {
        return Err(format!("a biped R\\PL gave {:?}", edges_of(&biped)?));
    }
    Ok(())
}

fn every_corner_attribute() -> Vec<Planted> {
    vec![
        planted_text((0x0008, 0x0020), *b"DA", "20240305"),
        planted_text((0x0008, 0x0030), *b"TM", "143012.5"),
        planted_text((0x0008, 0x0080), *b"LO", "Example Institution"),
        planted_text((0x0010, 0x0010), *b"PN", "Doe^Jane"),
        planted_text((0x0010, 0x0030), *b"DA", "19700102"),
        planted_text((0x0010, 0x0040), *b"CS", "F"),
        planted_text((0x0018, 0x0050), *b"DS", "5.0"),
        planted_text((0x0018, 0x0060), *b"DS", "120"),
        planted_text((0x0018, 0x0080), *b"DS", "500"),
        planted_text((0x0018, 0x0081), *b"DS", "15"),
        planted_text((0x0018, 0x1153), *b"IS", "200000"),
        planted_text((0x0020, 0x1041), *b"DS", "-12.50"),
    ]
}

#[test]
fn inspect_json_reports_the_corner_fields() -> Result<(), String> {
    // Birth 1970-01-02, study 2024-03-05, no Patient's Age: 54 years, since
    // (3, 5) is not before (1, 2). Study Time 143012.5 drops its fraction.
    // Exposure is absent, so 200000 µAs over 1000 is 200 mAs. Each DS token
    // is a JSON number already and is kept as stored, `-12.50` included.
    let file = corner_object(&every_corner_attribute())?;
    let json = inspect_json(&file, 0).map_err(|_| "inspect refused the CT".to_owned())?;
    let fields = [
        "\"patientName\":\"Doe^Jane\"",
        "\"studyDate\":\"20240305\"",
        "\"edgeSource\":\"\"",
        "\"patientNameText\":\"Doe, Jane\"",
        "\"birthDate\":\"1970-01-02\"",
        "\"sex\":\"F\"",
        "\"age\":\"54 years\"",
        "\"ageSource\":\"computed\"",
        "\"studyDateText\":\"2024-03-05\"",
        "\"studyTime\":\"14:30:12\"",
        "\"institution\":\"Example Institution\"",
        "\"sliceLocation\":-12.50",
        "\"sliceThickness\":5.0",
        "\"kvp\":120",
        "\"repetitionTime\":500",
        "\"echoTime\":15",
        "\"exposureMas\":200",
        "\"exposureSource\":\"ExposureInuAs\"",
    ];
    let planes_at = json
        .find("\"planes\":")
        .ok_or_else(|| "inspect has no planes".to_owned())?;
    for needle in fields {
        match json.find(needle) {
            Some(at) if at < planes_at => {}
            Some(_) => return Err(format!("{needle} is after the planes tail")),
            None => return Err(format!("inspect lacks {needle}: {json}")),
        }
    }
    // The HTTP scan returns the same object.
    let response = one_exchange(&http_post_to("/inspect", "", &file))?;
    let (_, body) = split_http(&response)?;
    if body != json.as_bytes() {
        return Err("the /inspect reply differs from inspect_json".to_owned());
    }
    // A stored Patient's Age 045Y wins over the dates, Exposure 200 wins
    // over the µAs value, a multi-valued DS and one that is not a number are
    // null, and a sex outside M, F and O is empty.
    let mut planted = every_corner_attribute();
    planted.push(planted_text((0x0010, 0x1010), *b"AS", "045Y"));
    planted.push(planted_text((0x0018, 0x1152), *b"IS", "200"));
    planted.retain(|(tag, _, _)| {
        *tag != (0x0020, 0x1041) && *tag != (0x0010, 0x0040) && *tag != (0x0018, 0x0050)
    });
    planted.push(planted_text((0x0020, 0x1041), *b"DS", "1\\2"));
    planted.push(planted_text((0x0018, 0x0050), *b"DS", "five"));
    planted.push(planted_text((0x0010, 0x0040), *b"CS", "X"));
    let stored = corner_object(&planted)?;
    let json = inspect_json(&stored, 0).map_err(|_| "inspect refused the CT".to_owned())?;
    for needle in [
        "\"age\":\"45 years\"",
        "\"ageSource\":\"PatientAge\"",
        "\"exposureMas\":200",
        "\"exposureSource\":\"Exposure\"",
        "\"sliceLocation\":null",
        "\"sliceThickness\":null",
        "\"sex\":\"\"",
    ] {
        if !json.contains(needle) {
            return Err(format!("inspect lacks {needle}: {json}"));
        }
    }
    // The phantom writes none of the corner attributes. It has Image
    // Orientation (Patient), so its letters come from there.
    let phantom = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let json =
        inspect_json(&phantom, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    for needle in [
        "\"patientNameText\":\"\"",
        "\"birthDate\":\"\"",
        "\"sex\":\"\"",
        "\"age\":\"\"",
        "\"ageSource\":\"\"",
        "\"studyDateText\":\"\"",
        "\"studyTime\":\"\"",
        "\"institution\":\"\"",
        "\"sliceLocation\":null",
        "\"sliceThickness\":null",
        "\"kvp\":null",
        "\"repetitionTime\":null",
        "\"echoTime\":null",
        "\"exposureMas\":null",
        "\"exposureSource\":\"\"",
        "\"edgeSource\":\"ImageOrientationPatient\"",
    ] {
        if !json.contains(needle) {
            return Err(format!("the phantom inspect lacks {needle}"));
        }
    }
    Ok(())
}

#[test]
fn a_corner_attribute_that_is_not_text_does_not_refuse_the_scan() -> Result<(), String> {
    // Lead amendment 3. Each new attribute is planted as a binary US value
    // and then as an empty sequence. The scan still lists the file, the
    // frame still draws, and each corner field is empty or null. Study Date
    // and Patient's Birth Date stay valid in the age row, so an age shown
    // there would be a computed one replacing the unusable Patient's Age.
    let new_tags = [
        (0x0008, 0x0030),
        (0x0008, 0x0080),
        (0x0010, 0x0040),
        (0x0010, 0x1010),
        (0x0010, 0x2210),
        (0x0018, 0x0050),
        (0x0018, 0x0060),
        (0x0018, 0x0080),
        (0x0018, 0x0081),
        (0x0018, 0x1152),
        (0x0018, 0x1153),
        (0x0020, 0x0020),
        (0x0020, 0x1041),
    ];
    for (vr, value) in [(*b"US", 7_u16.to_le_bytes().to_vec()), (*b"SQ", Vec::new())] {
        let mut planted: Vec<Planted> = new_tags
            .iter()
            .map(|tag| (*tag, vr, value.clone()))
            .collect();
        planted.push(planted_text((0x0008, 0x0020), *b"DA", "20240305"));
        planted.push(planted_text((0x0010, 0x0030), *b"DA", "19700102"));
        let file = corner_object(&planted)?;
        let vr_name = String::from_utf8_lossy(&vr).into_owned();
        let json = inspect_json(&file, 0)
            .map_err(|_| format!("a {vr_name} corner attribute refused the scan"))?;
        for needle in [
            "\"sex\":\"\"",
            "\"age\":\"\"",
            "\"ageSource\":\"\"",
            "\"studyTime\":\"\"",
            "\"institution\":\"\"",
            "\"sliceLocation\":null",
            "\"sliceThickness\":null",
            "\"kvp\":null",
            "\"repetitionTime\":null",
            "\"echoTime\":null",
            "\"exposureMas\":null",
            "\"exposureSource\":\"\"",
            "\"edgeLeft\":\"\"",
            "\"edgeSource\":\"\"",
        ] {
            if !json.contains(needle) {
                return Err(format!("a {vr_name} value left {needle} out: {json}"));
            }
        }
        if edges_of(&file)? != [""; 5].map(str::to_owned) {
            return Err(format!("a {vr_name} Patient Orientation gave letters"));
        }
    }
    // A Patient's Name and Patient's Birth Date that are not text show
    // nothing, rather than their numbers as a name or a date.
    let file = corner_object(&[
        ((0x0010, 0x0010), *b"US", 7_u16.to_le_bytes().to_vec()),
        ((0x0010, 0x0030), *b"US", 7_u16.to_le_bytes().to_vec()),
    ])?;
    let json = inspect_json(&file, 0).map_err(|_| "a binary name refused the scan".to_owned())?;
    if !json.contains("\"patientNameText\":\"\"") || !json.contains("\"birthDate\":\"\"") {
        return Err(format!("a binary name or birth date was shown: {json}"));
    }
    Ok(())
}

#[test]
fn the_scale_bar_reads_the_length_tool_calibration() -> Result<(), String> {
    // Lead amendment 1. The preview reply carries F-211's choice and
    // `/measure` reports the same one. Each row moves one input and both
    // move together. PS3.3 C.8.11.4 and C.8.7.5: Imager Pixel Spacing 0.2
    // alone is at the detector. With a magnification factor 2 it is
    // 0.2 / 2 = 0.1 mm at the patient. A Pixel Spacing 0.5 decides alone,
    // and 0.25\0.5 is row 0.25 then column 0.5, PS3.3 10.7.1.3. No spacing
    // is uncalibrated, and a Pixel Spacing of 0\0.5 refuses.
    let imager = planted_text((0x0018, 0x1164), *b"DS", "0.2\\0.2");
    let factor = planted_text((0x0018, 0x1114), *b"DS", "2");
    let segment = [ImagePoint { x: 0.0, y: 0.0 }, ImagePoint { x: 1.0, y: 0.0 }];
    for (name, planted, source, mm) in [
        (
            "detector",
            vec![imager.clone()],
            "detector",
            Some((0.2, 0.2)),
        ),
        (
            "factor",
            vec![imager.clone(), factor.clone()],
            "imager-spacing",
            Some((0.1, 0.1)),
        ),
        (
            "pixel",
            vec![planted_text((0x0028, 0x0030), *b"DS", "0.5\\0.5")],
            "pixel-spacing",
            Some((0.5, 0.5)),
        ),
        (
            "non-square",
            vec![planted_text((0x0028, 0x0030), *b"DS", "0.25\\0.5")],
            "pixel-spacing",
            Some((0.25, 0.5)),
        ),
        ("none", Vec::new(), "uncalibrated", None),
        (
            "refused",
            vec![planted_text((0x0028, 0x0030), *b"DS", "0\\0.5")],
            "",
            None,
        ),
    ] {
        let file = corner_object(&planted)?;
        let preview = preview_frame(&file, FrameRequest::default())
            .map_err(|error| format!("{name}: the frame was refused with {}", error.code()))?;
        if preview.calibration_source != source || preview.calibration_mm != mm {
            return Err(format!(
                "{name}: the preview carried {:?} {:?}",
                preview.calibration_source, preview.calibration_mm
            ));
        }
        let measured = measure_frame(&file, 0, MeasureShape::Length, &segment);
        match (measured, mm) {
            (Ok(json), Some((row, column))) => {
                let want = format!(
                    "\"calibration\":{{\"source\":\"{source}\",\"row_mm\":{row},\"column_mm\":{column}}}"
                );
                if !json.contains(&want) {
                    return Err(format!("{name}: /measure reported {json}"));
                }
            }
            (Ok(json), None) => {
                if !json.contains(&format!("\"calibration\":{{\"source\":\"{source}\"}}")) {
                    return Err(format!("{name}: /measure reported {json}"));
                }
            }
            (Err(PreviewError::Calibration), None) if source.is_empty() => {}
            (other, _) => return Err(format!("{name}: /measure gave {other:?}")),
        }
    }
    // The reply headers, source and value.
    let file = corner_object(&[imager])?;
    let response = one_exchange(&http_post(&file))?;
    let (header, _) = split_http(&response)?;
    if header_line(header, "X-Ocelli-Calibration-Source") != Some("detector")
        || header_line(header, "X-Ocelli-Calibration-Row") != Some("0.2")
        || header_line(header, "X-Ocelli-Calibration-Column") != Some("0.2")
    {
        return Err(format!("the reply did not carry the calibration: {header}"));
    }
    // A non-square pair keeps row and column apart. The bar reads the column.
    let file = corner_object(&[planted_text((0x0028, 0x0030), *b"DS", "0.25\\0.5")])?;
    let response = one_exchange(&http_post(&file))?;
    let (header, _) = split_http(&response)?;
    if header_line(header, "X-Ocelli-Calibration-Row") != Some("0.25")
        || header_line(header, "X-Ocelli-Calibration-Column") != Some("0.5")
    {
        return Err(format!("the reply swapped the calibration pair: {header}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// F-213. Window presets on CT only, every file window by its explanation,
// and the file window index.
//
// PS3.3 C.11.2.1.2.2: Window Center and Window Width are pairs, multiple
// values are alternative views, and Window Center & Width Explanation
// (0028,1055) has one value per pair. PS3.3 C.11.2.1.2.1 is the LINEAR
// function, `c' = c - 0.5`, `w' = w - 1`, `y = ((x - c') / w' + 0.5) * 255`
// inside the window. `display_index` addresses the largest n with
// `f32(n) <= y + 0.5`. Every file below is one MONOCHROME2 pixel storing 50
// with slope 1 and intercept 0, so the modality value is 50 under PS3.3
// C.11.1, Table C.11-1.

/// One windowed 1 by 1 frame. Each field is written only when it is `Some`.
#[derive(Default)]
struct WindowSpec<'a> {
    modality: Option<&'a str>,
    centers: Option<&'a str>,
    widths: Option<&'a str>,
    explanation: Option<&'a str>,
    function: Option<&'a str>,
    /// One VOI LUT Sequence item per entry, with its LUT Explanation.
    voi_luts: Option<&'a [Option<&'a str>]>,
}

fn window_file(spec: &WindowSpec<'_>) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    if let Some(modality) = spec.modality {
        data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad(modality))?);
    }
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    for (tag, value) in [
        ((0x0028, 0x0010), 1_u16),
        ((0x0028, 0x0011), 1),
        ((0x0028, 0x0100), 16),
        ((0x0028, 0x0101), 16),
        ((0x0028, 0x0102), 15),
        ((0x0028, 0x0103), 0),
    ] {
        data.extend(short_le(tag, *b"US", &value.to_le_bytes())?);
    }
    if let Some(centers) = spec.centers {
        data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad(centers))?);
    }
    if let Some(widths) = spec.widths {
        data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad(widths))?);
    }
    data.extend(rescale_one()?);
    if let Some(explanation) = spec.explanation {
        data.extend(short_le((0x0028, 0x1055), *b"LO", &text_pad(explanation))?);
    }
    if let Some(function) = spec.function {
        data.extend(short_le((0x0028, 0x1056), *b"CS", &text_pad(function))?);
    }
    if let Some(items) = spec.voi_luts {
        let mut body = Vec::new();
        for explanation in items {
            let content = match explanation {
                Some(text) => short_le((0x0028, 0x3003), *b"LO", &text_pad(text))?,
                None => Vec::new(),
            };
            body.extend(item(&content)?);
        }
        data.extend(long_le((0x0028, 0x3010), *b"SQ", &body)?);
    }
    data.extend(pixels_50()?);
    part10(data)
}

/// The AC1 file: centres `40\-600`, widths `400\1500`, LINEAR by default.
fn two_windows(modality: &str) -> Result<Vec<u8>, String> {
    window_file(&WindowSpec {
        modality: Some(modality),
        centers: Some("40\\-600"),
        widths: Some("400\\1500"),
        explanation: Some("WINDOW1\\WINDOW2"),
        ..WindowSpec::default()
    })
}

fn indexed(index: u32) -> FrameRequest {
    FrameRequest {
        window_index: Some(index),
        ..FrameRequest::default()
    }
}

fn refused_as(
    result: Result<Preview, PreviewError>,
    want: PreviewError,
    name: &str,
) -> Result<(), String> {
    match result {
        Err(error) if error == want => Ok(()),
        Err(error) => Err(format!("{name}: refused as {}", error.code())),
        Ok(preview) => Err(format!(
            "{name}: drew grey {:?} at centre {}",
            preview.grey, preview.window_center
        )),
    }
}

#[test]
fn a_file_window_index_selects_that_pair() -> Result<(), String> {
    // AC1. Index 0: c' = 39.5, w' = 399, -160 < 50 <= 239, so
    // y = ((50 - 39.5) / 399 + 0.5) * 255 = 134.21, grey 134.
    // Index 1: c' = -600.5, w' = 1499, -1350 < 50 <= 149, so
    // y = ((50 + 600.5) / 1499 + 0.5) * 255 = 238.16, grey 238.
    // Drawing index 0 for both, or reporting the first pair, is refused.
    let file = two_windows("CT")?;
    for (request, grey, center, width, index) in [
        (FrameRequest::default(), 134_u8, 40.0_f32, 400.0_f32, 0_u32),
        (indexed(0), 134, 40.0, 400.0, 0),
        (indexed(1), 238, -600.0, 1500.0, 1),
    ] {
        let preview = preview_frame(&file, request)
            .map_err(|error| format!("index {index} refused as {}", error.code()))?;
        if only_byte(&preview)? != grey {
            return Err(format!(
                "index {index} drew {:?}, wanted {grey}",
                preview.grey
            ));
        }
        if preview.window_center.to_bits() != center.to_bits()
            || preview.window_width.to_bits() != width.to_bits()
            || preview.window_function != "LINEAR"
        {
            return Err(format!(
                "index {index} reported {} {} {}",
                preview.window_center, preview.window_width, preview.window_function
            ));
        }
        if preview.window_index != Some(index) || preview.window_count != 2 {
            return Err(format!(
                "index {index} reported index {:?} of {}",
                preview.window_index, preview.window_count
            ));
        }
    }
    // The function header still applies with an index, as it does to the
    // file window. PS3.3 C.11.2.1.3.2 LINEAR_EXACT at index 1:
    // ((50 + 600) / 1500 + 0.5) * 255 = 237.99, grey 238. At index 0:
    // ((50 - 40) / 400 + 0.5) * 255 = 133.875, grey 134.
    let exact = preview_frame(
        &file,
        FrameRequest {
            window_index: Some(1),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|error| format!("an index with a function refused as {}", error.code()))?;
    if exact.window_function != "LINEAR_EXACT"
        || exact.window_center.to_bits() != (-600.0_f32).to_bits()
    {
        return Err("an index with a function did not keep its pair".to_owned());
    }
    // A preset and a numeric window report no file index.
    let preset = preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            ..FrameRequest::default()
        },
    )
    .map_err(|error| format!("lung refused as {}", error.code()))?;
    if preset.window_index.is_some() || preset.window_count != 2 {
        return Err("a preset reported a file index".to_owned());
    }
    // Through the route: the index header, and the two reply headers.
    let request = http_post_to("/preview", "X-Ocelli-Window-Index: 1\r\n", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200")
        || header_line(header, "X-Ocelli-Window-Index") != Some("1")
        || header_line(header, "X-Ocelli-Window-Count") != Some("2")
        || header_line(header, "X-Ocelli-Window-Center") != Some("-600")
        || body != [238]
    {
        return Err(format!("the index route answered {header}"));
    }
    let lung = one_exchange(&http_post_to(
        "/preview",
        "X-Ocelli-Window-Preset: lung\r\n",
        &file,
    ))?;
    let (header, _) = split_http(&lung)?;
    if header_line(header, "X-Ocelli-Window-Index") != Some("")
        || header_line(header, "X-Ocelli-Window-Count") != Some("2")
    {
        return Err(format!("a preset reply named an index: {header}"));
    }
    Ok(())
}

#[test]
fn a_file_window_index_is_refused_outside_the_pairs() -> Result<(), String> {
    // AC2. Clamping index 2 to the last pair, or falling to the automatic
    // window, would draw. Each is the window error before a grey byte.
    let file = two_windows("CT")?;
    refused_as(
        preview_frame(&file, indexed(2)),
        PreviewError::Window,
        "index 2",
    )?;
    let bare = mono_frame(false)?;
    refused_as(
        preview_frame(&bare, indexed(0)),
        PreviewError::Window,
        "no window",
    )?;
    refused_as(
        preview_frame(
            &file,
            FrameRequest {
                window_index: Some(0),
                window_preset: Some(WindowPreset::SoftTissue),
                ..FrameRequest::default()
            },
        ),
        PreviewError::Window,
        "index with a preset",
    )?;
    refused_as(
        preview_frame(
            &file,
            FrameRequest {
                window_index: Some(0),
                window_center: Some(40.0),
                window_width: Some(400.0),
                ..FrameRequest::default()
            },
        ),
        PreviewError::Window,
        "index with a numeric window",
    )?;
    // PS3.3 C.11.2.1.2.2 pairs by position. An empty interior centre is
    // not skipped: index 0 is that empty pair and is refused, and index 1
    // is the stored 40 and 400, grey 134 as in AC1.
    let hole = window_file(&WindowSpec {
        modality: Some("CT"),
        centers: Some("\\40"),
        widths: Some("\\400"),
        ..WindowSpec::default()
    })?;
    refused_as(
        preview_frame(&hole, indexed(0)),
        PreviewError::Window,
        "empty pair",
    )?;
    let second = preview_frame(&hole, indexed(1))
        .map_err(|error| format!("the pair after a hole refused as {}", error.code()))?;
    if only_byte(&second)? != 134 || second.window_count != 2 {
        return Err("the pair after an empty slot was not index 1".to_owned());
    }
    for (header, code) in [
        ("X-Ocelli-Window-Index: 2\r\n", "window"),
        ("X-Ocelli-Window-Index: one\r\n", "window"),
        ("X-Ocelli-Window-Index: -1\r\n", "window"),
        (
            "X-Ocelli-Window-Index: 0\r\nX-Ocelli-Window-Preset: lung\r\n",
            "window",
        ),
        (
            "X-Ocelli-Window-Index: 0\r\nX-Ocelli-Window-Index: 0\r\n",
            "header",
        ),
    ] {
        let response = one_exchange(&http_post_to("/preview", header, &file))?;
        let (reply, _) = split_http(&response)?;
        if !reply.contains("400") || header_line(reply, "X-Ocelli-Error") != Some(code) {
            return Err(format!("{header:?} answered {reply}"));
        }
    }
    Ok(())
}

#[test]
fn a_file_window_index_keeps_the_voi_lut_policy() -> Result<(), String> {
    // AC3. PS3.3 C.11.2.1.2.2 lets a renderer offer the VOI LUT and the
    // window pair as alternatives. Keeping the sequence through a preset
    // and an index is this project's policy, so index 0 is refused here.
    // The numeric override still draws, as F-206 made it:
    // LINEAR_EXACT centre 50 width 1, ((50 - 50) / 1 + 0.5) * 255 = 127.5,
    // grey 128.
    let file = window_file(&WindowSpec {
        modality: Some("CT"),
        centers: Some("40"),
        widths: Some("400"),
        voi_luts: Some(&[Some("Bone LUT")]),
        ..WindowSpec::default()
    })?;
    refused_as(
        preview_frame(&file, indexed(0)),
        PreviewError::Window,
        "index on a LUT",
    )?;
    let drawn = preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|error| format!("the override refused as {}", error.code()))?;
    if only_byte(&drawn)? != 128 || drawn.window_index.is_some() {
        return Err("the override on a LUT file did not draw grey 128".to_owned());
    }
    Ok(())
}

fn field<'a>(json: &'a str, name: &str) -> Result<&'a str, String> {
    let needle = format!("\"{name}\":");
    let start = json
        .find(&needle)
        .ok_or_else(|| format!("no {name} in {json}"))?
        + needle.len();
    let rest = json
        .get(start..)
        .ok_or_else(|| format!("{name} is short"))?;
    let (open, close) = match rest.as_bytes().first() {
        Some(b'[') => (b'[', b']'),
        Some(b'{') => (b'{', b'}'),
        _ => {
            let end = rest.find([',', '}']).unwrap_or(rest.len());
            return rest.get(..end).ok_or_else(|| format!("{name} is short"));
        }
    };
    let mut depth = 0_usize;
    for (at, byte) in rest.bytes().enumerate() {
        if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return rest.get(..=at).ok_or_else(|| format!("{name} is short"));
            }
        }
    }
    Err(format!("{name} does not close"))
}

fn windows_of(file: &[u8]) -> Result<String, String> {
    let json =
        inspect_json(file, 0).map_err(|error| format!("inspect refused as {}", error.code()))?;
    Ok(field(&json, "windows")?.to_owned())
}

#[test]
fn inspect_lists_every_window_by_its_explanation() -> Result<(), String> {
    // AC4. One row per Window Center and Window Width pair in file order.
    // The label is the explanation at the same position, PS3.3 C.11.2
    // (0028,1055), or `Window n` counted from 1 when that slot is absent
    // or empty. Extra explanations are ignored. The centre and width are
    // the file's DS tokens.
    let row = |center: &str, width: &str, explanation: &str, label: &str| {
        format!(
            "{{\"center\":{center},\"width\":{width},\"explanation\":\"{explanation}\",\"label\":\"{label}\"}}"
        )
    };
    let cases = [
        (
            "two explanations",
            Some("WINDOW1\\WINDOW2"),
            "40\\-600",
            "400\\1500",
            format!(
                "[{},{}]",
                row("40", "400", "WINDOW1", "WINDOW1"),
                row("-600", "1500", "WINDOW2", "WINDOW2")
            ),
        ),
        (
            "an empty first explanation",
            Some("\\WINDOW2"),
            "40\\-600",
            "400\\1500",
            format!(
                "[{},{}]",
                row("40", "400", "", "Window 1"),
                row("-600", "1500", "WINDOW2", "WINDOW2")
            ),
        ),
        (
            "one explanation for two pairs",
            Some("WINDOW1"),
            "40\\-600",
            "400\\1500",
            format!(
                "[{},{}]",
                row("40", "400", "WINDOW1", "WINDOW1"),
                row("-600", "1500", "", "Window 2")
            ),
        ),
        (
            "three explanations for two pairs",
            Some("A\\B\\C"),
            "40\\-600",
            "400\\1500",
            format!(
                "[{},{}]",
                row("40", "400", "A", "A"),
                row("-600", "1500", "B", "B")
            ),
        ),
        (
            "no explanation",
            None,
            "40",
            "400",
            format!("[{}]", row("40", "400", "", "Window 1")),
        ),
        (
            "an empty interior centre keeps its slot",
            Some("FIRST\\SECOND"),
            "\\40",
            "\\400",
            format!(
                "[{},{}]",
                row("null", "null", "FIRST", "FIRST"),
                row("40", "400", "SECOND", "SECOND")
            ),
        ),
    ];
    for (name, explanation, centers, widths, want) in cases {
        let file = window_file(&WindowSpec {
            modality: Some("CT"),
            centers: Some(centers),
            widths: Some(widths),
            explanation,
            ..WindowSpec::default()
        })?;
        let got = windows_of(&file)?;
        if got != want {
            return Err(format!("{name}: {got}, wanted {want}"));
        }
    }
    let none = windows_of(&mono_frame(false)?)?;
    if none != "[]" {
        return Err(format!("a file with no window listed {none}"));
    }
    // PS3.3 C.11.2 (0028,3003). Each VOI LUT Sequence item is listed by its
    // LUT Explanation, or `Lookup table n`.
    let lut = window_file(&WindowSpec {
        modality: Some("CT"),
        centers: Some("40"),
        widths: Some("400"),
        voi_luts: Some(&[Some("Bone LUT"), None]),
        ..WindowSpec::default()
    })?;
    let json = inspect_json(&lut, 0).map_err(|_| "inspect refused the LUT file".to_owned())?;
    let luts = field(&json, "voiLuts")?;
    let want = "[{\"explanation\":\"Bone LUT\",\"label\":\"Bone LUT\"},{\"explanation\":\"\",\"label\":\"Lookup table 2\"}]";
    if luts != want {
        return Err(format!("voiLuts: {luts}, wanted {want}"));
    }
    if field(
        &inspect_json(&two_windows("CT")?, 0).map_err(|_| "inspect".to_owned())?,
        "voiLuts",
    )? != "[]"
    {
        return Err("a file with no VOI LUT listed one".to_owned());
    }
    Ok(())
}

#[test]
fn presets_are_offered_on_ct_only() -> Result<(), String> {
    // AC5. The numbers are `WindowPreset::center` and `width`, written once
    // by the local process, so the page keeps no second table.
    let ct = inspect_json(&two_windows("CT")?, 0).map_err(|_| "inspect CT".to_owned())?;
    let mut want = String::from("[");
    for (at, (preset, token, name)) in [
        (WindowPreset::SoftTissue, "soft_tissue", "Soft tissue"),
        (WindowPreset::Lung, "lung", "Lung"),
        (WindowPreset::Bone, "bone", "Bone"),
    ]
    .into_iter()
    .enumerate()
    {
        if at > 0 {
            want.push(',');
        }
        want.push_str(&format!(
            "{{\"token\":\"{token}\",\"name\":\"{name}\",\"center\":{},\"width\":{}}}",
            preset.center(),
            preset.width()
        ));
    }
    want.push(']');
    let got = field(&ct, "presets")?;
    if got != want {
        return Err(format!("CT presets: {got}, wanted {want}"));
    }
    let mr = inspect_json(&two_windows("MR")?, 0).map_err(|_| "inspect MR".to_owned())?;
    if field(&mr, "presets")? != "[]" {
        return Err("an MR offered presets".to_owned());
    }
    Ok(())
}

#[test]
fn a_preset_on_mr_is_the_modality_error() -> Result<(), String> {
    // AC6. The presets are Hounsfield numbers. On MR the local process
    // refuses, whatever the page does. The same file as CT draws lung:
    // c' = -600.5, w' = 1499, y = 238.16, grey 238.
    let mr = two_windows("MR")?;
    refused_as(
        preview_frame(
            &mr,
            FrameRequest {
                window_preset: Some(WindowPreset::Lung),
                ..FrameRequest::default()
            },
        ),
        PreviewError::Modality,
        "lung on MR",
    )?;
    if PreviewError::Modality.code() != "modality"
        || PreviewError::Modality.to_string()
            != "these presets are in Hounsfield units and apply to CT only"
    {
        return Err("the modality error is not the planned code and sentence".to_owned());
    }
    let ct = preview_frame(
        &two_windows("CT")?,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            ..FrameRequest::default()
        },
    )
    .map_err(|error| format!("lung on CT refused as {}", error.code()))?;
    if only_byte(&ct)? != 238 {
        return Err("lung on CT was not grey 238".to_owned());
    }
    // A file with no Modality is not CT.
    let absent = window_file(&WindowSpec {
        centers: Some("40"),
        widths: Some("400"),
        ..WindowSpec::default()
    })?;
    refused_as(
        preview_frame(
            &absent,
            FrameRequest {
                window_preset: Some(WindowPreset::Bone),
                ..FrameRequest::default()
            },
        ),
        PreviewError::Modality,
        "bone with no Modality",
    )?;
    let response = one_exchange(&http_post_to(
        "/preview",
        "X-Ocelli-Window-Preset: lung\r\n",
        &mr,
    ))?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || header_line(header, "X-Ocelli-Error") != Some("modality") {
        return Err(format!("lung on MR through the route answered {header}"));
    }
    // AC7's order. A VOI LUT refusal is reported before the modality one,
    // so an MR with a VOI LUT Sequence and a preset is the window error.
    let lut_mr = window_file(&WindowSpec {
        modality: Some("MR"),
        centers: Some("40"),
        widths: Some("400"),
        voi_luts: Some(&[None]),
        ..WindowSpec::default()
    })?;
    refused_as(
        preview_frame(
            &lut_mr,
            FrameRequest {
                window_preset: Some(WindowPreset::Lung),
                ..FrameRequest::default()
            },
        ),
        PreviewError::Window,
        "lung on an MR with a VOI LUT",
    )
}

#[test]
fn windows_per_frame_follows_the_functional_group_source() -> Result<(), String> {
    // AC8. The enhanced fixture gives each frame its own Frame VOI LUT item,
    // so its window comes from a per-frame source and the page must ask
    // again for each frame. Frame 1's list is its own pair, 50 and 1.
    let enhanced = enhanced_frames()?;
    let first = inspect_json(&enhanced, 0).map_err(|_| "inspect frame 0".to_owned())?;
    if field(&first, "windowsPerFrame")? != "true" {
        return Err(format!("the enhanced file was not per frame: {first}"));
    }
    let second = inspect_json(&enhanced, 1).map_err(|_| "inspect frame 1".to_owned())?;
    if field(&second, "windows")?
        != "[{\"center\":50,\"width\":1,\"explanation\":\"\",\"label\":\"Window 1\"}]"
    {
        return Err(format!("frame 1 did not list its own window: {second}"));
    }
    let single = inspect_json(&two_windows("CT")?, 0).map_err(|_| "inspect single".to_owned())?;
    if field(&single, "windowsPerFrame")? != "false" {
        return Err("a single-frame file was per frame".to_owned());
    }
    Ok(())
}

#[test]
fn inspect_by_id_matches_the_posted_body() -> Result<(), String> {
    // AC17. The page asks again for a frame by id with no body, so a frame
    // change never posts the file again. The answer is the body form's.
    let file = enhanced_frames()?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id = header_line(header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?
        .to_owned();
    let by_body = exchange(
        addr,
        &http_post_to("/inspect", "X-Ocelli-Frame: 1\r\n", &file),
    )?;
    let by_id = exchange(
        addr,
        &http_post_to(
            "/inspect",
            &format!("X-Ocelli-File: {id}\r\nX-Ocelli-Frame: 1\r\n"),
            b"",
        ),
    )?;
    let (body_header, body_json) = split_http(&by_body)?;
    let (id_header, id_json) = split_http(&by_id)?;
    if !body_header.contains("200") || !id_header.contains("200") || body_json != id_json {
        return Err(format!(
            "by id {id_header} differs from the body form {body_header}"
        ));
    }
    // Review pass 1, D1. Once every frame is held the process drops the
    // posted bytes. The id form must still answer, from metadata the
    // process keeps for the id's lifetime, and give the same body.
    for frame in 0..2 {
        let drawn = exchange(
            addr,
            &preview_id(&id, &format!("X-Ocelli-Frame: {frame}\r\n")),
        )?;
        let (drawn_header, _) = split_http(&drawn)?;
        if !drawn_header.contains("200") {
            return Err(format!("frame {frame} did not draw: {drawn_header}"));
        }
    }
    let (files, frames, _) = held_counts(addr)?;
    if files != 0 || frames != 2 {
        return Err(format!(
            "after both frames the process held {files} files and {frames} frames"
        ));
    }
    let after = exchange(
        addr,
        &http_post_to(
            "/inspect",
            &format!("X-Ocelli-File: {id}\r\nX-Ocelli-Frame: 1\r\n"),
            b"",
        ),
    )?;
    let (after_header, after_json) = split_http(&after)?;
    if !after_header.contains("200") || after_json != body_json {
        return Err(format!(
            "by id after every frame is held answered {after_header}"
        ));
    }
    let both = exchange(
        addr,
        &http_post_to("/inspect", &format!("X-Ocelli-File: {id}\r\n"), &file),
    )?;
    let (both_header, _) = split_http(&both)?;
    if !both_header.contains("400") || header_line(both_header, "X-Ocelli-Error") != Some("header")
    {
        return Err(format!("an id with a body answered {both_header}"));
    }
    let gone = exchange(
        addr,
        &http_post_to(
            "/inspect",
            &format!("X-Ocelli-File: {}\r\n", other_generation(&id)?),
            b"",
        ),
    )?;
    let (gone_header, _) = split_http(&gone)?;
    if !gone_header.contains("410")
        || header_line(gone_header, "X-Ocelli-Error") != Some("file-gone")
    {
        return Err(format!("an unknown id answered {gone_header}"));
    }
    // The kept metadata leaves with the id through F-226's forget path.
    forget_exchange(addr, "", id.as_bytes())?;
    let forgotten = exchange(
        addr,
        &http_post_to(
            "/inspect",
            &format!("X-Ocelli-File: {id}\r\nX-Ocelli-Frame: 1\r\n"),
            b"",
        ),
    )?;
    let (forgotten_header, _) = split_http(&forgotten)?;
    if !forgotten_header.contains("410") {
        return Err(format!("a forgotten id answered {forgotten_header}"));
    }
    Ok(())
}

#[test]
fn an_empty_first_window_slot_opens_at_the_first_usable_pair() -> Result<(), String> {
    // Lead ruling on amendment 6. Centres `\40` and widths `\400`: slot 0 is
    // empty, slot 1 is 40 and 400. With no index the file opens drawn at the
    // first slot holding both values, PS3.3 C.11.2.1.2.1 LINEAR:
    // c' = 39.5, w' = 399, y = ((50 - 39.5) / 399 + 0.5) * 255 = 134.21,
    // grey 134, reported as index 1. Choosing the empty slot is refused at
    // its own index.
    let hole = window_file(&WindowSpec {
        modality: Some("CT"),
        centers: Some("\\40"),
        widths: Some("\\400"),
        ..WindowSpec::default()
    })?;
    let opened = preview_frame(&hole, FrameRequest::default())
        .map_err(|error| format!("the file opened as {}", error.code()))?;
    if only_byte(&opened)? != 134
        || opened.window_center.to_bits() != 40.0_f32.to_bits()
        || opened.window_index != Some(1)
        || opened.window_count != 2
    {
        return Err(format!(
            "opened grey {:?} centre {} index {:?}",
            opened.grey, opened.window_center, opened.window_index
        ));
    }
    refused_as(
        preview_frame(&hole, indexed(0)),
        PreviewError::Window,
        "the empty slot",
    )?;
    let response = one_exchange(&http_post_to("/preview", "", &hole))?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200")
        || header_line(header, "X-Ocelli-Window-Index") != Some("1")
        || body != [134]
    {
        return Err(format!("the route opened {header}"));
    }
    Ok(())
}

/// The enhanced two-frame fixture as RLE Lossless, PS3.5 Annex G.
///
/// Each 1 by 1 frame of 16-bit samples is two RLE segments, the most
/// significant byte first (G.2), each a literal run of one byte: header
/// byte 0 then the byte (G.3.1). The RLE header is 16 little-endian `u32`,
/// the segment count then the offsets, so the segments sit at 64 and 66.
/// Stored 50 is high byte 0 and low byte 50. PS3.5 A.4 Basic Offset Table
/// carries each frame's first fragment offset.
fn enhanced_frames_rle() -> Result<Vec<u8>, String> {
    let plain = enhanced_frames()?;
    let pixel_at = plain
        .windows(4)
        .rposition(|window| window == [0xE0, 0x7F, 0x10, 0x00])
        .ok_or_else(|| "the fixture has no pixel data".to_owned())?;
    let mut fragment = Vec::new();
    fragment.extend_from_slice(&2_u32.to_le_bytes());
    fragment.extend_from_slice(&64_u32.to_le_bytes());
    fragment.extend_from_slice(&66_u32.to_le_bytes());
    fragment.extend_from_slice(&[0_u8; 52]);
    fragment.extend_from_slice(&[0, 0, 0, 50]);
    // The Basic Offset Table names both frames: 0, then one 8-byte item
    // header plus the 68-byte fragment, 76.
    let mut table = Vec::new();
    table.extend_from_slice(&0_u32.to_le_bytes());
    table.extend_from_slice(&76_u32.to_le_bytes());
    let mut pixels = item(&table)?;
    for _ in 0..2 {
        pixels.extend(item(&fragment)?);
    }
    pixels.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    pixels.extend_from_slice(&0xE0DD_u16.to_le_bytes());
    pixels.extend_from_slice(&0_u32.to_le_bytes());
    let mut data = plain
        .get(..pixel_at)
        .ok_or_else(|| "the fixture is short".to_owned())?
        .to_vec();
    // The dataset starts after the 128-byte preamble, `DICM` and the meta.
    let meta = file_meta("1.2.840.10008.1.2.1")?;
    let start = 132 + meta.len();
    let body = data.split_off(start);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.5")?);
    bytes.extend(body);
    bytes.extend_from_slice(&0x7FE0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes());
    bytes.extend(pixels);
    Ok(bytes)
}

#[test]
fn a_compressed_per_frame_multiframe_is_inspected_by_id_after_every_frame_is_held()
-> Result<(), String> {
    // Review pass 2, D1 and S1. The kept reply does not depend on the
    // transfer syntax. Both RLE frames decode to 50 and the enhanced file's
    // LINEAR_EXACT windows give grey 64 and 128, as in
    // `an_enhanced_frame_uses_its_functional_group_window`.
    let file = enhanced_frames_rle()?;
    for (frame, grey) in [(0_u32, 64_u8), (1, 128)] {
        let preview = preview_frame(
            &file,
            FrameRequest {
                frame,
                ..FrameRequest::default()
            },
        )
        .map_err(|error| format!("RLE frame {frame} refused as {}", error.code()))?;
        if only_byte(&preview)? != grey {
            return Err(format!("RLE frame {frame} drew {:?}", preview.grey));
        }
    }
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id = header_line(header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?
        .to_owned();
    // The pool keeps the frame-independent parts once and each frame's
    // fields beside them, so it holds something, and less than the two
    // whole replies. Its exact charge is pinned by the unit test
    // `kept_replies_are_charged_at_their_heap_bytes`.
    let whole: usize = (0..2)
        .map(|frame| inspect_json(&file, frame).map(|json| json.len()))
        .sum::<Result<usize, _>>()
        .map_err(|_| "inspect refused the RLE file".to_owned())?;
    let kept = json_number(&held_census(addr)?, "kept")?;
    if kept == 0 || kept >= whole {
        return Err(format!(
            "the pool holds {kept} bytes for two replies of {whole}"
        ));
    }
    for frame in 0..2 {
        let drawn = exchange(
            addr,
            &preview_id(&id, &format!("X-Ocelli-Frame: {frame}\r\n")),
        )?;
        let (drawn_header, _) = split_http(&drawn)?;
        if !drawn_header.contains("200") {
            return Err(format!("RLE frame {frame} by id: {drawn_header}"));
        }
    }
    let (files, frames, _) = held_counts(addr)?;
    if files != 0 || frames != 2 {
        return Err(format!(
            "the process held {files} files and {frames} frames"
        ));
    }
    let by_body = exchange(
        addr,
        &http_post_to("/inspect", "X-Ocelli-Frame: 1\r\n", &file),
    )?;
    let by_id = exchange(
        addr,
        &http_post_to(
            "/inspect",
            &format!("X-Ocelli-File: {id}\r\nX-Ocelli-Frame: 1\r\n"),
            b"",
        ),
    )?;
    let (_, body_json) = split_http(&by_body)?;
    let (id_header, id_json) = split_http(&by_id)?;
    if !id_header.contains("200") || id_json != body_json {
        return Err(format!("the RLE file by id answered {id_header}"));
    }
    Ok(())
}

#[test]
fn a_file_whose_windows_do_not_change_by_frame_keeps_nothing() -> Result<(), String> {
    // S1. A one-frame file, and a multiframe whose windows are top level,
    // never ask by id, so neither spends the pool.
    let addr = viewer_on(8 * 1024 * 1024)?;
    for file in [two_windows("CT")?, tagged_frame(2, &[50, 0, 60, 0])?] {
        let posted = exchange(addr, &http_post_to("/file", "", &file))?;
        let (header, _) = split_http(&posted)?;
        if !header.contains("200") {
            return Err(format!("the post answered {header}"));
        }
    }
    let kept = json_number(&held_census(addr)?, "kept")?;
    if kept != 0 {
        return Err(format!("files without per-frame windows kept {kept} bytes"));
    }
    Ok(())
}

/// Review pass 3, D2. Three 1 by 1 frames storing 50, slope 1 and intercept
/// 0 in the shared group. Frame 0's per-frame item has no Frame VOI LUT, so
/// it falls back to the top-level `40\-600`, `400\1500`, `TOPA\TOPB`.
/// Frames 1 and 2 carry 300 and 1500 explained `F1ONLY` per frame.
fn late_per_frame() -> Result<Vec<u8>, String> {
    own_window_frames(&[false, true, true])
}

/// [`late_per_frame`]'s file with one 1 by 1 frame per entry, each `true`
/// frame carrying its own `F1ONLY` window and each `false` frame taking the
/// top-level one.
fn own_window_frames(own_frames: &[bool]) -> Result<Vec<u8>, String> {
    let rescale = item(&concat_bytes(&[
        &short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?,
        &short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
    ]))?;
    let shared = item(&long_le((0x0028, 0x9145), *b"SQ", &rescale)?)?;
    let own = item(&long_le(
        (0x0028, 0x9132),
        *b"SQ",
        &item(&concat_bytes(&[
            &short_le((0x0028, 0x1050), *b"DS", &text_pad("300"))?,
            &short_le((0x0028, 0x1051), *b"DS", &text_pad("1500"))?,
            &short_le((0x0028, 0x1055), *b"LO", &text_pad("F1ONLY"))?,
        ]))?,
    )?)?;
    let empty = item(&[])?;
    let per_frame: Vec<u8> = own_frames
        .iter()
        .flat_map(|own_window| {
            if *own_window {
                own.clone()
            } else {
                empty.clone()
            }
        })
        .collect();
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2.1"),
    )?);
    data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad("CT"))?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le(
        (0x0028, 0x0008),
        *b"IS",
        &text_pad(&own_frames.len().to_string()),
    )?);
    for (tag, value) in [
        ((0x0028, 0x0010), 1_u16),
        ((0x0028, 0x0011), 1),
        ((0x0028, 0x0100), 16),
        ((0x0028, 0x0101), 16),
        ((0x0028, 0x0102), 15),
        ((0x0028, 0x0103), 0),
    ] {
        data.extend(short_le(tag, *b"US", &value.to_le_bytes())?);
    }
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("40\\-600"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("400\\1500"))?);
    data.extend(short_le((0x0028, 0x1055), *b"LO", &text_pad("TOPA\\TOPB"))?);
    data.extend(long_le((0x5200, 0x9229), *b"SQ", &shared)?);
    data.extend(long_le((0x5200, 0x9230), *b"SQ", &per_frame)?);
    data.extend(long_le(
        (0x7FE0, 0x0010),
        *b"OW",
        &50_u16.to_le_bytes().repeat(own_frames.len()),
    )?);
    part10(data)
}

#[test]
fn later_frames_with_their_own_windows_are_per_frame() -> Result<(), String> {
    // Review pass 3, D2. The decision reads every frame, not frame 0's. The
    // scan of frame 0 says per frame, so the page asks frame 1 for its list,
    // and the process keeps the replies. Frame 0 lists the top-level pair,
    // frame 1 its own. Frame 1 at its first window, PS3.3 C.11.2.1.2.1
    // LINEAR centre 300 width 1500: c' = 299.5, w' = 1499,
    // ((50 - 299.5) / 1499 + 0.5) * 255 = 85.06, grey 85.
    let file = late_per_frame()?;
    let first = inspect_json(&file, 0).map_err(|_| "inspect frame 0".to_owned())?;
    if field(&first, "windowsPerFrame")? != "true" {
        return Err(format!("frame 0 did not say per frame: {first}"));
    }
    if !field(&first, "windows")?.contains("\"TOPB\"") {
        return Err(format!("frame 0 lost the top-level list: {first}"));
    }
    let second = inspect_json(&file, 1).map_err(|_| "inspect frame 1".to_owned())?;
    if field(&second, "windows")?
        != "[{\"center\":300,\"width\":1500,\"explanation\":\"F1ONLY\",\"label\":\"F1ONLY\"}]"
    {
        return Err(format!("frame 1 did not list its own window: {second}"));
    }
    let drawn = preview_frame(
        &file,
        FrameRequest {
            frame: 1,
            ..FrameRequest::default()
        },
    )
    .map_err(|error| format!("frame 1 refused as {}", error.code()))?;
    if only_byte(&drawn)? != 85 {
        return Err(format!("frame 1 drew {:?}", drawn.grey));
    }
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    if !header.contains("200") || json_number(&held_census(addr)?, "kept")? == 0 {
        return Err("the replies of a late per-frame file were not kept".to_owned());
    }
    Ok(())
}

#[test]
fn a_middle_frame_alone_with_its_own_window_is_per_frame() -> Result<(), String> {
    // Review pass 4, S1. Five frames, and only frame 2 carries its own
    // window. A decision that read only frame 1, only the last frame or only
    // frame 0 would say no, and the page would name frame 2 with the
    // top-level label beside its own window.
    let file = own_window_frames(&[false, false, true, false, false])?;
    let first = inspect_json(&file, 0).map_err(|_| "inspect frame 0".to_owned())?;
    if field(&first, "windowsPerFrame")? != "true" {
        return Err(format!("frame 0 did not say per frame: {first}"));
    }
    let middle = inspect_json(&file, 2).map_err(|_| "inspect frame 2".to_owned())?;
    if field(&middle, "windows")?
        != "[{\"center\":300,\"width\":1500,\"explanation\":\"F1ONLY\",\"label\":\"F1ONLY\"}]"
    {
        return Err(format!("frame 2 did not list its own window: {middle}"));
    }
    if !field(
        &inspect_json(&file, 3).map_err(|_| "inspect frame 3".to_owned())?,
        "windows",
    )?
    .contains("\"TOPB\"")
    {
        return Err("frame 3 lost the top-level list".to_owned());
    }
    Ok(())
}

/// F-225's reproducer. The CT base data set of the plan's acceptance table,
/// every Type 1 and Type 2 attribute of the seven PS3.3 modules present,
/// with four changes: Patient's Age `040 `, Institution Name of 170 `A`,
/// SOP Instance UID and Modality present with zero length.
fn conformance_reproducer(rows: u16, columns: u16) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0018), *b"UI", &[])?);
    data.extend(short_le((0x0008, 0x0020), *b"DA", &text_pad("19991231"))?);
    data.extend(short_le((0x0008, 0x0030), *b"TM", &text_pad("153045"))?);
    data.extend(short_le((0x0008, 0x0050), *b"SH", &text_pad("A1"))?);
    data.extend(short_le((0x0008, 0x0060), *b"CS", &[])?);
    data.extend(short_le(
        (0x0008, 0x0080),
        *b"LO",
        &text_pad(&"A".repeat(170)),
    )?);
    data.extend(short_le((0x0008, 0x0090), *b"PN", &text_pad("N"))?);
    data.extend(short_le(
        (0x0010, 0x0010),
        *b"PN",
        &text_pad("SECRET^NAME"),
    )?);
    data.extend(short_le((0x0010, 0x0020), *b"LO", &text_pad("P1"))?);
    data.extend(short_le((0x0010, 0x0030), *b"DA", &text_pad("19991231"))?);
    data.extend(short_le((0x0010, 0x0040), *b"CS", &text_pad("O"))?);
    data.extend(short_le((0x0010, 0x1010), *b"AS", b"040 ")?);
    data.extend(short_le((0x0018, 0x0050), *b"DS", &text_pad("5"))?);
    data.extend(short_le((0x0018, 0x5100), *b"CS", &text_pad("HFS"))?);
    data.extend(short_le((0x0020, 0x000D), *b"UI", &ui(b"9.8.7"))?);
    data.extend(short_le((0x0020, 0x000E), *b"UI", &ui(b"9.8.8"))?);
    data.extend(short_le((0x0020, 0x0010), *b"SH", &text_pad("S1"))?);
    data.extend(short_le((0x0020, 0x0011), *b"IS", &text_pad("1"))?);
    data.extend(short_le((0x0020, 0x0013), *b"IS", &text_pad("1"))?);
    data.extend(short_le((0x0020, 0x0032), *b"DS", &text_pad("0\\0\\0"))?);
    data.extend(short_le(
        (0x0020, 0x0037),
        *b"DS",
        &text_pad("1\\0\\0\\0\\1\\0"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &rows.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &columns.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0030), *b"DS", &text_pad("1\\1"))?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    let pixels = vec![0_u8; usize::from(rows) * usize::from(columns) * 2];
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    part10(data)
}

#[test]
fn conformance_by_id_matches_the_body_form() -> Result<(), String> {
    // F-225 AC32. The expected findings are the plan's reproducer, each
    // derived from the standard and not from a run of the checker.
    //   (0008,0018) SOP Instance UID, Type 1 in PS3.3 Table C.12-1, zero
    //     length. PS3.5 7.4.1: "The Length of the Value Field shall not be
    //     zero." TYPE1-EMPTY, no value number, no measure.
    //   (0008,0060) Modality, Type 1 in PS3.3 Table C.7-5a, zero length.
    //     TYPE1-EMPTY as above.
    //   (0008,0080) Institution Name, LO, PS3.5 Table 6.2-1 "64 chars
    //     maximum". 170 characters, VALUE-LENGTH value 1, 170 against 64.
    //   (0010,1010) Patient's Age, AS, PS3.5 Table 6.2-1 "nnnD, nnnW, nnnM,
    //     nnnY" in "4 bytes fixed". `040 ` has a SPACE where the unit letter
    //     goes, so AS-FORMAT value 1 and no measure.
    // Four findings and no fifth, so the badge reads 4.
    let file = conformance_reproducer(1, 1)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (posted_header, _) = split_http(&posted)?;
    if !posted_header.starts_with("HTTP/1.1 200 ") {
        return Err(format!("the reproducer did not open: {posted_header}"));
    }
    let id = header_line(posted_header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?;
    let by_id = exchange(
        addr,
        &http_post_to("/conformance", &format!("X-Ocelli-File: {id}\r\n"), &[]),
    )?;
    let (id_header, id_body) = split_http(&by_id)?;
    if !id_header.starts_with("HTTP/1.1 200 ")
        || header_line(id_header, "Content-Type") != Some("application/json; charset=utf-8")
    {
        return Err(format!("the id form was not JSON: {id_header}"));
    }
    let by_body = exchange(addr, &http_post_to("/conformance", "", &file))?;
    let (body_header, body_body) = split_http(&by_body)?;
    if !body_header.starts_with("HTTP/1.1 200 ") || body_body != id_body {
        return Err("the body form did not answer the same bytes as the id form".to_owned());
    }
    let json = std::str::from_utf8(id_body).map_err(|_| "the report is not text".to_owned())?;
    let scope = format!("{{\"scope\":\"{}\"", ocelli_dicom::CONFORMANCE_SCOPE);
    if !json.starts_with(&scope) {
        return Err(format!("the report does not open with its scope: {json}"));
    }
    let expected = [
        "{\"tag\":\"(0008,0018)\",\"keyword\":\"SOPInstanceUID\",\"path\":[],\"rule\":\"TYPE1-EMPTY\",\"value\":null,\"measure\":null}",
        "{\"tag\":\"(0008,0060)\",\"keyword\":\"Modality\",\"path\":[],\"rule\":\"TYPE1-EMPTY\",\"value\":null,\"measure\":null}",
        "{\"tag\":\"(0008,0080)\",\"keyword\":\"InstitutionName\",\"path\":[],\"rule\":\"VALUE-LENGTH\",\"value\":1,\"measure\":{\"unit\":\"characters\",\"measured\":170,\"limit\":64}}",
        "{\"tag\":\"(0010,1010)\",\"keyword\":\"PatientAge\",\"path\":[],\"rule\":\"AS-FORMAT\",\"value\":1,\"measure\":null}",
    ];
    for finding in expected {
        if !json.contains(finding) {
            return Err(format!("the report lacks {finding}: {json}"));
        }
    }
    if json.matches("\"rule\":\"").count() != expected.len() {
        return Err(format!(
            "the report does not hold exactly four findings: {json}"
        ));
    }
    for secret in ["040", "SECRET", "NAME", &"A".repeat(64)] {
        if json.contains(secret) {
            return Err(format!("the report repeats a value, {secret}"));
        }
    }

    // A non-Part-10 body answers exactly what `/headers` answers.
    let refused = exchange(addr, &http_post_to("/conformance", "", b"nope"))?;
    let headers_refused = exchange(addr, &http_post_to("/headers", "", b"nope"))?;
    if refused != headers_refused || !refused.starts_with(b"HTTP/1.1 4") {
        return Err(format!(
            "the refusal differs from /headers: {}",
            String::from_utf8_lossy(&refused)
        ));
    }
    // An id with a body, and an id the process does not hold, answer as
    // `/headers` does.
    let id_head = format!("X-Ocelli-File: {id}\r\n");
    let both = exchange(addr, &http_post_to("/conformance", &id_head, b"nope"))?;
    let headers_both = exchange(addr, &http_post_to("/headers", &id_head, b"nope"))?;
    if both != headers_both || !both.starts_with(b"HTTP/1.1 400 ") {
        return Err("an id with a body was not refused as /headers refuses it".to_owned());
    }
    let unknown = "X-Ocelli-File: 999999.999999\r\n";
    let gone = exchange(addr, &http_post_to("/conformance", unknown, &[]))?;
    let headers_gone = exchange(addr, &http_post_to("/headers", unknown, &[]))?;
    if gone != headers_gone || !gone.starts_with(b"HTTP/1.1 410 ") {
        return Err(format!(
            "an unknown id was not refused as /headers refuses it: {}",
            String::from_utf8_lossy(&gone)
        ));
    }

    // `/headers` is unchanged by the new route. Its id form answers what
    // `headers_json` answers for the same bytes.
    let headers = exchange(addr, &http_post_to("/headers", &id_head, &[]))?;
    let (_, headers_body) = split_http(&headers)?;
    let direct = headers_json(&file).map_err(|_| "headers_json refused the file".to_owned())?;
    if headers_body != direct.as_bytes() {
        return Err("the /headers reply changed".to_owned());
    }
    Ok(())
}

#[test]
fn the_page_shows_the_conformance_checks() -> Result<(), String> {
    // F-225 AC33, the page source half. The browser half is
    // `tests/browser/conformance.mjs`.
    let page = served_page("/")?;
    require(
        &page,
        &[
            // The badge sits in the Headers button and starts hidden.
            "<span class=\"sr\">Headers</span><span class=\"badge\" id=\"headers-badge\" hidden></span></button>",
            // The Checks section is in the sheet above the headers table.
            "<h3 id=\"checks-title\">Checks</h3>",
            "id=\"checks-scope\"",
            // Decision 14: 300 ms after the first paint, cached per file,
            // at most 512 reports, through the library reader.
            "const CHECKS_DELAY_MS = 300;",
            "const CHECKS_CAP = 512;",
            "core.readConformanceReport(text)",
            "checksScope.textContent = report.scope;",
            "\"The checks need the library build. Run npm run build in the repository, then start the viewer again.\"",
            // Never beside a folder read or a thumbnail run.
            "if (filePostsOpen > 0 || thumbRuns > 0) {",
        ],
        "the conformance checks",
    )?;
    let sheet = page
        .find("<div id=\"modal\" hidden>")
        .ok_or_else(|| "the page has no headers sheet".to_owned())?;
    let checks = page
        .find("<section id=\"checks\"")
        .ok_or_else(|| "the sheet has no Checks section".to_owned())?;
    let table = page
        .find("<tbody id=\"header-rows\">")
        .ok_or_else(|| "the sheet has no headers table".to_owned())?;
    if !(sheet < checks && checks < table) {
        return Err("the Checks section is not inside the sheet above the table".to_owned());
    }
    // One function sends the request, and every `POST /file` is counted, so
    // the wait in `checksDue` sees each of them.
    if page.matches("fetch(\"/conformance\"").count() != 2
        || page.matches("fetch(\"/file\"").count() != 1
        || !page.contains("filePostsOpen += 1;\n  try {\n    return await fetch(\"/file\"")
    {
        return Err("a conformance or file request escapes its one sender".to_owned());
    }
    Ok(())
}

#[test]
fn the_pages_carry_their_own_policy() -> Result<(), String> {
    // CSP3 6.7.3.3 hashes element source. Each response must describe its
    // own literal, including the import map, without widening the policy.
    let mut policies = Vec::new();
    for (paths, page) in [
        (["/", "/index.html"], include_str!("../src/page.html")),
        (["/use", "/use.html"], include_str!("../src/use.html")),
        (["/more", "/more.html"], include_str!("../src/more.html")),
    ] {
        assert!(!page.contains('\r'), "page source has a carriage return");
        assert!(
            !page.contains(" style="),
            "page source has an inline style attribute"
        );
        let mut pair = Vec::new();
        for path in paths {
            let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
            let response = one_exchange(request.as_bytes())?;
            let (head, body) = split_http(&response)?;
            assert!(head.starts_with("HTTP/1.1 200 "), "{path}");
            assert_eq!(body, page.as_bytes(), "{path} served another page");
            let policy = head
                .lines()
                .find_map(|line| line.strip_prefix("Content-Security-Policy: "))
                .ok_or_else(|| format!("{path} has no Content-Security-Policy"))?;
            assert!(
                policy.starts_with("default-src 'none'; script-src 'self'"),
                "{path}"
            );
            assert!(policy.contains("'sha256-"), "{path} has no inline hashes");
            for forbidden in [
                "'unsafe-inline'",
                "'unsafe-eval'",
                "'wasm-unsafe-eval'",
                "blob:",
                "'unsafe-hashes'",
            ] {
                assert!(!policy.contains(forbidden), "{path} allows {forbidden}");
            }
            pair.push(policy.to_owned());
        }
        assert_eq!(pair.first(), pair.get(1), "aliases have different policies");
        policies.push(pair.remove(0));
    }
    assert_ne!(
        policies.first(),
        policies.get(1),
        "image and Use policies are not page-specific"
    );
    assert_ne!(
        policies.get(1),
        policies.get(2),
        "Use and More policies are not page-specific"
    );
    Ok(())
}

/// PS3.3 C.7.6.3, a native unsigned 8-bit colour test object.
fn colour_server_frame(
    rows: u16,
    columns: u16,
    space: &str,
    planar: u16,
    pixels: &[u8],
) -> Result<Vec<u8>, String> {
    let mut data = short_le((0x0008, 0x0016), *b"UI", &ui(b"1.2.840.10008.5.1.4.1.1.7"))?;
    for (tag, value) in [
        ((0x0028, 0x0002), 3),
        ((0x0028, 0x0006), planar),
        ((0x0028, 0x0010), rows),
        ((0x0028, 0x0011), columns),
        ((0x0028, 0x0100), 8),
        ((0x0028, 0x0101), 8),
        ((0x0028, 0x0102), 7),
        ((0x0028, 0x0103), 0),
    ] {
        data.extend(short_le(tag, *b"US", &value.to_le_bytes())?);
    }
    data.extend(short_le((0x0028, 0x0004), *b"CS", &text_pad(space))?);
    let mut value = pixels.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    data.extend(long_le((0x7fe0, 0x0010), *b"OB", &value)?);
    part10(data)
}

#[test]
fn colour_server_channels_and_held_window_survive_a_refusal() -> Result<(), String> {
    // D-29 uses the existing LINEAR formula per channel. C128/W100 maps
    // 100,30,200 to 57,0,255, inverse before rounding to 198,255,0.
    let file = colour_server_frame(1, 2, "RGB", 0, &[100, 30, 200, 10, 20, 30])?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, body) = split_http(&posted)?;
    let inspect = std::str::from_utf8(body).map_err(|error| error.to_string())?;
    assert!(inspect.contains("\"colour\":true,\"icc\":false"));
    let id = header_line(header, "X-Ocelli-File").ok_or("no held id")?;
    for (extra, expected) in [
        ("", vec![100, 30, 200, 10, 20, 30]),
        (
            "X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 100\r\n",
            vec![57, 0, 255, 0, 0, 0],
        ),
        (
            "X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 100\r\nX-Ocelli-Invert: 1\r\n",
            vec![198, 255, 0, 255, 255, 255],
        ),
    ] {
        let response = exchange(addr, &preview_id(id, extra))?;
        let (head, body) = split_http(&response)?;
        assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
        assert_eq!(header_line(head, "X-Ocelli-Channels"), Some("3"));
        assert_eq!(header_line(head, "X-Ocelli-Palette-Fallback"), Some("0"));
        assert_eq!(body, expected);
    }
    for bad in [
        "X-Ocelli-Window-Function: SIGMOID\r\n",
        "X-Ocelli-Window-Preset: lung\r\n",
        "X-Ocelli-Window-Index: 0\r\n",
    ] {
        let response = exchange(addr, &preview_id(id, bad))?;
        let (head, _) = split_http(&response)?;
        assert!(head.starts_with("HTTP/1.1 400 "));
        assert_eq!(
            header_line(head, "X-Ocelli-Error"),
            Some("colour-not-applicable")
        );
    }
    let response = exchange(addr, &preview_id(id, ""))?;
    let (head, body) = split_http(&response)?;
    assert_eq!(header_line(head, "X-Ocelli-Source"), Some("held"));
    assert_eq!(body, [100, 30, 200, 10, 20, 30]);
    let mono = one_exchange(&http_post(&mono_frame(true)?))?;
    let (head, _) = split_http(&mono)?;
    assert_eq!(header_line(head, "X-Ocelli-Channels"), Some("1"));
    Ok(())
}

#[test]
fn colour_server_native_packed_422_and_multi_band_planar_are_exact() -> Result<(), String> {
    // C.7.6.3.1.2: Y1 Y2 Cb Cr. The inverse matrix is worked in F239 AC5.
    let file = colour_server_frame(1, 2, "YBR_FULL_422", 0, &[50, 200, 100, 160])?;
    assert_eq!(
        preview_first_frame(&file)
            .map_err(|error| error.to_string())?
            .grey,
        [95, 37, 0, 245, 187, 150]
    );
    // More than one conversion band. Each plane has its own constant byte,
    // so using full-frame plane offsets inside a band shifts a channel.
    let (rows, columns) = (257_u16, 256_u16);
    let count = usize::from(rows) * usize::from(columns);
    let planar = [vec![10; count], vec![20; count], vec![30; count]].concat();
    let file = colour_server_frame(rows, columns, "RGB", 1, &planar)?;
    let preview = preview_first_frame(&file).map_err(|error| error.to_string())?;
    assert_eq!(preview.grey.len(), count * 3);
    assert!(
        preview
            .grey
            .chunks_exact(3)
            .all(|pixel| pixel == [10, 20, 30])
    );
    Ok(())
}

#[test]
fn colour_server_value_and_roi_are_pre_window_per_channel() -> Result<(), String> {
    // F239 AC13: population SD of [10,200] is95, [20,100] is40,
    // [30,50] is10. PS3.3 C.7.6.3 colours bypass modality rescale.
    let file = colour_server_frame(1, 2, "RGB", 0, &[10, 20, 30, 200, 100, 50])?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id = header_line(header, "X-Ocelli-File").ok_or("no held id")?;
    let extra = format!(
        "X-Ocelli-File: {id}\r\nX-Ocelli-Points: 1.5,0.5\r\nX-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 2\r\nX-Ocelli-Invert: 1\r\n"
    );
    let response = exchange(addr, &http_post_to("/value", &extra, &[]))?;
    let (head, body) = split_http(&response)?;
    assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
    let text = std::str::from_utf8(body).map_err(|error| error.to_string())?;
    assert!(text.contains("\"rgb\":[200,100,50]"), "{text}");
    assert!(text.contains("\"text\":\"R 200.0 G 100.0 B 50.0\""));
    assert!(!text.contains("\"value\":") && !text.contains("unit_source"));
    let mut previous = None;
    for window in [
        "",
        "X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 2\r\nX-Ocelli-Invert: 1\r\n",
    ] {
        let extra = format!("X-Ocelli-File: {id}\r\nX-Ocelli-Shape: rectangle\r\n{window}");
        let response = exchange(addr, &http_post_to("/measure", &extra, b"0,0 2,1"))?;
        let (head, body) = split_http(&response)?;
        assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
        let text = std::str::from_utf8(body).map_err(|error| error.to_string())?;
        for expected in [
            "\"padding_excluded\":0,\"unit\":\"RGB\"",
            "\"r\":{\"count\":2,\"mean\":105,\"sd\":95,\"min\":10,\"max\":200}",
            "\"g\":{\"count\":2,\"mean\":60,\"sd\":40,\"min\":20,\"max\":100}",
            "\"b\":{\"count\":2,\"mean\":40,\"sd\":10,\"min\":30,\"max\":50}",
        ] {
            assert!(text.contains(expected), "missing {expected} in {text}");
        }
        if let Some(before) = &previous {
            assert_eq!(text, before);
        } else {
            previous = Some(text.to_owned());
        }
    }
    Ok(())
}

/// ICC-tagged native RGB, with its device range in Bits Stored. Pixel values
/// and profile bytes are synthetic. The profile fixtures have separate provenance.
fn icc_server_frame(bits: u16, pixels: &[u16], profile: Option<&[u8]>) -> Result<Vec<u8>, String> {
    if !pixels.len().is_multiple_of(3) {
        return Err("RGB triples required".to_owned());
    }
    let columns = u16::try_from(pixels.len() / 3).map_err(|error| error.to_string())?;
    let allocated: u16 = if bits <= 8 { 8 } else { 16 };
    let mut data = short_le((0x0008, 0x0016), *b"UI", &ui(b"1.2.840.10008.5.1.4.1.1.7"))?;
    for (tag, value) in [
        ((0x0028, 0x0002), 3),
        ((0x0028, 0x0006), 0),
        ((0x0028, 0x0010), 1),
        ((0x0028, 0x0011), columns),
        ((0x0028, 0x0100), allocated),
        ((0x0028, 0x0101), bits),
        ((0x0028, 0x0102), bits - 1),
        ((0x0028, 0x0103), 0),
    ] {
        data.extend(short_le(tag, *b"US", &value.to_le_bytes())?);
    }
    data.extend(short_le((0x0028, 0x0004), *b"CS", b"RGB ")?);
    if let Some(profile) = profile {
        data.extend(long_le((0x0028, 0x2000), *b"OB", profile)?);
    }
    let mut value = Vec::new();
    for pixel in pixels {
        if allocated == 8 {
            value.push(u8::try_from(*pixel).map_err(|error| error.to_string())?);
        } else {
            value.extend(pixel.to_le_bytes());
        }
    }
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    data.extend(long_le(
        (0x7fe0, 0x0010),
        if allocated == 8 { *b"OB" } else { *b"OW" },
        &value,
    )?);
    part10(data)
}

#[test]
fn matrix_icc_profiles_draw_hand_values_and_untagged_frames_keep_device_rgb() -> Result<(), String>
{
    // F240 AC1, AC2, AC3, AC5 and AC8. Expected bytes are from the plan,
    // independently checked with Little CMS, not captured from this renderer.
    let neutral = [64, 64, 64, 128, 128, 128, 192, 192, 192, 255, 255, 255];
    for (profile, input, expected) in [
        (
            include_bytes!("fixtures/icc_gamma1.icc").as_slice(),
            neutral.as_slice(),
            [137, 137, 137, 188, 188, 188, 225, 225, 225, 255, 255, 255].as_slice(),
        ),
        (
            include_bytes!("fixtures/icc_gamma22.icc").as_slice(),
            neutral.as_slice(),
            [62, 62, 62, 129, 129, 129, 193, 193, 193, 255, 255, 255].as_slice(),
        ),
        (
            include_bytes!("fixtures/icc_srgb_para.icc").as_slice(),
            neutral.as_slice(),
            [64, 64, 64, 128, 128, 128, 192, 192, 192, 255, 255, 255].as_slice(),
        ),
        (
            include_bytes!("fixtures/icc_swap.icc").as_slice(),
            [255, 0, 0, 0, 255, 0].as_slice(),
            [0, 255, 0, 255, 0, 0].as_slice(),
        ),
    ] {
        let file = icc_server_frame(8, input, Some(profile))?;
        assert_eq!(
            preview_first_frame(&file)
                .map_err(|error| error.to_string())?
                .grey,
            expected
        );
        let info = inspect_json(&file, 0).map_err(|error| error.to_string())?;
        assert!(
            info.contains("\"icc\":true,\"icc_applied\":true,\"icc_kind\":\"\""),
            "{info}"
        );
    }
    let high = icc_server_frame(
        12,
        &[4095, 4095, 4095, 2048, 2048, 2048, 0, 0, 0],
        Some(include_bytes!("fixtures/icc_gamma1.icc")),
    )?;
    assert_eq!(
        preview_first_frame(&high)
            .map_err(|error| error.to_string())?
            .grey,
        [255, 255, 255, 188, 188, 188, 0, 0, 0]
    );
    let plain = icc_server_frame(12, &[4095, 2048, 15], None)?;
    assert_eq!(
        preview_first_frame(&plain)
            .map_err(|error| error.to_string())?
            .grey,
        [255, 128, 0]
    );
    Ok(())
}

#[test]
fn unsupported_icc_profiles_keep_device_rgb_and_name_the_reason() -> Result<(), String> {
    for (profile, kind) in [
        (
            include_bytes!("fixtures/icc_lut.icc").as_slice(),
            "lookup-table",
        ),
        (b"TEST".as_slice(), "malformed"),
    ] {
        let file = icc_server_frame(8, &[10, 20, 30, 200, 100, 50], Some(profile))?;
        assert_eq!(
            preview_first_frame(&file)
                .map_err(|error| error.to_string())?
                .grey,
            [10, 20, 30, 200, 100, 50]
        );
        let info = inspect_json(&file, 0).map_err(|error| error.to_string())?;
        assert!(info.contains("\"icc_applied\":false"), "{info}");
        assert!(info.contains(&format!("\"icc_kind\":\"{kind}\"")), "{info}");
    }
    Ok(())
}

#[test]
fn a_held_icc_frame_applies_its_profile_without_encoded_bytes_or_reparsing() -> Result<(), String> {
    let file = icc_server_frame(
        8,
        &[64, 64, 64],
        Some(include_bytes!("fixtures/icc_gamma1.icc")),
    )?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id = header_line(header, "X-Ocelli-File").ok_or("no held id")?;
    for (index, extra, expected) in [
        (0, "", vec![137, 137, 137]),
        // After ICC the channel is above the W10 upper edge132, so255.
        // Windowing the raw64 first would instead produce0, proving order.
        (
            1,
            "X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 10\r\n",
            vec![255, 255, 255],
        ),
        (
            2,
            "X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 10\r\nX-Ocelli-Invert: 1\r\n",
            vec![0, 0, 0],
        ),
        (3, "", vec![137, 137, 137]),
    ] {
        let response = exchange(addr, &preview_id(id, extra))?;
        let (head, body) = split_http(&response)?;
        assert_eq!(
            header_line(head, "X-Ocelli-Source"),
            Some(if index == 0 { "decoded" } else { "held" })
        );
        assert_eq!(body, expected);
        if index == 0 {
            let response = exchange(addr, b"GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
            let (_, body) = split_http(&response)?;
            let census = std::str::from_utf8(body).map_err(|error| error.to_string())?;
            assert!(
                census.contains("\"files\":0") && census.contains("\"frames\":1"),
                "{census}"
            );
        }
    }
    let response = exchange(
        addr,
        &http_post_to(
            "/value",
            &format!("X-Ocelli-File: {id}\r\nX-Ocelli-Points: 0.5,0.5\r\n"),
            &[],
        ),
    )?;
    let (head, body) = split_http(&response)?;
    assert!(head.starts_with("HTTP/1.1 200 "), "{head}");
    assert!(
        std::str::from_utf8(body)
            .map_err(|error| error.to_string())?
            .contains("R 137.2 G 137.2 B 137.2")
    );
    Ok(())
}

/// A data set of `depth` nested Sequences, one Item per Sequence, with one
/// US element inside the innermost Item. PS3.5 section 7.5 allows the
/// nesting. At 20,000 a parse without the preflight would recurse past the
/// connection thread's stack and, under panic = "abort", end the process.
fn nested_sequence_data_set(depth: usize) -> Result<Vec<u8>, String> {
    let mut current = short_le((0x0028, 0x0010), *b"US", &2_u16.to_le_bytes())?;
    for _ in 0..depth {
        let item = item(&current)?;
        current = long_le((0x0008, 0x1110), *b"SQ", &item)?;
    }
    Ok(current)
}

#[test]
fn a_sequence_deep_enough_to_abort_an_unchecked_parse_is_refused_and_the_process_answers_next()
-> Result<(), String> {
    // F-234. The data set nests 20,000 Sequences deep, far past the depth at
    // which dicom-rs's recursive collector overflows a connection thread.
    // Without the preflight the process would die inside the parse, and this
    // test with it. With the preflight the refusal happens before any
    // recursion, `part10_error` maps it to `not-part10` on the wire, and the
    // next request on the process is served.
    let data_set = nested_sequence_data_set(20_000)?;
    let file = part10(data_set)?;
    let request = http_post_to("/preview", "", &file);
    let response = one_exchange(&request)?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: not-part10") {
        return Err("the deep file was not refused by the preview".to_owned());
    }
    let next = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (next_header, _) = split_http(&next)?;
    if !next_header.contains("200") {
        return Err("the process did not answer the next request".to_owned());
    }
    Ok(())
}

// F-227, the automatic window, viewer half. Every expected value below is
// computed by hand from the nearest-rank rule of the design, PS3.3 C.11.1
// per frame and PS3.3 C.11.2.1.3.2 LINEAR_EXACT, with grey `floor(y + 0.5)`.
// For `n` counted values the ranks are `ceil(n / 100)` and `ceil(99 n / 100)`,
// one-based, and the window runs from the low to the high percentile. The
// library moves each percentile at most 16 f32 units toward the other before
// it checks the edges, which moves no grey below by a whole step.

/// One monochrome 16-bit Part 10 file for the automatic window. No window
/// unless `window` names one, Modality `modality`, Rescale Slope `slope` and
/// intercept 0, and a Pixel Padding Value when `padding` names one.
struct AutoSpec<'a> {
    rows: u16,
    columns: u16,
    samples: &'a [i32],
    signed: bool,
    slope: &'a str,
    modality: &'a str,
    padding: Option<i16>,
    window: Option<(&'a str, &'a str)>,
}

fn auto_file(spec: &AutoSpec<'_>) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.128"),
    )?);
    data.extend(short_le(
        (0x0008, 0x0060),
        *b"CS",
        &text_pad(spec.modality),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le(
        (0x0028, 0x0010),
        *b"US",
        &spec.rows.to_le_bytes(),
    )?);
    data.extend(short_le(
        (0x0028, 0x0011),
        *b"US",
        &spec.columns.to_le_bytes(),
    )?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    let representation = u16::from(spec.signed);
    data.extend(short_le(
        (0x0028, 0x0103),
        *b"US",
        &representation.to_le_bytes(),
    )?);
    if let Some(padding) = spec.padding {
        data.extend(short_le((0x0028, 0x0120), *b"SS", &padding.to_le_bytes())?);
    }
    if let Some((center, width)) = spec.window {
        data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad(center))?);
        data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad(width))?);
    }
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad(spec.slope))?);
    let mut pixels = Vec::with_capacity(spec.samples.len() * 2);
    for sample in spec.samples {
        if spec.signed {
            let value = i16::try_from(*sample).map_err(|_| "signed sample".to_owned())?;
            pixels.extend_from_slice(&value.to_le_bytes());
        } else {
            let value = u16::try_from(*sample).map_err(|_| "unsigned sample".to_owned())?;
            pixels.extend_from_slice(&value.to_le_bytes());
        }
    }
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    part10(data)
}

/// Ten by ten, unsigned, PT, no window: stored `first` to `first + 99`.
fn auto_ramp(first: i32, slope: &str) -> Result<Vec<u8>, String> {
    let samples: Vec<i32> = (first..first + 100).collect();
    auto_file(&AutoSpec {
        rows: 10,
        columns: 10,
        samples: &samples,
        signed: false,
        slope,
        modality: "PT",
        padding: None,
        window: None,
    })
}

/// Ten by ten, stored 1 to 100, with the window centre 100 and width 200.
fn auto_windowed() -> Result<Vec<u8>, String> {
    let samples: Vec<i32> = (1..=100).collect();
    auto_file(&AutoSpec {
        rows: 10,
        columns: 10,
        samples: &samples,
        signed: false,
        slope: "1",
        modality: "PT",
        padding: None,
        window: Some(("100", "200")),
    })
}

/// AC4's frame B alone: 20 of stored -2000, then stored 51 to 100, signed,
/// Pixel Padding Value -2000, seven rows of ten.
fn auto_padded() -> Result<Vec<u8>, String> {
    let mut samples = vec![-2000; 20];
    samples.extend(51..=100);
    auto_file(&AutoSpec {
        rows: 7,
        columns: 10,
        samples: &samples,
        signed: true,
        slope: "1",
        modality: "CT",
        padding: Some(-2000),
        window: None,
    })
}

fn post_for_id(addr: SocketAddr, file: &[u8]) -> Result<String, String> {
    let posted = exchange(addr, &http_post_to("/file", "", file))?;
    let (header, _) = split_http(&posted)?;
    header_line(header, "X-Ocelli-File")
        .map(str::to_owned)
        .ok_or_else(|| format!("the file id was missing: {header}"))
}

/// `POST /series-window` with one `<id> <frame>` line per image.
fn series_window(addr: SocketAddr, lines: &[(&str, u32)]) -> Result<(String, String), String> {
    let mut body = String::new();
    for (id, frame) in lines {
        body.push_str(&format!("{id} {frame}\n"));
    }
    let reply = exchange(addr, &http_post_to("/series-window", "", body.as_bytes()))?;
    let (header, body) = split_http(&reply)?;
    let body =
        std::str::from_utf8(body).map_err(|_| "the series window reply was not text".to_owned())?;
    Ok((header.to_owned(), body.to_owned()))
}

/// A JSON string field written as `"key":"value"`.
fn json_text(body: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{key}\":\"");
    let start = body
        .find(&needle)
        .ok_or_else(|| format!("no {key} in {body}"))?
        + needle.len();
    let rest = body.get(start..).ok_or_else(|| format!("short {key}"))?;
    let end = rest.find('"').ok_or_else(|| format!("open {key}"))?;
    Ok(rest.get(..end).unwrap_or_default().to_owned())
}

/// The grey bytes and header of one preview by id.
fn preview_by_id(addr: SocketAddr, id: &str, extra: &str) -> Result<(String, Vec<u8>), String> {
    let reply = exchange(addr, &preview_id(id, extra))?;
    let (header, body) = split_http(&reply)?;
    Ok((header.to_owned(), body.to_vec()))
}

fn library_window(
    frames: &[(i32, f32)],
    declared: usize,
) -> Result<ocelli_pixel::AutoWindow, String> {
    let mut pool = ocelli_pixel::AutoWindowPool::new(declared)
        .map_err(|error| format!("the pool was refused: {error:?}"))?;
    for (first, slope) in frames {
        let mut stored = Vec::with_capacity(100);
        for value in *first..*first + 100 {
            let value = u16::try_from(value).map_err(|_| "an unsigned sample".to_owned())?;
            stored.push(ocelli_core::Stored(f32::from(value)));
        }
        let modality = ocelli_pixel::ModalityTransform::new(None, Some(*slope), Some(0.0))
            .map_err(|_| "the rescale was refused".to_owned())?;
        pool.add_frame(&stored, &modality, None)
            .map_err(|error| format!("a frame was refused: {error:?}"))?;
    }
    pool.finish()
        .map_err(|error| format!("the pool did not finish: {error:?}"))
}

fn close_to(value: &str, want: f32) -> Result<(), String> {
    let got = value
        .parse::<f32>()
        .map_err(|_| format!("{value} is not a number"))?;
    if (got - want).abs() > 0.01 {
        return Err(format!("{value} is not {want}"));
    }
    Ok(())
}

/// AC10. The pinned one-sample inputs, through the route. Stored 50, no
/// window: one counted value, the constant window centre 50 width 1, and
/// `((50 - 50) / 1 + 0.5) * 255 = 127.5` addresses grey 128. A zero-length
/// Window Center and Width carry no number and are the same. A windowed
/// frame is `none`.
#[test]
fn the_route_labels_the_automatic_window_per_image() -> Result<(), String> {
    for (name, file) in [
        ("missing", mono_frame(false)?),
        ("zero length", mono_zero_window()?),
    ] {
        let response = one_exchange(&http_post(&file))?;
        let (header, body) = split_http(&response)?;
        if header_line(header, "X-Ocelli-Auto-Window") != Some("image") || body != [128] {
            return Err(format!(
                "the {name} window was not image grey 128: {header} {body:?}"
            ));
        }
    }
    let response = one_exchange(&http_post(&mono_frame(true)?))?;
    let (header, _) = split_http(&response)?;
    if header_line(header, "X-Ocelli-Auto-Window") != Some("none") {
        return Err(format!("a windowed frame was not labelled none: {header}"));
    }
    Ok(())
}

/// AC11. AC3's two frames as two PT files with their own Rescale Slope.
/// Frame A stored 1 to 100 at slope 1, frame B stored 1 to 100 at slope 2.
/// The pool of modality values holds 1 to 100 and 2 to 200 by 2, n = 200,
/// ranks 2 and 198, so low 2 and high 196, centre 99, width 194. Frame B's
/// stored 50 is modality 100, `((100 - 99) / 194 + 0.5) * 255 = 128.81`,
/// grey 129. Frame A's stored 50 is `((50 - 99) / 194 + 0.5) * 255 = 63.09`,
/// grey 63. Pooling stored values, or one slope for both, gives centre 50
/// width 98 and grey 128 for B. B alone gives centre 100 width 196, grey 128.
#[test]
fn the_series_window_pools_each_file_rescale() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let a = post_for_id(addr, &auto_ramp(1, "1")?)?;
    let b = post_for_id(addr, &auto_ramp(1, "2")?)?;
    let (header, body) = series_window(addr, &[(&a, 0), (&b, 0)])?;
    if status_token(&header)? != "200" {
        return Err(format!("the series window was refused: {header} {body}"));
    }
    let center = json_text(&body, "center")?;
    let width = json_text(&body, "width")?;
    close_to(&center, 99.0)?;
    close_to(&width, 194.0)?;
    let library = library_window(&[(1, 1.0), (1, 2.0)], 2)?;
    if center != library.center.to_string() || width != library.width.to_string() {
        return Err(format!(
            "the route said {center} and {width}, the library {} and {}",
            library.center, library.width
        ));
    }
    if json_text(&body, "function")? != "LINEAR_EXACT"
        || json_text(&body, "basis")? != "percentile"
        || json_number(&body, "sampled")? != 2
        || json_number(&body, "skipped")? != 0
        || json_number(&body, "counted")? != 200
    {
        return Err(format!("the series window reply was {body}"));
    }
    let extra = format!("X-Ocelli-Series-Window: {center} {width}\r\n");
    let (b_header, b_grey) = preview_by_id(addr, &b, &extra)?;
    if header_line(&b_header, "X-Ocelli-Auto-Window") != Some("series")
        || b_grey.get(49) != Some(&129)
    {
        return Err(format!(
            "frame B's stored 50 was {:?}, wanted grey 129 under the series window: {b_header}",
            b_grey.get(49)
        ));
    }
    let (_, a_grey) = preview_by_id(addr, &a, &extra)?;
    if a_grey.get(49) != Some(&63) {
        return Err(format!(
            "frame A's stored 50 was {:?}, wanted 63",
            a_grey.get(49)
        ));
    }
    // With no series header the image is drawn at its own window.
    let (own_header, own_grey) = preview_by_id(addr, &b, "")?;
    if header_line(&own_header, "X-Ocelli-Auto-Window") != Some("image")
        || own_grey.get(49) != Some(&128)
    {
        return Err(format!(
            "frame B alone was {:?}, wanted grey 128 labelled image: {own_header}",
            own_grey.get(49)
        ));
    }
    Ok(())
}

/// AC12. A window in the file is used as it is. Stored 50 under the file's
/// LINEAR_EXACT centre 100 width 200 is `((50 - 100) / 200 + 0.5) * 255 =
/// 63.75`, grey 64, with or without a series header, and the reply is `none`.
#[test]
fn a_windowed_image_ignores_the_series_window() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let id = post_for_id(addr, &mono_frame(true)?)?;
    let (header, grey) = preview_by_id(addr, &id, "X-Ocelli-Series-Window: 50 1\r\n")?;
    if header_line(&header, "X-Ocelli-Auto-Window") != Some("none") || grey != [64] {
        return Err(format!("the file window was replaced: {header} {grey:?}"));
    }
    Ok(())
}

/// AC12. A half window and a VOI LUT Sequence keep the window refusal, with
/// and without a series header.
#[test]
fn a_half_window_is_not_auto() -> Result<(), String> {
    let mut half = Vec::new();
    half.extend(image_module()?);
    half.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    half.extend(rescale_one()?);
    half.extend(pixels_50()?);
    let half = part10(half)?;
    let lut = mono_with(false, &[(0x0028, 0x3010)])?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    for (name, file) in [("half window", half), ("VOI LUT Sequence", lut)] {
        let id = post_for_id(addr, &file)?;
        for extra in ["", "X-Ocelli-Series-Window: 50 1\r\n"] {
            let (header, _) = preview_by_id(addr, &id, extra)?;
            if status_token(&header)? != "400"
                || header_line(&header, "X-Ocelli-Error") != Some("window")
            {
                return Err(format!("the {name} was drawn with {extra:?}: {header}"));
            }
        }
    }
    Ok(())
}

/// AC13. A list of three files whose middle one has a window. Sampled 3,
/// skipped 1. The other two are AC2's frames, stored 1 to 100 and 101 to
/// 200 at slope 1: n = 200, ranks 2 and 198, low 2, high 198, centre 100,
/// width 196. The third file's stored 150 is `((150 - 100) / 196 + 0.5) *
/// 255 = 192.55`, grey 193. Replacing the skipped file with a neighbour, or
/// the third file alone (centre 150, width 98, grey 128), differ. A list of
/// windowed files only is `400 window`.
#[test]
fn a_mixed_series_skips_windowed_images() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let a = post_for_id(addr, &auto_ramp(1, "1")?)?;
    let m = post_for_id(addr, &auto_windowed()?)?;
    let b = post_for_id(addr, &auto_ramp(101, "1")?)?;
    let (header, body) = series_window(addr, &[(&a, 0), (&m, 0), (&b, 0)])?;
    if status_token(&header)? != "200" {
        return Err(format!("the mixed series was refused: {header} {body}"));
    }
    if json_number(&body, "sampled")? != 3 || json_number(&body, "skipped")? != 1 {
        return Err(format!("the mixed series counted {body}"));
    }
    let center = json_text(&body, "center")?;
    let width = json_text(&body, "width")?;
    close_to(&center, 100.0)?;
    close_to(&width, 196.0)?;
    let library = library_window(&[(1, 1.0), (101, 1.0)], 3)?;
    if center != library.center.to_string() || width != library.width.to_string() {
        return Err(format!("the mixed series said {center} {width}"));
    }
    let extra = format!("X-Ocelli-Series-Window: {center} {width}\r\n");
    let (_, grey) = preview_by_id(addr, &b, &extra)?;
    if grey.get(49) != Some(&193) {
        return Err(format!("stored 150 was {:?}, wanted 193", grey.get(49)));
    }
    let (header, body) = series_window(addr, &[(&m, 0), (&m, 0)])?;
    if status_token(&header)? != "400" || header_line(&header, "X-Ocelli-Error") != Some("window") {
        return Err(format!(
            "a windowed series was given a series window: {header} {body}"
        ));
    }
    // Review pass 16, D2. A `-` line is an image the page cannot offer, a
    // file changed on disk. Sampled, it is skipped and the rest still pool:
    // file A alone, stored 1 to 100, n = 100, ranks 1 and 99, centre 50,
    // width 98, sampled 2, skipped 1.
    let (header, body) = series_window(addr, &[(&a, 0), ("-", 0)])?;
    if status_token(&header)? != "200"
        || json_number(&body, "sampled")? != 2
        || json_number(&body, "skipped")? != 1
    {
        return Err(format!("a changed image was not skipped: {header} {body}"));
    }
    close_to(&json_text(&body, "center")?, 50.0)?;
    close_to(&json_text(&body, "width")?, 98.0)?;
    Ok(())
}

/// AC13. AC4's frame B alone, signed, Pixel Padding Value -2000, PS3.3
/// C.7.5.1.1.2. The 20 padded samples are left out, so 51 to 100 are
/// counted, n = 50, ranks 1 and 50, low 51, high 100, centre 75.5, width 49.
/// Stored 75 is `((75 - 75.5) / 49 + 0.5) * 255 = 124.90`, grey 125, stored
/// 51 and -2000 are grey 0 and stored 100 is 255. Ignoring the padding
/// counts -2000: n = 70, ranks 1 and 70, centre -950, width 2100, and
/// stored 75 is `((75 + 950) / 2100 + 0.5) * 255 = 251.96`, grey 252.
#[test]
fn a_padded_file_leaves_out_its_padding() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let id = post_for_id(addr, &auto_padded()?)?;
    let (header, grey) = preview_by_id(addr, &id, "")?;
    if header_line(&header, "X-Ocelli-Auto-Window") != Some("image") {
        return Err(format!("the padded file was not labelled image: {header}"));
    }
    let want = [(0, 0), (20, 0), (44, 125), (69, 255)];
    for (at, value) in want {
        if grey.get(at) != Some(&value) {
            return Err(format!(
                "sample {at} was {:?}, wanted {value}",
                grey.get(at)
            ));
        }
    }
    let (header, body) = series_window(addr, &[(&id, 0)])?;
    if status_token(&header)? != "200" || json_number(&body, "counted")? != 50 {
        return Err(format!("the padded series was {header} {body}"));
    }
    close_to(&json_text(&body, "center")?, 75.5)?;
    close_to(&json_text(&body, "width")?, 49.0)?;
    Ok(())
}

/// AC14. A series header names one window, so it is refused beside a
/// preset, a numeric window or a file window index, and when its width is
/// not a positive finite number. The list refuses a gone id with its id and
/// a malformed line as a header error.
#[test]
fn the_series_window_refuses_conflicts() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let id = post_for_id(addr, &auto_ramp(1, "1")?)?;
    let series = "X-Ocelli-Series-Window: 50 98\r\n";
    let conflicts = [
        format!("{series}X-Ocelli-Window-Preset: soft_tissue\r\n"),
        format!("{series}X-Ocelli-Window-Center: 40\r\nX-Ocelli-Window-Width: 400\r\n"),
        format!("{series}X-Ocelli-Window-Index: 0\r\n"),
        "X-Ocelli-Series-Window: 50 0\r\n".to_owned(),
        "X-Ocelli-Series-Window: 50 -1\r\n".to_owned(),
        "X-Ocelli-Series-Window: 50 inf\r\n".to_owned(),
        "X-Ocelli-Series-Window: NaN 98\r\n".to_owned(),
        "X-Ocelli-Series-Window: 50\r\n".to_owned(),
        // Review pass 16, N2. A function beside the series window would draw
        // its centre and width under another function and still say series.
        format!("{series}X-Ocelli-Window-Function: LINEAR\r\n"),
    ];
    for extra in &conflicts {
        let (header, _) = preview_by_id(addr, &id, extra)?;
        if status_token(&header)? != "400"
            || header_line(&header, "X-Ocelli-Error") != Some("window")
        {
            return Err(format!("{extra:?} was not the window refusal: {header}"));
        }
    }
    // Review pass 16, D1. Every gone sampled id is named, in sample order,
    // so the page posts them all in one round. Naming the first alone sent
    // the page round once per gone file and lost the series window at two.
    let gone = other_generation(&id)?;
    let also = other_generation(&post_for_id(addr, &auto_ramp(101, "1")?)?)?;
    let (header, body) = series_window(addr, &[(&gone, 0), (&id, 0), (&also, 0)])?;
    if status_token(&header)? != "410"
        || header_line(&header, "X-Ocelli-Error") != Some("file-gone")
        || body != format!("{{\"gone\":[\"{gone}\",\"{also}\"]}}")
    {
        return Err(format!("the gone ids were not all named: {header} {body}"));
    }
    for body in [&b"not an id\n"[..], b"", b"\n"] {
        let reply = exchange(addr, &http_post_to("/series-window", "", body))?;
        let (header, _) = split_http(&reply)?;
        if status_token(header)? != "400" || header_line(header, "X-Ocelli-Error") != Some("header")
        {
            return Err(format!(
                "the body {body:?} was not the header refusal: {header}"
            ));
        }
    }
    Ok(())
}

/// AC15, the page source. The series window's two strings travel from the
/// `/series-window` reply to `X-Ocelli-Series-Window` untouched, the first
/// image of a series waits for them, a thumbnail never sends them, a linked
/// Reset sends no window of the source's series, and the words the corner
/// adds are the reply's. Nothing named full range is left.
#[test]
fn the_page_draws_a_series_at_its_automatic_window() -> Result<(), String> {
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (_, body) = split_http(&response)?;
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    let lower = page.to_ascii_lowercase();
    if lower.contains("full range") || lower.contains("full-range") || page.contains("fullRange") {
        return Err("the page still names the full range".to_owned());
    }
    // The strings are kept and sent as they arrived.
    let ask = block_from(page, "async function askSeriesWindow(")?;
    if !ask.contains("fetch(\"/series-window\", { method: \"POST\", body: seriesBody(lines) })")
        || !ask.contains("return { center: reply.center, width: reply.width,")
    {
        return Err("the series window is not asked for and kept as text".to_owned());
    }
    // Review pass 16, D1 and D2. Nothing is posted to ask: each line carries
    // the id the page holds, a changed file is `-`, and only the gone ids the
    // reply names are posted again, while a round names one not yet posted.
    if ask.contains("ensureFile(")
        || !ask.contains("lines.push({ image, id: await heldId(image.file) });")
        || !ask.contains(
            "if (!gone.size || Array.from(gone).every((id) => posted.has(id))) return null;",
        )
        || !function_body(page, "seriesBody")?.contains("(line.id || \"-\")")
        || !block_from(page, "async function heldId(")?
            .contains("if (changedFiles.has(file)) return \"\";")
    {
        return Err("the series window posts files to ask, or stops at one gone file".to_owned());
    }
    let header = function_body(page, "seriesHeader")?;
    if !header.contains("return entry ? entry.center + \" \" + entry.width : \"\";")
        || !header.contains("view === PLAIN_VIEW")
    {
        return Err(
            "the series header is not the reply's text, or a thumbnail sends it".to_owned(),
        );
    }
    for arithmetic in [
        "Number(reply.center",
        "Number(reply.width",
        "Number(entry.",
        "parseFloat(",
    ] {
        if page.contains(arithmetic) {
            return Err(format!(
                "the page reads the series window as a number: {arithmetic}"
            ));
        }
    }
    if !page.contains("if (series) headers[\"X-Ocelli-Series-Window\"] = series;")
        || !function_body(page, "cacheKey")?.contains("seriesHeader(image, view)")
    {
        return Err("the series window is not in the request and its cache key".to_owned());
    }
    // The first image waits for the series window and takes no cached grey
    // of its own window first.
    let pump = block_from(page, "async function pumpMain(")?;
    let wait = pump
        .find("await ensureSeriesWindow(seriesOf(vp));")
        .ok_or_else(|| "the main view does not wait for the series window".to_owned())?;
    let fetch = pump
        .find("const grey = await fetchGrey(image, vp);")
        .ok_or_else(|| "the main view does not fetch".to_owned())?;
    if wait > fetch {
        return Err("the main view draws before the series window".to_owned());
    }
    if !page.contains(
        "const hit = seriesWindowWanted(vp, image) ? null : takeGrey(cacheKey(image, vp));",
    ) {
        return Err("a cached grey is drawn before the series window".to_owned());
    }
    // One request per series per open, cleared by a new open.
    let drop = function_body(page, "dropCache")?;
    if !drop.contains("seriesWindows.clear();") || !drop.contains("seriesWindowRefused.clear();") {
        return Err("a new open keeps the previous series windows".to_owned());
    }
    if !function_body(page, "ensureSeriesWindow")?.contains("if (pending) return pending;") {
        return Err("two views of one series ask twice".to_owned());
    }
    // F-209. A linked Reset copies the file window state, not a window, so
    // each target sends its own series' entry.
    let copy = function_body(page, "copyWindow")?;
    if copy.contains("seriesWindows")
        || !function_body(page, "resetView")?.contains("copyWindow(vp);")
    {
        return Err("a linked Reset carries the source's series window".to_owned());
    }
    // F-213. One Auto window row, the file window state.
    if !function_body(page, "chooseRow")?
        .contains("else if (row.kind === \"auto\") chooseWindow(vp, { index: 0 });")
        || !page.contains("rows.push({ kind: \"auto\", label: \"Auto window\",")
    {
        return Err("the window list has no Auto window row".to_owned());
    }
    let words = function_body(page, "autoWords")?;
    if !words.contains("if (auto === \"series\") return \"  Auto window\";")
        || !words.contains("if (auto === \"image\") return \"  Auto window, this image\";")
        || !page.contains("auto: response.headers.get(\"X-Ocelli-Auto-Window\") || \"none\",")
    {
        return Err("the corner words do not come from the reply".to_owned());
    }
    Ok(())
}
