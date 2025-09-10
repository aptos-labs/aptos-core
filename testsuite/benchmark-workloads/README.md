# Adding benchmarking workloads

In order to add a benchmark to the testsuite, follow the following steps:

1. Add your Move package to `packages` if you want to use a stable language version.
   Alternatively, for latest language version (which may be unstable) use `packages-experimental`.
2. Generate prebuilt packages.
   These packages are saved as binaries.
   Transaction generator library will load these binaries and generate transactions that publish packages.

   ```bash
   cd testsuite/benchmark-workloads
   ./generate.py
   ```
   Make sure to always run generation script in order to persist your new workloads.
3. In transaction generator, add CLI arguments to call entry functions from your packages or scripts.
   For that, just follow what is done for existing arguments [here](../../crates/transaction-workloads-lib/src/args.rs)
   and [here](../../crates/transaction-workloads-lib/src/move_workloads.rs).
