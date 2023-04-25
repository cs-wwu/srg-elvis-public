use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
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
use csv::Reader;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    // Process each stream
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

/* Read the incoming requests and send back the appropriate response */
fn handle_connection(mut stream: TcpStream) {
    // Recieve request
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();
    let status_line = "HTTP/1.1 200 OK";

    // Parse request and prepare response
    let contents = if request_line.contains(".jpg") { // image request, send bytes back
        get_bytes(generate_number("image_size.csv").unwrap())
    } else {  // html page request, send html page back
        generate_html();
        fs::read_to_string("page.html").unwrap()
    };

    // Send response 
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

    // Generate html page characteristics
    let size = generate_number("page_size.csv").unwrap();
    let num_links = generate_number("num_links.csv").unwrap();
    let num_images = generate_number("num_images.csv").unwrap();

    // Assemble the html file contents
    result += "<!DOCTYPE html>
    <html lang=\"en\">
      <head>
        <meta charset=\"utf-8\">
        <title>page</title>
      </head>
      <body>\n";
    for link in generate_links(num_links) { // insert links
        result += "<a href=\"";
        result += &link;
        result += "\">a</a>\n";
    }
    for img in generate_links(num_images) { // insert images
        result += "<img src=\"http://127.0.0.1:7878";
        result += &img;
        result += ".jpg\">";
    }
    // Add the appropriate number of bytes of data if the current page size is < size
    let current_page_size = empty_page_bytes + (link_bytes * num_links) + (image_bytes * num_images);
    if current_page_size < size {
        result.push_str(get_bytes(size - current_page_size).as_str());
    }
    result += "<body> \n</html>";
    
    // Write the result to a file 
    let mut file = File::create("page.html").unwrap();
    file.write_all(result.as_bytes()).unwrap();
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

/* Generates a string containing num_bytes 0's */
fn get_bytes(num_bytes: usize) -> String {
    let mut result = String::new();
    for _byte in 0..num_bytes {
        result += "0";
    }
    result
}

/* Returns a randomly selected number based on the distribution specified in data_file. 
    data_file is a .csv file where each row is of the format bucket,weight where
    bucket is a numerical value and weight is the number of data points that have that
    value. The number returned will be one of the bucket values. Buckets with higher
    weights are more likely to be selected. */
fn generate_number(data_file: &str) -> Result<usize, Box<dyn Error>> {
    let mut buckets: Vec<f64> = Vec::new();
    let mut weights: Vec<usize> = Vec::new();

    // Read from the file
    let mut rdr = Reader::from_path(data_file)?;
    for result in rdr.records() {
        let record = result?;
        buckets.push(record.get(0).unwrap().parse::<f64>().unwrap());
        weights.push(record.get(1).unwrap().parse::<usize>().unwrap());
    }

    // Randomly select the value to be returned
    let dist = WeightedAliasIndex::new(weights).unwrap();
    let mut rng = thread_rng();
    
    Ok(buckets[dist.sample(&mut rng)].round() as usize)
}

