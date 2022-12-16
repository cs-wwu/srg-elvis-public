use super::parser::core_parser;

/// main wrapper for parsing testing.
fn parser_testing(file_path: &str) -> Result<String, String> {
    let contents = fs::read_to_string(file_path)
        .expect("Should have been able to read the file");
    let fixed_string = contents.replace('\r', "");
    let res = core_parser(&fixed_string, file_path);
    match res {
        Ok(s) => {
            return Ok(format!("{:?}", s));
        }

        Err(e) => {
            return Err(e);
        }
    }
}


#[test]
fn parsing_test_1() {
    // let result = parser_testing("./elvis/src/parsing/testing/test1.txt");
    let result = "poop";
    // assert_eq!(result, ""Sim { networks: [Network { dectype: Network, options: [("id", "5")], ip: [IP { dectype: IP, options: [("range", "123.45.67.89-123.45.67.91")] }, IP { dectype: IP, options: [("range", "123.45.67.92-123.45.67.94")] }, IP { dectype: IP, options: [("ip", "192.168.1.121")] }] }, Network { dectype: Network, options: [("id", "1")], ip: [IP { dectype: IP, options: [("range", "12.34.56.789-14.34.56.789")] }] }], machines: [Machine { dectype: Machine, options: Some([("name", "test")]), interfaces: Interfaces { networks: [MachineNetwork { dectype: Network, options: [("id", "5")] }], protocols: [Protocol { dectype: Protocol, options: [("name", "IPv4")] }, Protocol { dectype: Protocol, options: [("name", "TCP")] }], applications: [Application { dectype: Application, options: [("name", "send_message"), ("message", "Hello!"), ("to", "10.0.0.1")] }] } }] }".to_string);
    assert_eq!(result, "bad".to_string());
}


