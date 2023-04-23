use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::sync::mpsc::RecvError;
use std::{
    fs,
    fs::File,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    io::Write,
    error::Error,
};
use rand_distr::WeightedAliasIndex;
use rand::prelude::*;
use csv::{Reader, StringRecord};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    // Process each connection 
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }    
}

// Send the html page data over the stream
fn handle_connection(mut stream: TcpStream) {
    generate_html();

    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect(); // delete me

    println!("Request: {:#?}", http_request); //delete me
    // let request_line = buf_reader.lines().next().unwrap().unwrap();
    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("page.html").unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

/* Randomly generate an html page with a size, number of links, and number of images
    based on the distributions in the given .csv files */ 
fn generate_html() {
    let mut result = String::new();
    // Constants
    let link_bytes = 28;
    let empty_page_bytes = 151;
    let image_bytes = 22;

    let size = generate_number("size_weights.csv").unwrap();
    let num_links = generate_number("num_links_weights.csv").unwrap();
    let num_images = generate_number("num_images_weights.csv").unwrap();

    result += "<!DOCTYPE html>
    <html lang=\"en\">
      <head>
        <meta charset=\"utf-8\">
        <title>page</title>
      </head>
      <body>\n";
    for link in generate_links(num_links) {
        result += "<a href=\"";
        result += &link;
        result += "\">a</a>\n";
    }
    for img in generate_links(num_images) {
        result += "<img src=\"http://127.0.0.1:7878";
        result += &img;
        result += ".jpg\">";
    }
    
    let current_page_size = empty_page_bytes + (link_bytes * num_links) + (image_bytes * num_images);
    for _byte in current_page_size..size {
        result += "0";
    }
    result += "<body> \n</html>";
    
    let mut file = File::create("page.html").unwrap();
    file.write_all(result.as_bytes());
}

// Generates a vector of links
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

// Radomly generate a number based on the distribution in the given .csv file
fn generate_number(data_file: &str) -> Result<usize, Box<dyn Error>> {
    let mut buckets: Vec<f64> = Vec::new();
    let mut weights: Vec<usize> = Vec::new();

    let mut rdr = Reader::from_path(data_file)?;
    for result in rdr.records() {
        let record = result?;
        buckets.push(record.get(0).unwrap().parse::<f64>().unwrap());
        weights.push(record.get(1).unwrap().parse::<usize>().unwrap());
    }
    let dist = WeightedAliasIndex::new(weights).unwrap();
    let mut rng = thread_rng();
    
    Ok(buckets[dist.sample(&mut rng)].round() as usize)
}

