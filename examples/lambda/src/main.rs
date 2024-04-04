use lambda_runtime::{service_fn, Error, LambdaEvent, Runtime};
use metrics_cloudwatch_embedded::lambda::MetricsLayer;
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

    Runtime::new(service_fn(function_handler))
        .layer(MetricsLayer::new(metrics))
        .run()
        .await?;

    Ok(())
}
