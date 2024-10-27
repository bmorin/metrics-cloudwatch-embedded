use lambda_http::{Body, Error, Request, Response};
use metrics_cloudwatch_embedded::lambda::handler::run_http;
use serde::Deserialize;
use tracing::{info, info_span};

#[derive(Deserialize)]
struct Payload {}

async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    info!("Hello from function_handler");

    metrics::counter!("requests", "Method" => "Default").increment(1);

    let resp = Response::builder().status(200).body("".into()).map_err(Box::new)?;
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
        .lambda_cold_start_span(info_span!("cold start"))
        .lambda_cold_start_metric("ColdStart")
        .with_lambda_request_id("RequestId")
        .init()
        .unwrap();

    info!("Hello from main");

    run_http(metrics, function_handler).await
}
