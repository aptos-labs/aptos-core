{{ frontmatter(name="move-replay", description="Replay a committed on-chain Aptos transaction locally to debug its outcome. Use when investigating a failed or unexpected transaction, reproducing an abort, or testing a local Move patch against a historical transaction.") }}

## When to Use This Skill

Use this skill whenever the user wants to:

- Understand why an on-chain transaction succeeded or failed (Move abort, execution failure, out-of-gas).
- Reproduce a transaction's behavior locally without re-submitting it.
- Test whether a *local* Move package fix would change the outcome of a committed transaction (regression check for a proposed patch).
- Inspect the storage reads a single transaction issued through the debugger.

The underlying tool is read-only: it fetches the committed transaction and aux info from the network, then executes it against the historical state. It does **not** mutate any on-chain state.

## Tool

Use the `{{ tool(name="move_replay_transaction") }}` MCP tool. Do not invoke the Aptos CLI's `aptos move replay` directly — this tool wraps it and returns structured JSON.

### Required Parameters

- **`txn_id`** (`u64`) — Committed ledger version of the transaction to replay.
- **`network`** (`string`) — One of `"mainnet"`, `"testnet"`, `"devnet"`, or a full REST endpoint URL (e.g. `"https://my-node.example.com/v1"`).

### Optional Parameters

- **`local_package_paths`** (`string[]`, default `[]`) — Paths to local Move packages whose modules override the on-chain versions during replay. Each path must point to a directory containing `Move.toml`. Use this to simulate a fix.
- **`named_addresses`** (`object`, default `{}`) — Named-address bindings (`{"name": "0xADDR"}`) used when compiling the local packages. Only consulted when `local_package_paths` is non-empty.
- **`node_api_key`** (`string`) — Bearer token sent as `Authorization: Bearer <key>` to the node. Use this when the public endpoint is rate-limited.
- **`trace`** (`bool`, default `false`) — When `true`, record a structured trace of debugger state-view requests (one `state_view { version, with_overrides }` entry per call) into the response. Off by default; tracing adds overhead. Only state-view requests are intercepted — the wrapper does not introspect Move bytecode execution itself.
- **`trace_storage_reads`** (`bool`, default `false`) — When `true`, additionally record one `storage_read` entry per state-view read. Off by default because a single replay typically issues hundreds of reads, which crowd out the higher-signal events. Only consulted when `trace` is `true`.
- **`max_trace_events`** (`usize`, default `500`) — Trace truncation limit. Only consulted when `trace` is `true`. Must be between `1` and the server-side cap of `100_000` (inclusive); requests outside that range fail fast with an `invalid_params` error. Raise it only if `truncated > 0` in the response.
- **`redact_storage_keys`** (`bool`, default `true`) — When `true`, storage-read trace entries omit the `Debug`-formatted `StateKey`. Only consulted when both `trace` and `trace_storage_reads` are `true`. Disable only when the key contents themselves are needed for debugging.

### Constraints

- Only **user** transactions are supported. Genesis, BlockMetadata, BlockEpilogue, StateCheckpoint, and ValidatorTransaction variants are rejected with a structured `invalid_params` error.
- The tool enforces a server-side timeout. If replay times out, suggest turning `trace_storage_reads` back off, dropping local overrides, or raising the server's `--tool-timeout`.

## Interpreting the Response

The tool returns a JSON object with these fields:

| Field | Meaning |
|---|---|
| `success` | `true` = `Keep(Success)`. `false` = `Keep(<any failure>)`. `null` = `Discard` or `Retry` (transaction was not committed in the normal sense). |
| `vm_status` | Human-readable VM status, same formatting as the Aptos CLI's `replay` command. |
| `abort` | Present only when the status is `MoveAbort`. Includes `location` (`"0xADDR::module_name"` or `"script"`), `code`, and optional `reason` / `description` if the module shipped abort metadata. |
| `execution_failure` | Present only when the status is `ExecutionFailure`. Includes `location`, `function` index, and `code_offset` within that function. |
| `transaction_hash` | Hex hash of the signed transaction. |
| `version` | Echo of the input `txn_id`. |
| `sender` | Sender address as a `0x…` hex literal. |
| `sequence_number` | Present when the transaction uses sequence-number replay protection; absent for orderless (nonce-based) transactions. |
| `gas_used`, `gas_unit_price` | Same as on-chain. |
| `local_override_in_use` | `true` iff `local_package_paths` was non-empty — i.e. the replay diverged from on-chain bytecode. |
| `trace` | Captured trace entries, only when `trace: true` was set on the request. Each entry is one of `state_view` (always exactly one per replay) or `storage_read` (zero by default; many when `trace_storage_reads: true`). |

### Reading the Status

1. **`success == true`** → transaction would commit normally. If the user expected a failure, double-check the inputs.
2. **`success == false` with `abort` populated** → a Move `abort` was hit. Report:
   - `abort.location` (which module),
   - `abort.code` (raw code),
   - `abort.reason` / `abort.description` if available (these come from `#[error]` / abort-info metadata in the module),
   - The matching constant in the source if the reason name is symbolic (e.g. `EINSUFFICIENT_BALANCE`).
3. **`success == false` with `execution_failure` populated** → a non-abort runtime failure (arithmetic overflow, type error, vector bounds, etc.). Report `location`, `function`, and `code_offset`; offer to disassemble the module if the user wants the exact bytecode site.
4. **`success == false` with neither populated** → likely `OutOfGas` or `MiscellaneousError`. The `vm_status` string carries the detail.
5. **`success == null`** → transaction was `Discard`ed or marked `Retry`. The `vm_status` string explains why; common causes are signature/validation issues that prevent execution.

## Workflows

### A. Plain Debugging — "Why did this transaction fail?"

1. Confirm with the user which `network` the transaction lives on.
2. Call `{{ tool(name="move_replay_transaction") }}` with just `txn_id` and `network`.
3. Read `success` first; then drill into `abort` or `execution_failure`.
4. If the user wants the source-level reason, query the module with `{{ tool(name="move_package_query") }}` (or read the module source) to find the constant matching `abort.code` / the function at `execution_failure.function`.

### B. Patch Testing — "Would my fix change this transaction's outcome?"

1. Ask for (or locate) the local Move package that re-implements the relevant module(s). It must be a buildable package with a `Move.toml`.
2. Determine the named-address bindings required to compile it. They must resolve every named address used in the package's source.
3. Call the tool with:
   - `local_package_paths` set to the package directory (or list of directories),
   - `named_addresses` mapping each name to its on-chain address,
   - the same `txn_id` and `network` as the failing transaction.
4. The response will have `local_override_in_use: true`. Compare its `success` / `abort` / `execution_failure` against the unmodified replay (workflow A) to see whether the patch changed behavior.
5. **Important**: if the patched module's bytecode is type-incompatible with the on-chain version (different public function signatures, removed structs, etc.), the VM will fail at link time — surface this clearly rather than treating it as a Move bug.

### C. Tracing — "Show me what the VM did step by step"

1. Start with `trace: true` alone. You will get exactly one `state_view { version, with_overrides }` entry — the state view the VM consumed for the run. With `with_overrides: false` you can confirm the on-chain path was taken; with `with_overrides: true` you can confirm the local-override path was taken. For most "why did this fail" questions this entry plus the structured `abort` / `execution_failure` fields are all you need.
2. Only set `trace_storage_reads: true` when you specifically need to see which `StateKey`s were touched during execution. Expect hundreds of entries per replay; raise `max_trace_events` (e.g. to `5000`) when you do this. Leave `redact_storage_keys: true` unless you need the `Debug`-formatted key bytes.
3. When reporting back, **enumerate the actual entries verbatim** — never collapse them to counts. The single `state_view` entry should always appear in the output you show the user; quote the `storage_read` entries one by one when the user asked for them.
4. If the response shows `truncated > 0`, the cap was hit. Either raise `max_trace_events` or turn `trace_storage_reads` back off.
5. The trace is at debugger-wrapper granularity — it does **not** introspect Move bytecode execution itself, so it cannot show the in-Move call frame that hit an abort. Use the structured `abort` / `execution_failure` fields plus a module query for that.

## Reporting Results

When summarizing for the user:

- Always quote `success`, `vm_status`, and the structured `abort` / `execution_failure` fields verbatim — these are the ground truth.
- When citing an abort, give both the symbolic reason (if present) **and** the raw code; the symbolic name can be absent for older modules.
- If `local_override_in_use == true`, label the result as "replayed with local overrides" so the user does not confuse it with the on-chain outcome.
- Do not speculate about state changes beyond what the tool returned. If the user wants deeper post-state inspection, suggest re-running with tracing enabled rather than guessing.
