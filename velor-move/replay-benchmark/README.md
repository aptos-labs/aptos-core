## A tool to replay, benchmark and analyze past Velor transactions

This tool allows to benchmark an ordered sequence of past transactions. The tool supports four
commands:

  1. `download`: Downloads transactions from the REST client and saves them locally into a single 
     file.
  2. `initialize`: Initializes the input states for benchmarking, and saves them locally into a 
     single file.
  3. `diff`: Compares execution outputs on two different input states.
  4. `benchmark`: Executes saved transactions on top of the saved state, measuring the time taken.


### Downloading past transactions

Users can download the past transactions using `download` command, specifying the first version in
the range (`--begin-version B`), and the end version (`--end-version E`) (exclusive). Also, a file
where to save transactions should be specified (`--transactions-file T`). Downloaded transactions
are split into blocks, to mimic on-chain behavior, so that later blocks can be executed one-by-one
using an executor. The specified range of transactions must correspond to full blocks (i.e., it is
not possible to download only first few transactions in a block - only the whole block).

Transactions are fetched from the fullnode via REST API. Users should provide fullnode's REST API
query endpoint using `--rest-endpoint E` flag. Examples of endpoints are:
  - devnet: https://api.devnet.velorlabs.com/v1
  - testnet: https://api.testnet.velorlabs.com/v1
  - mainnet: https://api.mainnet.velorlabs.com/v1

#### Example

```shell
velor-replay-benchmark download \
  --begin-version 2232125001 \
  --end-version 2232125093 \
  --rest-endpoint https://api.mainnet.velorlabs.com/v1 \
  --transactions-file transactions.file
```
saves transactions to `transactions.file` and outputs
```text
Got 93/93 txns from RestApi.
Downloaded 12 blocks with 93 transactions in total: versions [2232125001, 2232125093)
```


### Initializing the state for the past transactions

To benchmark the past transactions the input state has to be also downloaded and initialized. This
can be done via `initialize` command. One has to specify the file where the blocks of transactions
are saved (`--transactions-file T`) and where the inputs will be saved (`--inputs-file I`). Note
that the inputs are generated for each block of transactions. This way when each block is executed,
it runs on top of the pre-computed state, so there is no "commit" of block execution outputs.

The state is initialized based on the data fetched from the REST API endpoint. You should specify
the same endpoint you used to download transactions (i.e., use the same network). This is done in
the same way with `--rest-endpoint E` flag.

State initialization executes transactions and captures read-sets for each block. If there are
many reads, it is possible to run into HTTP request rate limits. Most likely this happens when
some cryptic errors show up, e.g.:

```text
...
Failed to fetch state value for StateKey::AccessPath { address: 0x1, path: "Code(0000000000000000000000000000000000000000000000000000000000000001::randomness)" }: Other("HTTP error 429 Too Many Requests: error decoding response body: expected value at line 1 column 1")
2025-01-21T12:01:51.931988Z [tokio-runtime-worker] ERROR velor-move/block-executor/src/view.rs:1088 [VM, StateView] Error getting data from storage for StateKey::AccessPath { address: 0x1, path: "Resource(0x1::transaction_fee::VelorFABurnCapabilities)" } {"name":"execution","txn_idx":1}
2025-01-21T12:01:51.933342Z [tokio-runtime-worker] ERROR velor-move/block-executor/src/view.rs:1088 [VM, StateView] Error getting data from storage for StateKey::TableItem { handle: d1321c17eebcaceee2d54d5f6ea0f78dae846689935ef53d1f0c3cff9e2d6c49, key: 209d4294bbcd1174d6f2003ec365831e64cc31d9f6f15a2b85399db8d5000960f6 } {"name":"execution","txn_idx":10}
thread 'tokio-runtime-worker' panicked at velor-move/replay-benchmark/src/state_view.rs:67:17:
Failed to fetch state value for StateKey::AccessPath { address: 0x1, path: "Code(0000000000000000000000000000000000000000000000000000000000000001::string)" }: Other("receiving on a closed channel")
2025-01-21T12:01:51.934526Z [tokio-runtime-worker] ERROR velor-move/velor-vm/src/velor_vm.rs:2885 [velor_vm] Transaction breaking invariant violation.
...
```

To learn more about the API quotas, see https://developers.velorlabs.com/docs/api-access/quotas.
It is possible to increase your quota by creating an API key in Velor Build. In order to do that,
follow instructions here: https://developers.velorlabs.com/docs/api-access/api-keys. Then, when
using the tool the key can be specified using `--api-key K` flag.

#### Example

```shell
velor-replay-benchmark initialize \
  --rest-endpoint https://api.mainnet.velorlabs.com/v1 \
  --transactions-file transactions.file \
  --inputs-file baseline-state.file
```
saves inputs to `baseline-state.file` file and outputs
```text
Generated inputs for block 1/12 in 8s
Generated inputs for block 2/12 in 9s
...
Generated inputs for block 12/12 in 25s
```

### Overriding the state for the past transactions

The benchmark runs every block on top of the saved state. Importantly, it is possible to override
the state. Currently, the only supported overrides are the following:

  1. Forcefully enable a feature flag (`--enable-features F1 F2 ...`).
  2. Forcefully disable a feature flag (`--disable-features F1 F2 ...`).
  3. Forcefully override the gas feature version (`--gas-feature-version V`).
  4. Override existing on-chain packages (`--override-packages P1 P2 P3`). The paths to the
     packages must be the path to the source directories.

Feature flags should be spelled in capital letters, e.g., `ENABLE_LOADER_V2`. For the full list of
available features, see [here](../../types/src/on_chain_config/velor_features.rs).

Overriding the state can be very useful if you want to experiment with a new feature or Move code,
and check its performance as well as the gas usage. For example, if there is a new feature that
makes MoveVM faster, overriding it for past transactions it is possible to see the execution performance
on historical workloads.

#### Example

```commandline
velor-replay-benchmark initialize \
  --rest-endpoint https://api.mainnet.velorlabs.com/v1 \
  --transactions-file transactions.file
  --enable-features ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE \
  --inputs-file experiment-state.file
```


### Comparing the execution when using overridden state

Overriding the state can change the execution behavior. The tool allows one to compare execution
outputs when using different states with different overrides. This can be done via `diff` command.

For comparison, specify the transactions file (`--transactions-file T`), as well as a pair of files
where the inputs are stored (`--inputs-file I1` and `--other-inputs-file I2`) - the comparison will
be made for execution outputs on top of these two states. It is also possible to control the number
of threads Block-STM uses to execute transactions for diff computation with `--concurrency-level L`
flag. By default, sequential execution is used.

The diff of the comparison is printed to the console, and the users of the tool can evaluate if the
differences are significant or not. Ideally, they are minor so that the execution behavior for the
past transactions does not change. For example, if the override makes transactions cheaper, it is
very likely that all transactions behave in the same way, and the only differences in outputs are
the gas used, events associated with transactions fees (`FeeStatement`), total token supply (fees
are burned) and the balance of the fee payer. By providing `--allow-different-gas-usage` flag, the
differences related to gas will be left out of comparison.

#### Example

```shell
velor-replay-benchmark diff \
  --transactions-file transactions.file \
  --inputs-file baseline.state \
  --other-inputs-file experiment-state.file \
  --allow-different-gas-usage
```
prints gas usage to the console in a CSV format:
```text
block, baseline.state (gas), experiment.state (gas)
1, 35, 35
2, 26, 26
...
11, 622, 622
12, 2076, 2071
```
The only difference between executions are gas-related (feature override made blocks cheaper). If
`--allow-different-gas-usage` flag is not provided, then the diff is also logged:
```text
Non-empty output diff for transaction 2232125041:
<<<<<<< BEFORE
gas_used: 59
========
gas_used: 58
>>>>>>> AFTER
<<<<<<< BEFORE
event "0000000000000000000000000000000000000000000000000000000000000001::transaction_fee::FeeStatement" data: [59, 0, 0, 0, 0, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
========
event "0000000000000000000000000000000000000000000000000000000000000001::transaction_fee::FeeStatement" data: [58, 0, 0, 0, 0, 0, 0, 0, 52, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
>>>>>>> AFTER
<<<<<<< BEFORE
write StateKey::AccessPath { address: 0x68c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e06, path: "Resource(0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>)" } op Modification(5ca5adc500000000004200000000000000020000000000000068c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e062b00000000000000030000000000000068c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e06, metadata:StateValueMetadata { inner: Some(StateValueMetadataInner { slot_deposit: 50000, bytes_deposit: 0, creation_time_usecs: 1700555822755002 }) })
========
write StateKey::AccessPath { address: 0x68c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e06, path: "Resource(0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>)" } op Modification(c0a5adc500000000004200000000000000020000000000000068c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e062b00000000000000030000000000000068c709c6614e29f401b6bfdd0b89578381ef0fb719515c03b73cf13e45550e06, metadata:StateValueMetadata { inner: Some(StateValueMetadataInner { slot_deposit: 50000, bytes_deposit: 0, creation_time_usecs: 1700555822755002 }) })
>>>>>>> AFTER
<<<<<<< BEFORE
write StateKey::TableItem { handle: 1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca, key: 0619dc29a0aac8fa146714058e8dd6d2d0f3bdf5f6331907bf91f3acd81e6935 } op Modification(aa8dd92120d693010000000000000000, metadata:StateValueMetadata { inner: None })
========
write StateKey::TableItem { handle: 1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca, key: 0619dc29a0aac8fa146714058e8dd6d2d0f3bdf5f6331907bf91f3acd81e6935 } op Modification(0e8ed92120d693010000000000000000, metadata:StateValueMetadata { inner: None })
>>>>>>> AFTER
Non-empty output diff for transaction 2232125042:
...
```


### Benchmarking and measurements

Transactions can be benchmarked using `benchmark` command. Users need to specify the files where
blocks of transactions are saved (`--transactions-file T`) and where the inputs for each block are
saved (`--inputs-file I`). Blocks of transactions are executed by a single executor instance one by
one. During the execution, the time is measured. As mentioned above, there is no "commit" of block
execution outputs. Also, signature verification is done prior to execution and does not contribute
to the reported runtime.

The tool supports two ways to measure the time:

  1. Measuring execution times for each of the executed blocks, and reporting them (default).
  2. Measuring the total execution time for all blocks of transactions. To enable this, use
     `--measure-overall-time` flag.

In both cases, the measurement is repeated at least 3 times (this can be configured by specifying
the number of repeats, `N`, using `--num-repeats N`). the median, mean, minimum and maximum, times
are reported (in microseconds).

When benchmarking, a list of concurrency levels (`--concurrency-levels L1 L2 ...`) has to be
provided. Concurrency level specifies the number of threads Block-STM will use to execute a block
of transactions. Typically, you want to have the concurrency level to match the number of cores. If
multiple concurrency levels are provided, the benchmark reports the measurements for each level.
This way it is possible to see how concurrency affects the runtime. 

In order to differentiate between cold and warm starts, there is an option to skip the measurement
for the first few blocks. By specifying `--num-block-to-skip N`, the tool will ignore measurements
for the first `N` blocks (the blocks will still be executed as a "warm-up").

Execution can also be configured. By using `--disable-paranoid-mode`, the Move VM will not use
runtime type checks, possible making execution faster.

#### Example

Benchmarking on-chain transactions (`ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE` is disabled):
```shell
velor-replay-benchmark benchmark \
  --transactions-file transactions.file \
  --inputs-file baseline-state.file \
  --num-blocks-to-skip 2 \
  --concurrency-levels 4 \
  --num-repeats 31
```
prints measurements for 10 blocks to the console
```text
concurrency level, block, median (us), mean (us), min (us), max (us)
4, 2, 10701, 11137.74, 10170, 22922
4, 3, 11678, 11949.84, 11459, 20218
4, 4, 5341, 5348.23, 5164, 5616
4, 5, 53871, 54126.52, 53237, 58044
4, 6, 16334, 16314.32, 15856, 16596
4, 7, 7845, 7844.77, 7634, 8032
4, 8, 13140, 13113.45, 12854, 13521
4, 9, 127062, 117830.45, 70342, 169842
4, 10, 6305, 6343.45, 5860, 7042
4, 11, 51917, 51963.16, 51508, 53646
```

Benchmarking with `ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE` feature enabled:
```shell
velor-replay-benchmark benchmark \
  --transactions-file transactions.file \
  --inputs-file experiment-state.file \
  --num-blocks-to-skip 2 \
  --concurrency-levels 4 \
  --num-repeats 31
```
shows speedups for certain blocks:
```text
concurrency level, block, median (us), mean (us), min (us), max (us)
4, 2, 11102, 12927.03, 10705, 42065
4, 3, 11733, 12226.06, 11494, 20036
4, 4, 5400, 5476.61, 5259, 6343
4, 5, 44129, 45534.39, 43544, 61366
4, 6, 16373, 16173.23, 10992, 23235
4, 7, 8086, 9900.55, 7799, 35909
4, 8, 12986, 13318.58, 9773, 25228
4, 9, 127551, 124062.03, 71639, 229356
4, 10, 6468, 6828.61, 5964, 11862
4, 11, 42064, 43327.74, 41656, 68311
```
