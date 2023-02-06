use super::parsing_data::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until},
    character::complete::char,
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
        // s0 = remaining string, s1 = string gotten by parsing
        Ok((s0, s1)) => {
            // parse what was inside of the section to get the type and remaining string
            let dec = get_type(s1);
            let dectype;
            let mut args: HashMap<String, String> = HashMap::new();
            match dec {
                // s2 = (remaining string, dectype)
                Ok(s2) => {
                    dectype = s2.1;

                    match arguments(s2.0) {
                        Ok(a) => {
                            if !a.0.is_empty() {
                                return Err(format!(
                                    "Line {:?}: extra argument at '{}'\n",
                                    *line_num, s2.0
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
                                *line_num, s2.0, e
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
            let mut num_new_line = 0;
            while s0.chars().nth(num_new_line) == Some('\n') {
                num_new_line += 1;
                *line_num += 1;
            }

            Ok((dectype, args, s0[num_new_line..].to_string()))
        }

        Err(e) => {
            return Err(format!("{e}"));
        }
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
            preceded(tag(" "), take_until("=")),
            char('='),
            delimited(char('\''), take_until("'"), char('\'')),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}
