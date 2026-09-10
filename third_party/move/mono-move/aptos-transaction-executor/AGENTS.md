# aptos-transaction-executor

The AptosVM transaction-execution layer on the MonoMove VM. User transactions
run prologue → payload → epilogue and produce an unmaterialized `TxnOutcome`;
system transactions run unmetered and fee-free. Block-level coordination lives
above this crate.

## Working assumptions

- Assume the latest feature set. All on-chain features and the latest gas
  feature version are enabled; supporting only that is sufficient. Do not port
  legacy validation paths, old gas versions, or feature-flag branches.
- Entry functions are the only supported payload. Anything else is a
  `TODO(completeness)`.
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

- Entry-function validation: `entry` visibility, no return values, admissible
  argument types, constructed arguments. Without the visibility check a signed
  payload naming any loadable function runs it. See the `TODO(security,
  completeness)` in `user_txn/execute.rs`.
- Multi-agent transactions are untested.

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
