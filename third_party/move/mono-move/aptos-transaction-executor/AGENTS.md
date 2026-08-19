# aptos-transaction-executor

The AptosVM transaction-execution layer on the MonoMove VM: one Aptos user
transaction, prologue → payload → epilogue, producing an unmaterialized
`TxnOutcome`. Block-level coordination lives above this crate.

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

There are critical issues we need to address before wiring up the transaction
executor to the block coordinator:

- Entry-function validation: `entry` visibility, no return values, admissible
  argument types. Without the visibility check a signed payload naming any
  loadable function runs it.
- Argument and signer checks: duplicate signers, signer counts, malformed
  arguments.
- `check_gas` pre-flight: transaction size, gas price bounds, intrinsic gas.
  Without it a zero gas price buys a full `max_gas_amount` budget.

## Modules

Modules stay private; anything public is re-exported from `lib.rs`, which holds
nothing else.

| Module | Purpose |
|---|---|
| `executor.rs` | `AptosTransactionExecutor`: the transaction lifecycle driver |
| `outcome.rs` | `TxnOutcome`: the unmaterialized conclusion, and `materialize()` |
| `errors.rs` | The typed outcome taxonomy: `DiscardReason`, `ExecutionStatus`, and the per-stage failure enums |
| `materialize/` | `txn_output.rs` drains the write set (with resource-group merge-back) into a `TransactionOutput`; `vm_status.rs` projects the taxonomy onto `VMStatus` |
| `providers.rs` | The `AptosDataProvider` trait: what write-set materialization needs from the data layer |
| `natives.rs` | The production native registry and the per-transaction native extensions |
| `calls.rs` | Making one function call in the shared interpreter context |
| `metadata.rs` | The slice of transaction metadata the prologue needs |
| `sys_calls.rs` | The unmetered prologue and epilogue calls |

## Testing

```bash
cargo test -p mono-move-aptos-transaction-executor
```

`tests/e2e.rs` builds genesis state with `FakeExecutor`, runs the same
transaction on the legacy VM and this crate, and compares status, write sets,
and events, masking only the gas-fee slots.
