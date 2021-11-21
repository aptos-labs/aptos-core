# API

This module provides REST API for client applications to query the Diem blockchain.

* [API specification](doc/openapi.yaml)
* [Documentation](https://diem.github.io/diem/diem_api/spec.html)

> For a Diem node, you can view the documentation at `/spec.html`.

## Overview

API routes and handlers are managed by `warp` framework; endpoints/handlers are grouped into files named by resource names (e.g. accounts, transactions).

Each handler defines:
1. Routes: all routes of the handlers supported in the file.
2. `warp` handler: an async function returns `Result<impl Reply, Rejection>`.
3. Endpoint handler: this may not be required if the endpoint logic is super simple.

`index.rs` is the root of all routes, it handles `GET /`API and connects all resources' routes with error handling.

The service is launched with a `Context` instance, which holds all external components (e.g. DiemDB, mempool sender).
The `Context` object also serves as a facade of external components, and sharing some general functionalities across
all handlers.

### Parameter Handling

There are four types HTTP input:

1. Path parameter.
2. Query parameter.
3. Request body.
4. Request header.

We process parameters in three stages:

1. Capture HTTP parameter values; this is done by `warp` routes definition.
2. Parse/deserialize HTTP parameter values into API internal data types; this should be done in the `warp` handler (the async function passed into `warp::Filter#and_then` function) or endpoint handler's constructor (`new` function).
3. Process internal data types; this is done by the endpoint handler functions.

For path parameter:

1. Create a `Param<TargetType>` type alias `TargetTypeParam` in the param.rs, and use it in the `warp` route definition for capturing the HTTP path parameter. We don't parse the parameter at this stage, because `warp` will drop error and return `not_found` error without a meaningful message.
2. The `TargetType` is required to implement `FromStr` trait, and `Param#parse` uses it for parsing the HTTP path parameter string and returning `400` error code with a meaningful invalid parameter error message.

Query parameters should not be required, always provide default values for the case they are not provided.

### Principles

To create easy to use API, the following principles are valued

1. [Robustness](https://en.wikipedia.org/wiki/Robustness_principle): be conservative in what you do, be liberal in what you accept from others. Specifically, the API should accept variant formats of valid input data, but be restricted to the output it produces. For example, an account address may have three valid hex-encoded formats: `0x1`, `0x00000000000000000000000000000001` and `00000000000000000000000000000001`; API accepts all of them as input, but all API should output consistent same format (`0x1`). The API should also only expose must-have and the most stable concepts as data structure.
2. Layered Architecture: the API is a layer on top of Diem core/blockchain. JSON is the primary content type we used, a client application should be able to do all aspects of interaction with Diem blockchain using JSON.
3. Compatible with JSON standard and most of the tools, e.g. output `string` type for `u64` instead of integer.

### Models

Models or types are defined in the `diem-api-types` package (in the directory `/api/types`).

These types handle the JSON serialization and deserialization between internal data types and API response JSON types.

`From` / `TryFrom` traits are implemented for converting between API data type and Diem core data types instead of special constructors.

Move data are converted by procedures defined in the `convert.rs`, because Move data type definitions are defined by the Move module stored in the Diem DB. We first retrieve Move data types from the database, then convert them into API data types.

When we convert internal Move struct values into JSON, the data type information will be lost, thus we can't direct convert move struct value JSON data back to any internal data structure while deserializing HTTP request data.
For this reason:
1. `diem_api_types::MoveValue` is only used internally for converting move values into JSON before we create external facing API types (e.g. `TransactionPayload`).
2. When deserializing API request JSON data, we first convert them into external facing API types with Move values as JSON value, then convert Move JSON values into internal move value type `move_core_types::value::MoveValue` when we need.

### Error Handling

Errors are handled by the `warp.Rejection` handler defined in the `index.rs` for all routes.
An `anyhow::Error` is considered as server internal error (500) by default.
All internal errors should be converted into `anyhow::Error` first.
An `diem_api_types.Error` is defined for converting `anyhow::Error` to `warp.Rejection` with HTTP error code.

## Testing

### Unit Test

Handler tests should cover all aspects of features and functions.

A `TestContext` is implemented to create components' stubs that API handlers are connected to.
These stubs are more close to real production components, instead of mocks, so that tests can ensure the API
handlers are working well with other components in the systems.
For example, we use real DiemDB implementation in tests for API layers to interact with the database.

Most of the utility functions are provided by the `TestContext`.

### Integration/Smoke Test

Run integration/smoke tests in `testsuite/smoke-test`

```
cargo test --test "forge" "api::"
```

### API Specification Test

* Run `scripts/dev_setup.sh -a` to setup tools.
* Run `make test` inside the `api` directory.

### [Failpoint](https://docs.rs/fail/latest/fail/) setup

Failpoint configuration example:

```
failpoints
  api::endpoint_index: 1%return
  api::endpoint_get_account_resources: 1%return
  api::endpoint_get_account_modules: 1%return
  api::endpoint_get_transaction: 1%return
  api::endpoint_get_transactions: 1%return
  api::endpoint_get_account_transactions: 1%return
  api::endpoint_submit_json_transactions: 1%return
  api::endpoint_submit_bcs_transactions: 1%return
  api::endpoint_create_signing_message: 1%return
  api::endpoint_get_events_by_event_key: 1%return
  api::endpoint_get_events_by_event_handle: 1%return
```

## Diem Node Operation

Please refer to [Operation](Operation.md) document for details, including configuration, logging, metrics etc.
