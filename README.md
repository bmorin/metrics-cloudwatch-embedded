metrics_cloudwatch_embedded
===========================

__The interface is not stable__

__This is a Minimum Viable Product for feedback, experimentation and iteration__


Purpose
-------

Provide a backend for the [`metrics` facade crate](https://crates.io/crates/metrics), 
to emit metrics in [CloudWatch Embedded Metrics Format](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html)

Intended for use with the [`lambda_runtime`](https://crates.io/crates/lambda_runtime), however `Collector::flush_to(...)` could be used 
for anything that writes logs that end up in CloudWatch.

How to use
----------

```rust
let metrics = metrics_cloudwatch_embedded::Builder::new()
    .cloudwatch_namespace("MyApplication")
    .init()
    .unwrap();

metrics::increment_counter!("requests", "Method" => "Default");

metrics
    .set_property("RequestId", "ABC123")
    .flush();
```

Limitations
-----------
* Histograms retain up to 100 values (the maximum for a single metric document) between calls to `Collector::flush()`, 
overflow is silently dropped
* Dimensions set at initialization via `Builder::with_dimension(...)` may not overlap with metric labels
* Only the subset of metric units in `metrics::Unit` are supported
[https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html]
* Registering different metric types with the same `metrics::Key` will silently fail

Thanks
------
* Simon Andersson (ramn) and contributors - For the metrics_cloudwatch crate I used as a reference
* Toby Lawrence (tobz) - For answering my metrics crate questions before I even had something working

