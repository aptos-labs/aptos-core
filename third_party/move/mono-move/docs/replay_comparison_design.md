# Replay Comparison Against AptosVM

How `mono-move-replay` checks that MonoMove produces the same transaction outputs as
AptosVM on real chain transactions, and why its verdicts can be trusted. For how to run the tool
and read its results, see the
[user guide](../replay/docs/GUIDE.md).

## 1. Motivation

MonoMove replaces the Move VM under the Aptos transaction executor. Unit tests and the
transactional testsuite cover what their authors thought of; mainnet covers what users actually
do. The tool replays real transactions on both VMs and compares their outputs, so that every
divergence on real traffic becomes a concrete, reproducible test failure.

Two VMs given the same transaction can still legitimately produce different outputs. The two
reasons are gas, which MonoMove does not meter yet, and state the replay never saw. A naive
comparison would drown real divergences in noise of this kind, or worse, call a divergence a
match. The design is organized around one rule:

> **No wrong verdicts.** Every outcome the tool reports is a statement it can back. Whatever it
> cannot decide gets a verdict that says so; it is never reported as a match or a mismatch.

---

## 2. Pipeline

```
 targets ──► capture ──► corpus ──► compare ──► results (JSONL) ──► aggregate
 (indexer)   (REST)        ▲          │                              (summary,
                           │          └─► replay (one record,         gap ranking)
 legacy dump ──► import ───┘              both outputs)
```

1. **Targets.** Optionally pick versions that cover many entry functions. A contiguous range is
   dominated by a few busy contracts.
2. **Capture / import.** Record each transaction with the state it read on chain and its full
   module closure (completed so that both replays find every key they read, for `import` and for
   `capture --complete`, §7.2), plus its on-chain status and auxiliary info, into a
   *corpus* (§7).
3. **Compare.** For each record: patch the state identically for both VMs (§3), run V1, keep the
   record only if V1 reproduces the chain's status when it is known (§4), run MonoMove in an isolated child process (§6),
   and diff the outputs (§5).
4. **Aggregate.** Merge many runs into one summary and a ranking of MonoMove's gaps.

---

## 3. Same state for both VMs

Both VMs run on a `PatchedState`: the record's state with a fixed set of overrides applied.
Only the override registry (`overrides.rs`) builds one, so the two VMs cannot see different
states.

| Override | Fires on | Why |
|---|---|---|
| `zero_gas` | every record | Every cost entry of `GasScheduleV2` is set to 0, and state-value metadata is stripped so a deleted slot cannot refund. MonoMove is unmetered, so gas is out of scope; zeroing it makes the outputs byte-comparable. |
| `integer_supply` | records that read the APT supply aggregator | The supply is rewritten as a plain integer. MonoMove does not implement the aggregator V1 natives, which mainnet reaches only through this supply. The abort codes of the two representations match, so the switch is visible only in the write set, which both VMs see identically. |
| `absent_features` | records of on-chain-framework corpora whose `Features` config was observed absent | Stores `Features` with every flag off. Early mainnet predates the config; without it the VM would fall back to a new chain's defaults, which enable paths the old framework lacks. A config that was never observed is left alone, so the record is `incomplete` rather than run on made-up flags. |

Gas needs more than a zeroed schedule. Some V1 charges are hard-coded outside the schedule:
value traversal on resource loads, and in natives. So V1 also runs the production meter with a
gas algebra that charges nothing (`ZeroChargeAlgebra`, `gas.rs`). The meter still enforces
dependency limits and tracks memory.

Overrides are applied at replay time. The corpus stores the state exactly as read from chain.

---

## 4. Deciding a verdict

`comparison::judge` decides each record. First the record's patched state is prepared; if that
fails, or the state cannot be read later, the verdict is `v1_error`. Then the checks run in this
order, and the first that applies decides:

| # | Check | Verdict |
|---|---|---|
| 1 | Transaction kind not replayed (only user, block-metadata and block-epilogue transactions are) | `skipped` |
| 1a | The transaction stopped at a metering limit on chain (out of gas, an execution/IO/storage/memory/dependency limit). It is not run: with gas zeroed, a loop that gas bounded on chain would never end. `capture` and `import` skip such transactions, and `replay` refuses them. | `not_comparable` |
| 2 | V1 read a key the corpus has no observation of (neither a value nor a recorded absence) | `incomplete` (vm `v1`) |
| 3 | V1 failed or panicked | `v1_error` |
| 4 | V1's status ≠ the on-chain status | `not_candidate` |
| 5 | V1 stopped at a metering limit (out of gas, execution/IO/storage/memory/dependency limit), as its VM status says: before gas feature version `RELEASE_V1_38` a memory-limit stop is kept as a plain `ExecutionFailure`, so on chain it is not recognized at step 1a; `capture` skips such a transaction, and `replay` does not run V2 after it | `not_comparable` |
| 6 | MonoMove's process exceeded its memory or time limit, read more than 1,000,000 distinct keys, or died | `mono_crash` |
| 7 | MonoMove read an unobserved key | `incomplete` (vm `v2`) |
| 8 | MonoMove could not be set up, or panicked | `mismatch` |
| 9 | MonoMove reported a feature it lacks (§5.2) | `mono_unsupported`, or `mismatch`; `not_comparable` for a native V1 lacks too (V1 failed with `MISSING_DEPENDENCY`) |
| 10 | Materializing MonoMove's output failed | `mismatch` |
| 11 | The outputs differ (§5) | `match` / `known_difference` / `mismatch` |

The order is what makes each verdict trustworthy:

- **Unobserved state is checked before anything is believed.** V1 on unobserved state is not V1
  on chain state, so neither its errors, the candidate filter, nor the comparison can trust it
  (#2 before #3–#5). MonoMove's reads are checked before its panics, gaps or outputs are judged
  (#7 before #8–#11). On unobserved state it may have failed only because the corpus guessed a
  key absent.
- **The chain filters candidates; V1 is the reference.** A record with an on-chain status is compared
  only if V1, on the patched state, reproduces it (#4); one without (an offline import) is compared
  unchecked, which the summary counts. After that, MonoMove is compared against
  V1's output, not the chain's. Zeroed gas and stripped metadata make the replayed write set
  differ from the committed one, identically for both VMs.
- **A crash is judged before completeness.** A runaway MonoMove can read keys V1 never does.
  Those reads must not turn a crash into `incomplete` (#6 before #7). The verdict lists a sample
  of the unobserved keys the crash read, for triage.
- **Harness failures are not verdicts.** If the isolation harness itself fails (§6.3), `compare`
  stops. Nothing is blamed on MonoMove.

`mismatch` and `mono_crash` are test failures. `aggregate` exits non-zero on either, and also when
no record reached the output comparison.

---

## 5. Comparing outputs

### 5.1 What is compared

`compare::diff_outputs` lists the differences in a fixed order: status, gas used, writes (by
key), native-position writes (by key), hot-state keys, events. Every status, gas, write,
native-position and hot-state difference is listed; none is
dropped, only classified. For events, it lists one count difference if the sequences differ in
length, and otherwise every position where they differ.

| Difference | Known? | Meaning |
|---|---|---|
| `Status` | no | Different `TransactionStatus`, including abort info |
| `GasUsed` | no | Both replays are gas-free, so any difference means the zero-gas setup broke on one side |
| `Write` | no | A write present on one side only, or with a different kind, bytes or metadata |
| `MissingMetadata` | **yes** | The same write, but only V1's carries state-value metadata. MonoMove does not emit metadata yet (`aptos-transaction-executor` `txn_output.rs`). |
| `UnchangedWrite` | **yes** | Only MonoMove writes a key, as a modification whose bytes equal the key's value before the transaction. MonoMove emits every copied-on-write entry as a write (same file). |
| `NativePosition` | no | A write in the native-position bucket, which the storage applier commits beside the main write set, present on one side only or different |
| `HotState` | no | Keys made hot on one side only |
| `EventCount` / `Event` | no | Events are emitted in a deterministic order, so the sequences must agree element by element |

`compare::classify` turns the list into a verdict. No difference is `match`. Only known
differences is `known_difference`: the VMs agree on what the transaction did, and the outputs
differ only by documented MonoMove TODOs. Anything else is `mismatch`.

`match` requires no difference at all. In practice, at the time of writing, agreeing records almost
always show a known difference: nearly every transaction writes metadata-carrying state, or has an
entry MonoMove rewrites unchanged. Once MonoMove emits metadata and stops writing unchanged
entries, such records become `match`.

### 5.2 Feature gaps

A MonoMove error that only means "not implemented yet" is a gap, not a bug. Examples are a missing
native, function values, and a stored value MonoMove cannot serialize yet. `mono_move_output::gap`
is the single place that decides which errors are gaps: exactly the errors the V1 mapping reports
as `NoV1Failure`. The transactional testsuite uses the same classifier. Gaps are reported with
their kind, and `aggregate` ranks them by frequency.

A gap must not hide a bug:

- Materializing an executed transaction attempts every write and every event, so all failures
  are reported. The gaps count only if *every* failure is a gap (`MaterializationError::gaps`).
- A gap hit while writing the output comes after execution, whose status is final. If that status
  differs from V1's, the record is a `mismatch`, not `mono_unsupported`.

---

## 6. Isolation

### 6.1 Why

A MonoMove bug can exhaust memory or never finish. #20699 is an example: the specializer loops
forever while lowering a function on a live mainnet path. That is beyond `catch_unwind`; in
process it would end the whole run with nothing written. So every MonoMove replay runs in a child
process, and its death is contained rather than the end of the run. In `compare` it is a verdict
(`mono_crash`); `replay` prints it; the state completion of `capture` and `import` keeps the record
and counts it (`v2_crashes`).

### 6.2 Protocol

The child is the same binary, re-run with the hidden `v2-worker` subcommand.

- **Request.** The parent writes the record's input on the child's stdin: its state, absences,
  override config, transaction and auxiliary info, plus the limits. The format is a u64 length
  followed by BCS.
- **Channel.** The child moves its real stdout to a private channel (`dup`), and points fd 1 at
  stderr, so nothing MonoMove prints can corrupt the protocol. Frames are length-prefixed BCS
  `Frame`s:
  - `Hello(protocol version, memory baseline)`: sent once the request is decoded and the state
    prepared, just before MonoMove starts.
  - `Read(key)`: each distinct key MonoMove reads, streamed as it happens, so a crash's reads are
    known.
  - `Done(outcome)` or `Failed(reason)`.

### 6.3 Limits and failures

- **Limits.** `--v2-memory-limit-mb` (default 4096) and `--v2-timeout-secs` (default 120).
  - Both the parent, which polls every 10 ms, and a watchdog thread in the child enforce them, so
    they hold even if the parent is gone.
  - They are measured from `Hello`: memory above the baseline the child reported, time from
    `Hello`.
  - Starting is held to a separate bound: the larger of the time limit and 60 s, and memory
    proportional to the request (three times its size plus 256 MB).
  - Whichever notices first stops the run. The child's watchdog exits with 97 (time) or 98
    (memory); the parent kills the child.
  - Before `Hello`, an overrun of the start-up memory allowance stops the worker only if it
    lasts two consecutive polls: a `Hello` the worker has sent may not have been read yet.
- **Budget.** Each child reserves its memory limit plus its startup allowance (three times its
  request plus 256 MB). Children run at once only while their reservations fit in a per-process
  budget: three quarters of the host's or container's memory (the cgroup limit, if lower), or
  `--v2-memory-budget-mb`. An explicit budget limits only how many children run at once, never
  how much each may use; it must hold at least the limit plus the minimum allowance of 257 MB.
  In every case, the memory limit is first lowered, if needed, so that it plus 257 MB fits the
  host-derived budget. A reservation larger than the budget still runs, alone.
- **Crash.** After `Hello`, exceeding a limit, a signal, or an abnormal exit is a `mono_crash`.
  An abnormal exit carries the last 4 KB of the child's stderr; a limit carries the limit's reason.
  A kill at a limit wins over an outcome the worker managed to write before it, if the parent had
  not read that outcome yet: the run exceeded the limit, which is the verdict. Once the parent has
  read an outcome, the limits no longer apply; the worker gets 2 s to exit, and the outcome stands.
  More than 1,000,000 distinct reads is also a crash: the run has run away.
- **Harness failure.** A failure of the harness, not of MonoMove, stops the command. That is
  anything that ends the child before `Hello` (it cannot start, speaks another protocol version,
  or exceeds the startup bounds); a `Failed` frame or a broken protocol at any point; and a
  normal exit without a result.
- **Worker binary.** On Linux the worker is `/proc/self/exe`. Elsewhere it is the current
  executable, pinned when the command starts and checked by file identity, so a rebuilt binary is
  never run as the worker.

---

## 7. Corpus

### 7.1 Format

```text
<corpus>/
  manifest.json    # identity, origin, shard index (with a SHA3-256 per shard)
  modules.pack     # every distinct module value, content-addressed across the corpus,
                   # and the framework sets
  shards/<n>.bcs   # a batch of records plus the distinct non-module values they read
```

- **Identity.** A value is identified by the SHA3-256 of its full BCS serialization, metadata
  included. Shared values are stored once per shard; modules once per corpus.
- **Framework sets.** Each framework release is stored once as a set of modules, and records refer
  to it. A legacy dump shrinks from 1.45 GB to 16 MB.
- **Record.** Each record holds the transaction, its auxiliary info, the values it reads, the keys
  it read and found absent, its framework set, and its on-chain status and gas when known.
  Offline imports have no on-chain status and no auxiliary info.
- **Integrity.** Shards are checked against the manifest's hashes when loaded. The format is
  versioned (`FORMAT_VERSION`).
- **Write once.** A corpus is written only to a new or empty directory (not a symlink, and with no
  `..` after a directory name), so one corpus never overwrites another, and a published corpus is
  immutable. The other commands refuse an output inside a corpus: `compare` and `replay` an `--out`
  inside it, hard-linked to one of its files, or a symlink to nothing; `aggregate` outputs that
  alias each other or a `--results` input.

### 7.2 State completion

A record holds what one V1 run read on chain, but a replay can read more:

- MonoMove reads keys V1 does not.
- The overrides change V1's path.
- Legacy dumps never recorded the keys V1 found absent.

`completion::complete` replays both VMs on the patched record state, exactly as `compare` will,
and fetches every key either reads that the record has no observation of, as a value or as an
absence. A fetched value can lead to new reads, so it repeats, for up to 8 rounds, until no key
is left. MonoMove is re-run only when a value it saw changed. A crashed MonoMove run's reads are
fetched once, best-effort, and only if it reported at most 10,000 of them. The record is kept
either way, so the crash stays in the corpus as a test case.

`import` completes by default (not with `--skip-completion` or `--offline`), since legacy dumps
lack absences and their feature flags are dropped.
`capture` completes only with `--complete`: V1's capture plus the framework and the module closure
has sufficed on every fresh mainnet capture so far, and completion costs a V1 rerun and a MonoMove
child per record. A record MonoMove reads beyond the capture compares as `incomplete` (`vm: v2`),
so nothing is judged on guessed state, and it can be recaptured with `--complete`.

### 7.3 Capture

- **Fetching.** Committed transactions, their status and their auxiliary info are fetched a shard
  at a time (so memory holds one shard), in pages over REST, and retried, so a transient failure (a rate limit) does not cost the run. The aux info
  is fetched directly, because the validator interface silently substitutes `None` when it fails,
  and a missing transaction index changes what natives such as `monotonically_increasing_counter`
  return.
- **What runs.** Only V1, on the chain state at the transaction's version, through a view that
  records every read (state completion is opt-in, §7.2). `--concurrency` (default 32) transactions
  are captured at once. A transaction that stopped at a metering limit on chain is skipped: with
  gas zeroed, a loop that gas bounded on chain would never end.
- **Framework.** A release is identified by the `code::PackageRegistry` values of `0x1`, `0x3` and
  `0x4`. A registry changes only by an upgrade, which adds a package or raises a package's
  `upgrade_number`, so it never returns to an earlier value. The releases of a shard are
  therefore resolved together: the registries are read at its first and last version, and only
  if they differ is the range split to find the upgrade. A shard that spans no upgrade costs two
  lookups. A version whose release cannot be looked up is skipped. Each release is fetched once per
  run (retried) and seeded into every capture of that release,
  so the prologue never misses a framework module.
- **Module closure.** The full module dependency closure is recorded, not only the modules the
  on-chain execution loaded, because MonoMove links the whole closure.
- **Skips.** A record that read a framework module outside its release is skipped, because the
  record refers to the release as a set and would lose that module.

### 7.4 Legacy import

Dumps written by `aptos-e2e-comparison-testing` lack the framework modules, the reads that found
no value, the on-chain status and the auxiliary info. `import` fills these in from an archive
node:

- **Framework.** The on-chain framework per release (`--framework onchain`), or the binary's own
  (`head`). The framework set owns its accounts' modules, so a record never mixes two frameworks:
  its copy of a module wins over one the dump recorded, a recorded module it lacks is absent, and
  a module it lacks is never fetched from chain (a dependency on one leaves the closure
  incomplete, and the record is skipped).
- **Status and aux info.** Fetched as in capture.
- **Feature flags.** The dump tool could override flags and record them as if read from chain,
  and nothing in a dump says whether it did. So recorded flags are always dropped and fetched
  again by completion.
- **Module closure.** User modules the legacy VM never loaded are fetched.

Chain reads are retried on transient failures, and cached under `--cache-dir`, scoped by chain
id and endpoint. The cache is best-effort: an entry that cannot be read or written only costs a
refetch. The cache directory and `--out` may not be inside one another; this is checked on the
real directories, by device and inode.

Legacy dumps predate the framework MonoMove targets. Its executor calls
`transaction_validation::versioned_prologue`, added in April 2026, so their records compare as
`mono_unsupported`. They remain useful for checking V1 replay.

### 7.5 Stopped runs

- **Per-record failures.** A failure with one record skips it, and the report counts skips by
  reason. That includes a version whose framework release cannot be looked up (its state pruned,
  say).
- **Run-level failures.** A failure that is not one record's stops the run: a harness failure, or
  a corpus write that fails. The records written so far are kept: the
  directory is marked with a `STOPPED` file that says why, and moved to `<out>.partial` (or
  `<out>.partial.<n>` if that is taken), leaving `--out` free for a complete run. If the move
  fails, the marked corpus stays at `--out`, and the error says so. `compare`
  opens such a corpus and warns about it.
- **Exceptions.** If `--out` is the working directory, it cannot be moved and stays in place,
  marked. If the marker cannot be written, the records are still written, but without a
  manifest, so they cannot pass for a complete corpus, and `compare` cannot open them.
- **Nothing written.** A run that stops before writing any record leaves `--out` as it was.

---

## 8. Limits of the approach

- **Gas parity is out of scope.** It needs a metered MonoMove and a separate mode with the real
  schedule.
- **No Block-STM.** Transactions replay one at a time against their recorded state. Parallel
  execution is not exercised.
- **Coverage is what the chain did.** A path no transaction in the corpus took is not tested.
  `targets` widens the coverage across entry functions, and `--per-function` within each one.
- **Known differences are trusted by kind.** A bug that happens to look exactly like a missing
  metadata entry or an unchanged write would be classified as known. Both are documented TODOs;
  once fixed, their classification should be removed.
