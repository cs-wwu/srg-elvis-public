use criterion::{criterion_group, criterion_main, Criterion};
use elvis::simulations;
use tokio::runtime::Runtime;

fn basic(c: &mut Criterion) {
    c.bench_function("Basic", |b| b.to_async(runtime()).iter(simulations::basic));
}

fn latent(c: &mut Criterion) {
    c.bench_function("Latent", |b| {
        b.to_async(runtime()).iter(simulations::latent)
    });
}

fn telephone_multi(c: &mut Criterion) {
    c.bench_function("Telephone Multi", |b| {
        b.to_async(runtime()).iter(simulations::telephone_multi)
    });
}

fn telephone_single(c: &mut Criterion) {
    c.bench_function("Telephone Single", |b| {
        b.to_async(runtime()).iter(simulations::telephone_single)
    });
}

fn unreliable(c: &mut Criterion) {
    c.bench_function("Unreliable", |b| {
        b.to_async(runtime()).iter(simulations::unreliable)
    });
}

fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

criterion_group!(
    benches,
    basic,
    latent,
    telephone_multi,
    telephone_single,
    unreliable
);
criterion_main!(benches);
