use super::general_parser;
use super::machines_parser;
use super::networks_parser;
use super::num_tabs_to_string;
use super::parsing_data::*;
use std::fs;

/// This is the core parsing logic that runs through our input file.
///
/// Takes in a string of the contents of the file and the file path of that file.
/// Returns the resulting sim, or an error message.
// TODO: make it so we only have to pass a file path through to this function
pub fn core_parser(file_path: String) -> Result<Sim, String> {
    let s = fs::read_to_string(&file_path)
        .expect("Should have been able to read the file")
        .replace('\r', "")
        .replace("    ", "\t");
    let mut networks = Networks::new();
    let mut machines = Machines::new();

    let num_tabs = 0;
    let mut remaining_string = s;
    let mut line_num = 1;

    // loops until we run out of input
    while !remaining_string.is_empty() {
        let res = general_parser(&remaining_string, &mut line_num);
        match res {
            Ok((dectype, options, rem)) => {
                remaining_string = rem;

                // the only types that won't result in an error are Templates, Networks, and Machines
                match dectype {
                    DecType::Template => {}
                    DecType::Networks => {
                        match networks_parser(
                            dectype,
                            options,
                            remaining_string,
                            num_tabs + 1,
                            &mut line_num,
                        ) {
                            Ok(n) => {
                                // update the remaining string and networks list if we got a result
                                remaining_string = n.1;
                                for new_nets in n.0 {
                                    if networks.contains_key(&new_nets.0) {
                                        return Err(format!("Errors at {}:\n\n{}Line {:?}: Unable to insert Network into Networks due to duplicate id: {}", file_path, num_tabs_to_string(num_tabs), line_num, new_nets.0));
                                    }
                                    networks.insert(new_nets.0, new_nets.1);
                                }
                            }

                            Err(e) => {
                                return Err(format!("Errors at {file_path}:\n\n{e}\n"));
                            }
                        }
                    }
                    DecType::Machines => {
                        match machines_parser(
                            dectype,
                            options,
                            remaining_string,
                            num_tabs + 1,
                            &mut line_num,
                        ) {
                            Ok(n) => {
                                // update the remaining string and machines list if we got a result
                                remaining_string = n.1;
                                for new_machine in n.0 {
                                    machines.push(new_machine);
                                }
                            }

                            Err(e) => {
                                return Err(format!("Errors at {file_path}:\n\n{e}\n"));
                            }
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n",
                            file_path,
                            line_num - 1,
                            dectype
                        ));
                    }
                }
            }

            Err(e) => {
                return Err(format!("Errors at {file_path}:\n\n{e}"));
            }
        }
    }

    Ok(Sim { networks, machines })
}
