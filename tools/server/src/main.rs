// impl Page {
//     fn new(size: usize, links: Vec<String>, images:Vec<usize> ) -> Self {
//         Self { size, links, images}
//     }
//  }
struct Page {
    size: usize,
    links: Vec<String>,  // list of all website urls found
    images: Vec<usize>, // list of all images urls found
 }

fn generate_size() -> usize {
    10
}

fn generate_links() -> Vec<String> {
    let mut links = Vec::new();
    for _i in 1..10 {
        links.push("https://abcd.efg".to_string());
    }
    links
}

fn generate_imgs() -> Vec<usize> {
    let mut links = Vec::new();
    for _i in 1..10 {
        links.push(10);
    }
    links

}

fn main() {
    let links = generate_links();
    println!("{:?}", links)
}