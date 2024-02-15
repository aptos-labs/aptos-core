---
id: move-spec-test
title: Move Specification Test tool
---

# Move Specification Test tool

## Summary

This tool is used to test the quality of the Move specifications.

## Overview

The Move Specification Test tool uses the Move Mutator tool to generate mutants
of the Move code. Then, it runs the Move Prover tool to check if the mutants
are killed (so Prover will catch an error) by the original specifications.
If the mutants are not killed, it means that the specification has issues and
is incorrect or not tight enough to catch such cases, so it should be improved.

Move Specification Test tool can be used on:
- whole Move packages (projects)
- specific modules only

It cannot be used with single Move files.

The tool generates a report in a JSON format. The report contains information
about the number of mutants tested and killed and also the differences between
the original and modified code.

## Example Usage

Please build the whole repository first. In the `aptos-core` directory, run:
```bash
cargo build --release
```

Then build also the `move-cli` tool:
```bash
cargo build -r -p move-cli
```

Check if the tool is working properly by running its tests:
```bash
cargo test -p move-spec-test
```

The Move Specification Test tool demands the Move Prover to be installed and
configured correctly. Please refer to the Move Prover documentation for more
details.

## Usage

Before checking if the tool works, please make sure that the Move Prover is
installed and configured correctly. Especially, ensure that all the
dependencies and backend tools are installed and accessible.

In case of any problems with the backend tools, please try to prove any of the
below examples with the Move Prover tool. If the Move Prover tool works,
the Move Specification Test tool should work as well.

To check if Move Specification Test tool works, run the following command:
```bash
./target/release/move spec-test -p third_party/move/tools/move-mutator/tests/move-assets/same_names
```

or

```bash
./target/release/aptos move spec-test --package-dir third_party/move/tools/move-mutator/tests/move-assets/same_names
```

There should be output generated similar to the following (there may also be
some additional Prover logs visible):
```text
Total mutants tested: 4
Total mutants killed: 4

╭────────────────────────────────────────────────┬────────────────┬────────────────┬────────────╮
│ Module                                         │ Mutants tested │ Mutants killed │ Percentage │
├────────────────────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m1/m1_1/Negation.move::Negation_m1_1 │ 1              │ 1              │ 100.00%    │
├────────────────────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m2/Negation.move::Negation_m2        │ 1              │ 1              │ 100.00%    │
├────────────────────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m1/Negation.move::Negation_m1        │ 1              │ 1              │ 100.00%    │
├────────────────────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/Negation.move::Negation_main         │ 1              │ 1              │ 100.00%    │
╰────────────────────────────────────────────────┴────────────────┴────────────────┴────────────╯
```

The specification testing tool respects `RUST_LOG` variable, and it will print
out as much information as the variable allows. There is possibility to enable
logging only for the specific modules. Please refer to the[env_logger](https://docs.rs/env_logger/latest/env_logger/)
documentation for more details.

To generate a report in a JSON format, use the `-o` option:
```bash
./target/release/move spec-test -p third_party/move/tools/move-mutator/tests/move-assets/poor_spec -o report.json
```

The sample report generated for the above test will look as follows:
```json
{
  "files": {
    "sources/Sum.move": [
      {
        "module": "Sum",
        "tested": 4,
        "killed": 0,
        "mutants_alive_diffs": [
          "--- original\n+++ modified\n@@ -1,6 +1,6 @@\n module TestAccount::Sum {\n     fun sum(x: u128, y: u128): u128 {\n-        let sum_r = x + y;\n+        let sum_r = x - y;\n\n         spec {\n                 // Senseless specification - mutator will change + operator to -*/ but spec won't notice it.\n",
          "--- original\n+++ modified\n@@ -1,6 +1,6 @@\n module TestAccount::Sum {\n     fun sum(x: u128, y: u128): u128 {\n-        let sum_r = x + y;\n+        let sum_r = x * y;\n\n         spec {\n                 // Senseless specification - mutator will change + operator to -*/ but spec won't notice it.\n",
          "--- original\n+++ modified\n@@ -1,6 +1,6 @@\n module TestAccount::Sum {\n     fun sum(x: u128, y: u128): u128 {\n-        let sum_r = x + y;\n+        let sum_r = x / y;\n\n         spec {\n                 // Senseless specification - mutator will change + operator to -*/ but spec won't notice it.\n",
          "--- original\n+++ modified\n@@ -1,6 +1,6 @@\n module TestAccount::Sum {\n     fun sum(x: u128, y: u128): u128 {\n-        let sum_r = x + y;\n+        let sum_r = x % y;\n\n         spec {\n                 // Senseless specification - mutator will change + operator to -*/ but spec won't notice it.\n"
        ]
      }
    ]
  }
}
```

You can try to run the tool using other examples from the `move-mutator`
tests like:
```bash
./target/release/move spec-test -p third_party/move/tools/move-mutator/tests/move-assets/simple
```

You should see different results for different modules as it depends on the
quality of the specifications. Some modules like `Sum` has good specifications
and all mutants are killed, while others like `Operators` may lack some tests.

You can also try the Move Prover testsuite available in the
`third_party/move/move-prover/tests/sources/` directory.

To check some real-world examples, you can use the following places:
- `aptos-move/framework/move-stdlib`
- `aptos-move/framework/aptos-stdlib`

You should see the results of the tests for the whole stdlib packages. There
can be some modules which have better specifications quality (more mutants
killed) and some which have worse quality (fewer mutants killed). It is also
possible that some modules have no mutants killed at all which can be a sign
that there are no specifications at all, or they are not tight enough.

It's recommended to generate a report in a JSON format and analyze it to see
which mutants are not killed and what are the differences between the original
and modified code. This can help to improve the specifications and make them
more tight and correct or it may indicate that some specifications some
mutation operators are not applicable well to that kind of code.

## Command-line options

Command line options are slightly different when using the `move-cli` tool and
when using the `aptos` tool.

To check possible options run:
```bash
./target/release/move spec-test --help
```
or
```bash
./target/release/aptos move spec-test --help
```

When using the `move-cli` tool, the command line options are as follows:
```text
Usage: move spec-test [OPTIONS]

Options:
  -m, --move-sources <MOVE_SOURCES>
          The paths to the Move sources
  -p, --path <PACKAGE_PATH>
          Path to a package which the command should be run with respect to
  -i, --include-modules <INCLUDE_MODULES>
          Work only over specified modules [default: all]
  -v
          Print additional diagnostics if available
  -d, --dev
          Compile in 'dev' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used if this flag is set. This flag is useful for development of packages that expose named addresses that are not set to a specific value
      --mutator-conf <MUTATOR_CONF>
          Optional configuration file for mutator tool
      --prover-conf <PROVER_CONF>
          Optional configuration file for prover tool
      --test
          Compile in 'test' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used along with any code in the 'tests' directory
      --doc
          Generate documentation for packages
  -o, --output <OUTPUT>
          Save report to a JSON file
      --abi
          Generate ABIs for packages
  -u, --use-generated-mutants <USE_GENERATED_MUTANTS>
          Use previously generated mutants
      --install-dir <INSTALL_DIR>
          Installation directory for compiled artifacts. Defaults to current directory
      --verify-mutants
          Indicates if mutants should be verified and made sure mutants can compile
      --extra-prover-args <EXTRA_PROVER_ARGS>
          Extra arguments to pass to the prover
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
Usage: aptos move spec-test [OPTIONS]

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

  -i, --include-modules <INCLUDE_MODULES>
          Work only over specified modules
          
          [default: all]

      --mutator-conf <MUTATOR_CONF>
          Optional configuration file for mutator tool

      --prover-conf <PROVER_CONF>
          Optional configuration file for prover tool

  -o, --output <OUTPUT>
          Save report to a JSON file

  -u, --use-generated-mutants <USE_GENERATED_MUTANTS>
          Use previously generated mutants

      --verify-mutants
          Indicates if mutants should be verified and made sure mutants can compile

      --extra-prover-args <EXTRA_PROVER_ARGS>
          Extra arguments to pass to the prover

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```