use async_trait::async_trait;
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpStream},
    Control, Protocol, Session, Shutdown,
};
use rand::thread_rng;
use rand::Rng;
use select::document::Document;
use select::predicate::Name;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::{collections::HashSet, sync::RwLock};
use tokio::sync::Barrier;
use url::Url;

/// Struct that implements the parameters for a page
#[derive(Serialize, Deserialize, Debug)]
struct Page {
    /// size of the page
    size: usize,
    /// list of all website urls found
    links: Vec<String>,
    /// list of all image urls found
    images: Vec<String>,
}

/// Struct that implements the parameters for an image
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

/// User Behavior struct that just takes in the server_address for the sim
pub struct UserBehavior {
    server_address: Endpoint,
    pub num_pages_recvd: RwLock<u32>,
    pub num_imgs_recvd: RwLock<u32>,
}

impl UserBehavior {
    pub fn new(server_address: Endpoint) -> Self {
        Self {
            server_address,
            num_pages_recvd: RwLock::new(0),
            num_imgs_recvd: RwLock::new(0),
        }
    }

    /// Return list of the links that exist on html starting from
    /// http://100.42.0.0:80/
    /// aside: function based off webscraper with minor changes
    fn get_urls(html: &str) -> Vec<String> {
        // form a html document
        let document = Document::from(html);
        // extracting all links in the ip page and filter out bad urls
        // use hashmap to avoid duplicate values, aka visited pages
        let found_urls = document
            .find(Name("a"))
            .filter_map(|node| node.attr("href"))
            .map(|link| format!("http://100.42.0.0:80/{}", link))
            .collect();
        found_urls
    }

    /// Return list of image urls that exist on html
    /// aside: function based off webscraper with minor changes
    fn get_images(html: &str) -> Vec<String> {
        let document = Document::from(html);
        // filtering the found images
        let found_images = document
            .find(Name("img"))
            .filter_map(|node| node.attr("src"))
            .map(|link| link.to_string())
            .collect();
        found_images
    }

    /// returns a url str with the scheme and arguments removed
    fn strip_url(url: &str) -> String {
        // using Url crate to help make formatting easier
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
        // using Url crate to help make formatting easier
        let url = Url::parse(url).unwrap();
        let host = url.host_str().unwrap().to_string();
        // this will return true or false
        host.contains("search")
    }

    /// Description: while found_urls is not empty iterate through each link in the list starting from the front
    /// check if the current user is in found_urls if not, scrape each url in the list and add to found_urls
    /// during this for each url for 15 seconds using found equation decide whether the user is going
    /// to stay or leave page and print how long the user stayed on the page for. Download all the images on this
    /// page too Then add this url to visited if in found_urls, then skip this url and move on to the next
    /// one on the list
    async fn scrape_user_behavior(
        &self,
        server_address: Endpoint,
        links: &str,
        mut limit: i32,
        protocols: ProtocolMap,
    ) {
        // list of visited links, downloaded images, bad links, and unvisited links
        let mut visited = HashMap::new();
        let mut downloaded = HashMap::new();
        let mut baddies = Vec::new();
        let mut unvisited_urls: VecDeque<String> = VecDeque::new();
        let mut results: Vec<String> = Vec::new();
        unvisited_urls.push_back(links.to_string());

        let mut found_urls = HashSet::new();
        // inserting the starting url
        found_urls.insert(Self::strip_url(links));
        // connect to TcpStream using the server's address from the sim
        match TcpStream::connect(server_address, protocols.clone()).await {
            Ok(mut stream) => {
                // iterating while there are still more unvisited urls and
                // the limit has not been reached
                while !unvisited_urls.is_empty() && limit > 0 {
                    let mut x: i32 = 0;
                    let url = unvisited_urls.pop_front().unwrap();
                    // checking which link is being scraped
                    println!();
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
                            println!("user left after {} seconds", x);
                            break;
                        }
                    }
                    let links = vec![url.as_str()];
                    // Send http request to the url and receive respoinse.
                    // Return html in string and the size of the page in bytes if the response
                    // gives and error, tries the link again 3 times, if it still fails,
                    // add to fail list taken from original function as the stream can't
                    // be called
                    for &link in &(*links).to_vec() {
                        // iterating through all of the links and formatting a request
                        let request = format!(
                            "GET {} HTTP/1.1\r\nHost: 100.42.0.0:80\r\nConnection: keep-aliveUser-Agent: Mozilla/5.0\r\n\r\n",
                            link
                        );
                        // writing the http link to the stream with error checking
                        if let Err(err) = stream.write(request).await {
                            println!("Error writing to stream: {}", err);
                        }
                        // reading the server's response
                        let _received_msg: Vec<u8> = match stream.read().await {
                            Ok(received_msg) => {
                                // turning the received_msg into a String from bytes
                                let received_string = match String::from_utf8(received_msg.clone())
                                {
                                    Ok(s) => s,
                                    Err(_) => {
                                        // Handle conversion error here if needed
                                        String::from("Failed to convert bytes to String")
                                    }
                                };
                                // pushing the string to the results Vector
                                results.push(received_string);
                                received_msg
                            }
                            Err(e) => {
                                // Erroring out if the link fails
                                println!("Fail! {}", e);
                                Vec::new()
                            }
                        };
                    }
                    let res_text = results.remove(0);
                    let scraped_urls = Self::get_urls(&res_text);
                    let scraped_images = Self::get_images(&res_text);
                    let size = res_text.len();

                    println!("******Images found within this link******");
                    // given a list of image urls, check if it's downlaoded aka is
                    // it in 'downloaded' vector? if it's not: using downloaded_img
                    // helper download the image to a folder
                    // retrieve size of image once downloaded make a new Image() and add to
                    // 'downloaded' add to the list of found images in a page
                    // (regardless of whether it was downloaded before or not)
                    // this was taken fromt the original function due to not being
                    // able to clone the stream
                    for img in &scraped_images {
                        // checking if the image has already been downloaded
                        if !downloaded.contains_key(img) {
                            println!("Processing Img ...{}", img);
                            // create a request format for the image
                            let request = format!(
                                "GET {} HTTP/1.1\r\nHost: 100.42.0.0:80\r\nConnection: keep-aliveUser-Agent: Mozilla/5.0\r\n\r\n",
                                img
                            );
                            // writing the request to the created stream with error checking
                            if let Err(err) = stream.write(request).await {
                                println!("Error writing to stream: {}", err);
                            }
                            let _response = String::new();
                            // reading in the response or in this case receiving the jpg
                            // with error checking
                            let response = match stream.read().await {
                                Ok(bytes) => String::from_utf8(bytes).unwrap(),
                                Err(err) => {
                                    println!("Error reading from stream: {}", err);
                                    return; // Return an error condition
                                }
                            };
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
                                *self.num_imgs_recvd.write().unwrap() += 1;
                            } else {
                                println!("Fail! No response body.");
                                baddies.push(img.to_string());
                            }
                        }
                    }
                    // creating a page that holds all the scraped_urls
                    let new_page = Arc::new(Page::new(size, scraped_urls, scraped_images));
                    // add unvisited urls from scraped_urls to found_urls
                    for this_url in &new_page.links {
                        let stripped = Self::strip_url(this_url);
                        // if the link is valid turn this link from unvisited to visited
                        if !found_urls.contains(&stripped) && !(Self::is_search_result(&url)) {
                            unvisited_urls.push_back(this_url.to_string());
                            found_urls.insert(stripped);
                        }
                    }
                    // inserting the url into the page and decreasing the limit
                    visited.insert(url, new_page.clone());
                    limit -= 1;
                    *self.num_pages_recvd.write().unwrap() += 1;
                }
            }
            Err(e) => {
                // Erroring out if connecting to the stream fails
                println!("Fail! {}", e);
            }
        }
    }
}

#[async_trait]
impl Protocol for UserBehavior {
    async fn start(
        &self,
        _shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // creating the starting url and how many pages the user will sift through
        let start_url = "http://100.42.0.0:80/";
        let num_page = rand::thread_rng().gen_range(20..150);
        let cloned_protocols = protocols.clone();
        self.scrape_user_behavior(self.server_address, start_url, num_page, cloned_protocols)
            .await;
        Ok(())
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
