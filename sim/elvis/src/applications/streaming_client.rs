use std::time::Duration;

use crate::applications::{streaming_server, tcp_listener_server, tcp_stream_client};

use elvis_core::protocols::{tcp_listener, socket_api::socket::{ProtocolFamily::INET, Socket}};

use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address, socket_api::socket::SocketError, Endpoint, TcpListener, TcpStream,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{str, sync::{Arc, RwLock}};
use tokio::sync::Barrier;

pub struct StreamingClient {
    server_address: Endpoint,
    pub bytes_recieved: RwLock<u32>,
}

impl StreamingClient {
    pub fn new(server_address: Endpoint) -> Self {
        Self { server_address, bytes_recieved: RwLock::new(0)}
    }
}

#[async_trait::async_trait]
impl Protocol for StreamingClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        
        let local_host = Endpoint::new([100, 42, 0, 1].into(), 80);  // Temp workaround since local host isn't implemented
        let mut stream = TcpStream::connect(local_host, protocols).await.unwrap();

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

            stream.write(request.as_bytes()).await.unwrap();
            //stream.flush().unwrap(); flush is not implemented for TcpStream at the moment

            
            let mut buf = [0; 100];
            match stream.read().await {
                // had to change this from 0 to a vec, dunno why yet
                Ok(bytes_read) if bytes_read.is_empty() => {
                    println!("Connection closed by the server");
                    break Ok(());
                }
                Ok(bytes_read) => {
                    let response_text = String::from_utf8_lossy(&bytes_read);
                    process_http_response(&mut buffer, &response_text);
                    let buffer_len = buffer.iter().map(Vec::len).sum::<usize>();
                    println!("Buffer length: {}", buffer_len);
                    let buffer_cap = buffer.capacity();
                    println!("Buffer capacity: {}", buffer_cap);

                    play_video_segments(&mut buffer);
                }
                Err(err) => {
                    println!("Error reading from server: {:?}", err);
                    break Ok(());
                }
            }
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
