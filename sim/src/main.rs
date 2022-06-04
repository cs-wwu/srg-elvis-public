use sim::core::Message;
use sim::utils::print_type_of;
use bytes::Bytes;


fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    let data1 = "Hello World";
    let bytes1 = Bytes::from(data1);
    let mut message = Message::new();
    message = message.push(&bytes1);

    print_type_of(&message);
    println!("{:?}", &message);
}
