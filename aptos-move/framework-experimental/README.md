# README

All Move modules in this directory will be perpetually re-deployed on `devnet` at 0x7.

## Why this package
Some experimental framework features need to be previewed/dogfooded before putting under 0x1.
This package can be their temporary home.

## Writing an experimental Move module

When creating a Move module, make the directory name be the same as the source file name and as the package name.

For example, for the `veiled_coin` contract, create a `veiled_coin` directory with a `sources/veiled_coin.move` file in it that has a `module veiled_coin::some_module_name { /* ... */ }` in it.
This is because the testing harness will only assign an address to `veiled_coin`, based on the directory name, not based on what the named address is in `veiled_coin.move`.

Lastly, be sure your Move module name does not conflict with any other modules in this directory: since all modules get deployed at the same 0x7 address.

## Running tests

To run the tests for **all** experimental modules:

```
cd aptos-move/framework-experimental
cargo test -- --nocapture
```

To run tests for a specific module (e.g., `veiled_coin`):

```
cd aptos-move/framework-experimental
cargo test -- veiled_coin --nocapture
```
