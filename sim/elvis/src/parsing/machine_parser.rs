//! Contains all methods relevant to parsing [Machines] and their data

use super::core_parser::{general_error, general_parser, num_tabs_to_string};
use super::parsing_data::*;

/// Core machine parser.
///
/// 
/// Gets called at any [Machines] section.
/// Goes down the list of machines parses all data into
/// a Vec of machines to be handled by the coding parser later.
pub fn machines_parser<'a>(
    dec: DecType,
    _args: Params<'a>,
    s0: &'a str,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Machines<'a>, &'a str), String> {
    let mut machines = Machines::new();
    let mut remaining_string = s0;
    let machines_line_num = *line_num - 1;
    //Will loop until all machine have been read
    while !remaining_string.is_empty() {
        let mut t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t += 1;
        }
        // next line doesn't have enough tabs thus a network isn't being declared
        match t{
            t if t < num_tabs => {
                return Ok((machines, remaining_string))
            },
            t if t > num_tabs => {
                return Err(general_error(
                    num_tabs,
                    machines_line_num,
                    dec,
                    format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                ))
            },
            _ => (),
        }
        //Find the next machine to be parsed
        let parsed_machines = general_parser(&remaining_string[num_tabs as usize..], line_num);
        let dectype;
        let options;
        match parsed_machines {
            //If machine was parsed correctly
            Ok(net) => {
                //get the type, arguments, and the remaining data for said machine
                dectype = net.0;
                options = net.1;
                remaining_string = net.2;
            }

            Err(e) => {
                return Err(general_error(
                    num_tabs,
                    machines_line_num,
                    dec,
                    format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                ));
            }
        }
        match dectype {
            //If correct machine type parsed
            DecType::Machine => {
                //Now go get the actual parts of the machine
                //Such as the Protocols, Applications, and Networks
                let m = machine_parser(dectype, options, remaining_string, num_tabs + 1, line_num);
                match m {
                    Ok(n) => {
                        machines.push(n.0);
                        remaining_string = n.1;
                    }
                    Err(e) => {
                        return Err(general_error(num_tabs, machines_line_num, dec, e));
                    }
                }
            }
            _ => {
                return Err(general_error(
                    num_tabs,
                    machines_line_num,
                    dec,
                    format!(
                        "{}Line {:?}: expected type Network and got type {:?} instead.\n",
                        num_tabs_to_string(num_tabs + 1),
                        *line_num,
                        dectype
                    ),
                ));
            }
        }
    }

    Ok((machines, remaining_string))
}

/// Parses a singular [Machine], called from [machines_parser].
///
/// Will return a either Machine or an Error in a Result
/// Note: Will only parse a section of [Networks], [Protocols], or [Applications] once per machine
fn machine_parser<'a>(
    dec: DecType,
    args: Params<'a>,
    s0: &'a str,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Machine<'a>, &'a str), String> {
    let mut networks: MachineNetworks = MachineNetworks::new();
    let mut protocols: Protocols = Protocols::new();
    let mut applications: Applications = Applications::new();
    let machine_line_num = *line_num - 1;
    let mut remaining_string = s0;

    let mut req = vec![DecType::Networks, DecType::Protocols, DecType::Applications];
    // Parse the 3 types
    while !remaining_string.is_empty() {
        let mut t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t += 1;
        }
        // next line doesn't have enough tabs thus a network isn't being declared
        match t{
            t if t < num_tabs => {
                return Ok((
                    Machine {
                        dectype: dec,
                        options: Some(args),
                        interfaces: Interfaces {
                            networks,
                            protocols,
                            applications,
                        },
                    },
                    remaining_string,
                ))
            },
            t if t > num_tabs => {
                return Err(general_error(
                    num_tabs,
                    machine_line_num,
                    dec,
                    format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                ))
            },
            _ => ()
        }
        // Find the machine and arguments to be parsed
        let parsed_machine = general_parser(&remaining_string[num_tabs as usize..], line_num);
        match parsed_machine {
            Ok(machine) => {
                // Check if the type found is still availiable to be parsed
                if req.contains(&machine.0) {
                    // if type was still available,
                    // remove it from the list as to not parse another section of it
                    req.remove(req.iter().position(|x| *x == machine.0).unwrap());
                    // For each type call respective parsing method and store data
                    match machine.0 {
                        DecType::Networks => {
                            match machine_networks_parser(
                                machine.0,
                                machine.1,
                                machine.2,
                                num_tabs + 1,
                                line_num,
                            ) {
                                Ok(n) => {
                                    for net in n.0 {
                                        networks.push(net);
                                    }
                                    remaining_string = n.1;
                                }

                                Err(e) => {
                                    return Err(general_error(
                                        num_tabs,
                                        machine_line_num,
                                        dec,
                                        format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                                    ));
                                }
                            }
                        }
                        DecType::Protocols => {
                            match machine_protocols_parser(
                                machine.0,
                                machine.1,
                                machine.2,
                                num_tabs + 1,
                                line_num,
                            ) {
                                Ok(n) => {
                                    for protocol in n.0 {
                                        protocols.push(protocol);
                                    }
                                    remaining_string = n.1;
                                }

                                Err(e) => {
                                    return Err(general_error(
                                        num_tabs,
                                        machine_line_num,
                                        dec,
                                        format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                                    ));
                                }
                            }
                        }
                        DecType::Applications => {
                            match machine_applications_parser(
                                machine.0,
                                machine.1,
                                machine.2,
                                num_tabs + 1,
                                line_num,
                            ) {
                                Ok(n) => {
                                    for app in n.0 {
                                        applications.push(app);
                                    }
                                    remaining_string = n.1;
                                }

                                Err(e) => {
                                    return Err(general_error(
                                        num_tabs,
                                        machine_line_num,
                                        dec,
                                        format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(general_error(
                                num_tabs,
                                machine_line_num,
                                dec,
                                format!(
                                    "{}Line {:?}: Unexpected type {:?}.\n",
                                    num_tabs_to_string(num_tabs + 1),
                                    *line_num,
                                    machine.0
                                ),
                            ))
                        }
                    }
                } else {
                    return Err(general_error(
                        num_tabs,
                        machine_line_num,
                        dec,
                        format!(
                            "{}Line {:?}: Unexpected type {:?}.\n",
                            num_tabs_to_string(num_tabs + 1),
                            *line_num,
                            machine.0
                        ),
                    ));
                }
            }

            Err(e) => {
                return Err(general_error(
                    num_tabs,
                    machine_line_num,
                    dec,
                    format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                ));
            }
        }
    }
    // Return the machine found
    Ok((
        Machine {
            dectype: DecType::Machine,
            options: Some(args),
            interfaces: Interfaces {
                networks,
                protocols,
                applications,
            },
        },
        remaining_string,
    ))
}

/// Parses the [Network] from a machine.
/// Machine networks will have ID's or names to correlate with defined Networks
fn machine_networks_parser<'a>(
    dec: DecType,
    _args: Params<'a>,
    s0: &'a str,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(MachineNetworks<'a>, &'a str), String> {
    let mut networks = MachineNetworks::new();
    let mut remaining_string = s0;
    let networks_line_num = *line_num - 1;
    let mut t = 0;
    while remaining_string.chars().nth(t as usize) == Some('\t') {
        t += 1;
    }
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err("Invalid formatting".to_string());
    }
    while !remaining_string.is_empty() {
        let network = general_parser(&remaining_string[num_tabs as usize..], line_num);
        match network {
            Ok(n) => {
                if n.0 != DecType::Network {
                    return Err(general_error(
                        num_tabs,
                        networks_line_num,
                        dec,
                        format!(
                            "{}Line {:?}: expected type Network and got type {:?} instead.\n",
                            num_tabs_to_string(num_tabs + 1),
                            line_num,
                            n.0
                        ),
                    ));
                }
                networks.push(MachineNetwork {
                    dectype: n.0,
                    options: n.1,
                });
                remaining_string = n.2;
            }

            Err(e) => {
                return Err(general_error(num_tabs, networks_line_num, dec, e));
            }
        }
        t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t += 1;
        }
        match t {
            // next line doesn't have enough tabs thus a network isn't being declared
            t if t < num_tabs => break,
            // next line has too many tabs meaning there is something trying to be declared inside of this type (which can't happen)
            t if t > num_tabs => return Err(general_error(
                    num_tabs,
                    networks_line_num,
                    dec,
                format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                )),
            _ => (),
        }
    }

    Ok((networks, remaining_string))
}

/// Parses the [Protocol] from a machine.
/// Machine protocols will contain connection types such as TCP, UDP, or IPv4
fn machine_protocols_parser<'a>(
    dec: DecType,
    _args: Params<'a>,
    s0: &'a str,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Protocols<'a>, &'a str), String> {
    let mut protocols = Protocols::new();
    let mut remaining_string = s0;
    let protocols_line_num = *line_num - 1;
    let mut t = 0;
    while remaining_string.chars().nth(t as usize) == Some('\t') {
        t += 1;
    }
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err("Invalid formatting".to_string());
    }
    //While there are more protocols to parse
    while !remaining_string.is_empty() {
        //Find the specific protocol
        let protocol = general_parser(&remaining_string[num_tabs as usize..], line_num);
        match protocol {
            Ok(n) => {
                // Verfiy the protocol is of the correct type
                if n.0 != DecType::Protocol {
                    return Err(general_error(
                        num_tabs,
                        protocols_line_num,
                        dec,
                        format!(
                            "{}Line {:?}: expected type Protocol and got type {:?} instead.\n",
                            num_tabs_to_string(num_tabs + 1),
                            line_num,
                            n.0
                        ),
                    ));
                }
                //Store the found protocol
                protocols.push(Protocol {
                    dectype: n.0,
                    options: n.1,
                });
                remaining_string = n.2;
            }

            Err(e) => {
                return Err(general_error(num_tabs, protocols_line_num, dec, e));
            }
        }

        t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t += 1;
        }
        match t {
            // next line doesn't have enough tabs thus a network isn't being declared
            t if t < num_tabs => break,
            // next line has too many tabs meaning there is something trying to be declared inside of this type (which can't happen)
            t if t > num_tabs => return Err(general_error(
                    num_tabs,
                    protocols_line_num,
                    dec,
                format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                )),
            _ => (),
        }
    }
    Ok((protocols, remaining_string))
}

/// Parses the [Application] from a machine.
/// Machine Applications will contain a variety of information
/// Items such as the type of application and data for the application will be parsed here
fn machine_applications_parser<'a>(
    dec: DecType,
    _args: Params<'a>,
    s0: &'a str,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Applications<'a>, &'a str), String> {
    let mut apps = Applications::new();
    let mut remaining_string = s0;
    let applications_line_num = *line_num - 1;
    let mut t = 0;
    while remaining_string.chars().nth(t as usize) == Some('\t') {
        t += 1;
    }
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err("Invalid formatting".to_string());
    }
    //While there are more applications to be parsed
    while !remaining_string.is_empty() {
        let application = general_parser(&remaining_string[num_tabs as usize..], line_num);
        match application {
            Ok(n) => {
                // Verify the Application is of the correct type
                if n.0 != DecType::Application {
                    return Err(general_error(
                        num_tabs,
                        applications_line_num,
                        dec,
                        format!(
                            "{}Line {:?}: expected type Network and got type {:?} instead.\n",
                            num_tabs_to_string(num_tabs + 1),
                            line_num,
                            n.0
                        ),
                    ));
                }
                // Store the Application
                apps.push(Application {
                    dectype: n.0,
                    options: n.1,
                });
                remaining_string = n.2;
            }
            Err(e) => {
                return Err(general_error(num_tabs, applications_line_num, dec, e));
            }
        }
        t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t += 1;
        }
        match t {
            // next line doesn't have enough tabs thus a network isn't being declared
            t if t < num_tabs => break,
            // next line has too many tabs meaning there is something trying to be declared inside of this type (which can't happen)
            t if t > num_tabs => return Err(general_error(
                    num_tabs,
                    applications_line_num,
                    dec,
                format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                )),
            _ => (),
        }
    }
    Ok((apps, remaining_string))
}
