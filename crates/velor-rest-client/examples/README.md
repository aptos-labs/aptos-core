# REST client examples

Really these examples serve as end-to-end tests for the REST client. These are not standard tests because they are should not be run as part of the standard `cargo test` infra, since they expect an Velor API to be running already.

You can run examples like this, from the parent directory:
```
cargo run --example <dir>
```

For example:
```
cargo run --example account -- --api-url http://127.0.0.1:8080
```
