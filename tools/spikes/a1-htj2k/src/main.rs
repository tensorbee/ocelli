//! THROWAWAY SPIKE CODE. F-X006, Appendix A gate A1, the native half.
//!
//! `D_native` in the answer file: the same `openjp2` 0.6.1 compiled for the
//! host, reached through the same `decode_case` the wasm build exports. Writes
//! the canonical 12288 bytes to a file so `run.mjs` digests exactly the same
//! bytes for both targets.
//!
//! Usage: `a1-htj2k <case index 0..2> <output path>`

use std::io::Write;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(case) = args.next().and_then(|a| a.parse::<u32>().ok()) else {
        eprintln!("usage: a1-htj2k <case 0|1|2> <output path>");
        return std::process::ExitCode::from(64);
    };
    let Some(path) = args.next() else {
        eprintln!("usage: a1-htj2k <case 0|1|2> <output path>");
        return std::process::ExitCode::from(64);
    };

    match a1_htj2k::decode_case(case) {
        Ok(bytes) => {
            // Reported on every run, because it is the fact that decides
            // whether `u16::try_from` above could ever have refused a lossy
            // reconstruction, and a reader of the record should not have to
            // take that on trust.
            let samples: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            let low = samples.iter().min().copied().unwrap_or(0);
            let high = samples.iter().max().copied().unwrap_or(0);
            eprintln!(
                "case {case}: {} bytes, {} samples, min {low}, max {high}",
                bytes.len(),
                samples.len()
            );
            match std::fs::File::create(&path)
                .and_then(|mut file| file.write_all(&bytes))
            {
                Ok(()) => std::process::ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("write {path}: {error}");
                    std::process::ExitCode::from(74)
                }
            }
        }
        Err(code) => {
            eprintln!("case {case}: decode failed, raw error code {code}");
            std::process::ExitCode::from(1)
        }
    }
}
