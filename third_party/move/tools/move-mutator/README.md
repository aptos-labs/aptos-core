---
id: move-mutator
title: Move Mutation Tool
---

# move-mutator

## Summary

The `move-mutator` tool is a mutation testing tool for the Move language. 

## Overview

The Move mutator is a tool that mutates Move source code. It can be used to
help test the robustness of Move specifications and tests by generating 
different code versions (mutants).

Please refer to the design document for more details: [Move Mutator Design](doc/design.md).

## Example Usage

Please build the whole repository first. In the `aptos-core` directory, run:
```bash
cargo build -r
```

Then build also the `move-cli` tool:
```bash
cargo build -r -p move-cli
```

Check if the tool is working properly by running its tests:
```bash
cargo test -p move-mutator
```

## Usage

The `move-mutator` tool can be run using the `move-cli` tool or the `aptos`
tool. The command line options are slightly different for both tools. Last 
section of this document describes the command line options for both tools. For
the rest of this document, we will use the `aptos` tool.

```bash
./target/release/aptos move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```

By default, the output shall be stored in the `mutants_output` directory unless
otherwise specified.

The mutator tool respects `RUST_LOG` variable, and it will print out as much
information as the variable allows. To see all the logs run:
```bash
RUST_LOG=trace ./target/debug/aptos move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```
There is possibility to enable logging only for the specific modules. Please
refer to the [env_logger](https://docs.rs/env_logger/latest/env_logger/) documentation for more details.

There are also good tests in the Move Prover repository that can be used to
check the tool. To run them, just use:
```
./target/release/aptos move mutate -m third_party/move/move-prover/tests/sources/functional/arithm.move
./target/release/aptos move mutate -m third_party/move/move-prover/tests/sources/functional/bitwise_operators.move
./target/release/aptos move mutate -m third_party/move/move-prover/tests/sources/functional/nonlinear_arithm.move
./target/release/aptos move mutate -m third_party/move/move-prover/tests/sources/functional/shift.move
```
and observe `mutants_output` directory.

To generate mutants for all files within a test project (for the whole Move 
package) run:
```bash
./target/release/aptos move mutate --package-dir third_party/move/tools/move-mutator/tests/move-assets/simple/
```

Running the above command will generate mutants for all files within the
`simple` test project and should generate following output:
```
$ ./target/release/aptos move mutate --package-dir third_party/move/tools/move-mutator/tests/move-assets/simple/
{
  "Result": "Success"
}
```

You can also examine reports made inside the output directory.

It's also possible to generate mutants for a specific module by using the 
`--mutate-modules` option:
```bash
./target/release/aptos move mutate --package-dir third_party/move/tools/move-mutator/tests/move-assets/simple/ --mutate-modules "Sum"
```

The mutator tool generates:
- mutants (modified move source code)
- reports about mutants in JSON and text format.

Generating mutants for the whole package can be time-consuming. To speed up the
process, mutant verification is disabled by default. To enable it, use the
`--verify-mutants` option:
```bash
./target/release/aptos move mutate --package-dir third_party/move/tools/move-mutator/tests/move-assets/simple/ --verify-mutants
```
Mutants verification is done by compiling them. If the compilation fails,
the mutant is considered invalid. It's highly recommended to enable this option
as it helps to filter out invalid mutants, which would be a waste of time to
prove.

There are several test projects under: 
`third_party/move/tools/move-mutator/tests/move-assets/`
directory. They can be used to check the mutator tool as well.

## Command-line options

Command line options are slightly different when using the `move-cli` tool and
when using the `aptos` tool. Running tool using either of them will produce
the same results as they finally call the same entry point in the mutator code.

To check possible options run:
```bash
./target/release/move mutate --help
```
or
```bash
./target/release/aptos move mutate --help
```

The most notable difference is that the `move-cli` tool uses the `--path`/`-p`
option for pointing Move package, while the `aptos` tool uses the
`--package-dir` option.

When using the `move-cli` tool, the command line options are as follows:
```text
Usage: move mutate [OPTIONS]

Options:
  -m, --move-sources <MOVE_SOURCES>
          The paths to the Move sources
  -p, --path <PACKAGE_PATH>
          Path to a package which the command should be run with respect to
      --mutate-modules <MUTATE_MODULES>
          Module names to be mutated [default: all]
  -v
          Print additional diagnostics if available
  -d, --dev
          Compile in 'dev' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used if this flag is set. This flag is useful for development of packages that expose named addresses that are not set to a specific value
  -o, --out-mutant-dir <OUT_MUTANT_DIR>
          The path where to put the output files
      --test
          Compile in 'test' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used along with any code in the 'tests' directory
      --verify-mutants
          Indicates if mutants should be verified and made sure mutants can compile
      --doc
          Generate documentation for packages
  -n, --no-overwrite
          Indicates if the output files should be overwritten
      --abi
          Generate ABIs for packages
      --downsampling-ratio-percentage <DOWNSAMPLING_RATIO_PERCENTAGE>
          Remove averagely given percentage of mutants. See the doc for more details
      --install-dir <INSTALL_DIR>
          Installation directory for compiled artifacts. Defaults to current directory
  -c, --configuration-file <CONFIGURATION_FILE>
          Optional configuration file. If provided, it will override the default configuration
      --force
          Force recompilation of all packages
      --arch <ARCHITECTURE>
          
      --fetch-deps-only
          Only fetch dependency repos to MOVE_HOME
      --skip-fetch-latest-git-deps
          Skip fetching latest git dependencies
      --bytecode-version <BYTECODE_VERSION>
          Bytecode version to compile move code
      --compiler-version <COMPILER_VERSION>
          Compiler version to use [possible values: v1, v2]
  -h, --help
          Print help
```

When using the `aptos` tool, the command line options are as follows:
```text
Usage: aptos move mutate [OPTIONS]

Options:
      --dev
          Enables dev mode, which uses all dev-addresses and dev-dependencies
          
          Dev mode allows for changing dependencies and addresses to the preset [dev-addresses] and [dev-dependencies] fields.  This works both inside and out of tests for using preset values.
          
          Currently, it also additionally pulls in all test compilation artifacts

      --package-dir <PACKAGE_DIR>
          Path to a move package (the folder with a Move.toml file)

      --output-dir <OUTPUT_DIR>
          Path to save the compiled move package
          
          Defaults to `<package_dir>/build`

      --named-addresses <NAMED_ADDRESSES>
          Named addresses for the move binary
          
          Example: alice=0x1234, bob=0x5678
          
          Note: This will fail if there are duplicates in the Move.toml file remove those first.
          
          [default: ]

      --skip-fetch-latest-git-deps
          Skip pulling the latest git dependencies
          
          If you don't have a network connection, the compiler may fail due to no ability to pull git dependencies.  This will allow overriding this for local development.

      --bytecode-version <BYTECODE_VERSION>
          Specify the version of the bytecode the compiler is going to emit

      --skip-attribute-checks
          Do not complain about unknown attributes in Move code

      --check-test-code
          Do apply extended checks for Aptos (e.g. `#[view]` attribute) also on test code. NOTE: this behavior will become the default in the future. See https://github.com/aptos-labs/aptos-core/issues/10335
          
          [env: APTOS_CHECK_TEST_CODE=]

  -m, --move-sources <MOVE_SOURCES>
          The paths to the Move sources

      --mutate-modules <MUTATE_MODULES>
          Module names to be mutated
          
          [default: all]

  -o, --out-mutant-dir <OUT_MUTANT_DIR>
          The path where to put the output files

      --verify-mutants
          Indicates if mutants should be verified and made sure mutants can compile

  -n, --no-overwrite
          Indicates if the output files should be overwritten

      --downsampling-ratio-percentage <DOWNSAMPLING_RATIO_PERCENTAGE>
          Remove averagely given percentage of mutants. See the doc for more details

  -c, --configuration-file <CONFIGURATION_FILE>
          Optional configuration file. If provided, it will override the default configuration

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print versio
```