## A tool to replay and benchmark past Aptos transactions


### Benchmarking and measurements

This tool allows to benchmark an ordered sequence of past transactions, specifying the first (`--begin-version B`) and the last (`--end-version E`) versions.
Transactions are split into blocks, to mimic on-chain behaviour, and blocks are executed one-by-one using an executor.
During the execution, the time is measured.
Each block runs based on the pre-computed state, so there is no "commit" of block execution outputs.
Similarly, signature verification is also left out.
Hence, the benchmark reports the runtime only.

The tool supports two ways to measure the time:

  1. Measuring total execution time for all transactions (default).
  2. Measuring execution time for each of the executed blocks, and reporting all.
     To enable this, use `--measure-block-times` flag.

In both cases, the measurement is repeated at least 3 times (this can be configured by specifying the number of repeats,  `N`, using `--num-repeats N`), and the minimum, maximum, average and median times are reported (in microseconds).

When benchmarking, a list of concurrency levels (`--concurrency-levels L1 L2 ...`) has to be provided.
Concurrency level specifies the number of threads Block-STM will use to execute a block of transactions.
Typically, you want to have the concurrency level to match the number of cores.
If multiple concurrency levels are provided, the benchmark is run for all, reporting the measurements.
This way it is possible to see how concurrency affects the runtime. 

Finally, in order to differentiate between cold and warm states, there is an option to skip measurement for the first few blocks.
By specifying `--num-block-to-skip N`, the tool will not ignore measurements when reporting for the first `N` blocks.

### State overriding

The benchmark runs every block on top of the corresponding on-chain state.
However, it is possible to override the state.
Currently, the only supported overrides are feature flags:

  1. Feature flags can be forcefully enabled (`--enable-features F1 F2 ...`).
  2. Feature flags can be forcefully disabled (`--disable-features F1 F2 ...`).

Feature flags should be spelled in capital letters, e.g., `ENABLE_LOADER_V2`.
For the full list of available features, see [here](../../types/src/on_chain_config/aptos_features.rs).

Overriding the feature flags allows to see how having some feature on or off affects the runtime.
For example, if there is a new feature that improves the performance of MoveVM, with overrides it is possible to evaluate it on past transactions.

### Comparison to on-chain behavior

Overriding the state can change the execution behavior.
Hence, if any overrides are provided, the tool compares the on-chain outputs to new outputs obtained when execution on top of a modified state.
The diff of comparison is logged, and the users of the tool can evaluate if the differences are significant or not.
If the differences are not significant (e.g., only the gas usage has changed), the execution behaviour still stays the same.
Hence, the time measurements are still representative of the on-chain behavior.

### HTTP request rate limit quotas

Transactions are fetched from the fullnode via REST API.
Users should provide fullnode's REST API query endpoint using `--rest-endpoint E` flag.
For example, to fetch mainnet transactions, specify `--rest-endpoint https://mainnet.aptoslabs.com/v1`.

If too many transactions are fetched and executed (preparing the benchmark pre-executes the specified transactions and reads the state from the remote), it is possible to run into HTTP request rate limits.
To learn more about the API quotas, see https://developers.aptoslabs.com/docs/api-access/quotas.

It is possible to increase your quota by creating an API key in Aptos Build.
For that, follow instructions here: https://developers.aptoslabs.com/docs/api-access/api-keys.
Then, when using the tool the key can be specified using `--api-key K` flag.

### Examples

An end-to-end example for using the tool:

```commandline
aptos-replay-benchmark --begin-version 1944524532 \
                       --end-version 1944524714 \
                       --rest-endpoint https://mainnet.aptoslabs.com/v1 \
                       --concurrency-levels 2 4 \
                       --num-repeats 10 \
                       --num-blocks-to-skip 1 \
                       --enable-features ENABLE_LOADER_V2
```

Here, mainnet transactions from versions 1944524532 to 1944524714 are benchmarked.
There are two measurements: when Block-STM uses 2 threads, or 4 threads per block.
Each measurement is repeated 10 times, and the overall execution time is reported for each level.
Note that the reported time excludes the first block.
Additionally, `ENABLE_LOADER_V2` feature flag is forcefully enabled to see how it impacts the runtime for past transactions.
