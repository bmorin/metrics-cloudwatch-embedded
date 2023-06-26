//! # Builder
//!
//! Builder for metrics_cloudwatch_embedded::Collector

#![allow(dead_code)]
use super::{collector, Error};
use metrics::SharedString;
use std::fmt;

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
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            cloudwatch_namespace: Default::default(),
            default_dimensions: Default::default(),
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
    pub fn with_dimension(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.default_dimensions.push((name.into(), value.into()));
        self
    }

    /// Private helper for consuming the builder into collector configuration
    fn build(self) -> Result<collector::Config, Error> {
        Ok(collector::Config {
            cloudwatch_namespace: self.cloudwatch_namespace.ok_or("cloudwatch_namespace missing")?,
            default_dimensions: self.default_dimensions,
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

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            cloudwatch_namespace,
            default_dimensions,
        } = self;
        f.debug_struct("Builder")
            .field("cloudwatch_namespace", cloudwatch_namespace)
            .field("default_dimensions", default_dimensions)
            .finish()
    }
}
