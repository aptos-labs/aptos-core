# mono-move-replay-benchmark

Measures the execution-time speedup of the **MonoMove-backed Aptos transaction executor** (V2)
over the legacy **AptosVM** (V1) by replaying a transaction captured from a real network
(mainnet, testnet, or devnet) on both. Entry-function user transactions, block-metadata
transactions, and block-epilogue transactions are supported.

The two transaction outputs are compared strictly (status, write set, events, barring gas), and
a speedup is only reported when the VMs agree on what the transaction did.

Each side executes the full transaction — prologue, payload, epilogue, and materialization into a
`TransactionOutput`. **Gas is disabled** for both VMs completely, so the outputs carry no fee effects
and are byte-comparable. This is done by setting all non-structure entries in the gas schedule to 0,
and purging all state metadata to prevent refunds.

There is no Block-STM; execution is sequential against the captured read-set.

## Usage

The CLI has two subcommands: `capture` (fetch transactions from chain into an on-disk dump) and
`bench` (replay a dump on both VMs).

```bash
# 1. Capture a version range into a dump. Writes <version>_txns / <version>_inputs into --out-dir.
#    An API key avoids the low anonymous rate limit. --network defaults to mainnet
#    (testnet / devnet / a custom REST endpoint URL also work).
cargo run -p mono-move-replay-benchmark -- capture \
    --api-key <KEY> \
    --begin-version 5663916074 --end-version 5663916090 \
    --out-dir dump/

# 2. Benchmark every supported transaction in the dump on both VMs.
cargo run --release -p mono-move-replay-benchmark -- bench \
    --data-dir dump/ \
    --warmup 50 --samples 200 --limit 20
```

`capture` records each transaction together with the full module dependency closure it needs, so a
cold replay can resolve every module (not just the ones the original on-chain execution loaded).

There is also an option to benchmark a single transaction at a time.

```bash
cargo run -p mono-move-replay-benchmark -- bench \
      --transactions-file data/5663916074_txns \
      --inputs-file data/5663916074_inputs \
      --warmup 50 --samples 2000
```

For profiling, it is usually convenient to record measurements per VM.
For that, use `--vm (both | v1 | v2)` flag when running the benchmark.
For example, collecting the profile with samply for V1 VM is simply:

```bash
cargo build --release -p mono-move-replay-benchmar

samply record \
      ./target/release/mono-move-replay-benchmark bench \
      --transactions-file data/5663916074_txns \
      --inputs-file data/5663916074_inputs \
      --vm v1 --warmup 50 --samples 2000
```

## What is measured

Environments, providers, and caches are built once up front, and an untimed trial run warms the
module caches. The timer wraps one full transaction: validation, execution, and materialization
into a `TransactionOutput`.
