{{ frontmatter(name="move-replay", description="Replay a committed Aptos user transaction locally. Use to diagnose an on-chain outcome or compare it with local Move package overrides; this does not resubmit the transaction.") }}

## Replay workflow

Use `{{ tool(name="move_replay_transaction") }}` rather than invoking the Aptos
CLI directly. The tool reads the committed transaction and historical state,
then executes locally; it does not mutate on-chain state.

You need the committed ledger `txn_id` and its network (`mainnet`, `testnet`,
`devnet`, or a REST endpoint). Ask only when those cannot be determined from the
request or workspace.

### Diagnose the recorded transaction

1. Replay without local overrides or tracing.
2. Read `success` and `vm_status`, then use structured `abort` or
   `execution_failure` fields when present.
3. For a Move abort, report its module, raw code, and symbolic reason/description
   when available. Locate the matching source constant only when source-level
   explanation is useful.
4. For an execution failure, report its location, function index, and bytecode
   offset. Do not invent a source line without disassembly or source evidence.
5. `success: null` represents a discarded/retried status rather than a normal
   committed execution; explain it from `vm_status`.

### Compare a local patch

First capture the unmodified replay. Then pass buildable package roots in
`local_package_paths` and supply only the named-address bindings needed to
compile them. Confirm `local_override_in_use: true` and compare the structured
status with the baseline. Label the result as a local simulation, not the
historical on-chain outcome. A link/type incompatibility is a compatibility
failure, not evidence that the patch fixed the transaction.

### Trace only when needed

Start with `trace: true`, which records debugger state-view use. Enable
`trace_storage_reads` only when the user needs touched storage keys; it can
produce hundreds of events. Keep keys redacted unless their contents are
necessary, and raise `max_trace_events` only when the response reports
truncation. This trace does not expose Move call frames, so use the structured
failure plus source/bytecode inspection for the failing instruction.

Report exact status fields, but summarize long traces unless the user asks for
the individual entries. Never print an API key supplied for node access.
