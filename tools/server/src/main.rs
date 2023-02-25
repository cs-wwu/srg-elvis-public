use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

struct Page {
    size: usize,
    links: Vec<String>,  // list of all website urls found
    images: Vec<usize>, // list of all images urls found
 }

 impl Page {
    fn generate_size() -> usize {
        100
    }
    
    fn generate_links() -> Vec<String> {
        let mut links = Vec::new();
        for _i in 1..10 {
            let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
    
            links.push(["127.0.0.1:7878/", &rand_string].concat());
        }
        links
    }
    
    fn generate_imgs() -> Vec<usize> {
        let mut links = Vec::new();
        for _i in 1..10 {
            links.push(5);
        }
        links
    }

    fn generate() -> Self {
        Self { size: Self::generate_size(), links: Self::generate_links(), images: Self::generate_imgs()}
    }

    fn print(&self) {
        println!("Size: {}", self.size);
        println!("Links: {:?}", self.links);
        println!("Images: {:?}", self.images);
    }
 }

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }

    let page = Page::generate();
    page.print();
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let status_line = "HTTP/1.1 200 OK";
    let request_line = "hello.html";

    let contents = fs::read_to_string("hello.html").unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}