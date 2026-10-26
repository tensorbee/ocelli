//! Native diagnostic for a command, a raster draw and a manually enqueued
//! kind 6 event.
//!
//! `first`, `window` and `scroll` print one median millisecond. That output
//! is not `render.first_frame`, `interaction.window_level_drag` or
//! `interaction.scroll`. `interactive`, `idle` and `cine` run a workload and
//! then refuse. They print no numeric value.

use std::time::Instant;
use std::{env, error::Error, process::ExitCode};

use ocelli_core::Stored;
use ocelli_render::RasterQuality;
use ocelli_viewport::commands::StackViewport;
use ocelli_viewport::ring::KIND_PRESENTED;
use ocelli_wasm::{FrameMeta, Session};

const ITERATIONS: usize = 11;

fn main() -> ExitCode {
    if cfg!(debug_assertions) {
        eprintln!("presented_frame must be built with the release profile");
        return ExitCode::from(1);
    }
    let mode = env::args().nth(1).unwrap_or_default();
    let result = match mode.as_str() {
        "first" => measure("first"),
        "window" => measure("window"),
        "scroll" => measure("scroll"),
        "interactive" => exercise_then_refuse(exercise_interactive),
        "idle" => exercise_then_refuse(exercise_idle),
        "cine" => exercise_then_refuse(exercise_cine),
        _ => Err("mode must be first, window, scroll, interactive, idle or cine".into()),
    };
    match result {
        Ok(line) => {
            println!("{line}");
            ExitCode::from(0)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn measure(mode: &str) -> Result<String, Box<dyn Error>> {
    let mut session = open_session()?;
    let _ = present_once(&mut session, mode)?;
    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        samples.push(present_once(&mut session, mode)?);
    }
    let value = median(&mut samples);
    Ok(format!(
        "{{\"value\":{value},\"kind\":6,\"tier\":\"C\",\"iterations\":{ITERATIONS}}}"
    ))
}

fn exercise_then_refuse(
    body: fn(&mut Session) -> Result<(), Box<dyn Error>>,
) -> Result<String, Box<dyn Error>> {
    let mut session = open_session()?;
    body(&mut session)?;
    Ok(
        "{\"refused\":\"host\",\"reason\":\"no CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded.\"}"
            .to_owned(),
    )
}

fn open_session() -> Result<Session, Box<dyn Error>> {
    let viewport = StackViewport::try_identity(7)
        .map_err(|error| format!("identity viewport was refused: {error:?}"))?;
    let mut session = Session::new(viewport);
    session.bind_cpu(8.0, 8.0, 1.0)?;
    Ok(session)
}

fn present_once(session: &mut Session, mode: &str) -> Result<f64, Box<dyn Error>> {
    let command = command_for(mode);
    let started = Instant::now();
    let len = u32::try_from(command.len())?;
    let address = session.alloc(len);
    if address == 0 {
        return Err("command allocation was refused".into());
    }
    let ticket = session.bulk_ticket();
    {
        let Some(slot) = session.bulk_mut(address) else {
            return Err("the command slot was missing".into());
        };
        slot.copy_from_slice(&command);
    }
    if session.apply_commands(ticket, len) == 0 {
        return Err("the command batch was refused".into());
    }
    let generation = session.viewport().generation();
    let drawn = session.draw_cpu_mono(
        &[Stored(40.0)],
        [1, 1],
        [8, 8],
        RasterQuality::Settled { generation },
    )?;
    if !drawn.wrote {
        return Err("the settled draw wrote nothing".into());
    }
    let _seq = session.publish_cpu_frame(drawn.extent);
    let mut out = vec![0_u8; 16 + 48];
    if session.copy_ring(&mut out) < 16 + 48 {
        return Err("the ring had no presented event".into());
    }
    let kind = read_u32(&out, 16);
    if kind != KIND_PRESENTED {
        return Err("the drained event was not kind 6".into());
    }
    Ok(started.elapsed().as_secs_f64() * 1_000.0)
}

fn command_for(mode: &str) -> Vec<u8> {
    match mode {
        "window" => pack(7, &window_payload()),
        "scroll" => pack(11, &1_i32.to_le_bytes()),
        _ => pack(1, &fit_payload()),
    }
}

fn exercise_interactive(session: &mut Session) -> Result<(), Box<dyn Error>> {
    for _ in 0..8 {
        let _ = present_once(session, "window")?;
    }
    Ok(())
}

fn exercise_idle(session: &mut Session) -> Result<(), Box<dyn Error>> {
    let _ = present_once(session, "first")?;
    let generation = session.viewport().generation();
    for _ in 0..8 {
        let idle = session.draw_cpu_mono(
            &[Stored(40.0)],
            [1, 1],
            [8, 8],
            RasterQuality::Settled { generation },
        )?;
        if idle.wrote {
            return Err("an idle settle wrote a frame".into());
        }
    }
    Ok(())
}

fn exercise_cine(session: &mut Session) -> Result<(), Box<dyn Error>> {
    for generation in 1..5 {
        let sample = 40.0_f32.to_le_bytes();
        let address = session.alloc(4);
        if address == 0 {
            return Err("cine allocation was refused".into());
        }
        let ticket = session.bulk_ticket();
        {
            let Some(slot) = session.bulk_mut(address) else {
                return Err("the cine slot was missing".into());
            };
            slot.copy_from_slice(&sample);
        }
        session
            .commit_frame(
                ticket,
                4,
                FrameMeta {
                    viewport: 7,
                    columns: 1,
                    rows: 1,
                    generation,
                },
            )
            .map_err(|error| format!("cine commit refused: {error:?}"))?;
        let drawn = session.draw_committed_mono()?;
        if !drawn.wrote {
            return Err("cine did not draw the new source".into());
        }
        let _seq = session.publish_cpu_frame(drawn.extent);
        let mut out = vec![0_u8; 16 + 48];
        let _ = session.copy_ring(&mut out);
    }
    Ok(())
}

fn pack(kind: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&7_u16.to_le_bytes());
    let len = u32::try_from(payload.len()).unwrap_or(0);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn fit_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&8.0_f64.to_le_bytes());
    out.extend_from_slice(&8.0_f64.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out
}

fn window_payload() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&50.0_f64.to_le_bytes());
    out.extend_from_slice(&200.0_f64.to_le_bytes());
    out.push(0);
    out
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    let Some(slice) = bytes.get(offset..offset + 4) else {
        return 0;
    };
    let Ok(buf) = <[u8; 4]>::try_from(slice) else {
        return 0;
    };
    u32::from_le_bytes(buf)
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    let Some(value) = samples.get(samples.len() / 2).copied() else {
        return 0.0;
    };
    value
}
