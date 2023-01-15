I've rewritten the network type again. 😈

# API redesign

Previously, a simulation was set up as such:

```rust
let mut internet = Internet::new();
let network = internet.network(Reliable::new(1500));
internet.machine([ /* Protocols */ ], [network]);
internet.run().await;
```

The new interface looks like this:

```rust
let network = NetworkBuilder::new()::mtu(1500).build();
let machine = Machine::new([Pci::new_shared([network.tap()]), /* Other protocols */]);
run_internet(vec![machines], vec![network]).await;
```

There are several changes here:

- The `Internet` type that previously doled out `Machine`s and `Network`s is replaced with a freestanding `run_internet` function that takes the participating machines and networks as input. Where before the machine and network types were hidden behind the `Internet` interface with opaque handles used to mediate access, they are now more exposed to the user.
- Instead of a listing the networks a machine is attached to as a separate constructor parameter, I added a new `Pci` protocol that takes a list a list of `Tap`s. `Pci` acts like a set of expansion slots in a computer that network cards are plugged into. `Tap`s act as the network cards, and the user calls `Network::tap` to get an access point to the network.

In addition to providing a more uniform interface, this method of connecting machines to networks also considerably simplifies the simulation internals. A lot of rewiring of networks and taps that used to take place at the start of the simulation is unnecessary in this version. The design is overall more consistent and comprehensible for maintainers.

# General-purpose network

The main agenda item of this PR is to provide an initial implementation of an authoritative network abstraction. I believe that instead of having a network trait with varying implementations, it is better to have a single network type with sufficient customization to adapt to different kinds of networks. For the purposes of our simulation, the characteristics of different kinds of networks that we care about are enumerable:

- Maximum transmission unit
- Latency
- Throughput
- Data corruption probability
- Dropped frame probability

Perhaps there are some others I haven't considered, but whether the network be point-to-point, ethernet, WiFi, or something more exotic, the number of high-level properties by which these networks differ is small and tractable. The specific frame header representation and other details might change from one technology to another, but in general, they provide some subset the same key services:

- Unicast
- Multicast
- Broadcast
- Sender MAC address
- The protocol contained in the packet

I argue that only these high-level services are important for our simulation and not the specifics of any particular implementation. Elvis is a project about efficiently and realistically simulating Internet traffic at a macro scale with packets traversing multiple networks. The data link layer, being a local aspect of networking, does not fit well with that mission. Data link technology is also tightly coupled to hardware and physics considerations, neither of which we can provide useful models for. Attempting to accurately simulate what happens at a physical level would introduce significant overhead which runs counter to our focus on large-scale simulation.

What we want is not an Ethernet or a WiFi network. What we want is a general construct that provides networking services and the ability to model the high-level characteristics of different technologies. Organizing around a single network type is beneficial for several reasons:

- It reduces redundant logic. If each kind of network has its own type, we will likely be duplicating most of the logic for each implementation. Every network will need the same setup of channels with settings for MTU, latency, loss rate, and the like. The DRY principle improves maintenance and reliability.
- Both the interface and the internal logic are simplified when using concrete types rather than traits.
- We will acheive better performance if our development effort is focused on optimizing a single implementation. We may also see benefits due to better instruction cache use and removing polymorphic indirection.

At the moment, the network I have written supports unicast and broadcast with customization for MTU, latency, and throughput. It has the header data needed to implement DHCP as well. I think this is a good starting point that could be fleshed out into a very implementation over the coming quarter by one or two research students.
