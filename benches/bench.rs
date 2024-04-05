use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let metrics = metrics_cloudwatch_embedded::Builder::new()
        .cloudwatch_namespace("MyApplication")
        .with_dimension("Function", "My_Function_Name")
        .init()
        .unwrap();

    metrics::gauge!("four", "Method" => "Default").set(1.0);
    metrics::gauge!("score", "Method" => "Default").set(1.0);
    metrics::gauge!("andseven", "Method" => "Another").set(1.0);
    metrics::gauge!("years", "Method" => "YetAnother").set(1.0);

    c.bench_function("flush", |b| {
        b.iter(|| metrics.set_property("RequestId", "ABC123").flush(std::io::sink()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
