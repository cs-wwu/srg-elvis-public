use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::{
    fs,
    fs::File,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    io::Write
};
use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage};

fn main() {
    generate_html(10, 70, 10);
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }    
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let status_line = "HTTP/1.1 200 OK";
    let request_line = "page.html";

    let contents = fs::read_to_string("page.html").unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

fn generate_html(size: usize, num_links: usize, num_images: usize) {
    let mut result = String::new();
    result += "<!DOCTYPE html>
    <html lang=\"en\">
      <head>
        <meta charset=\"utf-8\">
        <title>page</title>
      </head>
      <body>
        <h1>This is a header</h1>
        <p>Here is some text</p>\n";
    for link in generate_links(num_links) {
        result += "        <a href=\"";
        result += &link;
        result += "\">This is a link</a>\n";
    }
    result += "      <body>\n";
    result += "</html>";
    
    generate_img(69);
    let mut file = File::create("page.html").unwrap();
    file.write_all(result.as_bytes());
}

fn generate_links(num_links: usize) -> Vec<String> {
    let mut links = Vec::new();
    for _i in 0..num_links {
        let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

        // links.push(["127.0.0.1:7878/", &rand_string].concat());
        links.push(["/", &rand_string].concat());

    }
    links
}

fn generate_img(img_size: usize) {
    let img: RgbImage = ImageBuffer::new(100, 100);
    img.save("test.png");
}