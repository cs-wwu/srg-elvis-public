use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// Define your main client function
pub fn client() {
    let mut stream = TcpStream::connect("localhost:8080").unwrap();
    let mut buffer = Vec::new();
    let mut video_segment = "video_segment_low";

    loop {
        let buffer_len = buffer.iter().map(Vec::len).sum::<usize>();
        let buffer_cap = buffer.capacity();
        let buffer_space = buffer_cap - buffer_len;

        if buffer_space > 0 && buffer_space < 4 {
            video_segment = "video_segment_med";
        } else if buffer_space >= 4 {
            video_segment = "video_segment_high";
        }

        let request = format!(
            "GET /{} HTTP/1.1\r\n\
            Host: localhost:8080\r\n\
            Connection: Keep-Alive\r\n\
            \r\n\r\n",
            video_segment
        );

        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut buf = [0; 100];
        match stream.read(&mut buf) {
            Ok(0) => {
                println!("Connection closed by the server");
                break;
            }
            Ok(bytes_read) => {
                let response_text = String::from_utf8_lossy(&buf[..bytes_read]);
                process_http_response(&mut buffer, &response_text);
                let buffer_len = buffer.iter().map(Vec::len).sum::<usize>();
                println!("Buffer length: {}", buffer_len);
                let buffer_cap = buffer.capacity();
                println!("Buffer capacity: {}", buffer_cap);

                play_video_segments(&mut buffer);
            }
            Err(err) => {
                println!("Error reading from server: {:?}", err);
                break;
            }
        }
    }
}

// Define a function to process the HTTP response and store the video segment in the buffer
fn process_http_response(buffer: &mut Vec<Vec<u8>>, response: &str) {
    // Split the response into headers and body
    let mut parts = response.splitn(2, "\r\n\r\n");
    if let (Some(headers), Some(body)) = (parts.next(), parts.next()) {
        // Check if the response was successful
        if headers.contains("200 OK") {
            // Store the video segment data in the buffer
            buffer.push(body.trim().as_bytes().to_vec());
        } else {
            println!("Request failed. Response: {}", headers);
        }
    } else {
        println!("Invalid response format");
    }
}

// Define a function to play the video segments in the buffer
fn play_video_segments(buffer: &mut Vec<Vec<u8>>) {
    let buffer_len = buffer.len();
    println!("Buffer length: {}", buffer_len);

    while let Some(segment) = buffer.pop() {
        println!("Playing video segment: {:?}", segment);
        // Process and play the video segment

        // Sleep for a duration before playing the next video segment
        sleep(Duration::from_secs(1));
    }
}

// Simulate sleep function since it's not included in the code snippet
fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}
