//! Contains all methods relevant to parsing [Networks] and their data
use super::parser_util::{general_error, general_parser, num_tabs_to_string};
use super::parsing_data::*;

/// Parses an entire [Networks] section.
///
///
/// Takes in a [DecType], [Params], remaining string, current number of tabs, and the current line number.
/// Returns either an error String, or a tuple containing [Networks] and the remaining string.
pub fn networks_parser(
    dec: DecType,
    _args: Params,
    s0: String,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Networks, String), String> {
    let mut networks = Networks::new();
    let mut remaining_string = s0;
    // save the line number we start on in this function for errors
    let networks_line_num = *line_num - 1;

    while !remaining_string.is_empty() {
        // count how many tabs there are at the beginning of the string
        let t = remaining_string.chars().take_while(|c| c == &'\t').count() as i32;
        match t {
            // next line doesn't have enough tabs thus a network isn't being declared
            t if t < num_tabs => break,
            // next line has too many tabs meaning there is something trying to be declared inside of this type (which can't happen)
            t if t > num_tabs => {
                return Err(general_error(
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
                ))
            }
            _ => (),
        }

        // parse everything after the tabs
        let parsed_networks = general_parser(&remaining_string[num_tabs as usize..], line_num);

        let dectype;
        let options;
        match parsed_networks {
            Ok(net) => {
                dectype = net.0;
                options = net.1;
                remaining_string = net.2;
            }

            Err(e) => {
                return Err(general_error(
                    num_tabs,
                    networks_line_num,
                    dec,
                    format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                ));
            }
        }
        // make sure the type we got was a [Network]
        match dectype {
            DecType::Network => {
                let network =
                    network_parser(dectype, options, remaining_string, num_tabs + 1, line_num);
                match network {
                    Ok(network) => {
                        match network.0.options.get("id") {
                            Some(id) => {
                                if networks.contains_key(id) {
                                    return Err(format!("{}Line {:?}: Unable to insert Network into Networks due to duplicate id: {}", num_tabs_to_string(num_tabs), networks_line_num, id));
                                }
                                networks.insert(id.to_string(), network.0);
                            }
                            None => {
                                return Err(format!("{}Line {:?}: Unable to parse Network in Networks due to missing id.", num_tabs_to_string(num_tabs), networks_line_num));
                            }
                        }
                        remaining_string = network.1;
                    }
                    Err(e) => {
                        return Err(format!(
                            "{}Line {:?}: Unable to parse inside of Networks due to: \n{}",
                            num_tabs_to_string(num_tabs),
                            networks_line_num,
                            e
                        ));
                    }
                }
            }
            _ => {
                return Err(general_error(
                    num_tabs,
                    networks_line_num,
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

    Ok((networks, remaining_string))
}

/// Parses a single [Network]. Takes in a [DecType], [Params], remaining string, current number of tabs, and the current line number.
/// Returns either an error String, or a tuple containing [Network] and the remaining string.
fn network_parser(
    dec: DecType,
    args: Params,
    s0: String,
    num_tabs: i32,
    line_num: &mut i32,
) -> Result<(Network, String), String> {
    let mut ips = IPs::new();
    let mut remaining_string = s0;
    // save the beginning of this declarations line num
    let network_line_num = *line_num - 1;

    let mut t = remaining_string.chars().take_while(|c| c == &'\t').count() as i32;
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err(general_error(
            num_tabs,
            *line_num,
            dec,
            format!(
                "{}Line {:?}: expected {} tabs and got {} tabs instead.\n",
                num_tabs_to_string(num_tabs + 1),
                *line_num,
                num_tabs,
                t
            ),
        ));
    }

    while !remaining_string.is_empty() {
        // save the line num at the beginning of this line
        let cur_line_num = *line_num;
        let network = general_parser(&remaining_string[num_tabs as usize..], line_num);
        match network {
            Ok(network) => {
                // error if the type inside isn't IP
                if network.0 != DecType::IP {
                    return Err(general_error(
                        num_tabs,
                        network_line_num,
                        dec,
                        format!(
                            "{}Line {:?}: expected type IP and got type {:?} instead.\n",
                            num_tabs_to_string(num_tabs + 1),
                            cur_line_num,
                            network.0
                        ),
                    ));
                }
                ips.push(IP {
                    dectype: network.0,
                    options: network.1,
                });
                remaining_string = network.2;
            }

            Err(e) => {
                return Err(general_error(
                    num_tabs,
                    network_line_num,
                    dec,
                    format!("{}{}", num_tabs_to_string(num_tabs + 1), e),
                ));
            }
        }

        // see how many tabs are on the next line and respond accordingly
        t = remaining_string.chars().take_while(|c| c == &'\t').count() as i32;
        match t {
            // next line doesn't have enough tabs thus a network isn't being declared
            t if t < num_tabs => break,
            // next line has too many tabs meaning there is something trying to be declared inside of this type (which can't happen)
            t if t > num_tabs => {
                return Err(general_error(
                    num_tabs,
                    network_line_num,
                    dec,
                    format!(
                        "{}Line {:?}: Invalid tab count. Expected {} tabs, got {} tabs.\n",
                        num_tabs_to_string(num_tabs + 1),
                        line_num,
                        num_tabs,
                        t
                    ),
                ))
            }
            _ => (),
        }
    }

    Ok((
        Network {
            dectype: dec,
            options: args,
            ip: ips,
        },
        remaining_string,
    ))
}
