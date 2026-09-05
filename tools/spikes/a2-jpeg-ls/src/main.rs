//! THROWAWAY SPIKE CODE. F-X006, Appendix A gate A2, the native half.
//!
//! Usage: `a2-jpeg-ls <candidate 0..2> <case 0|1> <output path>`

use std::io::Write;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let parsed = (
        args.next().and_then(|a| a.parse::<u32>().ok()),
        args.next().and_then(|a| a.parse::<u32>().ok()),
        args.next(),
    );
    let (Some(candidate), Some(case), Some(path)) = parsed else {
        eprintln!("usage: a2-jpeg-ls <candidate 0|1|2> <case 0|1> <output path>");
        return std::process::ExitCode::from(64);
    };

    match a2_jpeg_ls::decode_case(candidate, case) {
        Ok(bytes) => match std::fs::File::create(&path)
            .and_then(|mut file| file.write_all(&bytes))
        {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("write {path}: {error}");
                std::process::ExitCode::from(74)
            }
        },
        Err(code) => {
            eprintln!(
                "candidate {candidate} case {case}: decode failed, \
                 raw error code {code}"
            );
            eprintln!(
                "  library said: {}",
                a2_jpeg_ls::diagnose(candidate, case)
            );
            std::process::ExitCode::from(1)
        }
    }
}
