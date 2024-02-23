# Elvis 

Elvis is a virtual Internet simulator that offers a fast, scalable, and extensible approach to simulating large-scale computer networks. Rather than relying on virtual machines or containerization, it implements a user-space networking stack to reduce memory consumption and performance overhead due to context-switching and memory copies. This makes it possible to simulate ten of thousands of computers interacting over TCP/IP on consumer hardware for use cases such as the following:

- Researching the performance of novel networking protocols and technologies
- Teaching computer security concepts such as DDOS attacks that require thousands of participating computers
- Testing how distributed systems will behave in deployment

Elvis is a product of the Systems Research Group at Western Washington University. 

## Usage
Users have can write Elvis simulations either by using the Rust API or with the Elvis Network Description Language (NDL). To view the API documentation, use `cargo doc --open`. Example simulations in Rust are available in the `sim/elvis/src/simulations` directory and example simulations in NDL are available in `sim/elvis/src/ndl`. 

## Contributing
[Install Rust](https://www.rust-lang.org/tools/install), then
```
git clone github.com/cs-wwu/srg-elvis-public
cd srg-elvis/sim
cargo run
```

Elvis has a test suite that can be run with `cargo test`.
