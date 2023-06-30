metrics_cloudwatch_embedded
===========================
[![Crates.io version shield](https://img.shields.io/crates/v/metrics_cloudwatch_embedded.svg)](https://crates.io/crates/metrics_cloudwatch_embedded)
[![Crates.io license shield](https://img.shields.io/crates/l/metrics_cloudwatch_embedded.svg)](https://crates.io/crates/metrics_cloudwatch_embedded)

__The interface is not stable__

__This is a Minimum Viable Product for feedback, experimentation and iteration__


Purpose
-------

Provide a backend for the [`metrics` facade crate](https://crates.io/crates/metrics), 
to emit metrics in [CloudWatch Embedded Metrics Format](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html)

Simple Example
--------------

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

AWS Lambda Example
------------------
The Lambda Runtime intergration feature handles flushing metrics after each invoke via either `run()` 
alternatives or `MetricService` which inplements the [`tower::Service`](https://crates.io/crates/tower) trait.
It also provides optional helpers for emiting a metric on cold starts and decorating metric documents with 
request id and/or x-ray trace id.


In your Cargo.toml add:
```toml
metrics_cloudwatch_embedded = {  version = "0.3", features = ["lambda"] }
```

```rust
use lambda_runtime::{Error, LambdaEvent};
use metrics_cloudwatch_embedded::lambda::handler::run;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {}

#[derive(Serialize)]
struct Response {
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    metrics::increment_counter!("requests", "Method" => "Default");

    Ok( Response {})
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .compact()
        .init();

    let metrics = metrics_cloudwatch_embedded::Builder::new()
        .cloudwatch_namespace("MetricsExample")
        .with_dimension("Function", std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap())
        .lambda_cold_start_metric("ColdStart")
        .with_lambda_request_id("RequestId")
        .init()
        .unwrap();

    run(metrics, function_handler).await
}

```

Limitations
-----------
* Histograms retain up to 100 values (the maximum for a single metric document) between calls to
`collector::Collector::flush`, overflow will report an error via the `tracing` crate
* Dimensions set at initialization via `Builder::with_dimension(...)`
may not overlap with metric `labels`
* Only the subset of metric units in `metrics::Unit` are supported
<https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html>
* Registering different metric types with the same `metrics::Key` will fail with an error via the `tracing` crate
* The Embedded Metric Format supports a maximum of 30 dimensions per metric, attempting to register a metric with
more than 30 dimensions/labels will fail with an error via the `tracing` crate

Thanks
------
* Simon Andersson (ramn) and contributors - For the metrics_cloudwatch crate I used as a reference
* Toby Lawrence (tobz) - For answering my metrics crate questions before I even had something working

