#![forbid(unsafe_code)]
use std::path::Path;
use std::process::ExitCode;
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 || args[1] != "inventory" {
        eprintln!("usage: rustpilot-migration inventory <upstream-repo> <output-dir>");
        return ExitCode::FAILURE;
    }
    match rustpilot_migration::inventory(
        Path::new(&args[2]),
        Path::new(&args[3]),
        rustpilot_migration::UPSTREAM_REVISION,
    ) {
        Ok(summary) => {
            print!("{summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("inventory failed: {error}");
            ExitCode::FAILURE
        }
    }
}
