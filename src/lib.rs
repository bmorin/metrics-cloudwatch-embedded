//! Backend for the [metrics] crate to emit metrics in the CloudWatch Embedded Metrics Format
//! (<https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html>)
//!
//! Counters, Gauges and Histograms are supported.  
//!
//! __The interface is not stable__
//!
//! # Example
//! ```
//! let metrics = metrics_cloudwatch_embedded::Builder::new()
//!      .cloudwatch_namespace("MyApplication")
//!      .init()
//!      .unwrap();
//!
//!  metrics::increment_counter!("requests", "Method" => "Default");
//!
//!  metrics
//!      .set_property("RequestId", "ABC123")
//!      .flush(std::io::stdout());
//! ```
//!
//! # Implementation Details
//!
//! Intended for use with the [lambda_runtime], however [Collector::flush(...)](collector::Collector::flush)
//! could be used for anything that writes logs that end up in CloudWatch.
//!
//! * Counters are Guages are implented as [AtomicU64](std::sync::atomic::AtomicU64) via the
//! [CounterFn](metrics::CounterFn) and [GaugeFn](metrics::GaugeFn) implementations in the [metrics crate](metrics)
//! * Histograms are implemented as [mpsc::SyncSender](std::sync::mpsc::SyncSender)
//! * [serde_json] is used to serialize metric documents to simplify maintainence and for consistancy with other
//! crates in the ecosystem
//! * Registering and flushing of metrics uses state within a [Mutex](std::sync::Mutex), recording previously
//! registered metrics should not block on this [Mutex](std::sync::Mutex)
//! * Metric names are mapped to [metrics::Unit] regardless of their type and [labels](metrics::Label)
//! * Metric descriptions are unused
//!
//! # Limitations
//! * Histograms retain up to 100 values (the maximum for a single metric document) between calls to
//! [Collector::flush()](collector::Collector::flush), overflow will report an error via the [tracing] crate
//! * Dimensions set at initialization via [Builder::with_dimension(...)][builder::Builder::with_dimension]
//! may not overlap with metric [labels](metrics::Label)
//! * Only the subset of metric units in [metrics::Unit] are supported
//! <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html>
//! * Registering different metric types with the same [metrics::Key] will fail with an error via the [tracing] crate
//! * The Embedded Metric Format supports a maximum of 30 dimensions per metric, attempting to register a metric with
//! more than 30 dimensions/labels will fail with an error via the [tracing] crate
//!

pub use {builder::Builder, collector::Collector};

#[doc(hidden)]
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

mod builder;
mod collector;
mod emf;
#[cfg(feature = "lambda")]
pub mod lambda;
#[cfg(test)]
mod test;
