#![allow(dead_code)]
use super::{collector, Error};
use metrics::SharedString;

/// Builder for the Embedded Cloudwatch Metrics Collector
///
/// # Example
/// ```
///  let metrics = metrics_cloudwatch_embedded::Builder::new()
///      .cloudwatch_namespace("MyApplication")
///      .init()
///      .unwrap();
/// ```
pub struct Builder {
    cloudwatch_namespace: Option<SharedString>,
    default_dimensions: Vec<(SharedString, SharedString)>,
    #[cfg(feature = "lambda")]
    lambda_cold_start: Option<&'static str>,
    #[cfg(feature = "lambda")]
    lambda_request_id: Option<&'static str>,
    #[cfg(feature = "lambda")]
    lambda_xray_trace_id: Option<&'static str>,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            cloudwatch_namespace: Default::default(),
            default_dimensions: Default::default(),
            #[cfg(feature = "lambda")]
            lambda_cold_start: None,
            #[cfg(feature = "lambda")]
            lambda_request_id: None,
            #[cfg(feature = "lambda")]
            lambda_xray_trace_id: None,
        }
    }

    /// Sets the CloudWatch namespace for all metrics
    /// * Must be set or init() will return Err("cloudwatch_namespace missing")
    pub fn cloudwatch_namespace(self, namespace: impl Into<SharedString>) -> Self {
        Self {
            cloudwatch_namespace: Some(namespace.into()),
            ..self
        }
    }

    /// Adds a static dimension (name, value), that will be sent with each MetricDatum.
    /// * This method can be called multiple times with distinct names
    /// * Dimention names may not overlap with metrics::Label names
    /// * Metrics can have no more than 30 dimensions + labels
    pub fn with_dimension(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.default_dimensions.push((name.into(), value.into()));
        self
    }

    /// Emits a cold start metric with the given name once to mark a cold start
    ///
    /// This is to mimic the behavior of lambda power tools
    #[cfg(feature = "lambda")]
    pub fn lambda_cold_start_metric(mut self, name: &'static str) -> Self {
        self.lambda_cold_start = Some(name);
        self
    }

    /// Decorates every metric with request_id from the lambda request context as a property
    /// with the given name
    ///
    #[cfg(feature = "lambda")]
    pub fn with_lambda_request_id(mut self, name: &'static str) -> Self {
        self.lambda_request_id = Some(name);
        self
    }

    /// Decorates every metric with lambda_xray_trace_id from the lambda request context as a property
    /// with the given name
    ///
    #[cfg(feature = "lambda")]
    pub fn with_lambda_xray_trace_id(mut self, name: &'static str) -> Self {
        self.lambda_xray_trace_id = Some(name);
        self
    }

    /// Private helper for consuming the builder into collector configuration
    fn build(self) -> Result<collector::Config, Error> {
        Ok(collector::Config {
            cloudwatch_namespace: self.cloudwatch_namespace.ok_or("cloudwatch_namespace missing")?,
            default_dimensions: self.default_dimensions,
            #[cfg(feature = "lambda")]
            lambda_cold_start: self.lambda_cold_start,
            #[cfg(feature = "lambda")]
            lambda_request_id: self.lambda_request_id,
            #[cfg(feature = "lambda")]
            lambda_xray_trace_id: self.lambda_xray_trace_id,
        })
    }

    /// Intialize the metrics collector including the call to metrics::set_recorder
    pub fn init(self) -> Result<&'static collector::Collector, Error> {
        let config = self.build()?;
        let collector = Box::leak(Box::new(collector::Collector::new(config)));
        metrics::set_recorder(collector)?;
        Ok(collector)
    }
}
