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
./target/debug/move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```

or

```bash
./target/debug/aptos move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```

The output will be generated under the `mutants_output` (or any other selected) directory.

Mutator tool respects `RUST_LOG` variable, and it will print out as much information as the variable allows. To see all the logs run:
```bash
RUST_LOG=trace ./target/debug/move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/sources/Sum.move
```
There is possibility to enable logging only for the specific modules. Please refer to the [env_logger](https://docs.rs/env_logger/latest/env_logger/) documentation for more details.

To check possible options run:
```bash
./target/debug/move mutate --help
```
or
```bash
./target/debug/aptos move mutate --help
```

There are also good tests in the Move Prover repository that can be used to test the tool. To run them just use:
```
./target/debug/move mutate -m third_party/move/move-prover/tests/sources/functional/arithm.move
./target/debug/move mutate -m third_party/move/move-prover/tests/sources/functional/bitwise_operators.move
./target/debug/move mutate -m third_party/move/move-prover/tests/sources/functional/nonlinear_arithm.move
./target/debug/move mutate -m third_party/move/move-prover/tests/sources/functional/shift.move
```

To generate mutants for all files within a test project run (there are `Sum.move`, `Operators.move`, `Negation.move` and `StillSimple.move`):
```bash
./target/debug/move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/
```
or appropriately for `aptos`:
```bash
./target/debug/aptos move mutate -m third_party/move/tools/move-mutator/tests/move-assets/simple/
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
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 93, index stop: 94) written to mutants_output/StillSimple_0.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 93, index stop: 94) written to mutants_output/StillSimple_1.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 93, index stop: 94) written to mutants_output/StillSimple_2.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 93, index stop: 94) written to mutants_output/StillSimple_3.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 111, index stop: 112) written to mutants_output/StillSimple_4.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 111, index stop: 112) written to mutants_output/StillSimple_5.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 111, index stop: 112) written to mutants_output/StillSimple_6.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 111, index stop: 112) written to mutants_output/StillSimple_7.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 302, index stop: 303) written to mutants_output/StillSimple_8.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 302, index stop: 303) written to mutants_output/StillSimple_9.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 302, index stop: 303) written to mutants_output/StillSimple_10.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 302, index stop: 303) written to mutants_output/StillSimple_11.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 444, index stop: 445) written to mutants_output/StillSimple_12.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 444, index stop: 445) written to mutants_output/StillSimple_13.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 444, index stop: 445) written to mutants_output/StillSimple_14.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 444, index stop: 445) written to mutants_output/StillSimple_15.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 429, index stop: 430) written to mutants_output/StillSimple_16.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 429, index stop: 430) written to mutants_output/StillSimple_17.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 429, index stop: 430) written to mutants_output/StillSimple_18.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 429, index stop: 430) written to mutants_output/StillSimple_19.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 613, index stop: 614) written to mutants_output/StillSimple_20.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 613, index stop: 614) written to mutants_output/StillSimple_21.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 613, index stop: 614) written to mutants_output/StillSimple_22.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 613, index stop: 614) written to mutants_output/StillSimple_23.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 705, index stop: 706) written to mutants_output/StillSimple_24.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 705, index stop: 706) written to mutants_output/StillSimple_25.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 705, index stop: 706) written to mutants_output/StillSimple_26.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 705, index stop: 706) written to mutants_output/StillSimple_27.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 839, index stop: 840) written to mutants_output/StillSimple_28.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 839, index stop: 840) written to mutants_output/StillSimple_29.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 839, index stop: 840) written to mutants_output/StillSimple_30.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 839, index stop: 840) written to mutants_output/StillSimple_31.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 857, index stop: 858) written to mutants_output/StillSimple_32.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 857, index stop: 858) written to mutants_output/StillSimple_33.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 857, index stop: 858) written to mutants_output/StillSimple_34.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 857, index stop: 858) written to mutants_output/StillSimple_35.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 852, index stop: 853) written to mutants_output/StillSimple_36.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 852, index stop: 853) written to mutants_output/StillSimple_37.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 852, index stop: 853) written to mutants_output/StillSimple_38.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 852, index stop: 853) written to mutants_output/StillSimple_39.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 937, index stop: 938) written to mutants_output/StillSimple_40.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 937, index stop: 938) written to mutants_output/StillSimple_41.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 937, index stop: 938) written to mutants_output/StillSimple_42.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 937, index stop: 938) written to mutants_output/StillSimple_43.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 948, index stop: 949) written to mutants_output/StillSimple_44.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 948, index stop: 949) written to mutants_output/StillSimple_45.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 948, index stop: 949) written to mutants_output/StillSimple_46.move
Mutant: BinaryOperator(+, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 948, index stop: 949) written to mutants_output/StillSimple_47.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 944, index stop: 945) written to mutants_output/StillSimple_48.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 944, index stop: 945) written to mutants_output/StillSimple_49.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 944, index stop: 945) written to mutants_output/StillSimple_50.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 944, index stop: 945) written to mutants_output/StillSimple_51.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 952, index stop: 953) written to mutants_output/StillSimple_52.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 952, index stop: 953) written to mutants_output/StillSimple_53.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 952, index stop: 953) written to mutants_output/StillSimple_54.move
Mutant: BinaryOperator(*, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 952, index stop: 953) written to mutants_output/StillSimple_55.move
Mutant: BinaryOperator(/, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 954, index stop: 955) written to mutants_output/StillSimple_56.move
Mutant: BinaryOperator(/, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 954, index stop: 955) written to mutants_output/StillSimple_57.move
Mutant: BinaryOperator(/, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 954, index stop: 955) written to mutants_output/StillSimple_58.move
Mutant: BinaryOperator(/, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 954, index stop: 955) written to mutants_output/StillSimple_59.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 941, index stop: 942) written to mutants_output/StillSimple_60.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 941, index stop: 942) written to mutants_output/StillSimple_61.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 941, index stop: 942) written to mutants_output/StillSimple_62.move
Mutant: BinaryOperator(-, location: file hash: b63e76f992c022b7d34f479a1b1fe5bb744ec27afa38aa4fe22d081897a91fa0, index start: 941, index stop: 942) written to mutants_output/StillSimple_63.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_0.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_1.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_2.move
Mutant: BinaryOperator(+, location: file hash: cf19f52b0c8c5facfd7cc8f34dbd487a4ea8719e613bbab2f5abb24d51bc355e, index start: 86, index stop: 87) written to mutants_output/Sum_3.move
{
  "Result": "Success"
}

```
