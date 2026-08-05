# aptos-transaction-executor

Reimplementation of the AptosVM transaction-execution layer on the MonoMove VM.
Executes one Aptos user transaction — prologue → payload → epilogue —
producing an unmaterialized `TxnOutcome`. Block-level coordination
(sequencing, Block-STM, block metadata) lives above this crate.

Proof-of-concept scope: entry functions only. Scripts, multisig, module
publishing, keyless/account-abstraction auth, orderless transactions, mempool
validation, and view functions are `TODO(completeness)` placeholders. Gas is
incomplete (see Key design decisions).

## Interface

- `AptosTransactionExecutor` is a thin handle borrowing everything from the
  block coordinator: the execution guard, the native registry, the data layers
  (`ModuleProvider` for code, the runtime's `ResourceProvider` for resources),
  and the config snapshot (features, epoch-boundary storage usage). It owns
  nothing it reads and never touches a `StateView` itself.
- In: `SignedTransaction` (signature verification is the caller's job).
- Out: `TxnOutcome` — the transaction's typed conclusion, carrying its side
  effects in VM representation. `materialize()` renders it into a
  `TransactionOutput`; only this step needs the wider `AptosDataProvider`,
  whose answers must agree with the provider that served execution.
  Block-level consumers will eventually read the effects directly.
- `StateView`-backed implementations of the data traits live in the sibling
  `mono-move-aptos-state-view-providers` crate (a dev-dependency here, used by
  the e2e tests); Block-STM integration provides the production
  implementations.

## Modules

Modules stay private; anything public at the crate level is re-exported from
`lib.rs`, which holds nothing else.

| Module | Purpose |
|---|---|
| `executor.rs` | `AptosTransactionExecutor`: the transaction lifecycle driver |
| `outcome.rs` | `TxnOutcome`: the unmaterialized transaction conclusion, and `materialize()` |
| `errors.rs` | The typed outcome taxonomy: `DiscardReason`, `CommitStatus`, and the per-stage failure enums |
| `materialize/` | Rendering into the storage-facing formats: `txn_output.rs` drains the write set (with resource-group merge-back) into a `TransactionOutput`; `vm_status.rs` projects the taxonomy onto `VMStatus` |
| `providers.rs` | The `AptosDataProvider` trait: what write-set materialization needs from the data layer |
| `natives.rs` | Native function wiring: the production native registry and the per-transaction native extensions |
| `calls.rs` | Making one function call in the shared interpreter context |
| `metadata.rs` | The slice of transaction metadata the prologue needs |
| `sys_calls.rs` | Unmetered framework system calls with Rust-built arguments: the transaction prologue and epilogue |

## Key design decisions

- **One session, checkpoints.** No per-stage change sets, no respawned
  sessions, no view overlays: prologue, payload, and epilogue run in one
  interpreter context, a failed payload rolls the heap and read-write set back
  to the post-prologue checkpoint, and writes are drained exactly once at the
  end.
- **Only the current feature path.** The versioned prologue/epilogue is the
  only validation path; legacy variants and old gas versions are intentionally
  not ported.
- **Gas is incomplete.** Execution is metered in MonoMove's uncalibrated
  units against the transaction's gas limit; IO gas, storage fees, and
  refunds are not charged yet, and the prologue/epilogue run unmetered.
  Gas amounts therefore diverge from the legacy VM by design; differential
  tests compare outputs with only the fee-embedding slots masked.
- **The write-set drain in `materialize/txn_output.rs` is a stopgap**, and where it should
  ultimately live is an open question. Real publication (modification
  detection, storage metadata, refunds) belongs inside the runtime, and the
  runtime already has `SessionEffects::write_set()` — but that path knows
  nothing about resource groups, which this drain handles via the data
  provider. Resolving the two (and how much of resource groups the VM layer
  should see at all) needs a design doc before either side hardens.

## Testing

```bash
cargo test -p mono-move-aptos-transaction-executor
```

`tests/e2e.rs` builds genesis state with `FakeExecutor` (dev-dependency), runs
the same transaction on the legacy VM and this crate, and compares status,
write sets, and events, masking only the gas-fee slots.