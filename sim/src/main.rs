use elvis::{core::Message};

fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    let message = Message::new(b"Body").with_header(b"Header").slice(3, 8);
    let expected = b"derBo";
    println!("{}", message.iter().eq(expected.iter().cloned()));
}
