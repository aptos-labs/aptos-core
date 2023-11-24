# move-mutator

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
cargo test -p move-mutator
```

## Usage

To check if it works, run the following command:
```bash
./target/debug/move mutate sources third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```

or

```bash
./target/debug/aptos move mutate --move-sources third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```

The output will be generated under `mutants_output` directory.

To generate mutants for all files within a test project run:
```bash
./target/debug/move mutate sources third_party/move/tools/move-mutator/tests/move-assets/simple/
```
or appropriately for `aptos`:
```bash
./target/debug/aptos move mutate --move-sources third_party/move/tools/move-mutator/tests/move-assets/simple/
```

Running the above command (`aptos` one) will generate mutants for all files within the `simple` test project and should generate following output:
```
./target/debug/aptos move mutate --move-sources third_party/move/tools/move-mutator/tests/move-assets/simple/sources/
Executed move-mutator with the following options: Options { move_sources: ["third_party/move/tools/move-mutator/tests/move-assets/simple/sources/"] } 
 config: BuildConfig { dev_mode: false, test_mode: true, generate_docs: false, generate_abis: false, generate_move_model: false, full_model_generation: false, install_dir: None, force_recompilation: false, additional_named_addresses: {}, architecture: None, fetch_deps_only: false, skip_fetch_latest_git_deps: false, compiler_config: CompilerConfig { bytecode_version: None, known_attributes: {"bytecode_instruction", "deprecated", "event", "expected_failure", "legacy_entry_fun", "native_interface", "resource_group", "resource_group_member", "test", "test_only", "verify_only", "view"}, skip_attribute_checks: false, compiler_version: None } } 
 package path: "/home/test/Projects/aptos-core"
Mutant: BinaryOperator(+, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 77, index stop: 78) written to mutants_output/Operators_0.move
Mutant: BinaryOperator(+, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 77, index stop: 78) written to mutants_output/Operators_1.move
Mutant: BinaryOperator(+, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 77, index stop: 78) written to mutants_output/Operators_2.move
Mutant: BinaryOperator(+, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 77, index stop: 78) written to mutants_output/Operators_3.move
Mutant: BinaryOperator(-, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 133, index stop: 134) written to mutants_output/Operators_4.move
Mutant: BinaryOperator(-, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 133, index stop: 134) written to mutants_output/Operators_5.move
Mutant: BinaryOperator(-, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 133, index stop: 134) written to mutants_output/Operators_6.move
Mutant: BinaryOperator(-, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 133, index stop: 134) written to mutants_output/Operators_7.move
Mutant: BinaryOperator(*, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 189, index stop: 190) written to mutants_output/Operators_8.move
Mutant: BinaryOperator(*, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 189, index stop: 190) written to mutants_output/Operators_9.move
Mutant: BinaryOperator(*, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 189, index stop: 190) written to mutants_output/Operators_10.move
Mutant: BinaryOperator(*, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 189, index stop: 190) written to mutants_output/Operators_11.move
Mutant: BinaryOperator(%, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 245, index stop: 246) written to mutants_output/Operators_12.move
Mutant: BinaryOperator(%, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 245, index stop: 246) written to mutants_output/Operators_13.move
Mutant: BinaryOperator(%, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 245, index stop: 246) written to mutants_output/Operators_14.move
Mutant: BinaryOperator(%, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 245, index stop: 246) written to mutants_output/Operators_15.move
Mutant: BinaryOperator(/, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 301, index stop: 302) written to mutants_output/Operators_16.move
Mutant: BinaryOperator(/, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 301, index stop: 302) written to mutants_output/Operators_17.move
Mutant: BinaryOperator(/, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 301, index stop: 302) written to mutants_output/Operators_18.move
Mutant: BinaryOperator(/, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 301, index stop: 302) written to mutants_output/Operators_19.move
Mutant: BinaryOperator(&, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 357, index stop: 358) written to mutants_output/Operators_20.move
Mutant: BinaryOperator(&, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 357, index stop: 358) written to mutants_output/Operators_21.move
Mutant: BinaryOperator(|, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 412, index stop: 413) written to mutants_output/Operators_22.move
Mutant: BinaryOperator(|, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 412, index stop: 413) written to mutants_output/Operators_23.move
Mutant: BinaryOperator(^, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 468, index stop: 469) written to mutants_output/Operators_24.move
Mutant: BinaryOperator(^, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 468, index stop: 469) written to mutants_output/Operators_25.move
Mutant: BinaryOperator(<<, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 523, index stop: 525) written to mutants_output/Operators_26.move
Mutant: BinaryOperator(>>, location: file hash: 871072646343dc04b16c8b19009982460f2afbc55f14d2f1c2710e741d9a0ce1, index start: 579, index stop: 581) written to mutants_output/Operators_27.move
Mutant: UnaryOperator(!, location: file hash: 2adb714d41fdc23364c90e2fd21f9807a887ffd59343df63f9dfedde3fc62233, index start: 72, index stop: 73) written to mutants_output/Negation_0.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_0.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_1.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_2.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_3.move
{
  "Result": "Success"
}

```