# Summary

The Move Query tool offers functions to build knowledge, such as call 
graph(s) and bytecode type, about Move bytecode. It has two modes:

- **Integrated Mode**: This mode is integrated as a part of the `aptos move` toolchain.

- **Standalone Mode**: This mode is supported by a standalone tool `move-querier`.

# Integrated Mode

**Usage**: `aptos move query [OPTIONS] <--package-path <PACKAGE_PATH>|--bytecode-path <BYTECODE_PATH>>`

- Available `OPTIONS` include (one must be provided):

    - `--dump-call-graph`: Dump the call graph(s) from bytecode.
     
    - `--check-bytecode-type`: Check the type of the bytecode (`script`, `module`, or `unknown`).

**Design**: This mode is integrated into the `aptos move` toolchain similarly as the `decompiler` and the `disassembler`. It reuses the uniform interfaces from [bytecode.rs](../../../../crates/aptos/src/move_tool/bytecode.rs) to support command line parsing and redirect the execution to the `Querier::query()` function implemented in [querier.rs](./src/querier.rs). 

# Standalone Mode

**Usage**: ` move-querier [OPTIONS] --bytecode-path <BYTECODE_PATH>`

- Available `OPTIONS` include (one must be provided):

    - `--dump-call-graph`: Dump the call graph(s) from bytecode.
     
    - `--check-bytecode-type`: Check the type of the bytecode (`script`, `module`, or `unknown`).

**Design**: This mode is compiled from [main.rs](./src/main.rs). The main functionality is supported by the `Querier::query()` function from [querier.rs](./src/querier.rs).

# **Test**

**Testsuite**: Pre-prepared move bytecode files are available at [tests](./tests).

**Usage** (the test cases are set up to test the `--dump-call-graph` command):

- Enter folder [`move-querier`](./)
- Run `cargo test`