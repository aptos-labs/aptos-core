# Fuzz Test Suite

## Introduction
This directory contains tools and scripts essential for fuzz testing on Aptos Core. Fuzz targets run continuously on daily versions of `main` on Google's OSS-Fuzz infrastructure.

## `fuzz.sh`
`fuzz.sh` is the main script to perform common fuzzing-related operations.

### Usage
The script includes several functions to manage and execute fuzz tests:
- `add`: Add specified fuzz target.
    ```bash
    ./fuzz.sh add <fuzz_target_name>
    ```

- `build`: Build specified fuzz targets or all targets.
    ```bash
    ./fuzz.sh build <fuzz_target|all> [target_dir]
    ```

- `build-oss-fuzz`: Build fuzz targets specifically for OSS-Fuzz.
    ```bash
    ./fuzz.sh build-oss-fuzz <target_dir>
    ```

- `list`: List all existing fuzz targets.
    ```bash
    ./fuzz.sh list
    ```
- `run`: Run a specific fuzz target, optionally with a testcase.
    ```bash
    ./fuzz.sh run <fuzz_target> [testcase]
    ```
- `test`: Test all fuzz targets with predefined parameters.
    ```bash
    ./fuzz.sh test
    ```

## Writing Fuzz Targets

### Setting Up Fuzz Targets
To set up a fuzz harness in Aptos-core using `cargo-fuzz`:
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
Create a `.zip` archive containing your fuzzer's corpus and name it according to the following format: `[fuzzer_name]_seed_corpus.zip` (e.g., `move_aptosvm_publish_and_run_seed_corpus.zip`). Follow these steps for hosting and integrating the archive:

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

## References
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/)
- [Google OSS-Fuzz](https://google.github.io/oss-fuzz/)
- [Arbitrary](https://docs.rs/arbitrary/latest/arbitrary/)

## Contribute
Contributions to enhance the `fuzz.sh` script and the fuzz testing suite are welcome.