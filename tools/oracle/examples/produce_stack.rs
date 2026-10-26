//! Produce settled stack candidates without hiding comparison failures in a test.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use ocelli_oracle::produce::{ProducePaths, produce_applicable, produce_tier_a};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("usage: produce_stack REFERENCE CORPUS OUTPUT [tier-a]");
        return ExitCode::from(1);
    }
    let mut args = args.into_iter();
    let (Some(reference), Some(corpus), Some(output)) = (args.next(), args.next(), args.next())
    else {
        return ExitCode::from(1);
    };
    let gpu = match args.next() {
        None => false,
        Some(mode) if mode == "tier-a" => true,
        Some(_) => return ExitCode::from(1),
    };
    let output = PathBuf::from(output);
    let paths = ProducePaths {
        reference_run: PathBuf::from(reference),
        corpus_root: PathBuf::from(corpus),
        reference_subset: output.join("reference-selected"),
        candidate_run: output.join("candidate"),
        dispositions: output.join("dispositions.json"),
    };
    let summary = if gpu {
        produce_tier_a(&paths)
    } else {
        produce_applicable(&paths)
    };
    // Debug deliberately prints aggregate counts and static reasons, not case data.
    println!("{summary:?}");
    if summary.resources != "ran" || !summary.accounts() || summary.produced == 0 {
        return ExitCode::from(1);
    }
    if summary.failed > 0 || summary.unmeasured > 0 || summary.unrun > 0 {
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
