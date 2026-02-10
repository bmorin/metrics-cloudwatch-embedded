#![allow(dead_code)]
use super::{collector, Error};
use metrics::SharedString;
use std::sync::Arc;
use std::time::Duration;

/// Type alias for an auto-flush writer factory.
///
/// The factory is called each time a flush is needed to obtain a fresh writer.
/// This allows for flexible output destinations (stdout, files, buffers, etc.)
pub type AutoFlushWriterFactory = Arc<dyn Fn() -> Box<dyn std::io::Write + Send> + Send + Sync>;

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
    timestamp: Option<u64>,
    emit_zeros: bool,
    auto_flush_interval: Option<Duration>,
    auto_flush_writer: Option<AutoFlushWriterFactory>,
    #[cfg(feature = "lambda")]
    lambda_cold_start_span: Option<tracing::span::Span>,
    #[cfg(feature = "lambda")]
    lambda_cold_start: Option<&'static str>,
    #[cfg(feature = "lambda")]
    lambda_request_id: Option<&'static str>,
    #[cfg(feature = "lambda")]
    lambda_xray_trace_id: Option<&'static str>,
}

impl Builder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Builder {
            cloudwatch_namespace: Default::default(),
            default_dimensions: Default::default(),
            timestamp: None,
            emit_zeros: false,
            auto_flush_interval: None,
            auto_flush_writer: None,
            #[cfg(feature = "lambda")]
            lambda_cold_start_span: None,
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

    /// Sets the timestamp for flush to a constant value to simplify tests
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// If set to true, the collector will emit a zero value metrics instead of skipping
    /// defaults to `false`
    pub fn emit_zeros(mut self, emit_zeros: bool) -> Self {
        self.emit_zeros = emit_zeros;
        self
    }

    /// Enable auto-flush with the default interval of 30 seconds.
    ///
    /// Spawns a background tokio task that periodically flushes metrics to stdout.
    /// This is useful for long-running Lambda functions or to capture metrics
    /// before a potential timeout or crash.
    ///
    /// For a custom interval, use [`Self::with_auto_flush_interval`].
    /// For a custom output writer, use [`Self::with_auto_flush_writer`].
    ///
    /// # Example
    /// ```no_run
    /// let metrics = metrics_cloudwatch_embedded::Builder::new()
    ///     .cloudwatch_namespace("MyApplication")
    ///     .with_auto_flush()
    ///     .init()
    ///     .unwrap();
    /// ```
    pub fn with_auto_flush(self) -> Self {
        self.with_auto_flush_interval(super::DEFAULT_AUTO_FLUSH_INTERVAL)
    }

    /// Enable auto-flush with a custom interval.
    ///
    /// Spawns a background tokio task that periodically flushes metrics to stdout.
    /// This is useful for long-running Lambda functions or to capture metrics
    /// before a potential timeout or crash.
    ///
    /// For the default 30-second interval, use [`Self::with_auto_flush`].
    /// For a custom output writer, use [`Self::with_auto_flush_writer`].
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// let metrics = metrics_cloudwatch_embedded::Builder::new()
    ///     .cloudwatch_namespace("MyApplication")
    ///     .with_auto_flush_interval(Duration::from_secs(15))
    ///     .init()
    ///     .unwrap();
    /// ```
    pub fn with_auto_flush_interval(mut self, interval: Duration) -> Self {
        self.auto_flush_interval = Some(interval);
        self
    }

    /// Enable auto-flush with a custom interval and custom writer factory.
    ///
    /// Spawns a background tokio task that periodically flushes metrics using
    /// the provided writer factory. The factory is called each time a flush
    /// is performed, allowing flexible output destinations.
    ///
    /// This is useful for:
    /// - Using the crate outside of AWS Lambda where stdout isn't desired
    /// - Testing auto-flush behavior by capturing output to a buffer
    /// - Writing metrics to files, network streams, or other destinations
    ///
    /// For stdout output, use [`Self::with_auto_flush`] or [`Self::with_auto_flush_interval`].
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    /// use std::sync::{Arc, Mutex};
    ///
    /// // Example: Capture auto-flush output for testing
    /// let buffer = Arc::new(Mutex::new(Vec::new()));
    /// let buffer_clone = buffer.clone();
    ///
    /// let metrics = metrics_cloudwatch_embedded::Builder::new()
    ///     .cloudwatch_namespace("MyApplication")
    ///     .with_auto_flush_writer(Duration::from_secs(15), move || {
    ///         let buffer = buffer_clone.clone();
    ///         Box::new(MutexWriter(buffer)) as Box<dyn std::io::Write + Send>
    ///     })
    ///     .init()
    ///     .unwrap();
    ///
    /// // Helper wrapper to make Arc<Mutex<Vec<u8>>> implement Write
    /// struct MutexWriter(Arc<Mutex<Vec<u8>>>);
    /// impl std::io::Write for MutexWriter {
    ///     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    ///         self.0.lock().unwrap().write(buf)
    ///     }
    ///     fn flush(&mut self) -> std::io::Result<()> {
    ///         self.0.lock().unwrap().flush()
    ///     }
    /// }
    /// ```
    pub fn with_auto_flush_writer<F, W>(mut self, interval: Duration, writer_factory: F) -> Self
    where
        F: Fn() -> W + Send + Sync + 'static,
        W: std::io::Write + Send + 'static,
    {
        self.auto_flush_interval = Some(interval);
        self.auto_flush_writer = Some(Arc::new(move || {
            Box::new(writer_factory()) as Box<dyn std::io::Write + Send>
        }));
        self
    }

    /// Passes a tracing span to drop after our cold start is complete
    ///
    /// *requires the `lambda` feature flag*
    ///
    #[cfg(feature = "lambda")]
    pub fn lambda_cold_start_span(mut self, cold_start_span: tracing::span::Span) -> Self {
        self.lambda_cold_start_span = Some(cold_start_span);
        self
    }

    /// Emits a cold start metric with the given name once to mark a cold start
    ///
    /// *requires the `lambda` feature flag*
    ///
    #[cfg(feature = "lambda")]
    pub fn lambda_cold_start_metric(mut self, name: &'static str) -> Self {
        self.lambda_cold_start = Some(name);
        self
    }

    /// Decorates every metric with request_id from the lambda request context as a property
    /// with the given name
    ///
    /// *requires the `lambda` feature flag*
    ///
    #[cfg(feature = "lambda")]
    pub fn with_lambda_request_id(mut self, name: &'static str) -> Self {
        self.lambda_request_id = Some(name);
        self
    }

    /// Decorates every metric with lambda_xray_trace_id from the lambda request context as a property
    /// with the given name
    ///
    /// *requires the `lambda` feature flag*
    ///
    #[cfg(feature = "lambda")]
    pub fn with_lambda_xray_trace_id(mut self, name: &'static str) -> Self {
        self.lambda_xray_trace_id = Some(name);
        self
    }

    /// Private helper for consuming the builder into a Collector (non-lambda)
    #[cfg(not(feature = "lambda"))]
    fn build(self) -> Result<collector::Collector, Error> {
        let config = collector::Config {
            cloudwatch_namespace: self.cloudwatch_namespace.ok_or("cloudwatch_namespace missing")?,
            default_dimensions: self.default_dimensions,
            timestamp: self.timestamp,
            emit_zeros: self.emit_zeros,
        };
        Ok(collector::Collector::new(config))
    }

    /// Private helper for consuming the builder into a Collector (lambda)
    #[cfg(feature = "lambda")]
    fn build(self) -> Result<collector::Collector, Error> {
        let config = collector::Config {
            cloudwatch_namespace: self.cloudwatch_namespace.ok_or("cloudwatch_namespace missing")?,
            default_dimensions: self.default_dimensions,
            timestamp: self.timestamp,
            emit_zeros: self.emit_zeros,
            lambda_cold_start: self.lambda_cold_start,
            lambda_request_id: self.lambda_request_id,
            lambda_xray_trace_id: self.lambda_xray_trace_id,
        };
        Ok(collector::Collector::new(config, self.lambda_cold_start_span))
    }

    /// Intialize the metrics collector including the call to [metrics::set_global_recorder]
    pub fn init(self) -> Result<&'static collector::Collector, Error> {
        let auto_flush_interval = self.auto_flush_interval;
        let auto_flush_writer = self.auto_flush_writer.clone();
        let collector: &'static collector::Collector = Box::leak(Box::new(self.build()?));

        if let Some(interval) = auto_flush_interval {
            // Use provided writer factory or default to stdout
            let writer_factory = auto_flush_writer
                .unwrap_or_else(|| Arc::new(|| Box::new(std::io::stdout()) as Box<dyn std::io::Write + Send>));
            spawn_auto_flush_task(collector, interval, writer_factory);
        }

        metrics::set_global_recorder::<collector::Recorder>(collector.into()).map_err(|e| e.to_string())?;
        Ok(collector)
    }
}

/// Spawns a background tokio task that periodically flushes metrics.
///
/// The task uses `MissedTickBehavior::Skip` to avoid catching up on missed flushes.
/// One example is when a Lambda execution context is frozen and later resumed.
///
/// # Arguments
/// * `collector` - The metrics collector to flush
/// * `interval` - The interval between flushes
/// * `writer_factory` - A factory function that produces a writer for each flush
fn spawn_auto_flush_task(
    collector: &'static collector::Collector,
    interval: Duration,
    writer_factory: AutoFlushWriterFactory,
) {
    tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(interval);
        // Skip missed ticks when Lambda is frozen - don't try to "catch up"
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval_timer.tick().await;
            let writer = writer_factory();
            if let Err(e) = collector.flush(writer) {
                tracing::error!("Auto-flush failed: {e}");
            }
        }
    });
}
