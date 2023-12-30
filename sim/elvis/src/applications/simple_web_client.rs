use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{socket_api::socket::SocketError, Endpoint, TcpStream},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::{
    str,
    sync::{Arc, RwLock},
};
use tokio::sync::Barrier;

pub struct SimpleWebClient {
    server_address: Endpoint,
    pub num_pages_recvd: RwLock<u32>,
}

/// Client designed to test WebServer. Connects to server and requests html pages and image bytes
/// repeatedly until shut down.
impl SimpleWebClient {
    pub fn new(server_address: Endpoint) -> Self {
        Self {
            server_address,
            num_pages_recvd: RwLock::new(0),
        }
    }
}

impl Protocol for SimpleWebClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        // Create a new TcpStream connected to the server address
        let mut stream: TcpStream = TcpStream::connect(self.server_address, machine)
            .await
            .unwrap();

        // Currently doesn't matter what the request message is, the server just checks if it
        // contains common image file extensions.
        let page_request = String::from("Ferris!");
        let img_request = String::from("Ferris.jpg");

        // This is the html page generated by WebServer when the type is yahoo and the seed is 13
        let expected_msg: String = String::from("HTTP/1.1 200 OK\r\nContent-Length: 1175\r\n\r\n<!DOCTYPE html>\n            <html lang=\"en\">\n              <head>\n                <meta charset=\"utf-8\">\n                <title>page</title>\n              </head>\n              <body>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<a href=\"/seededlink\">a</a>\n<img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"><img src=\"http://100.42.0.1:80/seededlink.jpg\"></body> \n</html>");
        // This is the image data by the WebServer when the type is yahoo and the seed is 13
        let expected_img_bytes =
            String::from("HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\n0000000000000");

        loop {
            // Send request to the server
            stream.write(page_request.clone()).await.unwrap();

            // Recieve html page from the server
            let received_msg: Vec<u8> = match stream.read().await {
                Ok(received_msg) => received_msg,
                Err(SocketError::Shutdown) => return Ok(()),
                Err(e) => panic!("{:?}", e),
            };

            // Compare the recieved message string and the expected message string
            let recieved_msg_str = str::from_utf8(&received_msg).unwrap();
            assert_eq!(recieved_msg_str, expected_msg);

            // Send image download request to the server
            stream.write(img_request.clone()).await.unwrap();

            // Recieve image bytes from the server
            let received_img_bytes: Vec<u8> = match stream.read().await {
                Ok(received_img_bytes) => received_img_bytes,
                Err(SocketError::Shutdown) => return Ok(()),
                Err(e) => panic!("{:?}", e),
            };

            // Compare the recieved message string and the expected message string
            let recieved_img_bytes_str = str::from_utf8(&received_img_bytes).unwrap();
            assert_eq!(recieved_img_bytes_str, expected_img_bytes);

            // Iterate num_pages_recvd
            *self.num_pages_recvd.write().unwrap() += 1;
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
