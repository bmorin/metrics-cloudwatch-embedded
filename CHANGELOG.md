## v0.3.1 (2022-06-29)

* fixed lambda::handler::run_http and ambda::service::run_http

## v0.3.0 (2022-06-29)

* First draft of the lambda feature
    * MetricsService
    * lambda_cold_start_metric
    * with_lambda_request_id
    * with_lambda_xray_trace_id
* Added a check for more than 30 dimensions/labels

## v0.2.0 (2022-06-26)

* Fixed repository link
* Added a dependency on tracing so we can emit errors when failing to register a metric or overflowing a histogram

## v0.1.0 (2023-06-25)

Initial release