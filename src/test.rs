use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        let port = format!("{}", 7779);
        let metrics = builder::Builder::new()
            .cloudwatch_namespace("namespace")
            .with_dimension("Address", "10.172.207.225")
            .with_dimension("Port", port)
            .init()
            .unwrap();

        metrics::increment_counter!("success", "module" => "directory", "api" => "check_reserve");
        metrics::increment_counter!("not_found", "module" => "directory", "api" => "check_reserve");
        metrics::increment_counter!("success", "module" => "directory", "api" => "check_reserve");

        let mut output = Vec::new();
        metrics.flush_to_with_timestamp(1687657545423, &mut output).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();
        assert_eq!(
            output_str,
            r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found"},{"Name":"success"}]}]},"Address":"10.172.207.225","Port":"7779","api":"check_reserve","module":"directory","not_found":1,"success":2}
"#
        );
    }
}
