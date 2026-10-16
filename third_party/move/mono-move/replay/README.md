# mono-move-replay

Replays real transactions from mainnet (or testnet, devnet) on both the legacy **AptosVM** (V1)
and the **MonoMove-backed Aptos transaction executor** (V2), and checks that MonoMove produces the
same output. It replays user transactions, block metadata and block epilogues.

Both VMs run the full transaction (prologue, payload, epilogue, materialization) on the same
state, with gas zeroed so the outputs are byte-comparable. Every record gets a verdict the tool
can back: it agrees, it differs only in MonoMove's documented known differences, it is a real
mismatch or crash, or it could not be decided, and then the verdict says why.

- **[User guide](docs/GUIDE.md)** walks through a real run end to end: picking transactions,
  capturing them, comparing, and reading the summary and per-record results, with actual output.
  It also shows how to triage a mismatch down to a filed bug.
- **[Design](../docs/replay_comparison_design.md)** covers how verdicts are decided and why they
  can be trusted: the state overrides, the gas handling, the isolation of MonoMove in a child
  process, the corpus format and state completion.

Timing MonoMove against AptosVM is a separate tool, `mono-move-replay-benchmark`
(`../replay-benchmark`). Code both tools use the same way (gas-free state, read capture, the
module closure, transaction labels, CLI types) lives in `mono-move-replay-common`
(`../replay-common`).

## Quick start

```bash
cargo build --release -p mono-move-replay
B=target/release/mono-move-replay

# Pick versions covering many entry functions (optional; a plain range works too).
$B targets --from-version 7509350000 --to-version 7509372000 --per-function 3 --out versions.txt
# Capture them into a corpus (add --api-key <KEY> to avoid rate limits).
$B capture --versions-file versions.txt --out corpus --corpus-id mainnet-sample
# Compare: one JSON verdict per record in the file, a summary on stderr.
$B compare --corpus corpus --out results.jsonl
# Look at one record's full output on both VMs.
$B replay --corpus corpus --versions <version> --vm both
```

## Commands

| Command | Does |
|---|---|
| `targets` | Lists versions calling every entry function in a window, `--per-function` each |
| `capture` | Captures a version range or `--versions-file` from a network into a corpus |
| `survey` | Describes a legacy dump from `aptos-e2e-comparison-testing` |
| `import` | Converts a legacy dump into a corpus, filling in what it lacks from an archive node |
| `compare` | Compares V1 and MonoMove on every record (or `--versions`) of a corpus |
| `aggregate` | Merges `compare` results into one summary and gap ranking; fails on any mismatch or crash |
| `replay` | Prints a record's output (status, gas, write set, events) on V1, V2 or both |

`--help` on any command lists its flags.
