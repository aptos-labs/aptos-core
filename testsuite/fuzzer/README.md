# Fuzz Test Suite

## Introduction
This directory contains tools and scripts essential for fuzz testing on Velor Core. Fuzz targets run continuously on daily versions of `main` on Google's OSS-Fuzz infrastructure.

## `fuzz.sh`
`fuzz.sh` is the main script to perform common fuzzing-related operations.

### Usage
The script includes several functions to manage and execute fuzz tests:
- `add`: Add specified fuzz target.
    ```bash
    ./fuzz.sh add <fuzz_target_name>
    ```
- `block-builder`: Run rust utility to build fuzzers.
    ```bash
    ./fuzz.sh block-builder <utility> [args]
    ```
- `block-builder-recursive`: Runs block-builder on all Move.toml files in a directory.
    ```bash
    ./fuzz.sh block-builder-recursive <search_directory> <destination_directory>
    ```
- `build`: Build specified fuzz targets or all targets.
    ```bash
    ./fuzz.sh build <fuzz_target|all> [target_dir]
    ```
- `build-oss-fuzz`: Build fuzz targets specifically for OSS-Fuzz.
    ```bash
    ./fuzz.sh build-oss-fuzz <target_dir>
    ```
- `clean-coverage`: Clean coverage artifacts for a specific target or all targets.
    ```bash
    ./fuzz.sh clean-coverage <fuzz_target|all>
    ```
- `cmin`: Distillate corpora
    ```bash
    ./fuzz.sh cmin <fuzz_target> [corpora_directory]
    ```
- `coverage`: Generates coverage report in HTML format
    ```bash
    ./fuzz.sh coverage <fuzz_target>
    ```
    > rustup +nightly-2024-04-06 component add llvm-tools-preview
- `debug`: Run fuzzer with GDB and pass test_case as input
    ```bash
    ./fuzz.sh debug <fuzz_target> <test_case>
    ```
- `flamegraph`: Generates flamegraph report (might requires addition setups on the os)
    ```bash
    ./fuzz.sh flamegraph <fuzz_target> <test_case>
    ```
- `list`: List all existing fuzz targets.
    ```bash
    ./fuzz.sh list
    ```
- `monitor-coverage`: Monitors coverage for a fuzz target, regenerating when the corpus changes.
    ```bash
    ./fuzz.sh monitor-coverage <fuzz_target>
    ```
- `run`: Run a specific fuzz target, optionally with a testcase.
    ```bash
    ./fuzz.sh run <fuzz_target> [testcase]
    ```
- `test`: Test all fuzz targets with predefined parameters.
    ```bash
    ./fuzz.sh test
    ```
- `tmin`: Minimize a crashing input for a target.
    ```bash
    ./fuzz.sh tmin <fuzz_target> <crashing_input>
    ```

## Writing Fuzz Targets

### Setting Up Fuzz Targets
To set up a fuzz harness in Velor-core using `cargo-fuzz`:
#### Initialize Fuzz Target
Run the following command to initialize the fuzzing target. This creates and edits all the necessary files.
```bash
./fuzz.sh add fuzz_target_name
   ``` 

#### Create a Fuzz Target
The basic structure of a fuzz target in Rust using `cargo-fuzz` is:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Code to handle the fuzzer input and test the desired functionality
});
```

In example above, the fuzzing engine provides a random slice of bytes. Alternatively, it can provide struct-aware data by leveraging the `Arbitrary` trait, defined in the omonimous crate. The easiest way to implement the `Arbitrary` trait is to derive it (via the `derive_arbitrary` feature.) For example:

```rust
#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary)]
struct ComplexData {}

fuzz_target!(|data: ComplexData| {
    // Code to handle the fuzzer input and test the desired functionality
});
```

Note that `Arbitrary` must be implemented (or derived) for all types used in the `ComplexData` structure.

#### Implementing the fuzz logic
The `fuzz_target!` macro receives data from the fuzzer. Implement logic to convert the fuzzer input into a format that the targeted function or module can process. Check existing fuzz targets for examples.

### OSS-Fuzz Corpus
Create a `.zip` archive containing your fuzzer's corpus and name it according to the following format: `[fuzzer_name]_seed_corpus.zip` (e.g., `move_velorvm_publish_and_run_seed_corpus.zip`). Follow these steps for hosting and integrating the archive:

1. **Upload to Public Hosting:** If you choose Google Drive, ensure the archive is publicly accessible via a shared link.

2. **(GDrive Only) Modify the URL:** Replace `FILEID` in the URL template with your file's ID. The template URL is: 
   ```
   https://docs.google.com/uc?export=download&id=FILEID
   ```

3. **Update `fuzz.sh`:** Insert the modified URL into the `CORPUS_ZIPS` array within the "fuzz.sh" script.

When building in the OSS-Fuzz environment, `fuzz.sh` will place the corpus archive correctly alongside your fuzzer's binary. OSS-Fuzz then selects the proper archive, using its contents to feed the fuzzer.

### Best Practices for Writing Fuzz Targets
- **Focus on Target Functionality:** Choose functions or modules critical to your application's functionality and security.
- **Handle Diverse Inputs:** Ensure that the harness can handle a wide range of input formats and sizes.
- **Error Handling:** Implement robust error handling to intercept crashes or unwanted/unexpected behavior.
- **Performance Optimization:** Optimize for performance to enable more iterations and deeper fuzzing.

## Generate Corpora
Some fuzzers operate better if a good initial corpus is provided. In order to generate the corpus, utilities are available via `./fuzz.sh block-builder`. Once a corpus is obtained, to feed it to fuzzers running on OSS-Fuzz, building a ZIP archive with a specific name is required: `$FUZZERNAME_seed_corpus.zip`. Upload it to a publicly accessible cloud, e.g., GCP Bucket or S3; avoid GDrive. Obtain a public link and add it to the `CORPUS_ZIPS` array in `fuzz.sh`. It will automatically be downloaded and used inside Google's infrastructure.
### Velor-VM Publish & Run
`./fuzz.sh block-builder generate_runnable_state /tmp/modules.csv /tmp/Modules`
The CSV file is structured as follows:  
- Column 1: Module name  
- Column 2: Module address  
- Column 3: Base64-encoded bytecode of the module

You can generate a test case from any valid Move project (arguments to function calls might need attention TODO). It's helpful for testing new functionalities or increasing coverage completeness. For native functions, please follow the structure in the data folder.

> Create an entry function, which may accept a signer or no parameters. Generic T functions are not allowed as entry.

The first argument is the project, and the second one is the target directory.
`./fuzz.sh block-builder generate_runnable_state_from_project data/0x1/string/generic fuzz/corpus/move_velorvm_publish_and_run`

> Verify your testcase runs as expected by appending `DEBUG=1` while calling the fuzzer and using the newly generated test case as the second parameter.

#### Bulk Build
Use `./fuzz.sh block-builder generate_runnable_states_recursive data/0x1/ fuzz/corpus/move_velorvm_publish_and_run` to compile all the modules under a specific directory.

#### Steps (internal)
The following steps apply to wathever seed we might want to make available, remember to add the public link at the begin of `fuzz.sh`.
1. `gcloud auth login`
2. `gcloud storage cp gs://velor-core-corpora/move_velorvm_publish_and_run_seed_corpus.zip move_velorvm_publish_and_run_seed_corpus.zip`
3. `unzip move_velorvm_publish_and_run_seed_corpus.zip -d move_velorvm_publish_and_run_seed_corpus`
4. `./fuzz.sh block-builder generate_runnable_states_recursive data/0x1/ move_velorvm_publish_and_run_seed_corpus`
5. Normally we would run cmin but we assume that manually created inputs are fine and serve a specific purse.
6. `zip -r move_velorvm_publish_and_run_seed_corpus.zip move_velorvm_publish_and_run_seed_corpus`
7. `gsutil storage cp move_velorvm_publish_and_run_seed_corpus.zip gs://velor-core-corpora/move_velorvm_publish_and_run_seed_corpus.zip`
8. We need to restore ACL (public URL remain the same): `gsutil storage acl ch -u AllUsers:R gs://velor-core-corpora/move_velorvm_publish_and_run_seed_corpus.zip`

## Debug Crashes
Flamegraph and GDB are integrated into fuzz.sh for advanced metrics and debugging. A more rudimentary option is also available: since we have symbolized binaries, we can directly use the stack trace produced by the fuzzer. However, for INVARIANT_VIOLATIONS, the stack trace is incorrect. To obtain the correct stack trace, you can use the following command:
```bash
DEBUG_VM_STATUS=<status_reported_by_the_fuzzer> ./fuzz.sh run <fuzzer_target> <test_case>
```
This command is selective, so only the specified, comma-separated statuses will trigger the panic in PartialVMError.

## References
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/)
- [Google OSS-Fuzz](https://google.github.io/oss-fuzz/)
- [Arbitrary](https://docs.rs/arbitrary/latest/arbitrary/)
- [Native Functions](https://velor.dev/en/build/smart-contracts/move-reference?branch=mainnet&page=move-stdlib%2Fdoc%2Fmem.md)

## Contribute
Contributions to enhance the `fuzz.sh` script and the fuzz testing suite are welcome.