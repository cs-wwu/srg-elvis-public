use sim::core::Message;
use sim::utils::print_type_of;

fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    let message = Message::new(b"Body").with_header(b"Header");
    print_type_of(&message);

    let expected = b"HeaderBody";
    println!("{}", message.iter().eq(expected.iter().cloned()));
}
