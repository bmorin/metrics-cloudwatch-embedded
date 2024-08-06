//! Additional functionality for integration with [lambda_runtime] and [lambda_http]
//!
//! Inspired by Lambda Power Tools
//!
//! *this module requires the `lambda` feature flag*
//!
//! # Simple Example
//!
//! ```no_run
//! use lambda_runtime::{Error, LambdaEvent};
//! // This replaces lambda_runtime::run and lambda_runtime::service_fn
//! use metrics_cloudwatch_embedded::lambda::handler::run;
//! use serde::{Deserialize, Serialize};
//! use tracing::{info, info_span};
//!
//! #[derive(Deserialize)]
//! struct Request {}
//!
//! #[derive(Serialize)]
//! struct Response {}
//!
//! async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
//!
//!     // Do something important
//!
//!     info!("Hello from function_handler");
//!
//!     metrics::counter!("requests", "Method" => "Default").increment(1);
//!
//!     Ok(Response {})
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     tracing_subscriber::fmt()
//!         .json()
//!         .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
//!         .with_target(false)
//!         .with_current_span(false)
//!         .without_time()
//!         .init();
//!
//!     let metrics = metrics_cloudwatch_embedded::Builder::new()
//!         .cloudwatch_namespace("MetricsExample")
//!         .with_dimension("function", std::env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap())
//!         .lambda_cold_start_span(info_span!("cold start").entered())
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
//! INIT_START Runtime Version: provided:al2.v19    Runtime Version ARN: arn:aws:lambda:us-west-2::runtime:d1007133cb0d993d9a42f9fc10442cede0efec65d732c7943b51ebb979b8f3f8
//! {"level":"INFO","fields":{"message":"Hello from main"},"spans":[{"name":"cold start"}]}
//! START RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051 Version: $LATEST
//! {"_aws":{"Timestamp":1688294472338,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function"]],"Metrics":[{"Name":"ColdStart","Unit":"Count"}]}]},"Function":"MetricsTest","RequestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","ColdStart":1}
//! {"level":"INFO","fields":{"message":"Hello from function_handler"},"spans":[{"name":"cold start"},{"requestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","xrayTraceId":"Root=1-64a15448-4aa914a00d66aa066325d7e3;Parent=60a7d0c22fb2f001;Sampled=0;Lineage=16f3a795:0","name":"Lambda runtime invoke"}]}
//! {"_aws":{"Timestamp":1688294472338,"CloudWatchMetrics":[{"Namespace":"MetricsTest","Dimensions":[["Function","Method"]],"Metrics":[{"Name":"requests"}]}]},"Function":"MetricsTest","Method":"Default","RequestId":"fce53486-160d-41e8-b8c3-8ef0fd0f4051","requests":1}
//! END RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051
//! REPORT RequestId: fce53486-160d-41e8-b8c3-8ef0fd0f4051 Duration: 1.22 ms Billed Duration: 11 ms Memory Size: 128 MB Max Memory Used: 13 MB Init Duration: 8.99 ms
//! ```
//!
//! # Advanced Usage
//!
//! If you're building a more sophisticated [tower] stack, use [MetricsService] instead
//!

#![allow(dead_code)]
use super::collector::Collector;
use lambda_runtime::{LambdaEvent, LambdaInvocation};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Layer;

/// [tower::Layer] for automatically [flushing](super::Collector::flush()) after each request and enabling
/// `lambda` features in [Builder](super::Builder)
///
/// For composing your own [tower] stacks to input into the Rust Lambda Runtime
pub struct MetricsLayer {
    pub(crate) collector: &'static Collector,
}

impl MetricsLayer {
    pub fn new(collector: &'static Collector) -> Self {
        Self { collector }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService {
            metrics: self.collector,
            inner,
        }
    }
}

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

impl<S> tower::Service<LambdaInvocation> for MetricsService<S>
where
    S: tower::Service<LambdaInvocation>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MetricsServiceFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: LambdaInvocation) -> Self::Future {
        if let Some(prop_name) = self.metrics.config.lambda_request_id {
            self.metrics.set_property(prop_name, req.context.request_id.clone());
        }
        if let Some(prop_name) = self.metrics.config.lambda_xray_trace_id {
            self.metrics.set_property(prop_name, req.context.xray_trace_id.clone());
        }

        if let Some(counter_name) = self.metrics.config.lambda_cold_start {
            static COLD_START_BEGIN: std::sync::Once = std::sync::Once::new();
            COLD_START_BEGIN.call_once(|| {
                self.metrics
                    .write_single(counter_name, Some(metrics::Unit::Count), 1, std::io::stdout())
                    .expect("failed to flush cold start metric");
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
            this.metrics.flush(std::io::stdout()).expect("failed to flush metrics");

            static COLD_START_END: std::sync::Once = std::sync::Once::new();
            COLD_START_END.call_once(|| {
                this.metrics.end_cold_start();
            });

            return Poll::Ready(result);
        }

        Poll::Pending
    }
}

/// Helpers for starting the Lambda Rust runtime with a [tower::Service] with a
/// [TracingLayer] and a [MetricsLayer]
///
/// Reduces the amount of ceremony needed in `main()` for simple use cases
///
pub mod service {

    use core::fmt::Debug;

    use futures::Stream;
    use lambda_runtime::{layers::TracingLayer, Diagnostic, IntoFunctionResponse};
    use serde::{Deserialize, Serialize};
    use tower::Service;

    use super::*;

    /// Start the Lambda Rust runtime with a given [`tower::Service<LambdaEvent<Request>>`]
    /// which is then layered with [TracingLayer] and [MetricsLayer] with a given [Collector]
    pub async fn run<A, F, R, B, S, D, E>(metrics: &'static Collector, handler: F) -> Result<(), lambda_runtime::Error>
    where
        F: Service<LambdaEvent<A>, Response = R>,
        F::Future: Future<Output = Result<R, F::Error>>,
        F::Error: Into<Diagnostic> + std::fmt::Debug,
        A: for<'de> Deserialize<'de>,
        R: IntoFunctionResponse<B, S>,
        B: Serialize,
        S: Stream<Item = Result<D, E>> + Unpin + Send + 'static,
        D: Into<bytes::Bytes> + Send,
        E: Into<lambda_runtime::Error> + Send + Debug,
    {
        let runtime = lambda_runtime::Runtime::new(handler)
            .layer(TracingLayer::new())
            .layer(MetricsLayer::new(metrics));
        runtime.run().await
    }

    /// Start the Lambda Rust runtime with a given [tower::Service<lambda_http::Request>]
    /// which is then layered with [TracingLayer] and [MetricsLayer] with a given [Collector]
    pub async fn run_http<'a, R, S, E>(metrics: &'static Collector, handler: S) -> Result<(), lambda_runtime::Error>
    where
        S: Service<lambda_http::Request, Response = R, Error = E>,
        S::Future: Send + 'a,
        R: lambda_http::IntoResponse,
        E: std::fmt::Debug + Into<Diagnostic>,
    {
        run(metrics, lambda_http::Adapter::from(handler)).await
    }
}

/// Helpers for starting the Lambda Rust runtime with a handler function and
/// a [lambda_runtime::layers::TracingLayer] and a [MetricsLayer]
///
/// Reduces the amount of ceremony needed in `main()` for simple use cases
///
pub mod handler {

    use tower::service_fn;

    use super::*;

    /// Start the Lambda Rust runtime with a given [LambdaEvent] handler function
    /// which is then layered with [lambda_runtime::layers::TracingLayer] and [MetricsLayer] with a given [Collector]
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
        super::service::run(metrics, lambda_runtime::service_fn(handler)).await
    }

    /// Start the Lambda Rust runtime with a given [lambda_http::Request] handler function
    /// which is then layered with [lambda_runtime::layers::TracingLayer] and [MetricsLayer] with a given [Collector]
    pub async fn run_http<'a, T, F, Response>(
        metrics: &'static Collector,
        handler: T,
    ) -> Result<(), lambda_runtime::Error>
    where
        T: FnMut(lambda_http::Request) -> F,
        F: Future<Output = Result<Response, lambda_runtime::Error>> + Send + 'a,
        Response: lambda_http::IntoResponse,
    {
        super::service::run(metrics, lambda_http::Adapter::from(service_fn(handler))).await
    }
}
