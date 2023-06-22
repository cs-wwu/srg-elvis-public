use clap::{Arg, Command};
use rand::thread_rng;
use rand::Rng;
use select::document::Document;
use select::predicate::Name;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::rc::Rc;
use std::time::Duration;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
struct Page {
    size: usize,
    /// list of all website urls found
    links: Vec<String>,
    /// list of all image urls found
    images: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    size: usize,
}

impl Page {
    fn new(size: usize, links: Vec<String>, images: Vec<String>) -> Self {
        Self {
            size,
            links,
            images,
        }
    }
}

impl Image {
    fn new(size: usize) -> Image {
        Self { size }
    }
}

/// Return list of the links that exist on html starting from
/// http://127.0.0.1:7878/
/// aside: function based off webscraper with minor changes
fn get_urls(html: &str) -> Vec<String> {
    // form a html document
    let document = Document::from(html);
    // extracting all links in the ip page and filter out bad urls
    // use hashmap to avoid duplicate values, aka visited pages
    let found_urls = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .map(|link| format!("http://127.0.0.1:7878/{}", link))
        .collect();
    found_urls
}

/// Return list of image urls that exist on html
/// aside: function based off webscraper with minor changes
fn get_images(html: &str) -> Vec<String> {
    let document = Document::from(html);
    let found_images = document
        .find(Name("img"))
        .filter_map(|node| node.attr("src"))
        .map(|link| link.to_string())
        .collect();
    found_images
}

/// helper function that error checks the proccess of downloading the image
fn downloaded_img_helper(
    downloaded: &mut HashMap<String, Image>,
    baddies: &mut Vec<String>,
    img: &String,
) -> Option<String> {
    match TcpStream::connect("127.0.0.1:7878") {
        Ok(mut stream) => {
            // getting an http request using TcpStream
            let request = format!("GET {} HTTP/1.1\r\nHost: 127.0.0.1:7878\r\n\r\n", img);
            if let Err(err) = stream.write(request.as_bytes()) {
                println!("Error writing to stream: {}", err);
                return None;
            }

            // Set a timeout for read operations
            if let Err(err) = stream.set_read_timeout(Some(Duration::new(3, 0))) {
                println!("Error setting read timeout: {}", err);
            }

            // listening for response
            let mut response = String::new();
            if let Err(err) = stream.read_to_string(&mut response) {
                println!("Error reading from stream: {}", err);
                return None;
            }

            // Parse the response to get the image bytes
            let mut parts = response.split("\r\n\r\n");

            // checking if http part of link exists and
            // can be retrieved and if so is converted
            // to byte array
            if let Some(body) = parts.nth(1) {
                let img_bytes = body.as_bytes();
                let size = img_bytes.len();
                downloaded.insert(img.to_string(), Image::new(size));
                println!("Success! -> size: {}", size);
            } else {
                println!("Fail! No response body.");
                baddies.push(img.to_string());
            }
        }
        Err(e) => {
            println!("Fail {}", e);
            baddies.push(img.to_string());
        }
    }
    None
}

/* given a list of image urls, check if it's downlaoded aka is
it in 'downloaded' vector? if it's not: download the image to a folder
retrieve size of image once downloaded make a new Image() and add to
'downloaded' add to the list of found images in a page
(regardless of whether it was downloaded before or not)
*/
fn download_img(
    img_urls: &Vec<String>,
    downloaded: &mut HashMap<String, Image>,
    baddies: &mut Vec<String>,
) {
    // iterating through all the imgs in img_url vector
    for img in img_urls {
        if !downloaded.contains_key(img) {
            println!("Processing Img ...{}", img);
            // "download the image" using TcpStream
            downloaded_img_helper(downloaded, baddies, img);
        }
    }
}

/// returns a url str with the scheme and arguments removed
fn strip_url(url: &str) -> String {
    let url = Url::parse(url).unwrap();
    let mut stripped_url = format!("{}{}", url.host_str().unwrap(), url.path());
    if stripped_url.starts_with("www.") {
        stripped_url.drain(0..4);
    }
    if stripped_url.ends_with('/') {
        stripped_url.pop();
    }
    stripped_url
}

/// returns true if the given link is a link to ip search
fn is_search_result(url: &str) -> bool {
    let url = Url::parse(url).unwrap();
    let host = url.host_str().unwrap().to_string();
    host.contains("search")
}

/* Send http request to the url and receive respoinse.
Return html in string and the size of the page in bytes if the response
gives and error, tries the link again 3 times, if it still fails,
add to fail list
*/
/// aside: function based off of webscraper and changed to work with TcpStream
fn request_http(links: Vec<&str>, _tries: u8, _baddies: &mut Vec<String>) -> Vec<Option<String>> {
    // storing the http links in response
    let mut results = vec![];
    for link in links {
        // using TcpStream to connect to server
        match TcpStream::connect("127.0.0.1:7878") {
            Ok(mut stream) => {
                let request = format!(
                    "GET {} HTTP/1.1\r\nHost: 127.0.0.1:7878\r\nUser-Agent: Mozilla/5.0\r\n\r\n",
                    link
                );
                // storing the request as a byte array
                match stream.write(request.as_bytes()) {
                    Ok(_) => {
                        // Set a timeout for the response
                        stream.set_read_timeout(Some(Duration::new(3, 0))).ok();
                        let mut response = String::new();
                        match stream.read_to_string(&mut response) {
                            Ok(_) => {
                                // Check if the response is a 404 error
                                if response.contains("HTTP/1.1 404 Not Found") {
                                    println!("Fail! 404 Not Found.");
                                    results.push(None);
                                } else {
                                    // Return the response text
                                    results.push(Some(response));
                                }
                            }
                            Err(e) => {
                                // try the link 3 times then stop if it still gives error
                                println!("Fail! {}", e);
                                results.extend(request_http(vec![link], _tries + 1, _baddies));
                            }
                        }
                    }
                    Err(e) => {
                        println!("Fail! {}", e);
                        results.extend(request_http(vec![link], _tries + 1, _baddies));
                    }
                }
            }
            Err(e) => {
                println!("Fail! {}", e);
                results.extend(request_http(vec![link], _tries + 1, _baddies));
            }
        }
    }
    results
}

/* Description: while found_urls is not empty iterate through each link in the list starting from the front
check if the current user is in found_urls if not, scrape each url in the list and add to found_urls
during this for each url for 15 seconds using found equation decide whether the user is going
to stay or leave page and print how long the user stayed on the page for. Download all the images on this
page too Then add this url to visited if in found_urls, then skip this url and move on to the next
one on the list
*/
fn scrape_user_behavior(link: &str, mut limit: i32) {
    // list of visited website
    let mut visited = HashMap::new();
    // list of downloaded images
    let mut downloaded = HashMap::new();
    // list of failed URLs
    let mut baddies = Vec::new();
    let mut unvisited_urls: VecDeque<String> = VecDeque::new();
    unvisited_urls.push_back(link.to_string());

    let mut found_urls = HashSet::new();
    found_urls.insert(strip_url(link));

    while !unvisited_urls.is_empty() && limit > 0 {
        let mut x: i32 = 0;
        let url = unvisited_urls.pop_front().unwrap();
        // checking which links is being scraped
        println!("Processing URL...{}", &url);
        // for every link for 15 seconds this section decides whether the user
        // will go or stay on a website link
        while x < 16 {
            x += 1;
            // generating random number from 0 to 100
            let decision_maker = thread_rng().gen_range(0..101);
            // offset to start at around 50% chance of staying or leaving
            let user_offset: f64 = 0.25;
            // x is going to be each second 1, 2, 3, ..., 15
            let user_distribution =
                (x as f64).sqrt().recip() / 2.0 / std::f64::consts::E + user_offset;
            // since numbers are generated 1 - 100 percentage needs to be in
            // same terms
            let user_hundred = (user_distribution * 100.0).floor();
            if user_hundred < decision_maker.into() {
                continue;
            } else if user_hundred > decision_maker.into() {
                println!("user left after {} seconds\n", x);
                break;
            }
        }
        let links = url.split('/').collect::<Vec<&str>>();
        let res = request_http((*links).to_vec(), 1, &mut baddies);
        if res.get(0).unwrap().is_none() {
            // ignore invalid url 404
            continue;
        }
        // scrape urls and imgs on the page
        let res_text = res.into_iter().next().unwrap().unwrap();
        let scraped_urls = get_urls(&res_text);
        let scraped_images = get_images(&res_text);
        let size = res_text.len();

        // printing links in hashmap, should NOT have dups
        println!("Success -> Size:{}", size);
        // download all images found
        println!("******Images found within this link******");
        download_img(&scraped_images, &mut downloaded, &mut baddies);

        let new_page = Rc::new(Page::new(size, scraped_urls, scraped_images));
        // add unvisited urls from scraped_urls to found_urls
        for this_url in &new_page.links {
            let stripped = strip_url(this_url);

            if !found_urls.contains(&stripped) && !(is_search_result(&url)) {
                unvisited_urls.push_back(this_url.to_string());
                found_urls.insert(stripped);
            }
        }
        visited.insert(url, new_page.clone());
        limit -= 1;
    }
}

fn main() {
    // parsing arguments using CLAP
    let user_parse = Command::new("User Behavior")
        .about("Link Parser")
        .author("Kaila Hulse")
        .arg(
            Arg::with_name("url_start")
                .short('u')
                .long("url_start")
                .takes_value(true)
                .help("starting url on current webpage"),
        )
        .arg(
            Arg::with_name("max_page")
                .short('m')
                .long("max_page")
                .takes_value(true)
                .help("Max number of links to iterate"),
        )
        .get_matches();

    // fetching the url from the user: need to start with http:/ or https:/
    let start_url = user_parse.value_of("url_start").unwrap();
    let http_head = &(start_url)[..4];

    if http_head.ne("http") {
        eprintln!("not a url");
        return;
    }
    // see how many pages to be crawled
    let max_page = user_parse.value_of("max_page");
    let _limit = max_page;
    let _limit = match max_page {
        None => {
            println!("No limit!");
            0
        }
        Some(s) => match s.parse::<i32>() {
            Ok(n) => {
                if n <= 0 {
                    println!("No negative nor zero");
                    return;
                }
                println!("Crawling {n} pages...");
                n
            }
            Err(_) => {
                println!("Not an integer");
                return;
            }
        },
    };
    scrape_user_behavior(start_url, _limit);
}