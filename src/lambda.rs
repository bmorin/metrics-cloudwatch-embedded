#![allow(dead_code)]
use super::collector::Collector;
use lambda_runtime::LambdaEvent;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct MetricsService<S> {
    metrics: &'static Collector,
    inner: S,
}

impl<S> MetricsService<S> {
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

pub async fn run_handler<T, F, Request, Response>(
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

pub async fn run_http<'a, R, S, E>(metrics: &'static Collector, inner: S) -> Result<(), lambda_runtime::Error>
where
    S: tower::Service<lambda_http::Request, Response = R, Error = E>,
    S::Future: Send + 'a,
    R: lambda_http::IntoResponse,
    E: std::fmt::Debug + std::fmt::Display,
{
    run(metrics, lambda_http::Adapter::from(inner)).await
}

pub async fn run_http_handler<'a, R, T, F, E>(
    metrics: &'static Collector,
    handler: T,
) -> Result<(), lambda_runtime::Error>
where
    T: FnMut(lambda_http::Request) -> F,
    F: Future<Output = Result<R, lambda_runtime::Error>> + Send + 'a,
    R: lambda_http::IntoResponse,
    E: std::fmt::Debug + std::fmt::Display,
{
    run(metrics, lambda_http::Adapter::from(lambda_http::service_fn(handler))).await
}
