use crate::core::Buf;

// Test buf creation
#[test]
fn test_buf_create() {
    let data = b"Hello";
    let buf = Buf::new(data);
    assert_eq!(data.len(), buf.len());
}

// Test buf clone
#[test]
fn test_buf_clone() {
    let data = b"Hello";
    let buf1 = Buf::new(data);
    let buf2 = buf1.clone();
    assert_eq!(buf1.len(), buf2.len());
    assert_eq!(buf1[..], buf2[..]);
}

// Test buf slice
#[test]
fn test_buf_slice() {
    let data = b"Hello World";
    let buf1 = Buf::new(data);
    let buf2 = buf1.slice(6, buf1.len());
    assert_eq!(b"World"[..], buf2[..]);
}
