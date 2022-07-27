use std::env;


/// Without arguments, main runs the default simulation
fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    // Run the default simulation
    println!("Running default simulation...");
    elvis::simulation::default_simulation();
    println!("Done");
}


