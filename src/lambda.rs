//! Additional functionality for integration with [lambda_runtime] and [lambda_http]
//!
//! Inspired by Lambda Power Tools
//!
//! *this module requires the `lambda` feature flag*
//!
//! # Simple Example
//! ```ignore
//!
//! use lambda_runtime::{Error, LambdaEvent};
//! // This replaces lambda_runtime::run and lambda_runtime::service_fn
//! use metrics_cloudwatch_embedded::lambda::handler::run;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct Request {}
//!
//! #[derive(Serialize)]
//! struct Response {}
//!
//! async fn function_handler(event: LambdaEvent<()>) -> Result<Response, Error> {
//!
//!     // Do something important
//!
//!     metrics::increment_counter!("requests", "Method" => "Default");
//!
//!     Ok(resp)
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     tracing_subscriber::fmt()
//!         .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
//!         .with_target(false)
//!         .without_time()
//!         .compact()
//!         .init();
//!
//!     let metrics = metrics_cloudwatch_embedded::Builder::new()
//!         .cloudwatch_namespace("MetricsExample")
//!         .with_dimension("Function", std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap())
//!         .lambda_cold_start_metric("ColdStart")
//!         .with_lambda_request_id("RequestId")
//!         .init()
//!         .unwrap();
//!
//!     run(metrics, function_handler).await
//! }
//! ```
//!
//! # Output
//!
//! ```plaintext
//! START RequestId: 4bd2d365-3792-46c8-9b6c-6132f9630fbb Version: $LATEST
//! {"_aws":{"Timestamp":1687947426188,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function"]],"Metrics":[{"Name":"ColdStart","Unit":"Count"}]}]},"Function":"MetricsTest","RequestId":"4bd2d365-3792-46c8-9b6c-6132f9630fbb","ColdStart":1}
//! {"_aws":{"Timestamp":1687947426188,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function","Method"]],"Metrics":[{"Name":"requests"}]}]},"Function":"MetricsTest","Method":"Default","RequestId":"4bd2d365-3792-46c8-9b6c-6132f9630fbb","requests":1}
//! END RequestId: 4bd2d365-3792-46c8-9b6c-6132f9630fbb`
//! ```
//! # Advanced Usage
//!
//! If you're building a more sophisticated [tower] stack, use [MetricsService] instead
//!

#![allow(dead_code)]
use super::collector::Collector;
use lambda_runtime::LambdaEvent;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// [tower::Service] for automatically [flushing](super::Collector::flush()) after each request and enabling
/// `lambda` features in [Builder](super::Builder)
///
/// For composing your own [tower] stacks to input into the Rust Lambda Runtime
pub struct MetricsService<S> {
    metrics: &'static Collector,
    inner: S,
}

impl<S> MetricsService<S> {
    /// Constructs a new [MetricsService] with the given [Collector] and inner [`tower::Service<LambdaEvent<Request>>`]
    /// to wrap
    pub fn new<Request, Response>(metrics: &'static Collector, inner: S) -> MetricsService<S>
    where
        S: tower::Service<LambdaEvent<Request>>,
    {
        Self { metrics, inner }
    }
}

impl<S, Request> tower::Service<LambdaEvent<Request>> for MetricsService<S>
where
    S: tower::Service<LambdaEvent<Request>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MetricsServiceFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LambdaEvent<Request>) -> Self::Future {
        if let Some(prop_name) = self.metrics.config.lambda_request_id {
            self.metrics.set_property(prop_name, req.context.request_id.clone());
        }
        if let Some(prop_name) = self.metrics.config.lambda_xray_trace_id {
            self.metrics.set_property(prop_name, req.context.xray_trace_id.clone());
        }

        if let Some(counter_name) = self.metrics.config.lambda_cold_start {
            static COLD_START: std::sync::Once = std::sync::Once::new();
            COLD_START.call_once(|| {
                // CONSIDER: We could just write the metrics document out instead
                metrics::describe_counter!(counter_name, metrics::Unit::Count, "");
                metrics::increment_counter!(counter_name);
                self.metrics.flush().unwrap();
            });
        }

        // Wrap the inner Future so we can flush after it's done
        MetricsServiceFuture {
            metrics: self.metrics,
            inner: self.inner.call(req),
        }
    }
}

#[pin_project]
#[doc(hidden)]
pub struct MetricsServiceFuture<F> {
    #[pin]
    metrics: &'static Collector,
    #[pin]
    inner: F,
}

impl<F, Response, Error> Future for MetricsServiceFuture<F>
where
    F: Future<Output = Result<Response, Error>>,
    Error: Into<Error>,
{
    type Output = Result<Response, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Poll::Ready(result) = this.inner.poll(cx) {
            let result = result.map_err(Into::into);

            // Flush our metrics after the inner service is finished
            this.metrics.flush().unwrap();

            return Poll::Ready(result);
        }

        Poll::Pending
    }
}

/// Helpers for starting the Lambda Rust runtime with a [tower::Service] wrapped by a [MetricsService]
///
/// Reduces the amount of ceremony needed in `main()` for simple use cases
///
pub mod service {

    use super::*;

    /// Start the Lambda Rust runtime with a given [`tower::Service<LambdaEvent<Request>>`]
    /// which is then wrapped by new [MetricsService] with a given [Collector]
    pub async fn run<S, Request, Response>(metrics: &'static Collector, inner: S) -> Result<(), lambda_runtime::Error>
    where
        S: tower::Service<LambdaEvent<Request>>,
        S::Future: std::future::Future<Output = Result<Response, S::Error>>,
        S::Error: std::fmt::Debug + std::fmt::Display,
        Request: for<'de> serde::Deserialize<'de>,
        Response: serde::Serialize,
    {
        lambda_runtime::run(MetricsService::new::<Request, Response>(metrics, inner)).await
    }

    /// Start the Lambda Rust runtime with a given [tower::Service<lambda_http::Request>]
    /// which is then wrapped by new [MetricsService] with a given [Collector]
    pub async fn run_http<'a, R, S, E>(metrics: &'static Collector, inner: S) -> Result<(), lambda_runtime::Error>
    where
        S: tower::Service<lambda_http::Request, Response = R, Error = E>,
        S::Future: Send + 'a,
        S::Error: std::fmt::Debug + std::fmt::Display,
        R: lambda_http::IntoResponse,
        E: std::fmt::Debug + std::fmt::Display,
    {
        run(metrics, lambda_http::Adapter::from(inner)).await
    }
}

/// Helpers for starting the Lambda Rust runtime with a handler function wrapped by the [MetricsService]
///
/// Reduces the amount of ceremony needed in `main()` for simple use cases
///
pub mod handler {

    use super::*;

    /// Start the Lambda Rust runtime with a given [LambdaEvent] handler function
    /// which is then wrapped by a new [MetricsService] with a given [Collector]
    pub async fn run<T, F, Request, Response>(
        metrics: &'static Collector,
        handler: T,
    ) -> Result<(), lambda_runtime::Error>
    where
        T: FnMut(LambdaEvent<Request>) -> F,
        F: Future<Output = Result<Response, lambda_runtime::Error>>,
        Request: for<'de> serde::Deserialize<'de>,
        Response: serde::Serialize,
    {
        lambda_runtime::run(MetricsService::new::<Request, Response>(
            metrics,
            lambda_runtime::service_fn(handler),
        ))
        .await
    }

    /// Start the Lambda Rust runtime with a given [lambda_http::Request] handler function
    /// which is then wrapped by a new [MetricsService] with a given [Collector]
    pub async fn run_http<'a, T, F, Response>(
        metrics: &'static Collector,
        handler: T,
    ) -> Result<(), lambda_runtime::Error>
    where
        T: FnMut(lambda_http::Request) -> F,
        F: Future<Output = Result<Response, lambda_runtime::Error>> + Send + 'a,
        Response: lambda_http::IntoResponse,
    {
        super::service::run(metrics, lambda_http::Adapter::from(lambda_http::service_fn(handler))).await
    }
}
