// use elvis::ndl::{generate_sim};
// use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

// use tokio::runtime::Runtime;

// fn ten_machines(c: &mut Criterion) {
//     let file_path = "./benches/ten_machines.txt";
//     c.bench_with_input(BenchmarkId::new("generate_sim", file_path), &file_path, |b, &s| {
//         b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
//     });
// }

// fn runtime() -> Runtime {
//     Runtime::new().unwrap()
// }
// criterion_group!(benches, ten_machines);
// criterion_main!(benches);
