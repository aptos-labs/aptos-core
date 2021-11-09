# Operation

## Web Server Configuration

Diem node generates the following default API configuration:

```
api:
  enabled: true
  address: "0.0.0.0:8080"
```

It's optional to configure TLS certificate and private key file paths for enabling TLS for the web server:
```
api:
  enabled: true
  address: "0.0.0.0:8080"
  tls_cert_path: <file path>
  tls_key_path: <file path>
```

When `api.enabled` is set to `true`, both API and JSON-RPC configured web server will serve the REST and JSON-RPC API.

### JSON-RPC is enabled

Regardless what the JSON-RPC `address` configuration is, JSON-RPC API is always available on the REST API port.

However, JSON-RPC service still reads configurations from `json_rpc` section.

To avoid enabling JSON-RPC and REST API at different ports, you can configure same address for `api.address` and
`json_rpc.address` (you are required to set them same, as well as `tls_cert_path` and `tls_key_path`
for `api` and `json_rpc`).


## Health check endpoint

Health check: `/-/healthy` returns 200

Health check endpoint accepts an optional query parameter `duration_secs`.

Health check returns 200 when `duration_secs` is provided and meet the following condition:
* `server latest ledger info timestamp >= server current time timestamp - duration_secs`

If no param is provided, server returns 200 to indicate HTTP server is running health.

## Logging

The request log level is set to DEBUG by default, 5xx error responses will be logged to ERROR level.

You can add `diem_api=DEBUG` into RUST_LOG environment to configure the log output.


## Metrics

### Requests Processed by Handler

The latency and counts of requests that are processed by a handler are recorded by a histogram
named `diem_api_requests` and labelled by:

* method: HTTP request method
* operation_id: request handler/operation id, it should be same `operationId` defined in [OpenAPI specification](doc/openapi.yaml), except couple cases that are not defined in the [OpenAPI specification](doc/openapi.yaml), e.g. `json_rpc`.
* status: HTTP response statuc code

This metrics covers all requests responses served the API handlers.
Some errors like invalid route path are not covered, because no handlers are used for processing the request.

### All Requests by Status

The latency and counts of requests regardless errors or hit any handlers are recorded by a histogram
named `diem_api_response_status` and labelled by:

* status: HTTP response status code

This metrics covers all requests responses served by the API web server.
