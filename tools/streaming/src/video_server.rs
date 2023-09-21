use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

// Define your main server function
pub fn server() {
    // Create a TCP listener socket and bind it to port 8080
    let listener = TcpListener::bind("localhost:8080").unwrap();

    let start_time = Instant::now();
    let timeout_duration = Duration::from_secs(10);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Spawn a new thread to handle the request
                thread::spawn(move || {
                    handle_http_get_request(stream);
                });
            }
            Err(err) => {
                println!("Error accepting incoming connection: {:?}", err);
            }
        }
        if start_time.elapsed() >= timeout_duration {
            println!("Server timeout");
            break; // Terminate the server if the elapsed time exceeds the timeout duration
        }
    }
}

// Define a function to handle incoming HTTP GET requests
fn handle_http_get_request(mut stream: TcpStream) {
    loop {
        // Read the request line by line
        let mut headers = String::new();
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(bytes_read) => {
                    // Handle the case where reading was successful
                    // Use `bytes_read` variable to determine the number of bytes read
                    // Process the data in the buffer as needed
                    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                    headers.push_str(&request);

                    if headers.contains("\r\n\r\n") {
                        break;
                    }
                }
                Err(err) => {
                    // Handle the case where reading encountered an error
                    // You can print an error message or handle the error accordingly
                    println!("Error reading from stream: {:?}", err);
                }
            }
        }

        // Process the request and generate the response
        let response = generate_http_response(&headers);

        // Send the response to the client
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

// Define a function to generate an HTTP response based on the request
fn generate_http_response(request: &str) -> String {
    // Extract the requested resource from the request (e.g., "/video_segment")
    let resource = match request.lines().next() {
        Some(line) => line.split_whitespace().nth(1).unwrap_or("/"),
        None => "/",
    };

    // Generate the response body based on the requested resource
    let response_body = match resource {
        "/video_segment_low" => {
            // Simulated video segment data
            vec![1u8; 10] // dummy video segment data
        }
        "/video_segment_med" => {
            // Simulated video segment data
            vec![2u8; 20] // dummy video segment data
        }
        "/video_segment_high" => {
            // Simulated video segment data
            vec![3u8; 40] // dummy video segment data
        }
        _ => {
            // If the requested resource is not found, return an empty response
            vec![]
        }
    };

    // Construct the response message
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
                            Content-Type: video/mp4\r\n\
                            Content-Length: {}\r\n\
                            \r\n\r\n",
        response_body.len()
    );

    // Concatenate the response headers and body
    let mut response_bytes = response.into_bytes();
    response_bytes.extend_from_slice(&response_body);

    String::from_utf8(response_bytes).unwrap()
}
