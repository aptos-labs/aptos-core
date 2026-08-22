# Framework Usage Coverage on Mainnet

Status: Implemented (initial version)

## Summary

This document proposes a tool that replays a mainnet ledger range with an
instrumented Move VM and reports which Aptos framework functions were invoked,
how often they were invoked, and which contracts invoked them. The report is
intended to provide evidence for framework feature deprecation decisions.

The implementation reuses archive replay and output verification, but it does
not reuse the existing Move bytecode coverage file format. Instead, the VM
exposes a low-overhead function-call observer. Replay workers aggregate
observations into deterministic JSON range shards, and a manually triggered
GitHub Actions workflow merges those shards into a canonical machine-readable
JSON report and a self-contained HTML view.

The first implementation requires sequential execution inside each replayed
transaction block. Replay ranges can still run concurrently, as they do today.
This avoids double-counting speculative Block-STM incarnations while preserving
most of the existing replay parallelism.

## Motivation

The framework accumulates APIs and implementation helpers over time. Before a
function or feature is deprecated, we need to know:

1. Whether the function is used on mainnet.
2. How many times it is invoked in a chosen ledger or time range.
3. Which published contracts depend on it.
4. Whether observed calls occurred in successful or aborted transactions.

Transaction payloads alone cannot answer these questions because a payload can
call through an arbitrary number of intermediate Move functions. Events and
write sets are also insufficient because many framework functions have no
unique state effect. Replay with VM-level function-entry instrumentation gives
the required dynamic call information while using the historical on-chain code
and state.

## Goals

- Count invocations of Move and native functions in framework packages.
- Attribute calls to the immediate caller module and the root transaction entry
  module.
- Associate usage with ledger versions and transaction outcomes.
- List framework functions with zero observed calls.
- Preserve archive replay's transaction-output verification.
- Bound worker memory and output size through local aggregation.
- Produce deterministic, mergeable output shards.
- Run over a version range or a UTC time range from a manually triggered CI job.
- Make incomplete reports, skipped ranges, and replay mismatches explicit.
- Add no meaningful overhead when collection is disabled.

## Non-goals

- This is not source-line or bytecode-instruction coverage.
- It does not prove that an unused function is safe to remove. Static references,
  upgrade compatibility, off-chain clients, and future scheduled use still need
  separate analysis.
- It does not initially support correct aggregation of speculative parallel
  Block-STM incarnations.
- It does not collect function arguments, local values, resources, or other
  potentially large execution data.
- It does not attempt to identify a legal or organizational owner for a caller
  address.
- Rust implementation details that are not exposed as Move functions are out of
  scope.

## Existing Infrastructure

### Move coverage

The existing Move coverage mechanism enables VM debugging and emits one text
line per executed bytecode instruction. Each line contains a function name and
program counter. `move-coverage` later converts those lines into counts indexed
by module, function, and bytecode offset.

That mechanism is useful for unit and end-to-end test coverage, but it is not a
good mainnet usage data source:

- It uses a process-global output path and writer.
- It emits one record per instruction rather than one record per function call.
- It has no transaction hash, ledger version, phase, outcome, or caller.
- Native functions have no Move bytecode and therefore do not appear.
- Its execution ID is currently a dummy value when parsing a raw trace.
- Writing and parsing a full instruction trace for a large mainnet range would
  create unnecessary I/O and storage cost.

The proposed observer is independent of `.mvtr` and `.mvcov`. Existing coverage
behavior and file compatibility remain unchanged.

### Archive replay

`aptos-debugger aptos-db replay-on-archive` already provides the core execution
behavior needed by this tool. It:

- Reads transactions and expected outputs from an archive database.
- Executes chunks against the historical state before the chunk.
- Splits work across independent ranges.
- Verifies the resulting transaction info, write set, and events.
- Handles epoch-ending transactions.

The replay-verify scheduler provisions archive snapshot clones, assigns ledger
ranges to Kubernetes workers, retries infrastructure failures, and cleans up
resources. The mainnet workflow already resolves human-readable UTC time inputs
to ledger versions.

The usage tool should share this machinery instead of creating a second replay
implementation.

## Terminology and Counting Semantics

### Function invocation

One invocation is counted when execution enters a function. This includes:

- A transaction entry function or script entrypoint.
- A non-generic or generic static Move call.
- A non-generic or generic native call.
- A closure call.
- A target reached through native dynamic dispatch.
- Recursive calls, each of which counts separately.

For native functions, the invocation is recorded immediately before calling the
native implementation. A native invocation therefore remains visible if the
native aborts. For Move functions, the invocation is recorded after the callee
frame has been successfully created.

Instruction counts are deliberately not collected. The number of times a
function executes a loop is not the number of times the function was called.

### Caller attribution

"Contract address" can mean several different things. The report retains all of
the useful interpretations instead of collapsing them:

- **Immediate caller module**: the module containing the call instruction. Its
  publishing address is the primary contract address in caller reports.
- **Root entry module**: the outermost module invoked by the transaction
  payload. This identifies the originating application when calls pass through
  other libraries.
- **Transaction sender and multisig address**: available while finalizing a
  transaction but intentionally not used as contract attribution or retained
  in aggregate report keys.

For example, given:

```text
0xaaa::application -> 0xbbb::library -> 0x1::coin
```

the immediate caller is `0xbbb::library`, the root entry module is
`0xaaa::application`, and the transaction sender is another independent field.

Direct transaction entry into a framework function has no Move caller. It is
represented with a `transaction` caller kind and the framework function as its
root entry function. Scripts and system transactions use their own caller kinds.

### Execution scope

The initial implementation observes calls made while executing user payloads.
It does not yet instrument prologue, epilogue, failure-cleanup, block metadata,
or other system sessions. As a result, an unused row means "not observed from a
committed user payload in this range," not "never used internally by Aptos."

### Transaction outcome

Usage is finalized with the committed transaction outcome:

- successful kept transaction
- kept abort
- discard

Aborted invocations are evidence of attempted use, but are reported separately
from successful use. Calls from speculative executions that do not become the
committed incarnation must not be counted.

## Target Framework Functions

The default target set is the exact set of module IDs in the framework release
bundle built into the replay image, covering:

- MoveStdlib
- AptosStdlib
- AptosFramework
- AptosToken
- AptosTokenObjects
- AptosTrading
- AptosExperimental

Filtering by exact module ID is preferred to filtering only by publishing
address. The run manifest records the image git SHA and a digest of the target
bundle so the meaning of "framework" is reproducible.

The tool enumerates function definitions from the framework release bundle
built into the replay image. This inventory supplies visibility, entry/native
flags, and zero-use rows. Historical functions absent from the current bundle
can still appear in call aggregates but do not receive an inventory row.

The initial report aggregates a stable `address::module::function` identity
across compatible upgrades. If revision-specific counts become necessary, the
schema can add the module bytecode hash to the aggregation key without changing
VM call semantics.

## VM Interface

Function usage is a separate concern from the existing runtime-check trace. In
particular, enabling usage collection must not make `TraceRecorder::is_enabled`
return true because that flag changes where paranoid runtime checks execute.

The Move VM extends its existing trace-recorder interface with a default no-op
callback:

```rust
pub trait TraceRecorder {
    fn record_function_call(
        &mut self,
        caller: Option<&LoadedFunction>,
        callee: &LoadedFunction,
        call_kind: FunctionCallKind,
    );
}
```

Existing recorders inherit the no-op default. Aptos VM wraps the selected trace
recorder only while an analysis sink is installed. The wrapper delegates
`is_enabled`, so collection cannot enable paranoid tracing behavior.

The interpreter invokes the callback at all call paths:

1. Initial entrypoint frame creation.
2. `Call` after successful frame creation, or immediately before native entry.
3. `CallGeneric` at the equivalent points.
4. `CallClosure` after resolution and frame creation, or before native entry.
5. The redirected target of native dynamic dispatch.

The callback receives stable function identifiers from `LoadedFunction`. It
does not receive arguments or values. Target filtering happens in the observer
before allocating an event.

The callback is infallible and does not change Move execution. Counter overflow,
serialization failure, or an incomplete collection causes the analysis command
to fail outside the VM.

## Aptos VM Integration

The Aptos layer supplies information the generic Move VM does not know:

- ledger version and transaction hash
- sender and multisig address while finalizing the transaction-local record
- root payload module
- final transaction status
- selected framework module set

A transaction-local recorder accumulates user-payload invocations. After VM
execution determines the final output status, the recorder converts its local
observations into a transaction usage record. Archive replay then joins the
transaction hash to its verified ledger version.

Aggregation first happens within a transaction. For each usage key, the
transaction recorder emits:

- the number of invocations in that transaction
- one distinct-transaction contribution

This gives exact invocation and distinct-transaction counts without storing a
row for every call.

Instrumentation is opt-in and installed only by the archive usage command. A
single process-wide sink registration is guarded against concurrent installs;
normal execution performs one cheap empty-sink check per user transaction. It
does not use or alter the existing global coverage file writer.

### Sequential execution requirement

The initial implementation requires VM block execution concurrency level one.
This guarantees that every transaction is executed once and eliminates
speculative incarnation accounting. The archive scheduler can still run many
independent range workers and many independent replay chunks in each worker.

The command rejects a greater VM concurrency level rather than silently
producing inflated counts.

A later parallel implementation should carry the transaction usage sidecar in
the block executor's speculative output and aggregate it only when that output
is committed. Keying a global counter by transaction hash is not sufficient
because stale speculative incarnations can finish in a different order.

## Data Model

Workers emit deterministic, versioned JSON. A future format change is permitted
through the schema version.

Conceptually, an aggregate key contains:

```text
callee address, module, function
immediate caller kind, address, module
root caller kind, address, module
transaction outcome
```

The value contains:

```text
invocation_count: u64
transaction_count: u64
first_version: u64
last_version: u64
```

Per-function totals are complete. Caller identities are transaction-controlled,
so detailed immediate-call paths use a global row limit. Root entry attribution
is collected separately with a row limit per callee; this prevents high-cardinality
traffic to one popular function from hiding which entry function reached an
otherwise rare framework function. Reports expose both limits and all dropped
counts. The HTML distinguishes an unobserved function from an observed function
whose detailed attribution was truncated.

Type arguments are not part of the key. They would create high cardinality and
are not needed to decide whether a generic function is used.

Each shard contains:

- schema version
- processed inclusive version range
- the replay image git SHA
- ledger timestamps for the first and last version in the range
- the target module list and complete function inventory
- processed ledger transaction and user-transaction usage record counts
- exact per-function aggregates, bounded per-function root-entry attribution,
  and bounded immediate-call-path aggregates

Each shard is named deterministically by its exact range. Retrying a range
overwrites its previous object, preventing retry double-counting.

## Archive Replay Command

The worker operation is exposed by a new command:

```shell
aptos-debugger aptos-db framework-usage \
  --start-version 1000000 \
  --end-version 1999999 \
  --target-db-dir /mnt/archive/db \
  --output /tmp/framework-usage-1000000-1999999.json \
  --html-output /tmp/framework-usage-1000000-1999999.html \
  --replay-concurrency-level 1 \
  --enable-storage-sharding
```

`--html-output` is optional. When present, the command writes a self-contained,
interactive report alongside the machine-readable JSON. The view puts
unobserved and rarely observed externally callable functions first, supports
search and visibility filters, and exposes immediate caller and root payload
paths. Its classifications are evidence for review rather than automated
removal decisions.

The framework usage path opens the archive database read-only and does not run
replay-on-archive's legacy write-mode initialization. This permits analysis of
read-only archive mounts and prevents the tool from creating newly introduced
database directories as a side effect.

In the example, `/mnt/archive/db` is the database mount inside the replay CI
worker. A local invocation must replace it with the root of an existing,
unpruned Aptos archive database; the repository does not include a mainnet
archive fixture.

The existing archive replay loop should be extracted into a shared internal
runner with an optional transaction-result consumer. Both replay verify and
framework usage then use the same transaction loading, epoch boundary handling,
execution, and output verification.

For every chunk, the usage command:

1. Loads transactions and their expected committed data.
2. Executes with the instrumented executor.
3. Verifies transaction info, write sets, and events exactly as replay verify
   does today.
4. Joins finalized usage with ledger versions and outcomes.
5. Aggregates into a worker-local map.
6. Atomically writes the completed shard.

A transaction-output mismatch fails the worker. A shard from a failed worker is
never accepted as complete.

## Range Completeness

Replay verify currently knows ranges that may be skipped for operational or
compatibility reasons. Silently applying those skips would make an unused
function result unsafe.

The usage workflow therefore follows these rules:

- By default, any skipped or unavailable subrange makes the run fail.
- The scheduler does not apply replay verify's known skip ranges in usage mode.
- The merger validates that successful shards are non-overlapping and cover the
  exact requested range.
- Empty ranges and archive start/end clipping are reported, not silently
  normalized.

## Distributed Execution and CI

The workflow is manually triggered and is separate from replay verify, for
example:

```text
.github/workflows/framework-usage-mainnet.yaml
.github/workflows/workflow-run-replay-verify-on-archive.yaml
```

Inputs mirror replay verify:

- `IMAGE_TAG`
- `START_VERSION` and `END_VERSION`
- `START_TIME` and `END_TIME`
- `DRY_RUN`

Version and time inputs remain mutually exclusive. UTC times are resolved to
versions before tasks are created, and the resolved range is included in the
manifest.

The replay scheduler is extended with a framework-usage worker mode while
retaining the same archive snapshot provisioning and cleanup behavior. The CI
workflow writes result shards to a run-specific prefix in the fixed
`aptos-framework-usage-reports` bucket. Before scheduling workers, it requires
that the bucket enforce public access prevention and uniform bucket-level
access. Object names use the requested range and workflow run identity. The
worker must successfully upload the shard before its Kubernetes job is
considered successful.

After every task succeeds, the GitHub runner downloads and merges the shards.
The merge step rejects:

- incompatible schema versions
- differing binary or target bundle identities
- overlapping ranges
- unexplained gaps
- failed replay verification

The final private report contains:

- `framework-usage.html`: self-contained interactive deprecation evidence view
- `framework-usage.json`: complete per-function totals, bounded root-entry and
  immediate-caller aggregates, truncation metadata, and run metadata; this is
  the canonical machine-readable report

The workflow publishes both the self-contained HTML and merged JSON under a
path unique to the workflow run and retry attempt on the private GitHub Pages
site in `aptos-labs/aptos-core-private`, using its `gh-pages` branch. The Actions
job summary links directly to both reports and shows its resolved UTC time
range, ledger-version range, and processed transaction count. Access to the
rendered report follows the private repository's GitHub access controls. The
repository and branch are fixed in the workflow rather than dispatch inputs.
Each publication also updates stable `framework-usage/<network>/index.html` and
`framework-usage/<network>/framework-usage.json` URLs. The merged JSON records
its UTC generation timestamp, which the HTML displays in its header.

The result bucket should have a lifecycle policy. CI cleanup always removes
pods and temporary PVCs, and final reports remain available through the private
Pages site.

## Report Interpretation

The JSON provides exact invocation and distinct-transaction counts, separates
successful and aborted outcomes, and retains immediate and root caller paths.
The HTML derives its deprecation view from that canonical data. A zero-count
function is a deprecation candidate, not an automatic removal candidate. A
review must additionally consider:

- static references from published bytecode
- public API and bytecode compatibility rules
- view functions called off chain, which committed transaction replay does not
  observe
- transactions outside the selected time range
- scheduled governance or operational use
- feature flags and rarely executed recovery paths

View-function usage is explicitly absent from mainnet transaction replay. If it
is needed, it requires API telemetry or another data source and must be labeled
separately from committed execution.

## Testing Strategy

The initial implementation includes an end-to-end Move execution test for
entry/static-call/root-caller attribution, a no-op tracing behavior test, and a
collector test for invocation versus distinct-transaction counts. The following
cases remain useful follow-up coverage, especially before expanding execution
scope or parallelism.

### Move VM tests

- Entry, static, generic, native, and closure calls.
- Native dynamic dispatch.
- Recursion and repeated calls.
- Script entrypoints.
- Native and Move aborts.
- No observations from the no-op observer.
- Coexistence with full runtime-check tracing.

### Aptos VM tests

- Immediate and root caller attribution through an intermediate contract.
- Success, kept abort, and discard outcomes.
- Prologue, epilogue, cleanup, and system sessions when those scopes are added.
- Target module filtering.
- No changes to gas, status, events, or write sets when instrumentation is
  enabled.

### Replay and aggregation tests

- A small archive range produces expected counts and still verifies outputs.
- Epoch boundaries and framework upgrades.
- Deterministic output independent of worker completion order.
- Exact distinct-transaction counts.
- Shard merge, retry overwrite, overlap rejection, and gap rejection.
- Schema, target identity, and overflow failures.
- Zero-use inventory generation.

### Performance tests

Benchmark archive replay with collection disabled and enabled. The observer
executes once per function invocation and filters non-framework callees before
allocation, so it should be substantially cheaper than instruction tracing.
Replay TPS, peak memory, shard size, and object-upload time should be recorded
before selecting final worker and range sizes.

## Implementation Layout

The implementation is split into four layers:

1. A generic Move VM function-call callback.
2. An opt-in Aptos VM transaction recorder and framework target filter.
3. The archive `framework-usage` command, inventory, and deterministic shard.
4. GCS shard upload, scheduler merge, and the manually triggered mainnet
   workflow.

Parallel Block-STM support is a follow-up. It should be implemented only by
associating usage with the committed incarnation, not by deduplicating an
already aggregated global trace.

## Alternatives Considered

### Extend `.mvcov`

Rejected. It lacks transaction and caller context, excludes native functions,
and produces instruction-scale data. Changing it would also mix test source
coverage with production dependency analysis.

### Analyze transaction payloads only

Rejected. It identifies only direct entry functions and misses transitive calls,
private helpers, closures, and native functions.

### Static bytecode call graph

Useful as a complementary report but insufficient by itself. It cannot resolve
all dynamic closure dispatch, cannot provide execution frequency, and treats
unreachable or dormant paths as usage.

### VM logging for every call

Rejected as the primary representation. Logs are expensive at this scale,
difficult to make exactly-once across retries, and awkward to validate for
complete range coverage. Structured, deterministic shards are easier to audit.

### Enable parallel Block-STM immediately

Deferred. Correctness requires collection from the committed transaction
incarnation. Sequential VM execution already matches the current archive replay
workflow configuration, while independent ranges preserve substantial
parallelism.

## Open Questions

1. Which object-storage bucket and Kubernetes service account should own result
   shard uploads?
2. What retention period is appropriate for raw shards and merged artifacts?
3. Should a future workflow expose package selection instead of always
   analyzing the complete built-in framework bundle?
4. Is an exact module-revision split required, or is stable function identity
   across compatible revisions sufficient?
5. What maximum acceptable replay slowdown should gate enabling the workflow on
   very large ranges?

The implemented defaults are a complete framework target set, stable function
identity across compatible upgrades, sequential VM execution, failure on any
range gap, and a successful-user-payload view as the primary deprecation signal.
