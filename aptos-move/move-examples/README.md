# README

**WARNING:** These Move examples have NOT been audited. If using it in a production system, proceed at your own risk.
Particular care should be taken with Move examples that contain complex cryptographic code (e.g., `drand`, `veiled_coin`).

## Writing a Move example

When creating a Move example, make the directory name be the same as the source file name and as the package name.

For example, for the `drand` randomness beacon example, create a `drand` directory with a `sources/drand.move` file in it that has a `module drand::some_module_name { /* ... */ }` in it.
This is because the testing harness will only assign an address to `drand`, based on the directory name, not based on what the named address is in `drand.move`.

## Running tests

To run the tests for **all** examples:

```
cargo test -- --nocapture
```

To run tests for a specific example (e.g., `hello_blockchain`):

```
cargo test -- hello_blockchain --nocapture
```
