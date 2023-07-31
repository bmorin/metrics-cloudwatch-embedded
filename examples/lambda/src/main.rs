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
        req_id: event.context.request_id,
    };

    info!("Hello from function_handler");

    metrics::increment_counter!("requests", "Method" => "Default");

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
