//! Parses the command line arguments.
//!
//! Currently parses only logging and allows specifying an ndl file.
//! Basic usage for running the basic example with logging to a file:
//!
//! ```cargo run -- --ndl ./ndl/basic.ndl --log```
//!
//! For logging to stdout
//!
//! ```cargo run -- --ndl ./ndl/basic.ndl --stdout```
//!
//! Requires adding parse_cli() function at start of main.

use chrono;
use clap::Parser;
use std::{
    fs::{create_dir_all, OpenOptions},
    sync::Arc,
};

use tracing_subscriber::{prelude::*, fmt, Registry, };
use tracing::Subscriber;

use crate::ndl::generate_and_run_sim;

/// Stores the different command line arguments.
#[derive(Parser)]
struct Args {
    /// Should logging to a file be enabled
    #[arg(short, long)]
    log: bool,
    /// Should logging to stdout be enabled
    #[arg(short, long)]
    stdout: bool,
    /// Path to a .ndl file
    #[arg(short, long, default_value = "")]
    ndl: String,
}

/// Parses command line arguments and allows for quick checking of them.
pub async fn initialize_from_arguments() {
    let cli = Args::parse();
    if cli.log || cli.stdout{
        initialize_logging(cli.stdout, cli.log);
    }
    if !cli.ndl.is_empty() {
        let mut file_path: String = cli.ndl.clone();
        if !file_path.ends_with(".ndl") {
            file_path += ".ndl";
        }
        generate_and_run_sim(file_path).await;
    }
}

/// Initializes the event protocol. Only should be called once when the sim starts.
/// Allows for event! to be called and writes to a log file in elvis-core/src/logs or to stdout.
/// During Tests -- cargo test -- logs will not be generated for the time being
fn initialize_logging(stdout:bool, file:bool) {
   // TODO(carsonhenrich) Clean up this code I really don't like how I'm doing this but trying to do it another way lead to headaches
    let subscriber: Box<dyn Subscriber + Send + Sync> = 
    match (stdout, file) {
        (true, true) => {
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
            Box::new(Registry::default()
                .with(fmt::Layer::default().json().with_writer(Arc::new(file)))
                .with(fmt::Layer::default().pretty().without_time().with_file(false).with_writer(std::io::stdout)))
        }
        (true, false) => {
            Box::new(Registry::default()
            .with(fmt::Layer::default().pretty().without_time().with_file(false).with_writer(std::io::stdout)))
        },
        (false, true) => {
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
            Box::new(Registry::default()
            .with(fmt::Layer::default().json().with_writer(Arc::new(file))))
        },
        _ => Box::new(Registry::default()),
    };
    // set the global default so all events/logs go to the same subscriber and
    // subsequently the same file TODO: Talk to tim on handling errors properly
    tracing::subscriber::set_global_default(subscriber).unwrap()
}
