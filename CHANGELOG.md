## v0.4.0 (2022-07-02)

* added Builder::lambda_cold_start_span() for tracking cold starts in traces
* added Collector::write_single() for writing a single metric
* Builder::lambda_cold_start_metric() now calls into Collector::flush_single() under the hood
* reduced memory allocations Collector::flush() by replacing a couple single element vectors with arrays
* removed Collector::flush_to, Collector::flush inputs std::io::Write 
* replaced Collector::flush_to_with_timestamp with Builder::with_timestamp

## v0.3.1 (2022-06-29)

* fixed lambda::handler::run_http and lambda::service::run_http

## v0.3.0 (2022-06-29)

* First draft of the lambda feature
    * added MetricsService
    * added Builder::lambda_cold_start_metric()
    * added Builder::with_lambda_request_id()
    * added Builder::with_lambda_xray_trace_id()
* Added a check for more than 30 dimensions/labels

## v0.2.0 (2022-06-26)

* Fixed repository link
* Added a dependency on tracing so we can emit errors when failing to register a metric or overflowing a histogram

## v0.1.0 (2023-06-25)

Initial release