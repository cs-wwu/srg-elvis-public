use criterion::{criterion_group, criterion_main, Criterion};
use elvis::simulations;
use tokio::runtime::{Builder, Runtime};

fn basic(c: &mut Criterion) {
    c.bench_function("basic", |b| b.to_async(runtime()).iter(simulations::basic));
}

fn telephone_multi(c: &mut Criterion) {
    c.bench_function("telephone_multi", |b| {
        b.to_async(runtime()).iter(simulations::telephone_multi)
    });
}

fn telephone_single(c: &mut Criterion) {
    c.bench_function("telephone_single", |b| {
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

fn tcp_gigabyte(c: &mut Criterion) {
    let mut group = c.benchmark_group("low_samples");
    group.sample_size(10);
    group.bench_function("tcp_gigabyte", |b| {
        b.to_async(runtime()).iter(simulations::tcp_gigabyte_bench)
    });
}

fn runtime() -> Runtime {
    Builder::new_current_thread().enable_time().build().unwrap()
}

criterion_group!(
    benches,
    basic,
    telephone_multi,
    telephone_single,
    tcp_gigabyte,
    socket_basic,
    ping_pong_multi,
    socket_ping_pong,
);
criterion_main!(benches);
