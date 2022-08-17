use criterion::{criterion_group, criterion_main, Criterion};
use elvis::simulations::basic;

fn criterion_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("UDP/IPv4 delivery", |b| b.to_async(&runtime).iter(basic));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
