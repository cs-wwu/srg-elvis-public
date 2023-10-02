use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{socket_api::socket::SocketError, Endpoint, TcpListener, TcpStream},
    Control, Protocol, Session, Shutdown,
};

use std::{str, sync::Arc};
use tokio::sync::Barrier;

pub struct VideoServer {
    pub bytes_sent: u32,
}

// Server that, at the request of a client, sends video segments of varying quality
impl VideoServer {
    pub fn new() -> Self {
        Self { bytes_sent: 0 }
    }
}

/**
 * streaming_server works in tandem with streaming_client. This server is designed to
 * wait for http requests from a client, then process and respond when one comes through.
 */
#[async_trait::async_trait]
impl Protocol for VideoServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let local_host = Endpoint::new([100, 42, 0, 1].into(), 80); // Temp workaround since local host isn't implemented

        let listener = TcpListener::bind(local_host, protocols).await.unwrap();

        // Continuously listen for and accept new connections
        loop {
            let _stream = match listener.accept().await {
                Ok(stream) => {
                    // Spawn a new thread to handle the request
                    tokio::spawn(async move {
                        handle_http_get_request(stream).await;
                    });
                }
                Err(SocketError::Shutdown) => {
                    println!("Error accepting incoming connection");
                    // This prevents the program from panicking on shutdown
                    shutdown.shut_down();
                    return Ok(());
                }
                Err(_) => panic!(),
            };
        }
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

// Handles incoming HTTP GET requests
async fn handle_http_get_request(mut stream: TcpStream) {
    loop {
        // Read the request line by line
        let mut headers = String::new();
        loop {
            match stream.read().await {
                Ok(bytes_read) => {
                    let request = String::from_utf8_lossy(&bytes_read);
                    headers.push_str(&request);

                    if headers.contains("\r\n\r\n") {
                        break;
                    }
                }
                Err(err) => {
                    println!("Error reading from stream: {:?}", err);
                }
            }
        }

        // Process the request and generate the response
        let response = generate_http_response(&headers);

        // Send the response to the client
        stream.write(response).await.unwrap();
    }
}

// Ggenerates an HTTP response based on the request
fn generate_http_response(request: &str) -> String {
    // Extract the requested resource from the request
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
            vec![2u8; 30] // dummy video segment data
        }
        "/video_segment_high" => {
            // Simulated video segment data
            vec![3u8; 80] // dummy video segment data
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
