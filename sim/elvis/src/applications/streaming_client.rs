use std::time::Duration;

use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        Endpoint, TcpStream,
    },
    Control, Protocol, Session, Shutdown,
};
use std::{str, sync::{Arc, RwLock}};
use tokio::sync::Barrier;


pub struct StreamingClient {
    server_address: Endpoint,
    pub bytes_recieved: RwLock<usize>,
}

// Client designed to test and work with streaming_server. Connects to server,
// requests video segments, and "plays" them until shut down. 
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
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        
        let local_host = Endpoint::new([100, 42, 0, 1].into(), 80);  // Temp workaround since local host isn't implemented

        // Create a new TcpStream connected to the server address
        let mut stream = TcpStream::connect(local_host, protocols).await.unwrap();

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
                Host: local_host\r\n\
                Connection: Keep-Alive\r\n\
                \r\n\r\n",
                video_segment
            );

            // Send the request to the server
            stream.write(request).await.unwrap();
            //stream.flush().unwrap(); flush is not implemented for TcpStream at the moment

            
            let mut recvd_low = 0;
            let mut recvd_med = 0;
            let mut recvd_high = 0;
            
            match stream.read().await {
                // had to change this from 0 to a vec, dunno why yet
                Ok(bytes_read) if bytes_read.is_empty() => {
                    println!("Connection closed by the server");
                    break Ok(());
                }
                Ok(bytes_read) => {
                    // counts number and type of bytes recieved from server
                    recvd_low = bytes_read.iter().filter(|&n| *n == 1).count();
                    total_rcvd += recvd_low;
                    recvd_med = bytes_read.iter().filter(|&n| *n == 2).count();
                    total_rcvd += recvd_med;
                    recvd_high = bytes_read.iter().filter(|&n| *n == 3).count();
                    total_rcvd += recvd_high;
                    println!("total recieved: {}", total_rcvd);

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
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
} 

// Processes the HTTP response and stores the video segment in the buffer
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


// Plays the video segments in the buffer
async fn play_video_segments(buffer: &mut Vec<Vec<u8>>) {
    //let buffer_len = buffer.iter().map(Vec::len).sum::<usize>(); // debugging
    //println!("Buffer length: {}", buffer_len); // debugging

    while let Some(segment) = buffer.pop() {
        println!("Playing video segment: {:?}", segment);
        
        // Sleep for a duration before playing the next video segment
        sleep(Duration::from_secs(1)).await;
    }
}

async fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}
