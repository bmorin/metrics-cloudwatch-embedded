#![allow(non_snake_case)]
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {}

#[derive(Serialize)]
struct Response {
    req_id: String,
}

async fn function_handler(
    metrics: &metrics_cloudwatch_embedded::Collector,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    let resp = Response {
        req_id: event.context.request_id.clone(),
    };

    metrics::increment_counter!("requests", "Method" => "Default");

    metrics
        .set_property("RequestId", event.context.request_id)
        .flush()?;
    Ok(resp)
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
        .cloudwatch_namespace("MetricsTest")
        .with_dimension(
            "Function",
            std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap(),
        )
        .init()
        .unwrap();

    run(service_fn(|event| function_handler(metrics, event))).await
}
