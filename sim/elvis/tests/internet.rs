use elvis::simulation;

#[tokio::test]
pub async fn internet() {
    simulation::default_simulation().await;
}
