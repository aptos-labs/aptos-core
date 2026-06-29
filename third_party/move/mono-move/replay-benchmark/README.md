# mono-move-replay-benchmark

Measures the execution-time speedup of **MonoMove** (V2) over the legacy Move VM (V1) by replaying
a real mainnet entry-function transaction on both. As a sanity check it also compares
the two outcomes coarsely (success / Move abort code / failure kind), so a speedup is only reported
when the VMs agree on what the transaction did.

It measures the bare VMs directly — there is no Aptos-VM prologue/epilogue, no Block-STM, and no
write-set formation in the timed region.

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

# 2. Benchmark every entry-function transaction in the dump on both VMs.
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

All setup is done once up front; the timer wraps only "deserialize/place the entry arguments +
execute the entry function". Each VM's per-run state reset is applied **outside** the timer.

- **V1 (legacy Move VM):** lazy module loading warmed by an untimed trial run, paranoid type checks
  off, a fresh empty data cache per run (resources are read + deserialized from the read-set during
  execution). Gas is *not* metered.
- **V2 (MonoMove):** lazy loading/lowering, the execution heap pre-allocated once and reset per run,
  resources served from a read-set-backed provider. Gas *is* metered.
