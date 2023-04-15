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

fn ping_pong(c: &mut Criterion) {
    c.bench_function("Ping Pong", |b| {
        b.to_async(runtime()).iter(simulations::ping_pong)
    });
}

fn ping_pong_multi(c: &mut Criterion) {
    c.bench_function("Ping Pong Multi", |b| {
        b.to_async(runtime()).iter(simulations::ping_pong_multi)
    });
}

fn socket_basic(c: &mut Criterion) {
    c.bench_function("Socket Basic", |b| {
        b.to_async(runtime()).iter(simulations::socket_basic)
    });
}

fn socket_ping_pong(c: &mut Criterion) {
    c.bench_function("Socket Ping Pong", |b| {
        b.to_async(runtime()).iter(simulations::socket_ping_pong)
    });
}

fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

criterion_group!(
    benches,
    //basic,
    //telephone_multi,
    //telephone_single,
    //socket_basic,
    ping_pong_multi,
    socket_ping_pong
);
criterion_main!(benches);
