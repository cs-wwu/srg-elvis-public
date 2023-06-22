mod streaming_client;
mod video_server;

fn main() {
    // start the client and server in separate threads
    let handle_client = std::thread::spawn(|| streaming_client::client());
    let handle_server = std::thread::spawn(|| video_server::server());

    // joins the threads
    handle_client.join().unwrap();
    handle_server.join().unwrap();
}
