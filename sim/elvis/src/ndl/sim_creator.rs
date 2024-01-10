//! Main ndl parsing file. Will take in a file path as an argument for the file to be parsed
use std::time::Duration;

use elvis_core::ExitStatus;

use super::generating::core_generator;
use super::parsing::parser::core_parser;

// TODO: Will be configured to accept full files in the future
/// Main wrapper for parsing and generating the sim.
/// Currently accepts file paths in string form
// While this technically runs the sim, the sim is started and run inside [core_generator]
pub async fn generate_and_run_sim(
    file_path: String,
    timeout: Option<Duration>,
) -> Option<ExitStatus> {
    let res = core_parser(file_path);
    match res {
        Ok(s) => Some(core_generator(s, timeout).await),
        Err(e) => {
            eprintln!("{e}");
            None
        }
    }
}
