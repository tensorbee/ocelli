//! Offscreen canvas attachment for one render worker.
//!
//! The browser type is named here and nowhere else. `ocelli-render` creates
//! the surface from the instance that produced the device. Decode workers
//! load this module and do not have to call either export. The session type
//! is `crate::session`. One context serves every viewport in the
//! table.

use std::cell::RefCell;

use js_sys::Reflect;
use js_sys::Uint8ClampedArray;
use ocelli_render::{
    BlendMode, GpuContext, Pass, RasterResult, RenderGraph, ResolvedAdapter, StackPass,
    TierRequest, ViewportId, VoiParams, device_extent, resolve_adapter,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::ImageData;
use web_sys::OffscreenCanvas;
use web_sys::OffscreenCanvasRenderingContext2d;

use crate::session::bind::with_session;

/// Viewports one worker may attach. One context, not a pool.
const CAPACITY: usize = 8;

struct ViewportRow {
    id: ViewportId,
}

struct Table {
    opening: bool,
    unavailable: bool,
    adapter: Option<ResolvedAdapter>,
    context: Option<GpuContext>,
    graph: Option<RenderGraph>,
    rows: Vec<Option<ViewportRow>>,
}

impl Table {
    fn new() -> Self {
        let mut rows = Vec::new();
        rows.resize_with(CAPACITY, || None);
        Self {
            opening: false,
            unavailable: false,
            adapter: None,
            context: None,
            graph: None,
            rows,
        }
    }
}

thread_local! {
    static TABLE: RefCell<Table> = RefCell::new(Table::new());
}

enum OpenPhase {
    Ready,
    Unavailable,
    Open,
}

/// Attach `canvas` to `viewport` and size its bitmap from the CSS box.
///
/// The bitmap is set from [`device_extent`] before any device work, so a
/// later refusal still leaves the dimensions the caller can read. Tier C
/// sets that bitmap and does not attach a surface.
#[wasm_bindgen(js_name = attachOffscreenCanvas)]
pub async fn attach_offscreen_canvas(
    viewport: u32,
    canvas: OffscreenCanvas,
    css_width: f64,
    css_height: f64,
    dpr: f64,
) -> Result<(), String> {
    let extent = sized(&canvas, css_width, css_height, dpr)?;
    let index = row_index(viewport)?;
    ensure_device().await?;
    TABLE.with(|table| {
        let Ok(mut table) = table.try_borrow_mut() else {
            return Err("viewport table is busy".to_owned());
        };
        attach_bound(&mut table, index, canvas, extent)
    })
}

/// Resize a viewport that was attached, using the same four numbers.
///
/// The bitmap is updated even when no surface is bound. A bound surface is
/// reconfigured. Stored-sample uploads are not counted again.
#[wasm_bindgen(js_name = resizeOffscreenCanvas)]
pub fn resize_offscreen_canvas(
    viewport: u32,
    canvas: OffscreenCanvas,
    css_width: f64,
    css_height: f64,
    dpr: f64,
) -> Result<(), String> {
    let extent = sized(&canvas, css_width, css_height, dpr)?;
    let index = row_index(viewport)?;
    TABLE.with(|table| {
        let Ok(mut table) = table.try_borrow_mut() else {
            return Err("viewport table is busy".to_owned());
        };
        resize_bound(&mut table, index, extent)
    })
}

fn resize_bound(table: &mut Table, index: usize, extent: (u32, u32)) -> Result<(), String> {
    if table.unavailable || table.context.is_none() {
        return Err("tier C opens no device, so no surface was attached".to_owned());
    }
    let id = match table.rows.get(index).and_then(Option::as_ref) {
        Some(row) => row.id,
        None => return Err("this viewport has no surface".to_owned()),
    };
    let Table {
        context,
        adapter,
        graph,
        ..
    } = table;
    let (Some(context), Some(adapter), Some(graph)) =
        (context.as_ref(), adapter.as_ref(), graph.as_mut())
    else {
        return Err("tier C opens no device, so no surface was attached".to_owned());
    };
    graph
        .reconfigure_surface(context, adapter, id, extent)
        .map_err(|error| error.to_string())
}

fn sized(
    canvas: &OffscreenCanvas,
    css_width: f64,
    css_height: f64,
    dpr: f64,
) -> Result<(u32, u32), String> {
    let extent = device_extent(css_width, css_height, dpr).map_err(|error| error.to_string())?;
    canvas.set_width(extent.0);
    canvas.set_height(extent.1);
    Ok(extent)
}

fn row_index(viewport: u32) -> Result<usize, String> {
    let index = usize::try_from(viewport).unwrap_or(usize::MAX);
    if index >= CAPACITY {
        Err("viewport index is outside the table".to_owned())
    } else {
        Ok(index)
    }
}

async fn ensure_device() -> Result<(), String> {
    let phase = TABLE.with(|table| {
        let Ok(mut table) = table.try_borrow_mut() else {
            return Err("viewport table is busy".to_owned());
        };
        if table.context.is_some() {
            return Ok(OpenPhase::Ready);
        }
        if table.unavailable {
            return Ok(OpenPhase::Unavailable);
        }
        if table.opening {
            return Err("a device open is already in progress".to_owned());
        }
        table.opening = true;
        Ok(OpenPhase::Open)
    })?;
    match phase {
        OpenPhase::Ready => Ok(()),
        OpenPhase::Unavailable => {
            Err("tier C opens no device, so no surface was attached".to_owned())
        }
        OpenPhase::Open => {
            let opened = open_session().await;
            TABLE.with(|table| {
                let Ok(mut table) = table.try_borrow_mut() else {
                    return Err("viewport table is busy".to_owned());
                };
                table.opening = false;
                match opened {
                    Ok(Some((adapter, context))) => {
                        table.adapter = Some(adapter);
                        table.context = Some(context);
                        Ok(())
                    }
                    Ok(None) => {
                        table.unavailable = true;
                        Err("tier C opens no device, so no surface was attached".to_owned())
                    }
                    Err(message) => Err(message),
                }
            })
        }
    }
}

async fn open_session() -> Result<Option<(ResolvedAdapter, GpuContext)>, String> {
    let mut clock = browser_clock();
    let Some(adapter) = resolve_adapter(TierRequest::Auto, &mut clock).await else {
        return Ok(None);
    };
    let context = adapter.open().await.map_err(|error| error.to_string())?;
    Ok(Some((adapter, context)))
}

fn attach_bound(
    table: &mut Table,
    index: usize,
    canvas: OffscreenCanvas,
    extent: (u32, u32),
) -> Result<(), String> {
    if table.unavailable {
        return Err("tier C opens no device, so no surface was attached".to_owned());
    }
    let mut context = table
        .context
        .take()
        .ok_or_else(|| "tier C opens no device, so no surface was attached".to_owned())?;
    let outcome = attach_with_context(&mut context, table, index, canvas, extent);
    table.context = Some(context);
    outcome
}

fn attach_with_context(
    context: &mut GpuContext,
    table: &mut Table,
    index: usize,
    canvas: OffscreenCanvas,
    extent: (u32, u32),
) -> Result<(), String> {
    let surface = context
        .attach_target(wgpu::SurfaceTarget::OffscreenCanvas(canvas))
        .map_err(|error| error.to_string())?;
    let format = {
        let Some(adapter) = table.adapter.as_ref() else {
            return Err("tier C opens no device, so no surface was attached".to_owned());
        };
        context
            .configure_viewport(&surface, adapter, extent)
            .map_err(|error| error.to_string())?
    };
    if table.graph.is_none() {
        let graph =
            RenderGraph::new(context, CAPACITY, format).map_err(|error| error.to_string())?;
        table.graph = Some(graph);
    } else if table
        .graph
        .as_ref()
        .is_some_and(|graph| graph.color_format() != format)
    {
        return Err("the surface format is not the format the graph was compiled for".to_owned());
    }
    let existing = table
        .rows
        .get(index)
        .and_then(Option::as_ref)
        .map(|row| row.id);
    let id = if let Some(id) = existing {
        id
    } else {
        let (target, source) = placeholder_views(context.device(), format);
        let Some(graph) = table.graph.as_mut() else {
            return Err("the render graph was not created".to_owned());
        };
        match graph.register(context, target, source, stack_pass()) {
            Ok(id) => id,
            Err(error) => return Err(error.to_string()),
        }
    };
    let Some(graph) = table.graph.as_mut() else {
        return Err("the render graph was not created".to_owned());
    };
    if let Err(error) = graph.bind_surface(id, surface, extent) {
        if existing.is_none() {
            let _ = graph.remove(id);
        }
        return Err(error.to_string());
    }
    if let Some(slot) = table.rows.get_mut(index) {
        *slot = Some(ViewportRow { id });
    }
    Ok(())
}

fn placeholder_views(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> (wgpu::TextureView, wgpu::TextureView) {
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli canvas source"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli canvas placeholder"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    (
        target.create_view(&wgpu::TextureViewDescriptor::default()),
        source.create_view(&wgpu::TextureViewDescriptor::default()),
    )
}

fn stack_pass() -> Pass {
    Pass::Stack(StackPass {
        voi: VoiParams {
            center: 0.0,
            width: 1.0,
            slope: 1.0,
            intercept: 0.0,
            ymin: 0.0,
            ymax: 255.0,
            fn_kind: 0,
            invert: 0,
        },
        blend: BlendMode::Composite,
    })
}

fn browser_clock() -> impl FnMut() -> u64 {
    let origin = performance_now_millis();
    move || {
        let elapsed = performance_now_millis() - origin;
        saturating_u64(elapsed * 1_000_000.0)
    }
}

fn performance_now_millis() -> f64 {
    let global = js_sys::global();
    let Ok(performance) = js_sys::Reflect::get(&global, &JsValue::from_str("performance")) else {
        return 0.0;
    };
    let Ok(now) = js_sys::Reflect::get(&performance, &JsValue::from_str("now")) else {
        return 0.0;
    };
    let Ok(now) = now.dyn_into::<js_sys::Function>() else {
        return 0.0;
    };
    let Ok(value) = now.call0(&performance) else {
        return 0.0;
    };
    value.as_f64().unwrap_or(0.0)
}

/// Draw the four section 18.3 rows on a GL-only instance and return JSON.
///
/// The named user is `tools/tier-b/run.mjs`. The shipped module does not
/// carry `tier-b-probe`, so this export is absent from the measured artefact.
/// The readback is the test. It is not a product path.
#[cfg(feature = "tier-b-probe")]
#[wasm_bindgen(js_name = tierBSection183)]
pub async fn tier_b_section_183(canvas: OffscreenCanvas) -> String {
    use ocelli_render::{
        Section183Error, Tier, WebGlProbe, read_section_18_3, resolve_webgl, stored_format_name,
    };

    let mut clock = browser_clock();
    let probe = resolve_webgl(wgpu::SurfaceTarget::OffscreenCanvas(canvas), &mut clock).await;
    match probe {
        WebGlProbe::NoAdapter { reason } => {
            json_report("no-adapter", "", "", "", "", &reason, None)
        }
        WebGlProbe::Demoted { name, decided_by } => json_report(
            "demoted",
            "C",
            &name,
            "",
            decided_label(decided_by),
            "the WebGL adapter resolved tier C",
            None,
        ),
        WebGlProbe::Opened(opened) => {
            let tier = tier_label(opened.adapter.resolution().caps.tier);
            let renderer = match opened.adapter.resolution().evidence.candidate.as_ref() {
                Some(candidate) => candidate.name.clone(),
                None => String::new(),
            };
            let decided = decided_label(opened.adapter.resolution().evidence.decided_by);
            let mut context = match opened.adapter.open().await {
                Ok(context) => context,
                Err(error) => {
                    return json_report(
                        "unavailable",
                        tier,
                        &renderer,
                        "",
                        decided,
                        &error.to_string(),
                        None,
                    );
                }
            };
            if context.caps().tier == Tier::Cpu {
                return json_report(
                    "demoted",
                    "C",
                    &renderer,
                    "",
                    decided,
                    "open resolved tier C",
                    None,
                );
            }
            match read_section_18_3(&mut context, &opened.adapter) {
                Ok(pixels) => json_report(
                    "drew",
                    tier_label(context.caps().tier),
                    &renderer,
                    stored_format_name(pixels.format),
                    decided,
                    "",
                    Some(&pixels),
                ),
                Err(Section183Error::Unavailable) => json_report(
                    "unavailable",
                    tier_label(context.caps().tier),
                    &renderer,
                    "",
                    decided,
                    "no preserving format or the colour target cannot be read as f32",
                    None,
                ),
                Err(Section183Error::TierC) => json_report(
                    "demoted",
                    "C",
                    &renderer,
                    "",
                    decided,
                    "the graph is unavailable on tier C",
                    None,
                ),
                Err(Section183Error::NotSubmitted) => json_report(
                    "unavailable",
                    tier_label(context.caps().tier),
                    &renderer,
                    "",
                    decided,
                    "the frame was not submitted",
                    None,
                ),
                Err(Section183Error::Readback(reason)) => json_report(
                    "unavailable",
                    tier_label(context.caps().tier),
                    &renderer,
                    "",
                    decided,
                    &reason,
                    None,
                ),
            }
        }
    }
}

#[cfg(feature = "tier-b-probe")]
fn tier_label(tier: ocelli_render::Tier) -> &'static str {
    match tier {
        ocelli_render::Tier::A => "A",
        ocelli_render::Tier::B => "B",
        ocelli_render::Tier::Cpu => "C",
    }
}

#[cfg(feature = "tier-b-probe")]
fn decided_label(decided: ocelli_render::DecidedBy) -> &'static str {
    match decided {
        ocelli_render::DecidedBy::Override => "Override",
        ocelli_render::DecidedBy::NoAdapter => "NoAdapter",
        ocelli_render::DecidedBy::NoDevice => "NoDevice",
        ocelli_render::DecidedBy::Benchmark => "Benchmark",
        ocelli_render::DecidedBy::AdapterType => "AdapterType",
        ocelli_render::DecidedBy::RendererString => "RendererString",
        ocelli_render::DecidedBy::CandidateKept => "CandidateKept",
    }
}

#[cfg(feature = "tier-b-probe")]
fn json_report(
    status: &str,
    tier: &str,
    renderer: &str,
    format: &str,
    decided_by: &str,
    reason: &str,
    pixels: Option<&ocelli_render::Section183Pixels>,
) -> String {
    let (linear, exact, cpu_linear, cpu_exact) = match pixels {
        Some(pixels) => (
            json_row(&pixels.linear),
            json_row(&pixels.linear_exact),
            json_row(&pixels.cpu_linear),
            json_row(&pixels.cpu_linear_exact),
        ),
        None => (
            "[0,0,0,0]".to_owned(),
            "[0,0,0,0]".to_owned(),
            "[0,0,0,0]".to_owned(),
            "[0,0,0,0]".to_owned(),
        ),
    };
    format!(
        "{{\"status\":\"{}\",\"tier\":\"{}\",\"renderer\":\"{}\",\"format\":\"{}\",\"decidedBy\":\"{}\",\"reason\":\"{}\",\"linear\":{},\"exact\":{},\"cpuLinear\":{},\"cpuExact\":{}}}",
        json_escape(status),
        json_escape(tier),
        json_escape(renderer),
        json_escape(format),
        json_escape(decided_by),
        json_escape(reason),
        linear,
        exact,
        cpu_linear,
        cpu_exact,
    )
}

#[cfg(feature = "tier-b-probe")]
fn json_row(values: &[f32; 4]) -> String {
    let mut text = String::from("[");
    let mut first = true;
    for value in values {
        if !first {
            text.push(',');
        }
        first = false;
        text.push_str(&format!("{value:?}"));
    }
    text.push(']');
    text
}

#[cfg(feature = "tier-b-probe")]
fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other if other.is_control() => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// Saturating conversion for the browser clock.
///
/// `std::time::Instant` panics on this target, and an `as` cast is denied.
/// The probe only needs a few seconds of nanoseconds, which are exact in `f64`.
fn saturating_u64(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let Ok(biased) = u32::try_from(biased) else {
        return u64::MAX;
    };
    if biased < 1023 {
        return 0;
    }
    let unbiased = biased - 1023;
    if unbiased >= 64 {
        return u64::MAX;
    }
    let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    if unbiased >= 52 {
        mantissa << (unbiased - 52)
    } else {
        mantissa >> (52 - unbiased)
    }
}

struct Publication {
    context: OffscreenCanvasRenderingContext2d,
    image: ImageData,
    pixels: Uint8ClampedArray,
    extent: [u32; 2],
}

struct CpuSlot {
    canvas: Option<OffscreenCanvas>,
    publication: Option<Publication>,
}

thread_local! {
    static CPU_CANVAS: RefCell<CpuSlot> = const {
        RefCell::new(CpuSlot {
            canvas: None,
            publication: None,
        })
    };
}

struct StagedFrame {
    extent: [u32; 2],
    generation: u32,
}

/// Bind the session CPU raster to an owned canvas. The shell does not receive pixels.
#[wasm_bindgen(js_name = bindCpuCanvas)]
pub fn bind_cpu_canvas(
    canvas: OffscreenCanvas,
    css_width: f64,
    css_height: f64,
    dpr: f64,
) -> Result<(), String> {
    let extent = sized(&canvas, css_width, css_height, dpr)?;
    let bound = with_session(|session| session.bind_cpu(css_width, css_height, dpr));
    let device = match bound {
        Some(Ok(device)) => device,
        Some(Err(error)) => return Err(error.to_string()),
        None => return Err("session is busy".to_owned()),
    };
    if device != [extent.0, extent.1] {
        return Err("canvas bitmap and cpu extent disagree".to_owned());
    }
    store_canvas(canvas, device)
}

/// Resize the bound CPU canvas.
///
/// The session accepts or refuses the request before the bitmap changes. An
/// unchanged device extent does not assign canvas width or height, because
/// that assignment clears the displayed frame even when the numbers match.
/// A changed extent replaces the retained image after settlement is invalidated.
#[wasm_bindgen(js_name = resizeCpuCanvas)]
pub fn resize_cpu_canvas(css_width: f64, css_height: f64, dpr: f64) -> Result<(), String> {
    canvas_is_bound()?;
    let extent = device_extent(css_width, css_height, dpr).map_err(|error| error.to_string())?;
    let device = match with_session(|session| session.resize_cpu(css_width, css_height, dpr)) {
        Some(Ok(device)) => device,
        Some(Err(error)) => return Err(error.to_string()),
        None => return Err("session is busy".to_owned()),
    };
    if device != [extent.0, extent.1] {
        return Err("canvas bitmap and cpu extent disagree".to_owned());
    }
    if bitmap_matches(device)? {
        return Ok(());
    }
    let canvas = take_canvas()?;
    canvas.set_width(device[0]);
    canvas.set_height(device[1]);
    store_canvas(canvas, device)
}

/// Draw the committed monochrome frame into the retained image, blit it, then publish kind 6.
///
/// A draw that writes nothing does not blit and does not publish. Settlement
/// stays open until `putImageData` returns, so a failed blit can be retried.
/// There is no pixel getter.
#[wasm_bindgen(js_name = publishCpuCanvas)]
pub fn publish_cpu_canvas() -> Result<(), String> {
    publish_after(stage_committed, true)
}

/// Draw interleaved native RGB when `photometric` is 1.
///
/// Every other code returns unavailable and does not draw, blit or publish.
/// There is no pixel getter and no colour threshold.
#[wasm_bindgen(js_name = publishCpuColor)]
pub fn publish_cpu_color(photometric: u32) -> Result<(), String> {
    publish_after(
        move |session| {
            session
                .select_supported_color(photometric)
                .map_err(|error| error.to_string())?;
            stage_color(session)
        },
        true,
    )
}

/// Draw the remembered shade at the reduced extent and publish kind 6.
///
/// The event keeps the refining flag. This call does not accept the
/// presentation, so the generation stays unsettled until a full settle.
/// A failed blit publishes nothing.
#[wasm_bindgen(js_name = publishCpuInteraction)]
pub fn publish_cpu_interaction() -> Result<(), String> {
    publish_after(stage_interaction, false)
}

fn publish_after(
    stage: impl FnOnce(&mut crate::session::Session) -> Result<Option<StagedFrame>, String>,
    accept: bool,
) -> Result<(), String> {
    let staged = match with_session(stage) {
        Some(Ok(Some(staged))) => staged,
        Some(Ok(None)) => return Ok(()),
        Some(Err(error)) => return Err(error),
        None => return Err("session is busy".to_owned()),
    };
    blit_retained()?;
    match with_session(|session| {
        if accept {
            accept_staged(session, &staged)
        } else {
            publish_staged(session, &staged)
        }
    }) {
        Some(result) => result,
        None => Err("session is busy".to_owned()),
    }
}

fn store_canvas(canvas: OffscreenCanvas, extent: [u32; 2]) -> Result<(), String> {
    let publication = match retain_publication(&canvas, extent) {
        Ok(publication) => publication,
        Err(error) => {
            restore_canvas(canvas);
            return Err(error);
        }
    };
    CPU_CANVAS.with(|slot| {
        let Ok(mut slot) = slot.try_borrow_mut() else {
            restore_canvas(canvas);
            return Err("cpu canvas is busy".to_owned());
        };
        slot.canvas = Some(canvas);
        slot.publication = Some(publication);
        Ok(())
    })
}

fn canvas_is_bound() -> Result<(), String> {
    CPU_CANVAS.with(|slot| {
        let Ok(slot) = slot.try_borrow() else {
            return Err("cpu canvas is busy".to_owned());
        };
        if slot.canvas.is_none() {
            Err("no cpu canvas is bound".to_owned())
        } else {
            Ok(())
        }
    })
}

/// True when the displayed bitmap and the retained image already have `device`.
///
/// Skipping the width and height assignment is what keeps a repeated resize
/// from clearing a settled frame.
fn bitmap_matches(device: [u32; 2]) -> Result<bool, String> {
    CPU_CANVAS.with(|slot| {
        let Ok(slot) = slot.try_borrow() else {
            return Err("cpu canvas is busy".to_owned());
        };
        let Some(canvas) = slot.canvas.as_ref() else {
            return Err("no cpu canvas is bound".to_owned());
        };
        let Some(publication) = slot.publication.as_ref() else {
            return Ok(false);
        };
        Ok(publication.extent == device
            && canvas.width() == device[0]
            && canvas.height() == device[1])
    })
}

fn take_canvas() -> Result<OffscreenCanvas, String> {
    CPU_CANVAS.with(|slot| {
        let Ok(mut slot) = slot.try_borrow_mut() else {
            return Err("cpu canvas is busy".to_owned());
        };
        slot.canvas
            .take()
            .ok_or_else(|| "no cpu canvas is bound".to_owned())
    })
}

fn restore_canvas(canvas: OffscreenCanvas) {
    CPU_CANVAS.with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut() {
            slot.canvas = Some(canvas);
        }
    });
}

fn retain_publication(canvas: &OffscreenCanvas, extent: [u32; 2]) -> Result<Publication, String> {
    if extent[0] == 0 || extent[1] == 0 {
        return Err("cpu canvas extent is zero".to_owned());
    }
    let context_value = canvas
        .get_context("2d")
        .map_err(js_error)?
        .ok_or_else(|| "cpu canvas has no 2d context".to_owned())?;
    let context: OffscreenCanvasRenderingContext2d = context_value
        .dyn_into()
        .map_err(|_| "cpu canvas context is not an OffscreenCanvas 2d context".to_owned())?;
    let image = ImageData::new_with_sw(extent[0], extent[1]).map_err(js_error)?;
    let pixels = live_pixels(&image)?;
    let bytes = extent[0]
        .checked_mul(extent[1])
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "cpu canvas extent overflows".to_owned())?;
    if pixels.length() != bytes {
        return Err("retained image length disagrees with the device extent".to_owned());
    }
    Ok(Publication {
        context,
        image,
        pixels,
        extent,
    })
}

fn live_pixels(image: &ImageData) -> Result<Uint8ClampedArray, String> {
    let value = Reflect::get(image.as_ref(), &JsValue::from_str("data")).map_err(js_error)?;
    value
        .dyn_into()
        .map_err(|_| "cpu image data is not a clamped array".to_owned())
}

fn stage_committed(session: &mut crate::session::Session) -> Result<Option<StagedFrame>, String> {
    let drawn = session
        .draw_committed_mono()
        .map_err(|error| error.to_string())?;
    stage_drawn(session, drawn)
}

fn stage_color(session: &mut crate::session::Session) -> Result<Option<StagedFrame>, String> {
    let drawn = session
        .draw_committed_rgb()
        .map_err(|error| error.to_string())?;
    stage_drawn(session, drawn)
}

fn stage_interaction(session: &mut crate::session::Session) -> Result<Option<StagedFrame>, String> {
    let drawn = session
        .draw_interaction()
        .map_err(|error| error.to_string())?;
    stage_drawn(session, drawn)
}

fn stage_drawn(
    session: &mut crate::session::Session,
    drawn: RasterResult,
) -> Result<Option<StagedFrame>, String> {
    if !drawn.wrote {
        return Ok(None);
    }
    session
        .defer_cpu_presentation()
        .map_err(|error| error.to_string())?;
    copy_presented(session)?;
    Ok(Some(StagedFrame {
        extent: drawn.extent,
        generation: drawn.generation,
    }))
}

fn copy_presented(session: &crate::session::Session) -> Result<(), String> {
    let device = session.cpu_device().map_err(|error| error.to_string())?;
    let frame = session.presented_frame();
    let Ok(len) = u32::try_from(frame.len()) else {
        return Err("cpu frame does not fit in the retained image".to_owned());
    };
    CPU_CANVAS.with(|slot| {
        let Ok(slot) = slot.try_borrow() else {
            return Err("cpu canvas is busy".to_owned());
        };
        let Some(publication) = slot.publication.as_ref() else {
            return Err("no cpu canvas is bound".to_owned());
        };
        if publication.extent != device || publication.pixels.length() != len {
            return Err("cpu frame length disagrees with the retained image".to_owned());
        }
        publication.pixels.copy_from(frame);
        Ok(())
    })
}

fn blit_retained() -> Result<(), String> {
    CPU_CANVAS.with(|slot| {
        let Ok(slot) = slot.try_borrow() else {
            return Err("cpu canvas is busy".to_owned());
        };
        let Some(publication) = slot.publication.as_ref() else {
            return Err("no cpu canvas is bound".to_owned());
        };
        publication
            .context
            .put_image_data(&publication.image, 0.0, 0.0)
            .map_err(js_error)
    })
}

fn accept_staged(
    session: &mut crate::session::Session,
    staged: &StagedFrame,
) -> Result<(), String> {
    if session.viewport().generation() != staged.generation {
        return Err("a newer frame is current, so the staged blit was not accepted".to_owned());
    }
    session
        .accept_cpu_presentation()
        .map_err(|error| error.to_string())?;
    let _seq = session.publish_cpu_frame(staged.extent);
    Ok(())
}

/// Publish kind 6 without clearing refining or recording settlement.
fn publish_staged(
    session: &mut crate::session::Session,
    staged: &StagedFrame,
) -> Result<(), String> {
    if session.viewport().generation() != staged.generation {
        return Err("a newer frame is current, so the staged blit was not accepted".to_owned());
    }
    let _seq = session.publish_cpu_frame(staged.extent);
    Ok(())
}

fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "canvas blit failed".to_owned())
}
