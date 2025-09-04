# Contributing to the Move standard library

This guide describes the process for adding, removing, and changing the Move modules and transaction scripts in the standard library.

## Overview

Every state change in the Velor blockchain occurs via executing a Move *entry function* or a *script* embedded in a [SignedTransaction](../../types/src/transaction/mod.rs). Entry functions and scripts invoke procedures of Move *modules* that update published *resources*. The Move standard library consists of [modules](modules/) initially published in the genesis transaction.

## Environment Setup

Start by following the general Velor setup advice [here](../../CONTRIBUTING.md). Nothing else is strictly required, but you may want to consider a Move syntax highlighter for your editor (asking it to interpret `.move` files as Rust source is a decent start).

<!-- TODO: editor-specific suggestions, bash aliases -->

### Building

Execute

`cargo run`

inside `stdlib` to compile all of the standard library modules, transaction scripts, and supporting Rust wrappers. It is important to do this before running any tests that exercise your change.

### Testing

Most tests for the standard library live [here](../e2e-move-tests) and can be run with `cargo test`.

## Changing the standard library

### Modules

- Add or edit the relevant `.move` file under [modules](modules/)
- [Build](#building) your changes and address compiler errors as needed
- Once the stdlib builds, add new end-to-end [tests](#testing)

## Coding conventions

### Naming
- **Module names**: are camel case e.g., `VelorAccount`, `Velor`
- **Type names**: are camel case e.g., `WithdrawalCapability`, `KeyRotationCapability`
- **Function names**: are lower snake case e.g., `register_currency`
- **Constant names**: are upper snake case e.g., `TREASURY_COMPLIANCE_ADDRESS`
- Generic types should be descriptive, or anti-descriptive where appropriate (e.g. `T` for the Vector generic type parameter, `VelorAccount` for the core `VelorAccount` resource, `deposit<CoinType>(t: CoinType)` for depositing a token in the `Velor` module). Most of the time the "main" type in a module should be the same name as the module e.g., `Velor::Velor`, `VelorAccount::VelorAccount`.
- **Module file names**: are the same as the module name e.g., `VelorAccount.move`
- **Script file names**: should be lower snake case and named after the name of the “main” function in the script.
- **Mixed file names**: If the file contains multiple modules and/or scripts, the file name should be lower_snake_case, where the name does not match any particular module/script inside.

### Imports
- Functions and constants are imported and used fully qualified from the module in which they are declared, and not imported at the top level.
- Types are imported at the top-level. Where there are name clashes, `as` should be used to rename the type locally as appropriate.
 e.g. if there is a module
```rust
module Foo {
    resource struct Foo { }
    public const CONST_FOO: u64 = 0;
    public fun do_foo(): Foo { Foo{} }
    ...
}
```
this would be imported and used as:
```rust
module Bar {
    use 0x1::Foo::{Self, Foo};

    public fun do_bar(x: u64): Foo {
        if (x == Foo::CONST_FOO) {
            Foo::do_foo()
        } else {
            abort 0
        }
    }
    ...
}
```
And, if there is a local name-clash when importing two modules:
```rust
module OtherFoo {
    resource struct Foo {}
    ...
}

module Importer {
    use 0x1::OtherFoo::Foo as OtherFoo;
    use 0x1::Foo::Foo;
....
}
```


### Comments

- Each module, struct, resource, and public function declaration should be commented
- Move has both doc comments `///`, regular single-line comments `//`, and block comments `/* */`


## Formatting
We plan to have an autoformatter to enforce these conventions at some point. In the meantime...

- Four space indentation except for `script` and `address` blocks whose contents should not be indented
- Break lines longer than 100 characters
