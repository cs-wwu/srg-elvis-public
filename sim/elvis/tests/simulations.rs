#[tokio::test]
async fn basic() {
    elvis::simulations::basic().await
}

#[tokio::test]
async fn latent() {
    elvis::simulations::latent().await
}

#[tokio::test]
async fn telephone_multi() {
    elvis::simulations::telephone_multi().await
}

#[tokio::test]
async fn telephone_single() {
    elvis::simulations::telephone_single().await
}

#[tokio::test]
async fn unreliable() {
    elvis::simulations::unreliable().await
}

#[tokio::test]
async fn query() {
    elvis::simulations::query().await
}
