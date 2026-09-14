# aptos-transaction-executor

The AptosVM transaction-execution layer on the MonoMove VM. User transactions
run prologue → payload → epilogue and produce an unmaterialized `TxnOutcome`;
system transactions run unmetered and fee-free. Block-level coordination lives
above this crate.

## Working assumptions

- Assume the latest feature set. All on-chain features and the latest gas
  feature version are enabled; supporting only that is sufficient. Do not port
  legacy validation paths, old gas versions, or feature-flag branches.
- Entry functions and scripts are the only supported payloads. Anything else
  is a `TODO(completeness)`.
- Gas is deliberately incomplete: MonoMove's units are uncalibrated, IO gas and
  storage fees are not charged. Do not treat a gas mismatch against the legacy VM
  as a regression.
- Past the prologue, a transaction always commits and charges the fee.
- Materialization is optional. Nothing on the execution path may call into
  `materialize/` -- it is up to the higher-level coordinator to decide when to
  call it.

## Before wiring to the block coordinator

Still open before the transaction executor can be wired to the block
coordinator:

- Argument deserialization has no deserializer yet for signed integers, so
  entry functions taking them are refused. Script payloads still decode their
  arguments natively, without the value checks.
- Multi-agent transactions are untested.

## Transaction arguments

An entry function's arguments are deserialized in Move by the VM-provided
`txn_arg` module (`txn_arg/`, compiled into `src/user_txn/txn_arg.mv`), whose
`deserialize<T>` the specializer resolves per concrete `T`. The executor
generates a script per entry function that deserializes each argument and
calls it (`user_txn/trampoline.rs`), so a transaction is one root call.
Framework constructors run as part of deserialization, so a bad `String` or
`Object<T>` aborts where AptosVM's would. Public structs and enums are
deserialized by a module the loader generates per defining module, calling
their `pack$` functions. A parameter type with no deserializer fails to lower,
which the executor reports as `INVALID_MAIN_FUNCTION_SIGNATURE`.

## Conventions

Modules stay private; anything public is re-exported from `lib.rs`, which holds
nothing else.

## Testing

```bash
cargo test -p mono-move-aptos-transaction-executor
```

`tests/e2e.rs` builds genesis state with `FakeExecutor`, runs the same
transaction on the legacy VM and this crate, and compares status, write sets,
and events, masking only the gas-fee slots.
