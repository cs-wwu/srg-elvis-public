use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until},
    character::complete::char,
    error::{context, VerboseError},
    multi::many0,
    sequence::{delimited, preceded, separated_pair},
    IResult,
};

// Temp testing usage
// place:     
//  let s: &str = "[Machine name='test' net-id='1' net-id2='4' net-id3='2'][Machine name='test' net-id='3' net-id2='2']";
//  generate_sim(s);
// in a sim file and import generate sim to test

/// Core parsing struct
/// Parsed inputs become a part of the overall Schema
/// Then are parsed into their respective structs (Machine, Network, etc..)
#[derive(Debug, PartialEq, Eq)]
pub struct Schema<'a> {
    dectype: DecType,
    options: Option<Params<'a>>,
}

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
    IPtype,
    Commstd,
    Applications,
    App,
}

// Each Param should be in the format x=y
pub type Param<'a> = (&'a str, &'a str);
pub type Params<'a> = Vec<Param<'a>>;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

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
            "iptype" => DecType::IPtype,
            "commstd" => DecType::Commstd,
            "applications" => DecType::Applications,
            "app" => DecType::App,
            _ => unimplemented!("No other dec types supported"),
        }
    }
}
// Type specific structs

///Machine struct
/// Holds core machine info before turning into code
/// Contains the following info:
/// name, list of protocols, list of networks
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Machine<'a> {
    name: Option<&'a str>,
    protocols: Option<Params<'a>>,
    networks: Option<Vec<&'a str>>,
}

// TODO: Will be configured to accept full files in the future
/// main wrapper for parsing.
/// Currently only accepts single line string inputs
pub fn generate_sim(s: &str) {
    // println!("{:?}", get_all_sections(s));
    let basic_schema = get_all_sections(s);
    let mut machines: Vec<Machine> = Vec::new();
    for item in basic_schema.unwrap() {
        if item.dectype == DecType::Machine {
            machines.push(parse_machine(item.options.unwrap()));
        }
    }
    println!("{:?}", machines);
}

/// Returns a vector of schemas from a list of correctly formatted declarations.
fn get_all_sections(s: &str) -> Result<Vec<Schema>, String> {
    let mut sec = section(s);
    match sec {
        Ok(mut sec_safe) => {
            let mut schemas: Vec<Schema> = vec![];

            // until we run out of information to get schemas for
            while !sec_safe.1.is_empty() {
                let schema = get_schema(sec_safe.1);
                match schema {
                    Ok(t) => {
                        schemas.push(t);
                    }
                    Err(e) => return Err(e),
                }

                // only if there was more information to parse do we go into here
                if !sec_safe.0.is_empty() {
                    sec = section(sec_safe.0);
                    match sec {
                        Ok(temp) => {
                            sec_safe = temp;
                        }

                        Err(e) => {
                            return Err(format!("{}", e));
                        }
                    }
                } else {
                    // set it to this so the while loop fails
                    sec_safe.1 = "";
                }
            }
            Ok(schemas)
        }

        Err(e) => {
            return Err(format!("{}", e));
        }
    }
}

/// Should get a single section (everything between brackets) as input.
/// Will return a schema for that section.
/// Errors on any remaining strings that it couldn't parse.
fn get_schema(s: &str) -> Result<Schema, String> {
    let dec = dectype(s);
    let args = arguments(dec.clone().unwrap().0);

    let temp = args.unwrap();

    if !temp.0.is_empty() {
        return Err(format!("Improper Input for Schema at \"{}\"", temp.0));
    }

    Ok(Schema {
        dectype: dec.unwrap().1,
        options: Some(temp.1),
    })
}

/// grabs everything between brackets "[]"
// TODO: add behavior to ignore spaces in here?
fn section(input: &str) -> Res<&str, &str> {
    context("section", delimited(char('['), take_until("]"), char(']')))(input)
        .map(|(next_input, res)| (next_input, res))
}

/// grabs the type from the beginning of each section
/// For example, would turn "Template name='test'" into having a dec type and the remainder of the string
fn dectype(input: &str) -> Res<&str, DecType> {
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
            tag_no_case("Commstd"),
            tag_no_case("Applications"),
            tag_no_case("App"),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res.into()))
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

///Parse a machine
fn parse_machine<'a>(item: Vec<(&str, &'a str)>) -> Machine<'a> {
    let mut name: &str = "";
    // let protocols: Params;
    let mut networks: Vec<&'a str> = Vec::new();
    for opt in item {
        if opt.0 == "name" {
            name = opt.1;
        } else if opt.0.contains("net-id") {
            networks.push(opt.1);
        }
    }
    Machine {
        name: Some(name),
        protocols: None,
        networks: Some(networks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{
        error::{ErrorKind, VerboseError, VerboseErrorKind},
        Err as NomErr,
    };

    #[test]
    fn test_dectype() {
        // TODO: fix this behavior where it keeps the space before the next stuff
        assert_eq!(
            dectype("Template test='hello'"),
            Ok((" test='hello'", DecType::Template))
        );
        assert_eq!(
            dectype("Networks test='hello'"),
            Ok((" test='hello'", DecType::Networks))
        );
        assert_eq!(
            dectype("Network test='hello'"),
            Ok((" test='hello'", DecType::Network))
        );
        assert_eq!(
            dectype("IP test='hello'"),
            Ok((" test='hello'", DecType::IP))
        );
        assert_eq!(
            dectype("Machines test='hello'"),
            Ok((" test='hello'", DecType::Machines))
        );
        assert_eq!(
            dectype("Machine test='hello'"),
            Ok((" test='hello'", DecType::Machine))
        );
        assert_eq!(
            dectype("Protocols test='hello'"),
            Ok((" test='hello'", DecType::Protocols))
        );
        assert_eq!(
            dectype("IPtype test='hello'"),
            Ok((" test='hello'", DecType::IPtype))
        );
        assert_eq!(
            dectype("Commstd test='hello'"),
            Ok((" test='hello'", DecType::Commstd))
        );
        assert_eq!(
            dectype("Applications test='hello'"),
            Ok((" test='hello'", DecType::Applications))
        );
        assert_eq!(
            dectype("App test='hello'"),
            Ok((" test='hello'", DecType::App))
        );
        assert_eq!(
            dectype("Potato test='hi'"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("Potato test='hi'", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("Potato test='hi'", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    ("Potato test='hi'", VerboseErrorKind::Context("dectype")),
                ]
            }))
        );
    }

    #[test]
    fn test_section() {
        // two proper calls
        assert_eq!(
            section("[Template name='test']"),
            Ok(("", "Template name='test'"))
        );
        assert_eq!(
            section("[Template name='test'][Machine name='hellothere']"),
            Ok(("[Machine name='hellothere']", "Template name='test'"))
        );

        // checks to make sure it errors when no brackets are present
        assert_eq!(
            section("Template name='test'"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("Template name='test'", VerboseErrorKind::Char('[')),
                    ("Template name='test'", VerboseErrorKind::Context("section")),
                ]
            }))
        );

        //checks when just right bracket is missing
        assert_eq!(
            section("[Template name='test'"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    (
                        "Template name='test'",
                        VerboseErrorKind::Nom(ErrorKind::TakeUntil)
                    ),
                    (
                        "[Template name='test'",
                        VerboseErrorKind::Context("section")
                    ),
                ]
            }))
        );
    }

    #[test]
    fn test_arguments() {
        assert_eq!(
            arguments(" name='test' again='test2' potato"),
            Ok((" potato", vec![("name", "test"), ("again", "test2")]))
        );
        assert_eq!(arguments(" name='test'"), Ok(("", vec![("name", "test")])));
        // assert_eq!(arguments(" test123"), Ok(("test123", vec![""])));

        // TODO: are there any error cases?
    }

    #[test]
    fn test_get_schema() {
        // checks proper input to the function
        let s: &str = "Machine name='test' net-id='potato' net-id2='potato'";
        assert_eq!(
            get_schema(s),
            Ok(Schema {
                dectype: DecType::Machine,
                options: Some(vec![
                    ("name", "test"),
                    ("net-id", "potato"),
                    ("net-id2", "potato")
                ])
            })
        );

        // checks another proper input to the function
        let s: &str = "Machine";
        assert_eq!(
            get_schema(s),
            Ok(Schema {
                dectype: DecType::Machine,
                options: Some(vec![])
            })
        );

        // checks one variant of improper input to the function
        let s: &str = "Machine test";
        assert_eq!(
            get_schema(s),
            Err("Improper Input for Schema at \" test\"".to_string())
        );

        // checks another variant of improper input to the function
        let s: &str = "Machine test='test1' anothertest";
        assert_eq!(
            get_schema(s),
            Err("Improper Input for Schema at \" anothertest\"".to_string())
        );
    }

    #[test]
    fn test_get_all_sections() {
        let s: &str = "[Machine name='test'][App id='1']";
        assert_eq!(
            get_all_sections(s),
            Ok(vec![
                Schema {
                    dectype: DecType::Machine,
                    options: Some(vec![("name", "test")])
                },
                Schema {
                    dectype: DecType::App,
                    options: Some(vec![("id", "1")])
                }
            ])
        );

        // TODO: make this test case work (we need some way of ignoring spaces and new lines)
        // let s: &str = "[Machine name='test'] \n [App id='1']";
        // assert_eq!(
        //     get_all_sections(s),
        //     Ok(vec![
        //         Schema{dectype: DecType::Machine, options: Some(vec![("name", "test")])},
        //         Schema{dectype: DecType::App, options: Some(vec![("id", "1")])}
        //     ])
        // );

        let s: &str = "[Machine name='test']App id='1']";
        assert_eq!(
            get_all_sections(s),
            Err("Parsing Error: VerboseError { errors: [(\"App id='1']\", Char('[')), (\"App id='1']\", Context(\"section\"))] }".to_string())
        );

        let s: &str = "[Machine name='test' test][App id='1']";
        assert_eq!(
            get_all_sections(s),
            Err("Improper Input for Schema at \" test\"".to_string())
        );
    }
}
