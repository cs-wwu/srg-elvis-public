//! Main parsing file. Will take in a file path as an argument for the file to be parsed
use std::{fs};
use super::{parsing_data::*, num_tabs_to_string};
use super::machine_parser::machines_parser;
use super::network_parser::networks_parser;
use super::core_parser::general_parser;

// TODO: Will be configured to accept full files in the future
/// main wrapper for parsing.
/// Currently accepts file paths in string form (CLI input needed in the future)
pub fn generate_sim(file_path: &str) {
    let contents = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
    let fixed_string = contents.replace('\r', "").replace("    ", "\t");
    let res = core_parser(&fixed_string, file_path);
    match res {
        Ok(s) => {
            println!("{:?}", s);
        }

        Err(e) => {
            println!("{}", e);
        }
    }
}


/// This is the core parsing logic that runs through our input file. 
/// 
/// Takes in a string of the contents of the file and the file path of that file.
/// Returns the resulting sim, or an error message.
// TODO: make it so we only have to pass a file path through to this function
pub fn core_parser<'a>(s: &'a str, file_path: &str) -> Result<Sim<'a>, String> {
    let mut networks = Networks::new();
    let mut machines = Machines::new();

    let num_tabs = 0;
    let mut remaining_string = s;
    let mut line_num = 1;

    // loops until we run out of input
    while !remaining_string.is_empty() {
        let res = general_parser(remaining_string, &mut line_num);
        match res {
            Ok(info) => {
                let dectype = info.0;
                let options = info.1;
                remaining_string = info.2;

                // the only types that won't result in an error are Templates, Networks, and Machines
                match dectype {
                    DecType::Template => {
                        
                    },
                    DecType::Networks => {
                        match networks_parser(dectype, options, remaining_string, num_tabs + 1, &mut line_num) {
                            Ok(n) => {
                                // update the remaining string and networks list if we got a result
                                remaining_string = n.1;
                                for new_nets in n.0 {
                                    if networks.contains_key(new_nets.0) {
                                        return Err(format!("{}Line {:?}: Unable to insert Network into Networks due to duplicate id: {}", num_tabs_to_string(num_tabs), line_num, new_nets.0));
                                    }
                                    networks.insert(new_nets.0, new_nets.1);
                                }
                            }

                            Err(e) => {
                                return Err(format!("Errors at {}:\n\n{}\n", file_path, e));
                            }
                        }
                    },
                    DecType::Network => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Network));
                    },
                    DecType::IP => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::IP));
                    },
                    DecType::Machines => {
                        match machines_parser(dectype, options, remaining_string, num_tabs + 1, &mut line_num) {
                            Ok(n) => {
                                // update the remaining string and machines list if we got a result
                                remaining_string = n.1;
                                for new_machine in n.0 {
                                    machines.push(new_machine);
                                }
                            }

                            Err(e) => {
                                return Err(format!("Errors at {}:\n\n{}\n", file_path, e));
                            }
                        }
                    },
                    DecType::Machine => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Machine));
                    },
                    DecType::Protocols => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Protocols));
                    },
                    DecType::Protocol => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Protocol));
                    },
                    DecType::Applications => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Applications));                    
                    },
                    DecType::Application => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Application));                    
                    },
                }
            }

            Err(e) => {
                return Err(format!("Errors at {}:\n\n{}", file_path, e));
            }
        }
    }

    Ok(Sim{
        networks,
        machines 
    })
}
