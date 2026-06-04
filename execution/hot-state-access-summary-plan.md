# Hot-State Access Summary

This note documents how block hot-state promotion is computed and persisted, after moving the
promotion input off Block-STM's conflict-oriented `ReadWriteSummary` onto a deterministic
VM-boundary access summary.

## Architectural boundaries

- **Conflict accounting** stays a Block-STM / block-executor concern, driven by the speculative
  captured reads exposed as `ReadWriteSummary` and gated on `conflict_penalty_window()`. It is
  unchanged by this work.
- **Hotness observation** is a VM / execution-boundary concern. Reads are recorded at the VM
  resolver boundary; writes are derived from the final VM output.
- **Persisted hotness** is a transaction-output / write-set concern: the block epilogue output
  carries `BaseStateOp::MakeHot` ops, indexed by `StateUpdateRefs::index_write_sets`.

## Read recording (VM boundary)

`block_executor::hotness_recorder::HotnessReadRecorder` wraps the executor's
`ExecutorView + ResourceGroupView` and records every observed state key (value, metadata, exists,
size, resource-group, aggregator-v1 reads) into a deterministic `BTreeSet<StateKey>`. It is
installed in `block_executor::vm_wrapper` only when `hotness_in_epilogue()` is enabled, and the
recorded set is attached to the `VMOutput` (`set_hotness_reads`).

Module reads are NOT observed by the recorder (modules are served by the blanket-implemented
code/module cache, which cannot be wrapped, and warm-cache hits never reach the view). Module
hotness is instead sourced from the block executor's existing module-read tracking
(`CapturedReads`/`UnsyncReadSet::module_read_keys`), which sees module reads regardless of cache,
and unioned in at feed time.

Delayed-field ids do not map to hot-state KV keys and are excluded.

## Write derivation

Hotness writes are not stored; they are derived at feed time from `VMOutput::concrete_write_set_iter`
(`BeforeMaterializationOutput::hotness_writes`), which already yields resource, resource-group
(collapsed to the group key), aggregator-v1, and module write keys, and excludes delayed fields.
Deriving avoids a stored copy that could drift from the actual write set.

## Accumulation and the deterministic cap

`BlockHotStateOpAccumulator` collects keys read but never written in the block. The per-block
promotion cap (`max_promotions_per_block`) is applied over the final sorted candidate set in
`get_keys_to_make_hot` (take the N smallest from a `BTreeSet`), NOT while reads stream in. Streaming
the cap made the selected set depend on read-observation order, which is not deterministic across
validators and would produce divergent epilogue bytes / a state-root mismatch when the cap is hit.

## Gating

`hotness_in_epilogue()` is the single source of truth: it gates recorder installation,
accumulator instantiation/feeding (`BlockGasLimitProcessor`), and the V2-vs-V1 epilogue choice.
Hotness no longer depends on `conflict_penalty_window()` or `add_block_limit_outcome_onchain()`.

## VM-owned epilogue output and replay

The block-epilogue output's promotion set is produced by the VM path
(`block_executor::vm_wrapper::attach_hotness`), not by an executor-side patch. For a
`BlockEpiloguePayload::V2` it is `payload.to_make_hot ∪ epilogue_reads`, minus the epilogue's own
writes. `VMOutput::into_transaction_output` writes these as `MakeHot` ops in the output write set.

Replay parity is a V2-only guarantee: V2's `to_make_hot` is serialized, so transaction-output replay
reproduces hotness from the persisted output and transaction-input chunk re-execution recomputes the
epilogue reads from VM execution. V0 carries no hotness; V1 hotness is ephemeral (its `to_make_hot`
is not serialized), so V1 replay yields empty hotness.

## Out of scope

Sharded execution does not run through the VM-boundary recorder and may not accumulate/persist
hotness correctly; it is only kept compiling and non-panicking.
