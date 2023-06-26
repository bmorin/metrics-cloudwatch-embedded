//! # Collector
//!
//! Metrics Collector + Emitter returned from metrics_cloudwatch_embedded::Builder

#![allow(dead_code)]
use super::emf;
use metrics::SharedString;
use serde_json::value::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// The Embedded Metric Format supports a maximum of 100 values per key
const MAX_HISTOGRAM_VALUES: usize = 100;

/// Configuration via Builder
pub struct Config {
    pub cloudwatch_namespace: SharedString,
    pub default_dimensions: Vec<(SharedString, SharedString)>,
}

/// Histogram Handler implemented as mpsc::SyncSender<f64>
struct HistogramHandle {
    sender: mpsc::SyncSender<f64>,
}

impl metrics::HistogramFn for HistogramHandle {
    // Sends the metric value to our sync_channel
    // silently fails if we already have MAX_HISTOGRAM_VALUES values
    fn record(&self, value: f64) {
        self.sender.send(value).ok();
    }
}

// Metric information stored in an index
enum MetricInfo {
    Counter(CounterInfo),
    Gauge(GaugeInfo),
    Histogram(HistogramInfo),
}

struct CounterInfo {
    value: Arc<AtomicU64>,
}

struct GaugeInfo {
    value: Arc<AtomicU64>,
}

struct HistogramInfo {
    sender: mpsc::SyncSender<f64>,
    receiver: mpsc::Receiver<f64>,
}

/// Collector state used to register new metrics and flush
/// This lives within a mutex
struct CollectorState {
    /// Tree of labels to name to metric details
    /// We use metrics::Key for the inner map because metrics::Key::name() returns a &str
    /// otherwise we could use SharedString to save a little memory
    info_tree: BTreeMap<Vec<metrics::Label>, BTreeMap<metrics::Key, MetricInfo>>,
    /// Store units seperate because describe_xxx isn't scoped to labels
    /// Key is a String because metrics::Key::name() returns a &str
    /// otherwise we could use SharedString to save a little memory
    units: HashMap<String, metrics::Unit>,
    /// Properties to be written with metrics
    properties: BTreeMap<SharedString, Value>,
}

/// Embedded CloudWatch Metrics Collector + Emitter
///
/// Use [Builder](super::Builder) to construct
///
/// # Example
/// ```
/// let metrics = metrics_cloudwatch_embedded::Builder::new()
///      .cloudwatch_namespace("MyApplication")
///      .init()
///      .unwrap();
///
///  metrics::increment_counter!("requests", "Method" => "Default");
///
///  metrics
///      .set_property("RequestId", "ABC123")
///      .flush();
/// ```
pub struct Collector {
    state: Mutex<CollectorState>,
    config: Config,
}

impl Collector {
    pub fn new(config: Config) -> Self {
        Self {
            state: Mutex::new(CollectorState {
                info_tree: BTreeMap::new(),
                units: HashMap::new(),
                properties: BTreeMap::new(),
            }),
            config,
        }
    }

    /// Set a property to emit with the metrics
    /// * Properites persist accross flush calls
    /// * Setting a property with same name multiple times will overwrite the previous value
    /// * value types other than serde_json::Value::Number and serde_json::Value::String may not work
    pub fn set_property<'a>(&'a self, name: impl Into<SharedString>, value: impl Into<Value>) -> &'a Self {
        {
            let mut state = self.state.lock().unwrap();
            state.properties.insert(name.into(), value.into());
        }
        self
    }

    /// Removes a property to emit with the metrics
    pub fn remove_property<'a>(&'a self, name: impl Into<&'a str>) -> &'a Self {
        {
            let mut state = self.state.lock().unwrap();
            state.properties.remove(name.into());
        }
        self
    }

    /// Flush the current counter values to stdout
    pub fn flush(&self) -> std::io::Result<()> {
        self.flush_to(std::io::stdout())
    }

    /// Flush the current counter values to an implementation of std::io::Write
    pub fn flush_to(&self, writer: impl std::io::Write) -> std::io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;
        self.flush_to_with_timestamp(timestamp, writer)
    }

    /// Flush the current counter values with the given timestamp to simplify unit testing
    pub fn flush_to_with_timestamp(&self, timestamp: u64, mut writer: impl std::io::Write) -> std::io::Result<()> {
        // CONSIDER: we may be able to save some allocations moving this into self.state
        // or perhaps doing a swap for default dimensions and properties???
        let mut emf = emf::EmbeddedMetrics {
            aws: emf::EmbeddedMetricsAws {
                timestamp,
                cloudwatch_metrics: Vec::new(),
            },
            dimensions: BTreeMap::new(),
            properties: BTreeMap::new(),
            values: BTreeMap::new(),
        };

        emf.aws.cloudwatch_metrics.push(emf::EmbeddedNamespace {
            namespace: &self.config.cloudwatch_namespace,
            dimensions: vec![Vec::new()],
            metrics: Vec::new(),
        });

        for dimension in &self.config.default_dimensions {
            emf.aws.cloudwatch_metrics[0].dimensions[0].push(&dimension.0);
            emf.dimensions.insert(&dimension.0, &dimension.1);
        }

        // Delay aquiring the mutex until we need it
        let state = self.state.lock().unwrap();

        for (key, value) in &state.properties {
            emf.properties.insert(key, value.clone());
        }

        // Emit an embedded metrics string for each distinct label set
        for (labels, metrics) in &state.info_tree {
            emf.aws.cloudwatch_metrics[0].metrics.clear();
            emf.values.clear();
            let mut should_flush = false;

            for label in labels {
                emf.aws.cloudwatch_metrics[0].dimensions[0].push(label.key());
                emf.dimensions.insert(label.key(), label.value());
            }

            for (key, info) in metrics {
                match info {
                    MetricInfo::Counter(counter) => {
                        let value = counter.value.swap(0, Ordering::Relaxed);

                        // Omit this metric if there is no delta since last flushed
                        if value != 0 {
                            emf.aws.cloudwatch_metrics[0].metrics.push(emf::EmbeddedMetric {
                                name: key.name(),
                                unit: state.units.get(key.name()).map(emf::unit_to_str),
                            });
                            emf.values.insert(key.name(), value.into());
                            should_flush = true;
                        }
                    }
                    MetricInfo::Gauge(gauge) => {
                        let value = f64::from_bits(gauge.value.load(Ordering::Relaxed));

                        emf.aws.cloudwatch_metrics[0].metrics.push(emf::EmbeddedMetric {
                            name: key.name(),
                            unit: state.units.get(key.name()).map(emf::unit_to_str),
                        });
                        emf.values.insert(key.name(), value.into());
                        should_flush = true;
                    }
                    MetricInfo::Histogram(histogram) => {
                        let mut values: Vec<f64> = Vec::new();
                        while let Ok(value) = histogram.receiver.try_recv() {
                            values.push(value);
                        }

                        // Omit this metric if there is no new values since last flushed
                        if !values.is_empty() {
                            emf.aws.cloudwatch_metrics[0].metrics.push(emf::EmbeddedMetric {
                                name: key.name(),
                                unit: state.units.get(key.name()).map(emf::unit_to_str),
                            });
                            emf.values.insert(key.name(), values.into());
                            should_flush = true;
                        }
                    }
                }
            }

            // Skip if we have no data to flush
            if should_flush {
                serde_json::to_writer(&mut writer, &emf)?;
                writeln!(writer)?;
            }

            // Rollback our labels/dimensions (but keep any default dimensions)
            for label in labels {
                emf.aws.cloudwatch_metrics[0].dimensions[0].pop();
                emf.dimensions.remove(&label.key());
            }
        }

        Ok(())
    }

    /// update the unit for a metric name, disregard what metric type it is
    fn update_unit(&self, key: metrics::KeyName, unit: Option<metrics::Unit>) {
        let mut state = self.state.lock().unwrap();

        if let Some(unit) = unit {
            state.units.insert(key.as_str().to_string(), unit);
        } else {
            state.units.remove(key.as_str());
        }
    }
}

impl metrics::Recorder for Collector {
    fn describe_counter(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: SharedString) {
        self.update_unit(key, unit)
    }

    fn describe_gauge(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: SharedString) {
        self.update_unit(key, unit)
    }

    fn describe_histogram(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: SharedString) {
        self.update_unit(key, unit)
    }

    #[allow(clippy::mutable_key_type)] // metrics::Key has interior mutability
    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        // Build our own copy of the labels before aquiring the mutex
        let labels: Vec<metrics::Label> = key.labels().cloned().collect();

        let mut state = self.state.lock().unwrap();

        // Does this metric already exist?
        if let Some(label_info) = state.info_tree.get_mut(&labels) {
            if let Some(info) = label_info.get(key) {
                if let MetricInfo::Counter(info) = info {
                    return metrics::Counter::from_arc(info.value.clone());
                } else {
                    // Name already registered as something other than a counter
                    return metrics::Counter::noop();
                }
            } else {
                // Label exists, counter does not
                let value = Arc::new(AtomicU64::new(0));
                label_info.insert(key.clone(), MetricInfo::Counter(CounterInfo { value: value.clone() }));

                return metrics::Counter::from_arc(value);
            }
        }

        // Neither the label nor the counter exists
        let value = Arc::new(AtomicU64::new(0));
        let mut label_info = BTreeMap::new();
        label_info.insert(key.clone(), MetricInfo::Counter(CounterInfo { value: value.clone() }));
        state.info_tree.insert(labels, label_info);

        metrics::Counter::from_arc(value)
    }

    #[allow(clippy::mutable_key_type)] // metrics::Key has interior mutability
    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        // Build our own copy of the labels before aquiring the mutex
        let labels: Vec<metrics::Label> = key.labels().cloned().collect();

        let mut state = self.state.lock().unwrap();

        // Does this metric already exist?
        if let Some(label_info) = state.info_tree.get_mut(&labels) {
            if let Some(info) = label_info.get(key) {
                if let MetricInfo::Gauge(info) = info {
                    return metrics::Gauge::from_arc(info.value.clone());
                } else {
                    // Name already registered as something other than a gauge
                    return metrics::Gauge::noop();
                }
            } else {
                // Label exists, gauge does not
                let value = Arc::new(AtomicU64::new(0));
                label_info.insert(key.clone(), MetricInfo::Counter(CounterInfo { value: value.clone() }));

                return metrics::Gauge::from_arc(value);
            }
        }

        // Neither the label nor the gauge exists
        let value = Arc::new(AtomicU64::new(0));
        let mut label_info = BTreeMap::new();
        label_info.insert(key.clone(), MetricInfo::Gauge(GaugeInfo { value: value.clone() }));
        state.info_tree.insert(labels, label_info);

        metrics::Gauge::from_arc(value)
    }

    #[allow(clippy::mutable_key_type)] // metrics::Key has interior mutability
    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        // Build our own copy of the labels before aquiring the mutex
        let labels: Vec<metrics::Label> = key.labels().cloned().collect();

        let mut state = self.state.lock().unwrap();

        // Does this metric already exist?
        if let Some(label_info) = state.info_tree.get_mut(&labels) {
            if let Some(info) = label_info.get(key) {
                if let MetricInfo::Histogram(info) = info {
                    let histogram = Arc::new(HistogramHandle {
                        sender: info.sender.clone(),
                    });
                    return metrics::Histogram::from_arc(histogram);
                } else {
                    // Name already registered as something other than a histogram
                    return metrics::Histogram::noop();
                }
            } else {
                // Label exists, histogram does not
                let (sender, receiver) = mpsc::sync_channel(MAX_HISTOGRAM_VALUES);
                let histogram = Arc::new(HistogramHandle { sender: sender.clone() });
                label_info.insert(key.clone(), MetricInfo::Histogram(HistogramInfo { sender, receiver }));

                return metrics::Histogram::from_arc(histogram);
            }
        }

        // Neither the label nor the gauge exists
        let (sender, receiver) = mpsc::sync_channel(MAX_HISTOGRAM_VALUES);
        let histogram = Arc::new(HistogramHandle { sender: sender.clone() });
        let mut label_info = BTreeMap::new();
        label_info.insert(key.clone(), MetricInfo::Histogram(HistogramInfo { sender, receiver }));
        state.info_tree.insert(labels, label_info);

        metrics::Histogram::from_arc(histogram)
    }
}
