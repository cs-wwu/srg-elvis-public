use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration, SamplingMode,
};
use elvis::ndl::generate_sim;

use tokio::runtime::Runtime;

fn benchmark_sim(c: &mut Criterion) {
    let mut group = c.benchmark_group("Large Scale Sims");
    let file_path_ten = "./benches/ndl_benchmarks/ten_machines.txt";
    let file_path_hundred = "./benches/ndl_benchmarks/hundred_machines.txt";
    let file_path_thousand = "./benches/ndl_benchmarks/thousand_machines.txt";
    let file_path_ten_thousand = "./benches/ndl_benchmarks/ten_thousand_machines.txt";
    let file_path_fifty_thousand = "./benches/ndl_benchmarks/fifty_thousand_machines.txt";
    let file_path_hundred_thousand = "./benches/ndl_benchmarks/hundred_thousand_machines.txt";
    // group.sampling_mode(SamplingMode::Flat);
    // group.sample_size(50);
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_ten),
        &file_path_ten,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_hundred),
        &file_path_hundred,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_thousand),
        &file_path_thousand,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_ten_thousand),
        &file_path_ten_thousand,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_fifty_thousand),
        &file_path_fifty_thousand,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("generate_sim", file_path_hundred_thousand),
        &file_path_hundred_thousand,
        |b, &s| {
            b.to_async(runtime()).iter(|| generate_sim(s.to_string()));
        },
    );
    group.finish();
}

fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

criterion_group!(benches, benchmark_sim);
criterion_main!(benches);
