//! THROWAWAY SPIKE CODE. F-X013, native candidate driver.
//!
//! Usage: `x013-htj2k-route <case 0|1|2> <output path>`

use std::io::Write;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let case = args.next().and_then(|value| value.parse::<u32>().ok());
    let path = args.next();
    let (Some(case), Some(path)) = (case, path) else {
        eprintln!("usage: x013-htj2k-route <case 0|1|2> <output path>");
        return std::process::ExitCode::from(64);
    };

    match x013_htj2k_route::decode_case(case) {
        Ok(bytes) => match std::fs::File::create(&path).and_then(|mut file| file.write_all(&bytes))
        {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("write {path}: {error}");
                std::process::ExitCode::from(74)
            }
        },
        Err(code) => {
            eprintln!("case {case}: decode failed, raw error code {code}");
            std::process::ExitCode::from(1)
        }
    }
}
