**Move Specification Tester** and **Move Mutator** are tools to test the quality of the Move specifications.

## Overview

The Move mutator is a tool that mutates Move source code. It can be used to help test the robustness of Move specifications and tests by generating different code versions (mutants).

The Move Specification Test tool uses the Move Mutator tool to generate mutants of the Move code. Then, it runs the Move Prover tool to check if the mutants are killed (so Prover will catch an error) by the original specifications. If the mutants are not killed, it means that the specification has issues and is incorrect or not tight enough to catch such cases, so it should be improved.

## Install

To build the tools run:
```bash
$ cargo install --git https://github.com/eigerco/move-spec-testing.git --branch "eiger/move-spec-verifier" move-mutator
$ cargo install --git https://github.com/eigerco/move-spec-testing.git --branch "eiger/move-spec-verifier" move-spec-test
```

That will install the tools into `~/.cargo/bin` directory (at least on MacOS and Linux).
Ensure to have this path in your `PATH` environment. This step can be done with the below command.
```bash
$ export PATH=~/.cargo/bin:$PATH
```

To uninstall tools run:
```bash
$ cargo uninstall move-mutator
$ cargo uninstall move-spec-test
```

## Usage

To check how Move Specification Test tool works, it must be run over the Move code. Some examples are provided [here](https://github.com/eigerco/move-spec-testing/tree/eiger/move-spec-verifier/third_party/move/tools/move-mutator/tests/move-assets).

To start specification testing run the following command (assuming that is downloaded from the provided link):
```bash
$ move-spec-test -p third_party/move/tools/move-mutator/tests/move-assets/same_names
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

The specification testing tool respects `RUST_LOG` variable, and it will print out as much information as the variable allows. There is possibility to enable logging only for the specific modules. Please refer to the[env_logger](https://docs.rs/env_logger/latest/env_logger/) documentation for more details.

To generate a report in a JSON format, use the `-o` option:
```bash
$ move-spec-test -p third_party/move/tools/move-mutator/tests/move-assets/poor_spec -o report.json
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

To check possible options run:
```bash
$ move-spec-test --help
```

Move Specification tool runs Move Mutator internally, however there is possibility to run it manually. To check possible options run:
```bash
$ move-mutator --help
```

By default, the output shall be stored in the `mutants_output` directory unless otherwise specified. The mutator tool also respects `RUST_LOG` variable.

To generate mutants for all files within a test project (for the whole Move package) run:
```bash
$ move-mutator -p third_party/move/tools/move-mutator/tests/move-assets/simple/
```

## License

**Move Specification Tester** and **Move Mutator** are released under the open source [Apache License](LICENSE)

## About Us

[Eiger](https://www.eiger.co) helps leading technology companies to scale and develop their core technologies to gain an edge by providing expert teams in the most critical areas of modern web3 development.