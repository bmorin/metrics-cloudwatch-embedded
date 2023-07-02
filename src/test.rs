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
            .with_timestamp(1687657545423)
            .init()
            .unwrap();

        metrics::describe_counter!("success", metrics::Unit::Count, "");
        metrics::describe_histogram!("runtime", metrics::Unit::Milliseconds, "");

        metrics::increment_counter!("success", "module" => "directory", "api" => "a_function");
        metrics::increment_counter!("not_found", "module" => "directory", "api" => "a_function");
        metrics::describe_counter!("not_found", metrics::Unit::Count, "");
        metrics::increment_counter!("success", "module" => "directory", "api" => "b_function");
        metrics::increment_counter!("success", "module" => "directory", "api" => "a_function");
        metrics::gauge!("thing", 3.14, "module" => "directory", "api" => "a_function");
        metrics::histogram!("runtime", 4.0, "module" => "directory", "api" => "a_function");
        metrics::gauge!("thing", 7.11, "module" => "directory", "api" => "a_function");
        metrics::histogram!("runtime", 5.0, "module" => "directory", "api" => "a_function");

        let mut output = Vec::new();
        metrics.flush(&mut output).unwrap();
        let output_str = std::str::from_utf8(&output).unwrap();
        assert_eq!(
            output_str,
            r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":4614253070214989087}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":1}
"#
        );
    }
}
