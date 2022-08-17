#[tokio::test]
pub async fn internet() {
    elvis::simulation::default_simulation().await;
}
