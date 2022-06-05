use sim::core::{Buf, Message};
use sim::utils::print_type_of;


fn main() {
    println!("Elvis v{}", env!("CARGO_PKG_VERSION"));

    let message = Message::new();
    print_type_of(&message);

    let data1 = b"Body";
    let bytes = Buf::new(data1);
    let message = message.push(&bytes);

    let data2 = b"Header";
    let bytes = Buf::new(data2);
    let message = message.push(&bytes);

    let chunks = message.chunks();
    assert_eq!(2, chunks.len());
    assert_eq!(data2, &chunks[0][..]);
    assert_eq!(data1, &chunks[1][..]);
}
