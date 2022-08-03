use criterion::{criterion_group, criterion_main, Criterion};
use elvis::simulation::default_simulation;

async fn internet() {
    default_simulation().await;
}

fn criterion_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("UDP/IPv4 delivery", |b| {
        b.to_async(&runtime).iter(|| internet())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
