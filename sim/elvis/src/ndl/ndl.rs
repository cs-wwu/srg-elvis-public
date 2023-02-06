//! Main ndl parsing file. Will take in a file path as an argument for the file to be parsed
use super::generating::core_generator;
use super::parsing::parser::core_parser;

// TODO: Will be configured to accept full files in the future
/// main wrapper for parsing.
/// Currently accepts file paths in string form (CLI input needed in the future)
pub async fn generate_sim(file_path: String) {
    let res = core_parser(file_path);
    match res {
        Ok(s) => {
            // println!("{:?}", s);
            core_generator(s).await;
        }

        Err(e) => {
            println!("{e}");
        }
    }
}
