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
    println!("{:?}", core_parser(&contents.replace('\r', "")));
    // println!("{:?}", contents);
    // let basic_schema = get_all_sections(&contents);
    // println!("{:?}", basic_schema);
    // println!("{:?}", machines);
}

fn core_parser(s: &str) -> Result<Sim, String> {
    let mut networks = Networks::new();
    let mut machines = Machines::new();

    let num_tabs = 0;
    let mut remaining_string = s;
	//TODO: While loop
    while remaining_string != "" {
        let res = general_parser(remaining_string);
        match res {
            Ok(info) => {
                let dectype = info.0;
                let options = info.1;
                remaining_string = info.2;

                match dectype {
                    DecType::Template => {
                        
                    },
                    DecType::Networks => {
                        let temp = networks_parser(dectype, options, &remaining_string, num_tabs + 1);
                        match temp {
                            Ok(n) => {
                                remaining_string = n.1;
                                for new_nets in n.0 {
                                    networks.push(new_nets);
                                }

                                println!("{:?}", networks);
                            }

                            Err(e) => {
                                return Err(e);
                            }
                        }
                    },
                    DecType::Network => {
                        return Err("Cannot declare network here".to_string());
                    },
                    DecType::IP => {
                        return Err("Cannot declare IP here".to_string());
                    },
                    DecType::Machines => {
                        //TODO: change this
                        return Err("not actual error".to_string());
                    },
                    DecType::Machine => {
                        return Err("Cannot declare machine here".to_string());
                    },
                    DecType::Protocols => {
                        return Err("Cannot declare protocols here".to_string());
                    },
                    DecType::Protocol => {
                        return Err("Cannot declare protocol here".to_string());
                    },
                    DecType::Applications => {
                        return Err("Cannot declare applications here".to_string());
                    },
                    DecType::Application => {
                        return Err("Cannot declare appliication here".to_string());
                    },
                }
            }

            Err(_e) => {
                return Err("Parser failed".to_string());
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
fn arguments(input: &str) -> Res<&str, Vec<(&str, &str)>> {
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

fn general_parser(s: &str) -> Result<(DecType, Params, &str), String> {
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
                                return Err("Args format incorrect".to_string());
                            }
                        }

                        Err(e) => {
                            return Err(format!("{}", e));
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
                return Err("Not a new line after declaration".to_string());
            } else {
                while s0.chars().nth(num_new_line) == Some('\n') {
                    num_new_line += 1;
                }
            }
            
            Ok((dectype, args, &s0[num_new_line as usize..]))
        }

        Err(e) => {
            return Err(format!("{}", e));
        },
    }
}

fn networks_parser<'a>(dec: DecType, args: Params<'a>, s0: &'a str, num_tabs: i32) -> Result<(Networks<'a>, &'a str), String>{
    // println!("{:?}", dec);
    // println!("{:?}", args);
    // println!("s0: {:?}", s0);
	// println!("num tabs: {:?}", num_tabs);
    let mut networks = Networks::new();
    let mut remaining_string = s0;

    while remaining_string != "" {
        let mut t = 0;
        while t < num_tabs && remaining_string.chars().nth(t as usize) == Some('\t') {
            t+=1;
        }
        // next line doesn't have enough tabs thus a network isn't being declared
        if t != num_tabs {
            return Ok((networks, remaining_string));
        }
        
        let parsed_networks = general_parser(&remaining_string[num_tabs as usize..]);
        
        let dectype;
        let options;
        match parsed_networks {
            Ok(net) => {
                // TODO: check that dectype is network
                if net.0 != DecType::Network{
                    return Err("Invalid type entry for networks".to_string());
                }
                dectype = net.0;
                options = net.1;
                remaining_string = net.2;
            }

            Err(_e) => {
                return Err("Could not parse inside of networks".to_string());
            }
        }
        // println!("{:?}", dectype);
        match dectype {
            DecType::Network => {
                let net = network_parser(dectype, options, remaining_string, num_tabs+1);
                match net {
                    Ok(n) => {
                        // println!("{:?}", n);
                        networks.push(n.0);
                        remaining_string = n.1;
                    }
                    Err(e) =>{
                        return Err(e);
                    }
                }
                
            }
            _ => {
                return Err("Not a network type".to_string());
            }
        }
    }

    Ok((networks, remaining_string))
}


fn network_parser<'a>(dec: DecType, args: Params<'a>, s0: &'a str, num_tabs: i32) -> Result<(Network<'a>, &'a str), String>{
    
	let mut t = 0;
	let mut ips = IPs::new();
	let mut network;
	let mut remaining_string = s0;
    while t < num_tabs && remaining_string.chars().nth(t as usize) == Some('\t') {
        t+=1;
    }
    // next line doesn't have enough tabs thus a network isn't being declared
    if t != num_tabs {
        return Err("Invalid formatting".to_string());
    }
	
	loop {
		network = general_parser(&remaining_string[num_tabs as usize..]);
		match network {
			Ok(n) => {
				if n.0 != DecType::IP {
					return Err("invalid type entry into network".to_string());
				}
				ips.push(IP{dectype: n.0, options: n.1});
				remaining_string = n.2;
			}

			Err(e) => {
				return Err(e);
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