use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Because the metrics registar is a singleton, we need to run tests in forked processes
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
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

            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::counter!("not_found", "module" => "directory", "api" => "a_function").increment(1);
            metrics::describe_counter!("not_found", metrics::Unit::Count, "");
            metrics::counter!("success", "module" => "directory", "api" => "b_function").increment(1);
            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(3.15);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(4.0);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(7.11);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(5.0);

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            let output_str = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                output_str,
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":4614275588213125939}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":1}
"#
            );
        }
    }

    rusty_fork_test! {

        #[test]
        fn no_emit_zero() {
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

            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::counter!("not_found", "module" => "directory", "api" => "a_function").increment(1);
            metrics::describe_counter!("not_found", metrics::Unit::Count, "");
            metrics::counter!("success", "module" => "directory", "api" => "b_function").increment(1);
            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(3.15);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(4.0);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(7.11);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(5.0);

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            let output_str = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                output_str,
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":4614275588213125939}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":1}
"#
            );

            // Update a single metric and confirm only that metric is emitted
            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            let output_str = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                output_str,
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","success":1}
    "#
            );
        }
    }

    rusty_fork_test! {

        #[test]
        fn emit_zero() {
            let port = format!("{}", 7779);
            let metrics = builder::Builder::new()
                .cloudwatch_namespace("namespace")
                .with_dimension("Address", "10.172.207.225")
                .with_dimension("Port", port)
                .with_timestamp(1687657545423)
                .emit_zeros(true)
                .init()
                .unwrap();

            metrics::describe_counter!("success", metrics::Unit::Count, "");
            metrics::describe_histogram!("runtime", metrics::Unit::Milliseconds, "");

            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::counter!("not_found", "module" => "directory", "api" => "a_function").increment(1);
            metrics::describe_counter!("not_found", metrics::Unit::Count, "");
            metrics::counter!("success", "module" => "directory", "api" => "b_function").increment(1);
            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(3.15);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(4.0);
            metrics::gauge!("thing", "module" => "directory", "api" => "a_function").set(7.11);
            metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(5.0);

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            let output_str = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                output_str,
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":4614275588213125939}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":1}
"#
            );

            // Update a single metric and confirm only that metric is emitted
            metrics::counter!("success", "module" => "directory", "api" => "a_function").increment(1);

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            let output_str = std::str::from_utf8(&output).unwrap();

            assert_eq!(
                output_str,
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":0,"success":1,"thing":0}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":0}
"#
            );
        }
    }
}
