## A tool to replay, benchmark and analyze past Aptos transactions

This tool allows to benchmark an ordered sequence of past transactions.
The tool supports four commands:

  1. `download`: Downloads transactions from the REST client and saves them locally into a single 
     file.
  2. `initialize`: Initializes the state for benchmarking, and saves it locally into a single file.
  3. `diff`: Compares execution outputs on two different input states.
  4. `benchmark`: Executes saved transactions on top of the saved state, measuring the time taken.


### Downloading past transactions

Users can download the past transactions using `download` command, specifying the first version
(`--begin-version B`), the last version(`--end-version E`), and the file where to save
them(`--transactions-file O`).
Downloaded transactions are split into blocks, to mimic on-chain behavior, so that later blocks can
be executed one-by-one using an executor.

Transactions are fetched from the fullnode via REST API.
Users should provide fullnode's REST API query endpoint using `--rest-endpoint E` flag.
For example, to fetch mainnet transactions, specify
`--rest-endpoint https://mainnet.aptoslabs.com/v1`.

If too many transactions are fetched and executed (preparing the benchmark pre-executes the
specified transactions and reads the state from the remote), it is possible to run into HTTP
request rate limits.
To learn more about the API quotas, see https://developers.aptoslabs.com/docs/api-access/quotas.
It is possible to increase your quota by creating an API key in Aptos Build.
For that, follow instructions here: https://developers.aptoslabs.com/docs/api-access/api-keys.
Then, when using the tool the key can be specified using `--api-key K` flag.

#### Example

```commandline
aptos-replay-benchmark download \
  --rest-endpoint https://mainnet.aptoslabs.com/v1 \
  --begin-version 1944524532 \
  --end-version 1944524714 \
  --transactions-file transactions.file
```
Saves transactions to `transactions.file` and outputs:
```commandline
Got 100/183 txns from RestApi.
Got 183/183 txns from RestApi.
Downloaded 24 blocks with 183 transactions in total
```

### Initializing the state for the past transactions

Users need to initialize the state for the past transactions they wish to benchmark via
`initialize` command.
To do that, one has to specify the file where blocks of transactions are saved
(`--transactions-file T`) and where the inputs will be saved (`--inputs-file I`).
Note that there are as many inputs as there are blocks.
This way when each block is benchmarked, it is executed against the pre-computed state, so there is
no "commit" of block execution outputs.

#### Example

```commandline
aptos-replay-benchmark initialize \
  --transactions-file transactions.file \
  --inputs-file inputs-onchain.file
```
Saves inputs to `inputs-onchain.file` and outputs:
```commandline
Generated inputs and computed diffs for block 1/24 in 10s
Generated inputs and computed diffs for block 2/24 in 14s
...
Generated inputs and computed diffs for block 24/24 in 42s
```

### Overriding the state for the past transactions

The benchmark runs every block on top of the saved state.
Importantly, it is possible to override the state.
Currently, the only supported overrides are feature flags:

  1. Feature flags can be forcefully enabled (`--enable-features F1 F2 ...`).
  2. Feature flags can be forcefully disabled (`--disable-features F1 F2 ...`).
  3. Gas feature version can be overridden (`--gas-feature-version V`).

Feature flags should be spelled in capital letters, e.g., `ENABLE_LOADER_V2`.
For the full list of available features, see [here](../../types/src/on_chain_config/aptos_features.rs).

Overriding the feature flags allows one to see how having some feature on or off affects the
runtime.
For example, if there is a new feature that improves the performance of MoveVM, with overrides it
is possible to evaluate it on past transactions.

#### Example

```commandline
aptos-replay-benchmark initialize \
  --transactions-file transactions.file \
  --enable-features ENABLE_LOADER_V2 \
  --inputs-file inputs-with-v2-loader-enabled.file
```


### Comparing the execution when using overridden state

Overriding the state can change the execution behavior.
The tool can also compare outputs when using different states with different overrides via `diff`
command.
The diff of comparison is logged, and the users of the tool can evaluate if the differences are
significant or not.
If the differences are not significant (e.g., only the gas usage has changed), the execution
behavior still stays the same.
Hence, the time measurements are still representative of the on-chain behavior.

By providing `--allow-different-gas-usage` flag, gas fees related differences will be left out of
comparison.
That is, differences in account balance, total APT supply, and gas used will be ignored.

#### Example

```commandline
aptos-replay-benchmark diff \
  --transactions-file transactions.file \
  --inputs-file inputs-onchain.file \
  --other-inputs-file inputs-with-v2-loader-enabled.file
```


### Benchmarking and measurements

To benchmark transactions, users need to specify the files where blocks of transactions are saved
(`--transactions-file T`) and where the inputs are saved (`--inputs-file I`).
Blocks of transactions are executed by a single executor instance one by one.
During the execution, the time is measured.
There is no "commit" of block execution outputs and the signature verification is done prior to
execution.
Hence, the benchmark reports the runtime only.

The tool supports two ways to measure the time:

  1. Measuring execution time for each of the executed blocks, and reporting all (default).
  2. Measuring total execution time for all blocks of transactions. To enable this, use
     `--measure-overall-time` flag.

In both cases, the measurement is repeated at least 3 times (this can be configured by specifying
the number of repeats, `N`, using `--num-repeats N`), and the minimum, maximum, average and median
times are reported (in microseconds).

When benchmarking, a list of concurrency levels (`--concurrency-levels L1 L2 ...`) has to be
provided.
Concurrency level specifies the number of threads Block-STM will use to execute a block of
transactions.
Typically, you want to have the concurrency level to match the number of cores.
If multiple concurrency levels are provided, the benchmark is run for all, reporting the
measurements.
This way it is possible to see how concurrency affects the runtime. 

Finally, in order to differentiate between cold and warm states, there is an option to skip the
measurement for the first few blocks.
By specifying `--num-block-to-skip N`, the tool will not ignore measurements when reporting for
the first `N` blocks.

#### Example

Benchmarking on-chain transactions (`ENABLE_LOADER_V2` is disabled).
```commandline
aptos-replay-benchmark benchmark \
  --transactions-file transactions.file \
  --inputs-file inputs-onchain.file \
  --concurrency-levels 4 8 \
  --num-repeats 5 \
  --measure-overall-time \
  --num-blocks-to-skip 2
```

Outputs:
```commandline
[1/5] Overall execution time is 1514304us
[2/5] Overall execution time is 1511947us
[3/5] Overall execution time is 1507239us
[4/5] Overall execution time is 1503065us
[5/5] Overall execution time is 1515648us
Overall execution time (blocks 3-24): min 1503065us, average 1510440.60us, median 1511947us, max 1515648us

[1/5] Overall execution time is 1509407us
[2/5] Overall execution time is 1552896us
[3/5] Overall execution time is 1814179us
[4/5] Overall execution time is 1520596us
[5/5] Overall execution time is 1581643us
Overall execution time (blocks 3-24): min 1509407us, average 1595744.20us, median 1552896us, max 1814179us
```

Benchmarking on-chain transactions with `ENABLE_LOADER_V2` enabled. 
```commandline
aptos-replay-benchmark benchmark \
  --transactions-file transactions.file \
  --inputs-file inputs-with-v2-loader-enabled.file \
  --concurrency-levels 4 8 \
  --num-repeats 5 \
  --measure-overall-time \
  --num-blocks-to-skip 2
```

Outputs:
```commandline
[1/5] Overall execution time is 725591us
[2/5] Overall execution time is 713757us
[3/5] Overall execution time is 726949us
[4/5] Overall execution time is 719480us
[5/5] Overall execution time is 720387us
Overall execution time (blocks 3-24): min 713757us, average 721232.80us, median 720387us, max 726949us

[1/5] Overall execution time is 721737us
[2/5] Overall execution time is 724505us
[3/5] Overall execution time is 747087us
[4/5] Overall execution time is 739608us
[5/5] Overall execution time is 746812us
Overall execution time (blocks 3-24): min 721737us, average 735949.80us, median 739608us, max 747087us
```