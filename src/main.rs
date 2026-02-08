use std::process;

use clap::Parser;

use scriv::cli::Cli;

fn main() {
    let cli = Cli::parse();
    let code = scriv::run(cli).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        1
    });
    process::exit(code);
}
