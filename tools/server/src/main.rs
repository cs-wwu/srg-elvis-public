use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

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
    
            links.push(["https://elvis.edu/", &rand_string].concat());
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
    let page = Page::generate();
    page.print();
    let page2 = Page::generate();
    page2.print();
}