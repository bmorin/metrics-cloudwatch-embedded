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

Intended for use with the [`lambda_runtime`](https://crates.io/crates/lambda_runtime), however `Collector::flush_to(...)` could be used 
for anything that writes logs that end up in CloudWatch.

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

In your Cargo.toml
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

async fn function_handler(event: LambdaEvent<()>) -> Result<Response, Error> {
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
[Collector::flush()](collector::Collector::flush), overflow will report an error via the [tracing] crate
* Dimensions set at initialization via [Builder::with_dimension(...)][builder::Builder::with_dimension]
may not overlap with metric [labels](metrics::Label)
* Only the subset of metric units in [metrics::Unit] are supported
<https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html>
* Registering different metric types with the same [metrics::Key] will fail with an error via the [tracing] crate
* The Embedded Metric Format supports a maximum of 30 dimensions per metric, attempting to register a metric with
more than 30 dimensions/labels will fail with an error via the [tracing] crate

Thanks
------
* Simon Andersson (ramn) and contributors - For the metrics_cloudwatch crate I used as a reference
* Toby Lawrence (tobz) - For answering my metrics crate questions before I even had something working

