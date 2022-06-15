use elvis::{core::Message, utils::print_type_of};

fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    let message = Message::new(b"Body").with_header(b"Header").slice(3, 8);
    print_type_of(&message);

    let expected = b"derBo";
    println!("{}", message.iter().eq(expected.iter().cloned()));
}
