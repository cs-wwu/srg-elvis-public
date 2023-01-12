use elvis::parsing::core_parser;
use elvis::parsing::parsing_data::*;

/// main wrapper for parsing testing.
pub fn parser_testing(file_path: &str) -> Result<Sim, String> {
    let res = core_parser(file_path.to_string());
    match res {
        Ok(s) => {
            return Ok(s);
        }

        Err(e) => {
            return Err(e);
        }
    }
}

#[test]
fn parsing_options_fail_duplicate_argument() {
    let result = parser_testing("./tests/parsing_tests/options_fail_duplicate_argument.txt");
    let s: String = "Errors at ./tests/parsing_tests/options_fail_duplicate_argument.txt:\n\nLine 1: Unable to parse inside of Networks due to: \n\tLine 6: duplicate argument 'id'='5'\n\n".to_string();
    match result{
        Ok(_s) => {
            assert!(false);
        }
        Err(e) => {
            assert_eq!(e, s);
        }
    }
}