# NDL Application Specification

Here are the core applications currently avalable:

- [Send Message](#send_message)
- [Capture](#capture)

To see more info on machines, click [here](Machines).

# Adding New Applications

Adding a new application is very similar to adding a new protocol. After an application has been created, it can be added to the machine generator. In the machine generator there is an application match arm in which you add the name of the application such as `send_message` or `capture`. At this point you can access the `option` variable which will contain the not only the originally matched argument but any additional options you may need. This functions as a hashmap and should be treated as though the options do not exist before checking for them. You may need asserts such as
```assert!(app.options.contains_key("port"),"Capture application doesn't contain port.");```
After you check for the associated arguments, you can then call your application function and add it to the `protocols_to_be_added` vector. By the end of the loop, this will contain all applications and protocols that specific machine needs. You may be so inclined to create functions for each applications creation and just return the new application instead, this is valid and highly suggested. 
Here is an example of a capture application match arm.
```
"capture" => {
        assert!(
            app.options.contains_key("port"),
            "Capture application doesn't contain port."
        );
        assert!(
            app.options.contains_key("ip"),
            "Capture application doesn't contain ip."
        );
        assert!(
            app.options.contains_key("message_count"),
            "Capture application doesn't contain message_count."
        );
        let ip = ip_string_to_ip(
            app.options.get("ip").unwrap().to_string(),
            "capture declaration",
        );
        assert!(ip_table.contains_key(&ip.into()), "Invalid IP found in capture application. IP does not exist in ip table. Found: {:?}", ip);
        let port = string_to_port(app.options.get("port").unwrap().to_string());
        let message_count = app
            .options
            .get("message_count")
            .unwrap()
            .parse::<u32>()
            .expect("Invalid u32 found in Capture for message count");
        protocols_to_be_added.push(Capture::new_shared(
            ip.into(),
            port,
            message_count,
        ));
    }
```

# <a id="send_message"></a> Send_Message

This application will send a message from the machine it's on to another machine. This make look like `[Application name='send_message' message='Hello this is an awesome test message!' to='recv1' port='0xbeef']`.

## Extra Arguments

This protocol takes in the following arguments:
- `message`
    - **Definition**: A string that will be sent
    - **Type**: String
- `to`
    - **Definition**: The name of the machine we are sending to
    - **Type**: Machine name (String)
- `port`
    - **Definition**: The port we will be sending the message on
    - **Type**: Port (hex or u16)

# <a id="capture"></a> Capture

This application will wait to receive some number of messages on a port. This may look like `[Application name='capture' ip='123.45.67.90' port='0xbeef' message_count='1']`.

## Extra Arguments

This protocol takes in the following arguments:
- `ip`
    - **Definition**: The IP we wish to assign to this machine
    - **Type**: IP
- `port`
    - **Definition**: The port this capture will be listening on
    - **Type**: Port (hex or u16)
- `message_count`
    - **Definition**: How many messages this application should wait for
    - **Type**: u32
