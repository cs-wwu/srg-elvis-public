
use rand_distr::{Normal, Distribution};
use rand::thread_rng;
use rand::seq::SliceRandom;

extern crate timer;
extern crate chrono;
use std::sync::mpsc::channel;

use select::document::Document; 
use select::predicate::Name;
use url::Url;



/*
The user program will act as a person surfing the net.
The length of time a given webpage is visited by the user for
    can be represented as a normal distribution with a mean of
    15 seconds and a standard deviation of 2.5 seconds. (For now)

Produce duration to spend on first page
Start on first page (yahoo.com)
for(;;)
    Call coil (curl) on current page
    Extract possible URL's to navigate to next page
    Select random URL to be next page
    Produce duration to spend on current page
    Wait duration
    Go to next page
*/




//Function:         filter_url
//Parameter(s):     link: &str
//Return Value:     Option<String>
//Description:      Check if given link is a valid url
fn filter_url(link: &str) -> Option<String>{
    let url = Url::parse(link);
    match  url {
        //if the url is valid, aka has https:// check if it points to yahoo.com
        Ok(url) =>{
            if url.has_host() && url.host_str().unwrap().ends_with("yahoo.com") && !url.to_string().contains("beap.gemini"){ //points to yahoo
                Some(url.to_string())
            }else{ //discard if not yahoo-related
                None
            }
        },
        //if the url is not valid, add https:// to it so it can used with reqwest
        Err(_e) =>{
            if link.starts_with('/'){
                Some(format!("https://yahoo.com{}",link))
            }else{
                None
            }
        }
    }
}


//Function:         filter_img_url
//Parameter(s):     link: &str
//Return Value:     Option<String>
//Description:      Check if given link is a valid image url
fn filter_img_url(link: &str) -> Option<String>{
    if link.contains("https://s.yimg.com") {
        Some(link.to_string())
    }else {
        None
    }
}


//Function:         get_urls
//Parameter(s):     html: &str
//Return Value:     Vec<String>
//Description:      Return list of the links that exist on html
fn get_urls(html: &str) -> Vec<String> {
    let document = Document::from(html);
    let found_urls: Vec<String> = document.find(Name("a"))
        .filter_map(|node|node.attr("href"))
        .filter_map(filter_url)
        .collect();
    // print found_urls:
    // for x in found_urls.iter() {
    //     println!("{}", x);
    // }
    found_urls
}


//Function:         get_images
//Paramter(s):      html: &str
//Return Value:     Vec<String>
//Description:      Return list of image urls that exist on html
fn get_images(html: &str) -> Vec<String>{
    let document = Document::from(html);
    let found_images = document.find(Name("img"))
        .filter_map(|node| node.attr("src"))
        .filter_map(filter_img_url)
        .collect();
    // print found_images:
    // for x in found_images.iter() {
    //     println!("{}", x);
    // }
    found_images
}


//Function:         download_img
//Parameter(s):     img_urls: &Vec<String>
//Return Value:     N/A
//Description:      Download images in img_urls
fn download_img(img_urls: &Vec<String>){
    for img in img_urls{
        let img_bytes = reqwest::blocking::get(img).unwrap().bytes().unwrap();
    }
}


//Function:         http_requester
//Parameter(s):     link: &str
//Return Value:     Option<String>
//Description:      Get all text on given page, if invalid page return nothing
fn http_requester(link: &str) -> Option<String>{
    let client = reqwest::blocking::Client::new();
    let response = client.get(link).header("User-Agent", "Mozilla/5.0");
    
    //manually handle error in case of 404 url
    match response.send() {
        Ok(rep) =>{
            Some(rep.text().unwrap())
        },
        Err(_e) =>{
            None
        }
    }
}



fn main() {

    //todo: pull starting url from cmd line, pull time user runs for from cmd line (as option) default to forever

    /*
    todo:
    flip weighted coin per second to determine if/when user leaves page
    the longer the user is on the page the less likely they are to leave
    loop
        wait one second
        flip coin to see if user leaves page
    */

    //TODO: what to do if result is none

    //starting url is yahoo
    let mut cur_url = String::from("http://yahoo.com");
    
    loop {
        //Produce random duration on webpage based on normal distribution
        //Normal distrubution values: mean - 15.0, standard deviation - 2.5
        let normal: rand_distr::Normal<f64> = Normal::new(15.0, 2.5).unwrap();
        let duration_float = normal.sample(&mut rand::thread_rng());

        let result = http_requester(&cur_url);
        //if invalid url 404
        if result.is_none() {
            return
        }

        let result_text = result.unwrap();
        let possible_urls: Vec<String> = get_urls(&result_text);
        let images: Vec<String> = get_images(&result_text);
 
        //get random url from possible_urls and assign it to cur_url
        //todo: errors on unwrap here sometimes, can't see any reason why yet because printing possible_urls results in good output
        let mut rng = thread_rng();
        cur_url = possible_urls.choose(&mut rng).unwrap().to_string();
        println!("{}", cur_url);

        //create timer and wait for previously determined time
        let timer = timer::Timer::new();
        let (tx, rx) = channel();
        let duration_int = duration_float.round() as i64;
        let _guard = timer.schedule_with_delay(chrono::Duration::seconds(duration_int), move || {
            //this closure is executed on the scheduler thread so we want to move it away asap.
            let _ignored = tx.send(()); //avoid unwrapping here.
        });
        rx.recv().unwrap();
        println!("User left page after {} seconds", duration_int);

    }



}