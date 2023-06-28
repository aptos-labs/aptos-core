# Developer guide

Use this doc to get started with move-to-llvm compiler development.
It is highly encouraged to read the documents linked in the [reference](#references) section when planning to contribute.

## Dependencies

> zlib (apt install zlib1g-dev)
> [lld](https://lld.llvm.org/)
> [Solana port of LLVM](https://github.com/solana-labs/llvm-project)
> [Solana platform tools](https://github.com/solana-labs/platform-tools)

## Setup

Building requires a local build of [llvm-project](https://github.com/solana-labs/llvm-project)
from Solana's fork that supports the Solana variant of eBPF,
and testing requires an installation of the Solana [platform-tools](https://github.com/solana-labs/platform-tools).

Known working revisions of both:

- llvm-project: commit `33c3629caa59b59d8f585736a4a5194aa9e9377d`,
  tag `solana-tools-v1.36`,
  from the `solana-labs` repo
- platform-tools: version `1.36`

`platform-tools` can be extracted from the binary release.

Export two environment variables:

- `LLVM_SYS_150_PREFIX` - the path to the LLVM build directory
- `PLATFORM_TOOLS_ROOT` - the path at which `platform-tools` was extracted

### After a toolchain update

You might run into build errors because of incompatible artifacts etc. due to a toolchain update.
In that case you need to uninstall and reinstall. For example:

```
rustup toolchain uninstall 1.65.0
rustup toolchain install 1.66.0
```

### Instructions to build solana-labs/llvm-project

```sh
# Clone the fork of llvm-project maintained by solana-labs
$ git clone https://github.com/solana-labs/llvm-project
# Build the tools
$ mkdir -p llvm-project/build && cd llvm-project/build
$ git fetch origin solana-tools-v1.36:solana-tools-v1.36 && git checkout solana-tools-v1.36
$ cmake -GNinja -DLLVM_TARGETS_TO_BUILD="X86;SBF;BPF" -DCMAKE_BUILD_TYPE=RelWithDebInfo -DLLVM_ENABLE_PROJECTS="clang;libcxx;libcxxabi;libunwind" ../llvm
$ ninja clang
# Point LLVM_SYS_150_PREFIX (used by llvm-sys) to built tools
$ export LLVM_SYS_150_PREFIX=/path/to/llvm-project/build
```

### Instructions to get solana-labs/platform-tools

```sh
$ cd /path/to/platform-tools/releases/
# For OSX download  platform-tools-osx-x86_64.tar.bz2
$ wget https://github.com/solana-labs/platform-tools/releases/download/v1.36/platform-tools-linux-x86_64.tar.bz2
$ mkdir v1.36 && cd v1.36
$ tar -xf ../platform-tools-linux-x86_64.tar.bz2
$ ls /path/to/platform-tools/releases/v1.36
llvm  rust  version.md
$ export PLATFORM_TOOLS_ROOT=/path/to/platform-tools/releases/v1.36
```

## Building

```sh
# export LLVM_SYS_150_PREFIX
# export PLATFORM_TOOLS_ROOT
cargo build -p move-ir-compiler && cargo build -p move-compiler
```

## Testing

This project contains three test suites:

- `ir-tests` - converts Move IR (`.mvir`) to LLVM IR
- `move-ir-tests` - converts Move source (`.move`) to LLVM IR
- `rbpf-tests` - runs move as SBF in the `rbpf` VM

These test require the `move-ir-compiler` and `move-build` tools (See: [Build instructions](#building)). If you forget, the test harness will remind you what commands to run to build the tools.

Run the tests with any of these commands:

```sh
# export LLVM_SYS_150_PREFIX
# export PLATFORM_TOOLS_ROOT
cargo test -p move-mv-llvm-compiler --test ir-tests
cargo test -p move-mv-llvm-compiler --test move-ir-tests
cargo test -p move-mv-llvm-compiler --test rbpf-tests
```

The IR tests work by producing `.actual.ll` files and comparing them to
`.expected.ll` files. When introducing new tests, or making changes to the code
generator that invalidate existing tests, the "actual" files need to be promoted
to "expected" files. This can be done like

```sh
PROMOTE_LLVM_IR=1 cargo test -p move-mv-llvm-compiler --test move-ir-tests
```
Most new tests should be `move-ir-tests` or `rbpf-tests`,
as the Move IR is not stable nor easy to work with.

### Environment variables to control rbpf-tests
#### `TRACE`
Enable SBF instruction tracing/disassembly for a rbpf case. This is an extremely valuable debugging tool when an rbpf test crashes in the `move-native` library-- or perhaps worse-- in core rust libraries. To enable, set environment variable `TRACE` to a filename where the output will be directed. Setting `TRACE=` or `TRACE=stdout` writes the output to stdout.
```sh
TRACE=foo.txt cargo test -p move-mv-llvm-compiler --test rbpf-tests my_test_case
```
#### `DUMP`
Setting this environment variable will enable the test driver to output `// log` messages.
```sh
DUMP=1 cargo test -p move-mv-llvm-compiler --test rbpf-tests
```

## Test directives

Tests support "directives", written as comments at the top of the file,
that are interpreted by the test runner to determine if the test is successful:

They look like this:

```move
// abort 10

script {
  fun main() {
    assert!(1 == 2, 10);
  }
}
```

Supported directives include:

- `// ignore` - don't run the test, for broken tests.
- `// abort {code}` - expect an abort with code.
- `// log {string}` - expect a string to be logged by the `debug::print` function.
- `// signers {signer0,signer1,...}` - provide a list of signers to script `main`. Each signer is injected into a corresponding argument of main with type `signer`. See example below.

```move
// signers 0xcafe,0xf00d,0xc0ffee,0xb00
   ...
script {
    fun main(s1: signer, s2: signer, s3: signer, s4: signer) {
       ...
    }
}
```

`abort`, `log`, and `signers` are only supported by the `rbpf-tests` runner.

## Debugging

### Setting up llvm, llvm-sys for debugging

- Build llvm with debug symbols

### Debugging inside rbpf vm

Install [CodeLLDB plugin](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

- lldb with gdbserver

### Protip

----
In case `cargo build` fails in **Cargo.lock** file unable to resolve dependencies, try *regenerating* the **Cargo.lock** file with the following command.

> cargo generate-lockfile

----
Did you forget to set up environment variables?

> source ~/.profile

----
To update a test's expected output based on the existing output

> export UPDATE_BASELINE=1

And then run `cargo test`

**NB: Not working currently**. For IR tests:

```bash
cp move/language/tools/move-mv-llvm-compiler/tests/move-ir-tests/$test-build/modules/0_Test.actual.ll tests/move-ir-tests/$test-build/modules/0_Test.expected.ll
```

----
To generate a move bytecode module (.mv file) from mvir file

> move-ir-compiler -m a.mvir

----
To generate bytecode in text format
> move-disassembler --bytecode a.mv

----
To debug use the `RUST_BACKTRACE` environment variables

```sh
RUST_BACKTRACE=<value> rust-exe [args]
RUST_BACKTRACE=1 move-mv-llvm-compiler -b tests/BasicCoin.mv
RUST_BACKTRACE=full move-mv-llvm-compiler -b tests/BasicCoin.mv
```

----
Error: DEP_LLVM_CONFIG_PATH not set

DEP_LLVM_CONFIG_PATH is set by [llvm-sys](https://gitlab.com/taricorp/llvm-sys.rs/-/blob/main/build.rs#L452)
When this error occurs, it means that your llvm-sys isn't setup properly.

----
Instead of calling `--help` on the move-mv-llvm-compiler use `cargo run -- --help`

----
Use [RUST_LOG](https://docs.rs/env_logger/latest/env_logger/) environment variable to print compiler logs.
For example

> RUST_LOG=info move-mv-llvm-compiler -b tests/BasicCoin.mv

## Submission

Only github pull requests are accepted. Typically contributors would fork this repo
and contribute make changes to their fork in a branch. Then create a pull-request
to solana-labs/move repostitory. Add at least one reviewer.

Before creating a pull request, make sure to:
- Run all tests
- Run the code formatter `cargo x fmt`
- Run the linters to pass pre-submit checks
  - `cargo x lint`
  - `cargo x clippy --workspace --all-targets` Note that clippy sometimes [does not lint all files](https://users.rust-lang.org/t/why-does-clippy-not-always-display-suggestions-for-me/32120/4). You might want to `cargo clean` in that case.

## References

Recommended reading

- [bytecode-instruction-semantics](https://docs.google.com/spreadsheets/d/1b3ccBcM8p76GTR7p_a0Kz3cO-oIXvCa3G90bXw_W-io)
- [Tips for writing bytecode tools for Move](https://github.com/move-language/move/issues/817)

References on move

- [move book](https://move-language.github.io/move)
- [move paper](https://developers.libra-china.org/docs/assets/papers/libra-move-a-language-with-programmable-resources.pdf)
- [Presentation by Sam](https://www.youtube.com/watch?v=J1U_0exNFu0)
- [Presentation by Sam](https://www.youtube.com/watch?v=b_2jZ4YEfWc)
