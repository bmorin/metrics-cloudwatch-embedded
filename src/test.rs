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
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":7.11}
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
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":7.11}
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
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","success":1,"thing":7.11}"#.to_string() + "\n"
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
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"runtime","Unit":"Milliseconds"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":1,"runtime":[4.0,5.0],"success":2,"thing":7.11}
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
                r#"{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"not_found","Unit":"Count"},{"Name":"success","Unit":"Count"},{"Name":"thing"}]}]},"Address":"10.172.207.225","Port":"7779","api":"a_function","module":"directory","not_found":0,"success":1,"thing":7.11}
{"_aws":{"Timestamp":1687657545423,"CloudWatchMetrics":[{"Namespace":"namespace","Dimensions":[["Address","Port","module","api"]],"Metrics":[{"Name":"success","Unit":"Count"}]}]},"Address":"10.172.207.225","Port":"7779","api":"b_function","module":"directory","success":0}
"#
            );
        }
    }

    rusty_fork_test! {
        #[test]
        fn too_many_histogram_values() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
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

            for _ in 0..200 {
                metrics::histogram!("runtime", "module" => "directory", "api" => "a_function").record(4.0);
            }

            let mut output = Vec::new();
            metrics.flush(&mut output).unwrap();
            tx.send(()).unwrap();
        });

        rx.recv_timeout(Duration::from_secs(3))
            .expect("Test timed out after 3 seconds");
        }
    }

    rusty_fork_test! {
        /// Test that auto-flush can be configured and the collector initializes correctly.
        /// This test verifies the builder accepts auto-flush configuration and that
        /// metrics can still be recorded and manually flushed when auto-flush is enabled.
        #[test]
        fn auto_flush_with_manual_flush() {
            use std::time::Duration;

            // Create a tokio runtime for the auto-flush background task
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();

            rt.block_on(async {
                let metrics = builder::Builder::new()
                    .cloudwatch_namespace("auto_flush_test")
                    .with_dimension("test", "true")
                    .with_timestamp(1687657545423)
                    // Use a long interval so it doesn't fire during the test
                    .with_auto_flush_interval(Duration::from_secs(3600))
                    .init()
                    .unwrap();

                // Record some metrics
                metrics::counter!("test_counter", "label" => "value").increment(5);
                metrics::gauge!("test_gauge", "label" => "value").set(42.0);
                metrics::histogram!("test_histogram", "label" => "value").record(1.5);
                metrics::histogram!("test_histogram", "label" => "value").record(2.5);

                // Manual flush should still work
                let mut output = Vec::new();
                metrics.flush(&mut output).unwrap();
                let output_str = std::str::from_utf8(&output).unwrap();

                // Verify the output contains our metrics
                assert!(output_str.contains("test_counter"));
                assert!(output_str.contains("test_gauge"));
                assert!(output_str.contains("test_histogram"));
                assert!(output_str.contains("\"test_counter\":5"));
                assert!(output_str.contains("\"test_gauge\":42.0"));
                assert!(output_str.contains("[1.5,2.5]"));
            });
        }
    }

    rusty_fork_test! {
        /// Test that auto-flush default interval can be configured.
        #[test]
        fn auto_flush_default_interval() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();

            rt.block_on(async {
                let metrics = builder::Builder::new()
                    .cloudwatch_namespace("auto_flush_default")
                    .with_timestamp(1687657545423)
                    .with_auto_flush() // Uses DEFAULT_AUTO_FLUSH_INTERVAL (30s)
                    .init()
                    .unwrap();

                // Just verify it initializes and we can record metrics
                metrics::counter!("request_count").increment(1);

                let mut output = Vec::new();
                metrics.flush(&mut output).unwrap();
                let output_str = std::str::from_utf8(&output).unwrap();
                assert!(output_str.contains("request_count"));
            });
        }
    }

    rusty_fork_test! {
        /// Test that auto-flush with a custom writer captures output correctly.
        /// This verifies the with_auto_flush_writer functionality works as expected.
        #[test]
        fn auto_flush_custom_writer() {
            use std::sync::{Arc, Mutex};
            use std::time::Duration;

            // Helper wrapper to make Arc<Mutex<Vec<u8>>> implement Write
            struct MutexWriter(Arc<Mutex<Vec<u8>>>);
            impl std::io::Write for MutexWriter {
                fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                    self.0.lock().unwrap().write(buf)
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    self.0.lock().unwrap().flush()
                }
            }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();

            rt.block_on(async {
                // Create a shared buffer to capture auto-flush output
                let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
                let buffer_clone = buffer.clone();

                let _metrics = builder::Builder::new()
                    .cloudwatch_namespace("auto_flush_writer_test")
                    .with_timestamp(1687657545423)
                    // Use a short interval so the test completes quickly
                    .with_auto_flush_writer(Duration::from_millis(50), move || {
                        MutexWriter(buffer_clone.clone())
                    })
                    .init()
                    .unwrap();

                // Record some metrics
                metrics::counter!("auto_counter", "source" => "test").increment(42);
                metrics::gauge!("auto_gauge", "source" => "test").set(3.14);

                // Wait for auto-flush to trigger (wait longer than interval)
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Verify the buffer captured the metrics output
                let captured = buffer.lock().unwrap();
                let output_str = std::str::from_utf8(&captured).unwrap();

                assert!(output_str.contains("auto_counter"), "Expected auto_counter in output: {}", output_str);
                assert!(output_str.contains("auto_gauge"), "Expected auto_gauge in output: {}", output_str);
                assert!(output_str.contains("\"auto_counter\":42"), "Expected counter value 42 in output: {}", output_str);
                assert!(output_str.contains("\"auto_gauge\":3.14"), "Expected gauge value 3.14 in output: {}", output_str);
                assert!(output_str.contains("auto_flush_writer_test"), "Expected namespace in output: {}", output_str);
            });
        }
    }
}
