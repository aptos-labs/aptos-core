# Summary

The Move query tool offers functions to build knowledge, such as call graph
and dependency graph, about a move package. It has two modes:

- **Integrated Mode**: This mode is integrated as a part of the `aptos move` toolchain.

- **Standalone Mode**: This mode is supported by a standalone tool `move-querier`.

# Integrated Mode

**Usage**: `aptos move query [OPTIONS] <--package-path <PACKAGE_PATH>|--bytecode-path <BYTECODE_PATH>>`

- Available `OPTIONS` include (one must be provided):

    - `--dump-call-graph`: build an inter-module call graph for the move package
     
    - `--dump-dep-graph`: build an inter-module dependency graph for the move package

- Input mode:
    - `--package-path`:  a path to a folder containing a group of bytecode files
    - `--bytecode-path`: a path to an specific bytecode file
    - In both modes, the bytecode file(s) will be loaded as a package repsented as a move model


**Design**: All bytecode file(s) are loaded as CompiledModule into a move model via `load_package()` or `add_module()`. All query operations are performed on top of the move model.

# Standalone Mode

**Usage**: ` move-querier [OPTIONS] --package_path <PACKAGE_PATH>`

- Identical to the integratede mode, except for that this tool only takes `--package_path` for input

# Test

**Testsuite**: Example move source files are available at [tests](./tests).

**Usage** (the test cases are set up to test the `--dump-call-graph` command):

- Enter folder [`move-querier`](./)
- Run `cargo test`