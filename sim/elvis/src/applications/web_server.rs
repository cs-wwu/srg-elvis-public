use csv::Reader;
use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{socket_api::socket::SocketError, Endpoint, TcpListener, TcpStream},
    Control, Protocol, Session, Shutdown,
};
use rand::{distributions::Alphanumeric, prelude::*, rngs::StdRng, Rng};
use rand_distr::WeightedAliasIndex;
use std::{error::Error, str, sync::Arc};
use tokio::sync::Barrier;

/// Determines what set of data will be used to inform html page generation, more types will be
/// added in the future
pub enum WebServerType {
    Yahoo,
}

/// Background:
/// The DistributionData struct was designed to store data obtained by scraping hundreds of
/// thousands of pages from a particular website, such as Yahoo.com. The goal was to use this data
/// to fabricate a simulated web server that emulates the behavior of the target website. The data
/// collection included various attributes of each page, including page size, the number of links,
/// the number of images, and the size of those images. This data was stored in .csv files located
/// within the web_server folder. The data for each attribute (e.g. number of links) is read into a
/// separate DistributionData struct and then used by the WebServer to inform the generation of
/// html pages.
///
/// Usage:
/// For a specific index i, `buckets[i]` signifies the lower bound of the value range encompassed by
/// that bucket, while `buckets[i+1]` represents the upper bound. The difference between values at
/// adjacent indices is constant, so the upper bound for the last bucket can be inferred.
/// `Weights[i]` represents the count of web pages that fall within that particular range, providing
/// a clear representation of how the attribute's values are distributed across the dataset.
#[derive(Clone)]
struct DistributionData {
    buckets: Vec<f32>,
    weights: Vec<u32>,
}

pub struct WebServer {
    pub server_type: WebServerType,
    seed: Option<u32>, // If a seed is provided then all generated html pages will be identical
    data_folder: String,
}

impl WebServer {
    pub fn new(server_type: WebServerType, seed: Option<u32>) -> Self {
        let data_folder: String = match server_type {
            // Set the data_folder file path based on the server type
            WebServerType::Yahoo => String::from("src/applications/web_server/yahoo"),
        };
        Self {
            server_type,
            seed,
            data_folder,
        }
    }

    /// Read the specified .csv file into a DistrubutionData struct. Each row of the given csv must
    /// be in the format "bucket,weight" where bucket is a numerical value and weight is the number
    /// of data points that have that value. See DistributionData documentation for more details.
    fn read_csv(data_folder: &String, file_name: &str) -> Result<DistributionData, Box<dyn Error>> {
        let mut buckets: Vec<f32> = Vec::new();
        let mut weights: Vec<u32> = Vec::new();

        let path = format!("{}/{}", data_folder, file_name);
        let mut rdr = Reader::from_path(path)?;
        for result in rdr.records() {
            let record = result?;
            buckets.push(record.get(0).unwrap().parse::<f32>().unwrap());
            weights.push(record.get(1).unwrap().parse::<u32>().unwrap());
        }

        let data = DistributionData { buckets, weights };
        Ok(data)
    }
}

#[async_trait::async_trait]
impl Protocol for WebServer {
    async fn start(
        &self,
        shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let local_host = Endpoint::new([100, 42, 0, 0].into(), 80); // Temporary work around until localhost is implemented
        let mut listener = TcpListener::bind(local_host, protocols).await.unwrap();

        // Create DistributionData objects from .csv files
        let image_size = WebServer::read_csv(&self.data_folder, "image_size.csv").unwrap();
        let num_images = WebServer::read_csv(&self.data_folder, "num_images.csv").unwrap();
        let num_links = WebServer::read_csv(&self.data_folder, "num_links.csv").unwrap();
        let page_size = WebServer::read_csv(&self.data_folder, "page_size.csv").unwrap();

        // Continuously listen for and accept new connections
        loop {
            let stream = match listener.accept().await {
                Ok(stream) => stream,
                Err(SocketError::Shutdown) => {
                    // This prevents the program from panicking on shut down
                    shutdown.shut_down();
                    return Ok(());
                }
                Err(_) => panic!(),
            };
            let connection = ServerConnection::new(
                image_size.clone(),
                num_images.clone(),
                num_links.clone(),
                page_size.clone(),
                self.seed,
            );

            // Spawn a new tokio task to handle each client
            tokio::spawn(async move {
                connection.handle_connection(stream).await;
            });
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

/// Represents an ongoing connection between a WebServer and a particular client
struct ServerConnection {
    image_size: DistributionData,
    num_images: DistributionData,
    num_links: DistributionData,
    page_size: DistributionData,
    seed: Option<u32>, // If a seed is provided then all generated html pages will be identical
}

impl ServerConnection {
    pub fn new(
        image_size: DistributionData,
        num_images: DistributionData,
        num_links: DistributionData,
        page_size: DistributionData,
        seed: Option<u32>,
    ) -> Self {
        Self {
            image_size,
            num_images,
            num_links,
            page_size,
            seed,
        }
    }

    /// Read the incoming requests from clients and sends back an html page or image bytes.
    async fn handle_connection(&self, mut stream: TcpStream) {
        let mut rng = StdRng::from_entropy();
        let mut is_first_page = true;
        loop {
            // Recieve request
            let request_bytes: Vec<u8> = match stream.read().await {
                Ok(request_bytes) => request_bytes,
                Err(SocketError::Shutdown) => return, // This prevents the program from panicking on shut down
                Err(_) => panic!(),
            };
            let request_str = str::from_utf8(&request_bytes).unwrap();

            // Parse request and prepare response
            let status_line: &str = "HTTP/1.1 200 OK";
            let contents = if Self::is_img_request(request_str) {
                // Image request, send bytes back
                let img_size = &self.image_size.clone();
                Self::get_bytes(self.generate_number(img_size, &mut rng))
            } else {
                // html page request, send html page back
                self.generate_html(&mut rng, is_first_page)
            };

            // Send response
            let length = contents.len();
            let response: String =
                format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
            stream.write(response.into_bytes()).await.unwrap();
            is_first_page = false;
        }
    }

    /// Randomly generate an html page with a size, number of links, and number of images
    /// based on the DistributionData stored in self
    fn generate_html(&self, rng: &mut StdRng, is_first_page: bool) -> String {
        const LINK_BYTES: u32 = 28;
        const EMPTY_PAGE_BYTES: u32 = 151;
        const IMAGE_BYTES: u32 = 22;
        let mut result = String::new();

        // Generate html page characteristics
        let size = self.generate_number(&self.page_size, rng);
        let mut num_links = self.generate_number(&self.num_links, rng);
        let num_images = self.generate_number(&self.num_images, rng);

        // If this is the first page the client is recieving ensure that it has > 0 links
        if is_first_page {
            while num_links == 0 {
                num_links = self.generate_number(&self.num_links, rng);
            }
        }

        // Assemble the html file contents
        result += "<!DOCTYPE html>
            <html lang=\"en\">
              <head>
                <meta charset=\"utf-8\">
                <title>page</title>
              </head>
              <body>\n";
        for link in self.generate_links(num_links, rng) {
            result += format!("<a href=\"{}\">a</a>\n", &link).as_str();
        }
        for img in self.generate_links(num_images, rng) {
            result += format!("<img src=\"http://100.42.0.1:80{}.jpg\">", &img).as_str();
        }
        // Add the appropriate number of bytes of data if the current page size is < size
        let current_page_size =
            EMPTY_PAGE_BYTES + (LINK_BYTES * num_links) + (IMAGE_BYTES * num_images);
        if current_page_size < size {
            result.push_str(Self::get_bytes(size - current_page_size).as_str());
        }
        result += "</body> \n</html>";

        result
    }

    /// Generates a vector of randomly generated links if no seed is provided. If a seed is
    /// provided every link in the vector will be identical
    fn generate_links(&self, num_links: u32, rng: &mut StdRng) -> Vec<String> {
        let mut links = Vec::new();

        match self.seed {
            Some(_) => {
                // If a seed is provided, use the same string for every link
                for _i in 0..num_links {
                    links.push("/seededlink".to_string());
                }
            }
            None => {
                // Otherwise, randomly generate a string
                for _i in 0..num_links {
                    let rand_string: String = (rng)
                        .sample_iter(&Alphanumeric)
                        .take(10)
                        .map(char::from)
                        .collect();

                    links.push(["/", &rand_string].concat());
                }
            }
        }

        links
    }

    /// Generates a string containing <num_bytes> 0's
    fn get_bytes(num_bytes: u32) -> String {
        let mut result = String::new();
        for _byte in 0..num_bytes {
            result += "0";
        }

        result
    }

    /// Returns a randomly selected number based on the distribution specified in `data` if
    /// self.seed == None. The number returned will be one of the bucket values. Buckets with
    /// higher weights are more likely to be selected. If self.seed == Some then the value of seed
    /// will be returned instead. See DistributionData documentation for more details.
    fn generate_number(&self, data: &DistributionData, rng: &mut StdRng) -> u32 {
        match self.seed {
            Some(num) => num,
            None => {
                let dist = WeightedAliasIndex::new(data.weights.clone()).unwrap();

                data.buckets[dist.sample(rng)].round() as u32
            }
        }
    }

    /// Returns true if rqquest_str contains common image file extensions (case insensitive).
    /// Accepted file extensions: [".jpg", ".jpeg", ".png", ".webp", ".gif", ".svg"]
    fn is_img_request(request_str: &str) -> bool {
        let request = request_str.to_ascii_lowercase();
        let accepted_img_types = vec![".jpg", ".jpeg", ".png", ".webp", ".gif", ".svg"];
        for s in accepted_img_types {
            if request.contains(s) {
                return true;
            }
        }
        false
    }
}
