use criterion::{criterion_group, criterion_main, Criterion};
use elvis::simulations;
use tokio::runtime::Runtime;

fn basic(c: &mut Criterion) {
    c.bench_function("Basic", |b| b.to_async(runtime()).iter(simulations::basic));
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

fn tcp_gigabyte(c: &mut Criterion) {
    c.bench_function("TCP Gigabyte", |b| {
        b.to_async(runtime()).iter(simulations::tcp_gigabyte_bench)
    });
}

fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

criterion_group!(
    benches,
    // basic,
    // telephone_multi,
    // telephone_single,
    tcp_gigabyte,
);
criterion_main!(benches);
