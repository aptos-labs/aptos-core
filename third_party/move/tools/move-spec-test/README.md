# Move Specification Test tool

This tool is used to test the Move specification.

Please build the whole repository first. In the `aptos-core` directory, run:
```bash
cargo build
```

Then build also the `move-cli` tool:
```bash
cargo build -p move-cli
```

Check if the tool is working properly by running its tests:
```bash
cargo test -p move-spec-test
```

## Usage

To check if it works, run the following command:
```bash
./target/debug/move spec-test -p third_party/move/tools/move-mutator/tests/move-assets/same_names
```

or

```bash
./target/debug/aptos move spec-test --package-dir third_party/move/tools/move-mutator/tests/move-assets/same_names
```

There should be output generated similar to the following:
```bash
Total mutants tested: 4
Total mutants killed: 4

╭─────────────────────────────────┬────────────────┬────────────────┬────────────╮
│ File                            │ Mutants tested │ Mutants killed │ Percentage │
├─────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m2/Negation.move      │ 1              │ 1              │ 100.00%    │
├─────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m1/Negation.move      │ 1              │ 1              │ 100.00%    │
├─────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/Negation.move         │ 1              │ 1              │ 100.00%    │
├─────────────────────────────────┼────────────────┼────────────────┼────────────┤
│ ./sources/m1/m1_1/Negation.move │ 1              │ 1              │ 100.00%    │
╰─────────────────────────────────┴────────────────┴────────────────┴────────────╯
```

Specification testing tool respects `RUST_LOG` variable, and it will print out as much information as the variable allows. There is possibility to enable logging only for the specific modules. Please refer to the [env_logger](https://docs.rs/env_logger/latest/env_logger/) documentation for more details.

To check possible options run:
```bash
./target/debug/move spec-test --help
```
or
```bash
./target/debug/aptos move spec-test --help
```