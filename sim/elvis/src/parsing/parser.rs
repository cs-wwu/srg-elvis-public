use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until},
    character::complete::char,
    error::{VerboseError, context},
    sequence::{delimited, preceded, separated_pair},
    IResult,
    multi::many0,
};
use std::fs;


/// This is the type of creation we are working with
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DecType {
    Template,
    Networks,
    Network,
    IP,
    Machines,
    Machine,
    Protocols,
    Protocol,
    Applications,
    Application,
}

type Res<T, U> = IResult<T, U, VerboseError<T>>;

// Each Param should be in the format x=y
type Param<'a> = (&'a str, &'a str);
type Params<'a> = Vec<Param<'a>>;

type Networks<'a> = Vec<Network<'a>>;
type Protocols<'a> = Vec<Protocol<'a>>;
type Applications<'a> = Vec<Application<'a>>;
type Machines<'a> = Vec<Machine<'a>>;
type IPs<'a> = Vec<IP<'a>>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Interfaces<'a> {
    networks: Networks<'a>,
    protocols: Protocols<'a>,
    applications: Applications<'a>,
}

///Machine struct
/// Holds core machine info before turning into code
/// Contains the following info:
/// name, list of protocols, list of networks
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Machine<'a> {
    dectype: DecType,
    options: Option<Params<'a>>,
    interfaces: Interfaces<'a>
}


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Network<'a> {
    dectype: DecType,
    options: Params<'a>,
    ip: IPs<'a>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Protocol<'a> {
    dectype: DecType,
    options: Params<'a>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IP<'a> {
    dectype: DecType,
    options: Params<'a>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Application<'a> {
    dectype: DecType,
    options: Params<'a>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Sim<'a> {
    networks: Networks<'a>,
    machines: Machines<'a>
}



impl From<&str> for DecType {
    fn from(i: &str) -> Self {
        match i.to_lowercase().as_str() {
            "template" => DecType::Template,
            "networks" => DecType::Networks,
            "network" => DecType::Network,
            "ip" => DecType::IP,
            "machines" => DecType::Machines,
            "machine" => DecType::Machine,
            "protocols" => DecType::Protocols,
            "protocol" => DecType::Protocol,
            "applications" => DecType::Applications,
            "application" => DecType::Application,
            _ => unimplemented!("No other dec types supported"),
        }
    }
}

// TODO: Will be configured to accept full files in the future
/// main wrapper for parsing.
/// Currently accepts file paths in string form (CLI input needed in the future)
pub fn generate_sim(file_path: &str) {
    let contents = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
    // println!("contents: {:?}", );
    let fixed_string = contents.replace('\r', "");
    let res = core_parser(&fixed_string, file_path);
    match res {
        Ok(s) => {
            println!("{:?}", s);
        }

        Err(e) => {
            println!("{}", e);
        }
    }
    // println!("{:?}", contents);
    // let basic_schema = get_all_sections(&contents);
    // println!("{:?}", basic_schema);
    // println!("{:?}", machines);
}

fn core_parser<'a>(s: &'a str, file_path: &str) -> Result<Sim<'a>, String> {
    let mut networks = Networks::new();
    let mut machines = Machines::new();

    let num_tabs = 0;
    let mut remaining_string = s;
    let mut line_num = 1;
	//TODO: While loop
    while remaining_string != "" {
        let res = general_parser(remaining_string, &mut line_num);
        match res {
            Ok(info) => {
                let dectype = info.0;
                let options = info.1;
                remaining_string = info.2;

                match dectype {
                    DecType::Template => {
                        
                    },
                    DecType::Networks => {
                        let temp = networks_parser(dectype, options, &remaining_string, num_tabs + 1, &mut line_num);
                        match temp {
                            Ok(n) => {
                                remaining_string = n.1;
                                for new_nets in n.0 {
                                    networks.push(new_nets);
                                }

                                println!("Networks: {:?}", networks);
                                println!("Remaining string: \n{}", remaining_string);
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
                        //TODO: change this
                        return Err("not actual error".to_string());
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
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Applications));                    },
                    DecType::Application => {
                        return Err(format!("Errors at {}:\n\nLine {}: Cannot declare {:?} here.\n\n", file_path, line_num-1, DecType::Application));                    },
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

/// grabs the type from the beginning of each section
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

/// grabs everything between brackets "[]"
// TODO: add behavior to ignore spaces in here?
fn section(input: &str) -> Res<&str, &str> {
    context(
        "section", 
        delimited(
            char('['), 
           take_until("]"), 
            char(']')
        )
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

/// breaks down the arguments of our input
/// For example, turns "name='test' net-id='testing'" into a vector of strings containing "name='test'" and "net-id='testing'"
fn arguments<'a>(input: &'a str) -> Res<&str, Params> {
    context(
        "arguments",
        // many0(
        //     terminated(take_until(" "), tag(" ")),
        // )
        many0(separated_pair(
            preceded(tag(" "), take_until("=")),
            char('='),
            delimited(char('\''), take_until("'"), char('\'')),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

fn general_parser<'a>(s: &'a str, line_num: &mut i32) -> Result<(DecType, Params<'a>, &'a str), String> {
    let sec = section(s);
    match sec {
        // s0 = remaining string, s1 = string gotten by parsing
        Ok((s0, s1)) => {
            // parse what was inside of the section
            let dec = get_type(s1);
            let dectype;
            let args;
            match dec {
                // s2 = (remaining string, dectype)
                Ok(s2) => {
                    dectype = s2.1;
                    
                    match arguments(s2.0) {
                        Ok(a) => {
                            args = a.1;
                            if !a.0.is_empty() {
                                return Err(format!("Line {:?}: extra argument at '{}'\n", *line_num, s2.0))
                            }
                        }

                        Err(e) => {
                            return Err(format!("Line {:?}: unable to parse arguments at '{}' due to {}\n", *line_num, s2.0, e));
                        }
                    }

                    // at this point we have the dectype and the options (args) for said type
                }

                Err(e) => {
                    return Err(format!("{}", e));
                }
            }

            let mut num_new_line = 0;
            // check to see if next thing is part of previous section
            if !s0.starts_with('\n') {
                return Err(format!("Line {:?}: not a new line after declaration in '{}'\n", *line_num, s0));
            } else {
                while s0.chars().nth(num_new_line) == Some('\n') {
                    num_new_line += 1;
                    *line_num += 1;
                }
            }
            
            Ok((dectype, args, &s0[num_new_line as usize..]))
        }

        Err(e) => {
            return Err(format!("{}", e));
        },
    }
}

fn networks_parser<'a>(dec: DecType, args: Params<'a>, s0: &'a str, num_tabs: i32, line_num: &mut i32) -> Result<(Networks<'a>, &'a str), String>{
    // println!("{:?}", dec);
    // println!("{:?}", args);
    // println!("s0: {:?}", s0);
	// println!("num tabs: {:?}", num_tabs);
    let mut networks = Networks::new();
    let mut remaining_string = s0;
    let networks_line_num = line_num.clone() - 1;

    while remaining_string != "" {
        let mut t = 0;
        while remaining_string.chars().nth(t as usize) == Some('\t') {
            t+=1;
        }
        // next line doesn't have enough tabs thus a network isn't being declared
        if t != num_tabs {
            return Ok((networks, remaining_string));
        }
        
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
                return Err(general_error(num_tabs, networks_line_num, DecType::Networks, format!("{}{}", num_tabs_to_string(num_tabs + 1), e)));
            }
        }
        // println!("{:?}", dectype);
        match dectype {
            DecType::Network => {
                let net = network_parser(dectype, options, remaining_string, num_tabs+1, line_num);
                match net {
                    Ok(n) => {
                        networks.push(n.0);
                        remaining_string = n.1;
                    }
                    Err(e) =>{
                        return Err(format!("{}Line {:?}: Unable to parse inside of Networks due to: \n{}", num_tabs_to_string(num_tabs), networks_line_num, e));
                    }
                }
                
            }
            _ => {
                return Err(general_error(num_tabs, networks_line_num, DecType::Networks, format!("{}Line {:?}: expected type Network and got type {:?} instead.\n", num_tabs_to_string(num_tabs + 1), *line_num, dectype)));
            }
        }
    }

    Ok((networks, remaining_string))
}


fn network_parser<'a>(dec: DecType, args: Params<'a>, s0: &'a str, num_tabs: i32, line_num: &mut i32) -> Result<(Network<'a>, &'a str), String>{
    
	let mut t = 0;
	let mut ips = IPs::new();
	let mut network;
	let mut remaining_string = s0;
    let network_line_num = line_num.clone() - 1;
    while remaining_string.chars().nth(t as usize) == Some('\t') {
        t+=1;
    }
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err("Invalid formatting".to_string());
    }
	
	while remaining_string != "" {
        let cur_line_num = line_num.clone();
		network = general_parser(&remaining_string[num_tabs as usize..], line_num);
		match network {
			Ok(n) => {
				if n.0 != DecType::IP {
					return Err(general_error(num_tabs, network_line_num, DecType::Network, format!("{}Line {:?}: expected type IP and got type {:?} instead.\n", num_tabs_to_string(num_tabs+1), cur_line_num, n.0)));
				}
				ips.push(IP{dectype: n.0, options: n.1});
				remaining_string = n.2;
			}

			Err(e) => {
                return Err(general_error(num_tabs, network_line_num, DecType::Network, e));
				// return Err(format!("{}Line {:?}: Unable to parse inside of Network due to: \n{}{}", num_tabs_to_string(num_tabs), network_line_num, num_tabs_to_string(num_tabs + 1), e));
			}
		}

		t = 0;
		while t < num_tabs && remaining_string.chars().nth(t as usize) == Some('\t') {
			t+=1;
		}
		// next line doesn't have enough tabs thus a network isn't being declared
		if t != num_tabs {
			// return Ok(Network { dectype: dec, options: args, ip: ips });
			break;
		}
	}
	

	Ok((Network{
		dectype: dec,
		options: args,
		ip: ips,
	}, remaining_string))
}

fn num_tabs_to_string(num_tabs: i32) -> String{
    let mut temp = "".to_string();
    let mut temp_num = 0;

    while temp_num < num_tabs - 1 {
        temp += "\t";
        temp_num += 1;
    }

    return temp.to_string();
}

fn general_error(num_tabs: i32, line_num: i32, dec: DecType, msg: String) -> String {
    format!("{}Line {:?}: Unable to parse inside of {:?} due to: \n{}", num_tabs_to_string(num_tabs), line_num, dec, msg)
}