metrics_cloudwatch_embedded
===========================
[![Crates.io version shield](https://img.shields.io/crates/v/metrics_cloudwatch_embedded.svg)](https://crates.io/crates/metrics_cloudwatch_embedded)
[![Crates.io license shield](https://img.shields.io/crates/l/metrics_cloudwatch_embedded.svg)](https://crates.io/crates/metrics_cloudwatch_embedded)

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

metrics::counter!("requests", "Method" => "Default").increment(1);

metrics
    .set_property("RequestId", "ABC123")
    .flush(std::io::stdout());
```

AWS Lambda Example
------------------
The [Lambda Runtime](https://crates.io/crates/lambda-runtime) intergration feature handles flushing metrics 
after each invoke via either `run()` alternatives or `MetricService` which implements the 
[`tower::Service`](https://crates.io/crates/tower) trait.  

It also provides optional helpers for:
* emiting a metric on cold starts
* wrapping cold starts in a [`tracing`](https://crates.io/crates/tracing) span
* decorating metric documents with request id and/or x-ray trace id

In your Cargo.toml add:
```toml
metrics = "0.23"
metrics_cloudwatch_embedded = {  version = "0.5.1", features = ["lambda"] }
tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "env-filter", "json"] }
```

main.rs:
```rust
use lambda_runtime::{Error, LambdaEvent};
use metrics_cloudwatch_embedded::lambda::handler::run;
use serde::{Deserialize, Serialize};
use tracing::{info, info_span};

#[derive(Deserialize)]
struct Request {}

#[derive(Serialize)]
struct Response {
    req_id: String,
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let resp = Response {
        req_id: event.context.request_id.clone(),
    };

    info!("Hello from function_handler");

    metrics::counter!("requests", "Method" => "Default").increment(1);

    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .with_target(false)
        .with_current_span(false)
        .without_time()
        .init();

    let metrics = metrics_cloudwatch_embedded::Builder::new()
        .cloudwatch_namespace("MetricsTest")
        .with_dimension("function", std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap())
        .lambda_cold_start_span(info_span!("cold start").entered())
        .lambda_cold_start_metric("ColdStart")
        .with_lambda_request_id("RequestId")
        .init()
        .unwrap();

    info!("Hello from main");

    run(metrics, function_handler).await
}
```
CloudWatch log after a single invoke (cold start):
```plaintext
INIT_START Runtime Version: provided:al2.v19	Runtime Version ARN: arn:aws:lambda:us-west-2::runtime:d1007133cb0d993d9a42f9fc10442cede0efec65d732c7943b51ebb979b8f3f8
{"level":"INFO","fields":{"message":"Hello from main"},"spans":[{"name":"cold start"}]}
START RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051 Version: $LATEST
{"_aws":{"Timestamp":1688294472338,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function"]],"Metrics":[{"Name":"ColdStart","Unit":"Count"}]}]},"Function":"MetricsTest","RequestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","ColdStart":1}
{"level":"INFO","fields":{"message":"Hello from function_handler"},"spans":[{"name":"cold start"},{"requestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","xrayTraceId":"Root=1-64a15448-4aa914a00d66aa066325d7e3;Parent=60a7d0c22fb2f001;Sampled=0;Lineage=16f3a795:0","name":"Lambda runtime invoke"}]}
{"_aws":{"Timestamp":1688294472338,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function","Method"]],"Metrics":[{"Name":"requests"}]}]},"Function":"MetricsTest","Method":"Default","RequestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","requests":1}
END RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051
REPORT RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051 Duration: 1.22 ms Billed Duration: 11 ms Memory Size: 128 MB Max Memory Used: 13 MB Init Duration: 8.99 ms
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

Supported Rust Versions (MSRV)
------------------------------

This crate requires a minimum of Rust 1.65, and is not guaranteed to build on compiler versions earlier than that.

This may change when async traits are released to stable depending on the ripple effects to the ecosystem.

License
-------

This project is licensed under the Apache-2.0 License.  Apache-2.0 was chosen to match the [Lambda Runtime](https://crates.io/crates/lambda-runtime)

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.

Feedback
--------

Your feedback is important, if you evalute or use this crate please leave a post in our 
[Github Feedback Discussion](https://github.com/BMorinDrifter/metrics-cloudwatch-embedded/discussions/categories/feeback)

Thanks
------
* Simon Andersson (ramn) and contributors - For the metrics_cloudwatch crate I used as a reference
* Toby Lawrence (tobz) - For answering my metrics crate questions before I even had something working
