//! Parses the command line arguments.
//!
//! Currently parses only logging.
//! Basic usage for running the basic example with logging on:
//!
//! ```cargo run --example basic -- --log```
//!
//! Requires adding parse_args() function at start of main.

use chrono;
use clap::Parser;
use std::{
    fs::{create_dir_all, OpenOptions},
    path::Path,
    sync::Arc,
};
use tracing_subscriber::FmtSubscriber;

use crate::ndl::generate_and_run_sim;

/// Stores the different command line arguments.
#[derive(Parser)]
struct Args {
    ///Logging flag. Used to turn logging on or off.
    #[arg(short, long)]
    log: bool,
    ///File path to the ndl file to run as the Sim
    #[arg(short, long)]
    ndl: String,
}

/// Parses command line arguments and allows for quick checking of them.
pub async fn parse_args() {
    let cli = Args::parse();
    // Capture log flag for turning logging on or off
    if cli.log {
        initialize_logging();
    }
    //Capture required NDL filepath argument
    match Path::new(&cli.ndl).try_exists() {
        Ok(true) => generate_and_run_sim(cli.ndl).await,
        Ok(false) => eprintln!("Provided file: \'{}\' not found", cli.ndl),
        Err(e) => eprintln!("{e}"),
    }
}

/// Initializes the event protocol. Only should be called once when the sim starts.
/// Allows for event! to be called and writes to a log file in elvis-core/src/logs.
/// During Tests -- cargo test -- logs will not be generated for the time being
fn initialize_logging() {
    let main_path = "./logs";
    create_dir_all(main_path).unwrap();
    let file_path = format!(
        "{}/debug-{}.log",
        main_path,
        chrono::offset::Local::now().format("%y-%m-%d_%H-%M-%S")
    );
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path)
        .unwrap();
    let subscriber = FmtSubscriber::builder()
        .with_writer(Arc::new(file))
        .json()
        .finish();
    // set the global default so all events/logs go to the same subscriber and
    // subsequently the same file TODO: Talk to tim on handling errors properly
    tracing::subscriber::set_global_default(subscriber).unwrap()
}
