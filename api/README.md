# API

This module provides REST API for client applications to query the Diem blockchain.

The [API specification](blueprint.apib) is documented in [API Blueprint](https://apiblueprint.org) format.

## Overview

API routes and handlers are managed by `warp` framework; endpoints/handlers are grouped into files named by resource names (e.g. accounts, transactions).

Each handler defines:
1. Routes: all routes of the handlers supported in the file.
2. `warp` handler: an async function returns `Result<impl Reply, Rejection>`.
3. Resource struct and handle functions: this may not required if the endpoint logic is super simple.

All HTTP input parameters should be preprocessed and converted into the right type in the `warp` handler or resource struct constructor (`new` function).
Resource handle function should only accept typed input data.

`index.rs` is the root of all routes, it handles `GET /`API and connects all resources' routes with error handling.

The service is launched with a `Context` instance, which holds all external components (e.g. DiemDB, mempool sender).
The `Context` object also serves as a facade of external components, and sharing some general functionalities across
all handlers.

### Principles

To create easy to use API, the following principles are valued

1. [Robustness](https://en.wikipedia.org/wiki/Robustness_principle): be conservative in what you do, be liberal in what you accept from others. Specifically, the API should accept variant formats of valid input data, but be restricted to the output it produces. For example, an account address may have three valid hex-encoded formats: `0x1`, `0x00000000000000000000000000000001` and `00000000000000000000000000000001`; API accepts all of them as input, but all API should output consistent same format (`0x1`). The API should also only expose must-have and the most stable concepts as data structure.
2. Layered Architecture: the API is a layer on top of Diem core/blockchain. JSON is the primary content type we used, a client application should be able to do all aspects of interaction with Diem blockchain using JSON.
3. Compatible with JSON standard and most of the tools, e.g. output `string` type for `u64` instead of integer.

### Models

Models or types are defined in the `diem-api-types` module (in the directory `/api/types`).

API response data structures are optimized for usability across all different languages that may be used by client applications.

`From` / `TryFrom` traits are implemented for converting between API data type and Diem core data types instead of special constructors.

One exception is Move data, they are converted by procedures defined in the `convert.rs`, because Move data type definitions are defined by Move module stored in the Diem DB. However, once the type definition is retrived from database, the related `From` / `TryFrom` trait implemention is used to convert them into API data types.

### Error Handling

Errors are handled by the `warp.Rejection` handler defined in the `index.rs` for all routes.
An `anyhow::Error` is considered as server internal error (500) by default.
All internal errors should be converted into `anyhow::Error` first.
An `diem_api_types.Error` is defined for converting `anyhow::Error` to `warp.Rejection` with HTTP error code.

## Logging

The request log level is set to DEBUG by default.

You can add `diem_api=DEBUG` into RUST_LOG environment to configure the log output.

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

* Build diem-node: `cargo build -p diem-node`
* Install [dredd](https://dredd.org/en/latest/)
* Run `dredd` inside the 'api' directory.


### Render API into HTML Document


For example, use [snowboard](https://github.com/bukalapak/snowboard)

```
npm install -g snowboard
snowboard http blueprint.apib
open http://localhost:8088
```
