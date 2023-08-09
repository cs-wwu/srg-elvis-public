use super::parsing_data::*;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, tag_no_case, take_until, take_while1},
    character::{
        complete::{char, none_of},
        is_newline, is_space,
    },
    error::context,
    multi::many0,
    sequence::{delimited, preceded, separated_pair},
};

use std::collections::HashMap;

/// General parsing for any line of our NDL.
///
///
/// Takes in a string and the current line number of the file we are looking it.
/// Returns either an error string or a tuple containing the DecType it got, the Params it got inside of that DecType, and the remaining string after parsing.
pub fn general_parser(s: &str, line_num: &mut i32) -> Result<(DecType, Params, String), String> {
    // grab everything between brackets '[' and ']'
    let sec = section(s);

    match sec {
        // remaining_string = remaining string, parsed_string = string gotten by parsing
        Ok((remaining_string, parsed_string)) => {
            // parse what was inside of the section to get the type and remaining string
            let dec = get_type(parsed_string);
            let dectype;
            let mut args: HashMap<String, String> = HashMap::new();
            match dec {
                // tup_rem_type = (remaining string, dectype)
                Ok(tup_rem_type) => {
                    dectype = tup_rem_type.1;

                    match arguments(tup_rem_type.0) {
                        Ok(a) => {
                            if !a.0.is_empty() {
                                return Err(format!(
                                    "Line {:?}: extra argument at '{}'\n",
                                    *line_num, tup_rem_type.0
                                ));
                            }

                            for arg in &a.1 {
                                // makes sure that each argument is a unique one, otherwise error
                                if args.contains_key(arg.0) {
                                    return Err(format!(
                                        "Line {:?}: duplicate argument '{}'='{}'\n",
                                        *line_num, arg.0, arg.1
                                    ));
                                }

                                args.insert(arg.0.to_string(), arg.1.to_string());
                            }
                        }

                        Err(e) => {
                            return Err(format!(
                                "Line {:?}: unable to parse arguments at '{}' due to {}\n",
                                *line_num, tup_rem_type.0, e
                            ));
                        }
                    }

                    // at this point we have the dectype and the options (args) for said type
                }

                Err(e) => {
                    return Err(format!("{e}"));
                }
            }

            // get rid of any new lines
            let num_new_line = remaining_string.chars().take_while(|c| c == &'\n').count();
            *line_num += num_new_line as i32;

            Ok((dectype, args, remaining_string[num_new_line..].to_string()))
        }

        Err(e) => Err(format!("{e}")),
    }
}

/// Converts a number of tabs into a string with that many tabs in it.
pub fn num_tabs_to_string(num_tabs: i32) -> String {
    let mut temp = "".to_string();
    let mut temp_num = 0;

    while temp_num < num_tabs - 1 {
        temp += "\t";
        temp_num += 1;
    }

    temp.to_string()
}

/// Formats a general error message and returns that String.
pub fn general_error(num_tabs: i32, line_num: i32, dec: DecType, msg: String) -> String {
    format!(
        "{}Line {:?}: Unable to parse inside of {:?} due to: \n{}",
        num_tabs_to_string(num_tabs),
        line_num,
        dec,
        msg
    )
}

/// Grabs the type from the beginning of each section in [general_parser].
/// For example, would turn "Template name='test'" into having a dec type and the remainder of the string
fn get_type(input: &str) -> Res<&str, DecType> {
    context(
        "dectype",
        alt((
            tag_no_case("Template"),
            tag_no_case("Networks"),
            tag_no_case("Network"),
            tag_no_case("IPtype"),
            tag_no_case("IP"),
            tag_no_case("Machines"),
            tag_no_case("Machine"),
            tag_no_case("Protocols"),
            tag_no_case("Protocol"),
            tag_no_case("Applications"),
            tag_no_case("Application"),
            tag_no_case("RouterEntry"),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res.into()))
}

/// Grabs everything between brackets "[]" in [general_parser].
// TODO: add behavior to ignore spaces in here?
fn section(input: &str) -> Res<&str, &str> {
    context("section", delimited(char('['), take_until("]"), char(']')))(input)
        .map(|(next_input, res)| (next_input, res))
}

/// Breaks down the arguments of our input for the [general_parser].
/// For example, turns "name='test' net-id='testing'" into a vector of strings containing "name='test'" and "net-id='testing'"
fn arguments(input: &str) -> Res<&str, Vec<(&str, &str)>> {
    context(
        "arguments",
        many0(separated_pair(
            preceded(take_while1(check_space_or_newline), take_until("=")),
            char('='),
            delimited(
                tag("'"),
                alt((escaped(none_of("\\\'"), '\\', tag("'")), tag(""))),
                tag("'"),
            ),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

fn check_space_or_newline(chr: char) -> bool {
    is_space(chr as u8) || is_newline(chr as u8)
}
