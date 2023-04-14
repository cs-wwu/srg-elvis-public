use elvis::simulations::tcp_gigabyte_bench;
use std::time::Instant;

fn main() {
    let now = Instant::now();
    tcp_gigabyte_bench();
    println!("{:?}", now.elapsed());
}
