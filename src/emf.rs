//! # EMF
//!
//! Helpers for serializing CloudWatch Embedded Metrics via serde_json
//!
//! <https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html>

use serde::Serialize;
use serde_json::value::Value;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct EmbeddedMetrics<'a> {
    #[serde(rename = "_aws")]
    pub aws: EmbeddedMetricsAws<'a>,
    #[serde(flatten)]
    pub dimensions: BTreeMap<&'a str, &'a str>,
    #[serde(flatten)]
    pub properties: BTreeMap<&'a str, Value>,
    #[serde(flatten)]
    pub values: BTreeMap<&'a str, Value>,
}

#[derive(Serialize)]
pub struct EmbeddedMetricsAws<'a> {
    #[serde(rename = "Timestamp")]
    pub timestamp: u64,
    // This crate never uses more than one namespace in a metrics document
    #[serde(rename = "CloudWatchMetrics")]
    pub cloudwatch_metrics: [EmbeddedNamespace<'a>; 1],
}

#[derive(Serialize)]
pub struct EmbeddedNamespace<'a> {
    #[serde(rename = "Namespace")]
    pub namespace: &'a str,
    // This create builds a single dimension set with all dimensions
    #[serde(rename = "Dimensions")]
    pub dimensions: [Vec<&'a str>; 1],
    #[serde(rename = "Metrics")]
    pub metrics: Vec<EmbeddedMetric<'a>>,
}

#[derive(Serialize)]
pub struct EmbeddedMetric<'a> {
    #[serde(rename = "Name")]
    pub name: &'a str,
    #[serde(rename = "Unit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<&'a str>,
}

/// Convert a metrics::Unit into the cloudwatch string
///
/// <https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/API_MetricDatum.html>
pub fn unit_to_str(unit: &metrics::Unit) -> &'static str {
    match unit {
        metrics::Unit::Count => "Count",
        metrics::Unit::Percent => "Percent",
        metrics::Unit::Seconds => "Seconds",
        metrics::Unit::Milliseconds => "Milliseconds",
        metrics::Unit::Microseconds => "Microseconds",
        metrics::Unit::Nanoseconds => "Nanoseconds",
        metrics::Unit::Tebibytes => "Terabytes",
        metrics::Unit::Gigibytes => "Gigabytes",
        metrics::Unit::Mebibytes => "Megabytes",
        metrics::Unit::Kibibytes => "Kilobytes",
        metrics::Unit::Bytes => "Bytes",
        metrics::Unit::TerabitsPerSecond => "Terabits/Second",
        metrics::Unit::GigabitsPerSecond => "Gigabits/Second",
        metrics::Unit::MegabitsPerSecond => "Megabits/Second",
        metrics::Unit::KilobitsPerSecond => "Kilobits/Second",
        metrics::Unit::BitsPerSecond => "Bits/Second",
        metrics::Unit::CountPerSecond => "Count/Second",
    }
}

#[allow(unused_imports)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn embedded_metrics() {
        let mut metrics_test = EmbeddedMetrics {
            aws: EmbeddedMetricsAws {
                timestamp: 0,
                cloudwatch_metrics: [EmbeddedNamespace {
                    namespace: "GameServerMetrics",
                    dimensions: [vec!["Address", "Port"]],
                    metrics: Vec::new(),
                }],
            },
            dimensions: BTreeMap::new(),
            properties: BTreeMap::new(),
            values: BTreeMap::new(),
        };

        metrics_test.aws.timestamp = 1687394207903;

        metrics_test.dimensions.insert("Address", "10.172.207.225");
        metrics_test.dimensions.insert("Port", "7779");

        metrics_test.aws.cloudwatch_metrics[0].metrics.clear();
        metrics_test.values.clear();

        metrics_test.aws.cloudwatch_metrics[0].metrics.push(EmbeddedMetric {
            name: "FrameTime",
            unit: Some(unit_to_str(&metrics::Unit::Milliseconds)),
        });
        metrics_test.values.insert("FrameTime", json!(10.0));

        metrics_test.aws.cloudwatch_metrics[0].metrics.push(EmbeddedMetric {
            name: "CpuUsage",
            unit: Some(unit_to_str(&metrics::Unit::Percent)),
        });
        metrics_test.values.insert("CpuUsage", json!(5.5));

        metrics_test.aws.cloudwatch_metrics[0].metrics.push(EmbeddedMetric {
            name: "MemoryUsage",
            unit: Some(unit_to_str(&metrics::Unit::Kibibytes)),
        });
        metrics_test.values.insert("MemoryUsage", json!(10 * 1024));

        assert_eq!(
            serde_json::to_string(&metrics_test).unwrap(),
            r#"{"_aws":{"Timestamp":1687394207903,"CloudWatchMetrics":[{"Namespace":"GameServerMetrics","Dimensions":[["Address","Port"]],"Metrics":[{"Name":"FrameTime","Unit":"Milliseconds"},{"Name":"CpuUsage","Unit":"Percent"},{"Name":"MemoryUsage","Unit":"Kilobytes"}]}]},"Address":"10.172.207.225","Port":"7779","CpuUsage":5.5,"FrameTime":10.0,"MemoryUsage":10240}"#
        );
    }
}
