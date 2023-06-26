//! Backend for the [metrics] crate to emit metrics in the CloudWatch Embedded Metrics Format
//! (<https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html>)
//!
//! Counters, Gauges and Histograms are supported.  
//!
//! __The interface is not stable__
//!
//! __This is a Minimum Viable Product for feedback, experimentation and iteration__
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
//!      .flush();
//! ```
//!
//! # Implementation Details
//!
//! Intended for use with the AWS lambda_runtime, however [Collector::flush_to(...)](collector::Collector::flush_to)
//! could be used for anything that writes logs that end up in CloudWatch.
//!
//! * Counters are Guages are implented as [AtomicU64](std::sync::atomic::AtomicU64) via the
//! [CounterFn](metrics::CounterFn) and [GaugeFn](metrics::GaugeFn) implementations in the [metrics crate](metrics)
//! * Histograms are implemented as [mpsc::SyncSender](std::sync::mpsc::SyncSender)
//! * [serde_json] is used to serialize metric documents to simplify maintainence and for consistancy with other
//! crates in the ecosystem
//! * Registering and flushing of metrics uses state within a [Mutex](std::sync::Mutex), recording previously
//! registered metrics should not block on this [Mutex](std::sync::Mutex)
//! * Metric names are mapped to [metrics::Unit] reguardless of their type and [labels](metrics::Label)
//! * Due to the design of [metrics::Key::name()](metrics::Key::name) the orginal [metrics::SharedString] is
//! inaccessible when indexing, resuling in some memory inefficiency
//! * Metric descriptions are unused
//!
//! # Limitations
//! * Histograms retain up to 100 values (the maximum for a single metric document) between calls to
//! [Collector::flush()](collector::Collector::flush), overflow is silently dropped
//! * Dimensions set at initialization via [Builder::with_dimension(...)][builder::Builder::with_dimension]
//! may not overlap with metric [labels](metrics::Label)
//! * Only the subset of metric units in [metrics::Unit] are supported
//! <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html>
//! * Registering different metric types with the same [metrics::Key] will silently fail
//!

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub use {builder::Builder, collector::Collector};

mod builder;
mod collector;
mod emf;
#[cfg(test)]
mod test;
