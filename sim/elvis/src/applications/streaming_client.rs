use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpStream},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::{
    str,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;
use tokio::time::Duration;

pub struct StreamingClient {
    server_address: Endpoint,
    pub bytes_recieved: RwLock<usize>,
}

/**Client designed to test and work with streaming_server. Connects to server,
 * requests video segments, and "plays" them until shut down. I have commented out
 * the "playing" part, but if needed for debugging or to see it working in real time
 * it can be uncommented.
**/
impl StreamingClient {
    pub fn new(server_address: Endpoint) -> Self {
        Self {
            server_address,
            bytes_recieved: RwLock::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for StreamingClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        // Create a new TcpStream connected to the server address
        let mut stream = TcpStream::connect(self.server_address, machine)
            .await
            .unwrap();

        let mut buffer = Vec::new();

        // for reporting to video_streaming
        let mut total_rcvd = 0;

        // default segment is low quality
        let mut video_segment = "video_segment_low";

        loop {
            // buffer space computation for quality adjustment
            let buffer_len = buffer.iter().map(Vec::len).sum::<usize>();
            let buffer_cap = buffer.capacity();
            let buffer_space = buffer_cap - buffer_len;

            // increases quality of video segment based on available buffer space
            if buffer_space > 0 && buffer_space < 4 {
                video_segment = "video_segment_med";
            } else if buffer_space >= 4 {
                video_segment = "video_segment_high";
            }

            // Http get request that will be sent to server
            let request = format!(
                "GET /{} HTTP/1.1\r\n\
                Host: server_address\r\n\
                Connection: Keep-Alive\r\n\
                \r\n\r\n",
                video_segment
            );

            // Send the request to the server
            stream.write(request).await.unwrap();

            // Read the response from the server
            match stream.read().await {
                Ok(bytes_read) if bytes_read.is_empty() => {
                    println!("Connection closed by the server");
                    break Ok(());
                }
                Ok(bytes_read) => {
                    // counts number and type of bytes recieved from server
                    // low quality bytes recvd
                    total_rcvd += bytes_read.len();

                    let response_text = String::from_utf8_lossy(&bytes_read);
                    process_http_response(&mut buffer, &response_text).await;

                    // simulate playing the video segments
                    play_video_segments(&mut buffer).await;
                }
                Err(err) => {
                    println!("Error reading from server: {:?}", err);
                    break Ok(());
                }
            }
            // report bytes_recieved to video_streaming
            *self.bytes_recieved.write().unwrap() = total_rcvd;
        }
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}

/// Processes the HTTP response and stores the video segment in the buffer
async fn process_http_response(buffer: &mut Vec<Vec<u8>>, response: &str) {
    // Splits the response into headers and body
    let mut parts = response.splitn(2, "\r\n\r\n");
    if let (Some(headers), Some(body)) = (parts.next(), parts.next()) {
        // Checks if the response was successful
        if headers.contains("200 OK") {
            // Stores the video segment data in the buffer
            buffer.push(body.trim().as_bytes().to_vec());
        } else {
            println!("Request failed. Response: {}", headers);
        }
    } else {
        println!("Invalid response format");
    }
}

/// Plays the video segments in the buffer
async fn play_video_segments(buffer: &mut Vec<Vec<u8>>) {
    while let Some(_segment) = buffer.pop() {
        // Sleep for a duration before "playing" the next video segment
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
