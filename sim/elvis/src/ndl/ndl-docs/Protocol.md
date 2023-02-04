# NDL Protocol Specifications

Here are the core protocol types currently avalable:

- [IPv4](#ipv4)
- [UDP](#udp)

To see more info on machines, click [here](Machines.md).

# Adding new protocols

Protocols once created in elvis-core, can be added to the [machine generator](../generating/machine_generator.rs). In the machine generator, the protocol selector works off of a match statement. The following is an example of a match arm for a protocol.
```
"UDP" => protocols_to_be_added.push(Udp::new_shared() as SharedProtocol)
```
Adding new match arms is as simple as defining the protocol name in quotes (`"new_protocol"`) and then following traditional rust match formatting. You may be inclined to instead of defining protocols inline, creating a function to call in the match that returns the protocol to be added if the protocol is more complicated. At this point you can access the `option` variable which will contain the not only the originally matched argument (`option.1`) but any futher arguments you may need for the protocol. The following arguments can be access with calls to option such as `option.2`, `option.3`, etc...

# <a id="ipv4"></a> IPv4 Definition

This definition will give a machine the IPv4 protocol. This will be defined as `[Protocol name='IPv4']`

## Extra Arguments

This protocol takes in no extra arguments.

# <a id="udp"></a> UDP Definition

This definition will give a machine the UDP protocol. This will be defined as `[Protocol name='UDP']`

## Extra Arguments

This protocol takes in no extra arguments.
