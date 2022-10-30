//! Parses the command line arguments.
//! 
//! Currently parses only logging.
//! Basic usage for running the basic example with logging on:
//! 
//! ```cargo run --example basic -- --log```
//! 
//! Requires adding parse_cli() function at start of main.

use clap::Parser;
use crate::logging::init_events;

/// Stores the different command line arguments.
#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    log: bool
}

/// Parses command line arguments and allows for quick checking of them.
pub fn parse_cli() {
    let cli = Cli::parse();
    if cli.log{
        init_events();
    }
}
