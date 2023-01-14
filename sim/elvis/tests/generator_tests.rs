use elvis::ndl::generate_sim;



#[tokio::test]
async fn test1(){
    let file_path:String = "./tests/generator_tests/test1.txt".to_string();
    generate_sim(file_path).await;
}