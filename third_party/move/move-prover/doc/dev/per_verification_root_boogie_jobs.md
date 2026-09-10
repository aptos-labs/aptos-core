# Per-Verification-Root Boogie Jobs

## Status

This document records the selected design and the experiments which led to it.
The implementation generates root-specific monomorphization results and Boogie
programs from one package model, then verifies them through a bounded streaming
process queue.

## Summary

The Move Prover builds one package model containing many verification roots. Its
Boogie backend has historically placed all roots and the package-wide generated
theory in a small number of files. Consequently, a verification condition can be
affected by quantified definitions which it does not use. This has made small,
otherwise stable proofs slow or seed-sensitive when verified with the package.

The selected design generates one minimal Boogie program for each verification
root and runs those programs through a bounded concurrent process queue:

```text
package model and transformed bytecode
                 |
                 +-- dependency graph --+--> root A semantic closure --> A.bpl
                                        +--> root B semantic closure --> B.bpl
                                        +--> root C semantic closure --> C.bpl

                                       bounded process queue
                                  +--------+--------+--------+
                                  |        |        |        |
                                Boogie   Boogie   Boogie   Boogie
                                  +--------+--------+--------+
                                             |
                              source-ordered diagnostic reporting
```

Each program retains ordinary Boogie function definitions. Isolation comes from
the file boundary, not from procedure-local quantified assumptions. Boogie runs
one process per program with one VC worker; the Move Prover supplies the
cross-program parallelism.

## Terminology

- **Verification root**: a `VerificationRoot`, identified by a Move function,
  verification variant, and type instantiation. It produces one top-level Boogie
  verification procedure.
- **Semantic item**: an instantiated definition or axiom needed to interpret a
  root, including Move functions, spec functions, types, memories, native
  models, generated axioms, and closure semantics.
- **Monomorphization slice** (`MonoSlice`): the transitive semantic closure of a
  verification root.
- **Boogie job**: one generated BPL file, its source map, options, root identity,
  artifact paths, and eventual raw process result.

Boogie's `-vcsSplitOnEveryAssert` can split one verification procedure into
multiple solver VCs. Initially, this design uses one BPL per verification root,
not one BPL per Boogie-internal assertion split. If finer isolation is needed,
assertions can later be forked into explicit Move verification variants.

## Goals

1. Prevent unrelated package semantics from affecting a root's proof.
2. Preserve the meaning of all reachable definitions and specifications.
3. Bound the number of simultaneous Boogie and solver processes.
4. Preserve deterministic diagnostics when stable output is requested.
5. Build the package model and transformed bytecode only once.
6. Make each root's selected semantics and performance inspectable.
7. Retain explicit seed racing, timeout retry, and hard timeouts.

## Non-goals

- This design does not weaken or approximate verification conditions.
- It does not make functions opaque based on a caller heuristic.
- It does not initially split every assertion into a separate BPL.
- It does not require persistent caching of generated files.
- It does not parallelize model construction or mutate `GlobalEnv` concurrently.

## Why a File Boundary Is Required

Concrete validity and equality predicates are efficient when represented as
ordinary Boogie definitions:

```boogie
function $IsValid_PackageMetadata(v: Value): bool {
    $PackageMetadataValidBody(v)
}
```

An experiment replaced selected definitions with procedure-local equivalences:

```boogie
assume (forall v: Value ::
    $IsValid_PackageMetadata(v) <==> $PackageMetadataValidBody(v));
```

Both direct assumptions and an importer procedure were logically local, but they
changed a Boogie definition into a triggered quantified formula. On
`object_code_deployment::freeze_code_object`, ordinary definitions verified
seeds 0 through 3 in approximately 0.8--1.1 seconds. The quantified encoding
timed out at 60 seconds for seed 0, and another seed remained in Boogie for more
than three minutes. Local import is therefore not an acceptable encoding.

Separate files let every selected symbol keep its ordinary definition while
preventing that definition from entering unrelated solver contexts.

## Complete Per-Root Semantic Slices

### Dependency graph

Monomorphization already discovers package-wide instances using worklists. The
analysis must retain why each instance was discovered. Graph nodes include:

- verification roots and function variants;
- Move function instances;
- spec function and spec variable instances;
- struct, vector, tuple, table, and native type instances;
- global memories;
- function types, closures, and behavioral predicates;
- generated validity and equality definitions;
- generated instance axioms and native models;
- user axioms and global invariants where visibility requires them.

Edges record semantic requirements, for example:

```text
verification root -> instrumented function and specification
function instance -> called function instances
function instance -> parameter, local, and result types
spec expression -> spec functions, memories, and types
type instance -> field type instances and validity/equality semantics
closure construction -> target, captures, and behavioral predicates
generated axiom -> every symbol and type used by the axiom
```

Dependencies must be collected from the final transformed targets consumed by
the Boogie backend. Source-level calls alone miss instrumentation, lifted
lambdas, function values, and generated specification operations.

### Fixed-point closure

For every root, the analyzer traverses dependencies to a fixed point. Recursive
and mutually recursive functions naturally contribute an entire strongly
connected component.

The implementation reruns only monomorphization analysis for each root over the
already-transformed targets. It does not rebuild the model or bytecode pipeline.
This directly produces a root-specific `MonoInfo`; the retained `MonoSlice`
continues to select concrete struct theory. Source specifications used to emit
behavioral variants and function-field access declarations are analyzed
explicitly because they are not always present in transformed baseline targets.

### Global kernel versus selected semantics

Every root file contains the small semantic kernel required by all programs:

- primitive Boogie sorts and the Move value representation;
- arithmetic and memory infrastructure which is truly type-independent;
- selected vector theory infrastructure;
- debugging and tracing primitives.

Everything instantiated or user-program-specific should be selected from the
root closure where sound:

| Item | Initial placement |
| --- | --- |
| Primitive kernel | Global in every root file |
| Root verification procedure | Selected |
| Reachable baseline Move procedures | Selected |
| Concrete types and memories | Selected |
| Validity and equality definitions | Selected |
| Instantiated spec function bodies | Selected |
| Native models and generated instance axioms | Selected |
| Function-value and behavioral semantics | Selected |
| Global invariants | Selected by their instrumentation dependencies |
| User axioms | Global until an explicit visibility rule is established |

An unclassified item remains global during rollout. A missing dependency or root
slice is an internal error; there is no havoc or weakening fallback.

## Generation Architecture

`GlobalEnv`, `FunctionTargetsHolder`, and `CodeWriter` are not designed for
concurrent mutation. BPL generation therefore remains on the model-owning
thread:

1. Build the model and run the bytecode pipeline once.
2. Enumerate verification roots in deterministic order.
3. Apply the timeout-estimate filter.
4. Compute a root-specific `MonoInfo` from the transformed targets.
5. Generate one BPL and source map for that root.
6. Submit the `BoogieJob` to the process queue.
7. Consume completed results before generating more jobs once the queue is full.

Generation and solving are pipelined. At most `proc_cores` jobs, including their
`CodeWriter` source maps and generated BPLs, are retained by the queue. This
bounds aggregate memory and temporary disk usage without introducing shard
barriers. Generation remains synchronous on the model-owning thread; already
started child processes continue running while the next root is generated.

Suggested artifact names are deterministic:

```text
boogie.vc_0042.bpl
boogie.vc_0042.log
```

The BPL header should include the human-readable root, variant, instantiation,
and semantic-item counts. The ordinal is only a filename component and never a
semantic identity.

## Concurrent Boogie Queue

### Runtime choice

The backend already depends on Tokio and launches Boogie with
`tokio::process::Command`. The current `ProverTaskRunner` races seeds for one
BPL. It is generalized to run all BPL jobs under one single-worker Tokio
runtime and one global semaphore. BPL generation remains on the model-owning
thread, while the worker polls child-process deadlines so generation cannot
delay cancellation. This avoids multiplying worker threads when test runners
invoke many prover instances in parallel.

No new scheduling crate is needed:

- `libtest-mimic`, used by transactional suites, is a test harness rather than a
  production job API.
- `aptos-bounded-executor` is a thin Tokio/semaphore wrapper. It does not provide
  per-root seed cancellation or ordered result aggregation, and depending on an
  Aptos crate from the Move prover would create an undesirable layering edge.
- Rayon is intended for CPU-bound in-process work, not asynchronously waiting on
  cancellable child processes.
- Tokio plus `FuturesUnordered` and `StreamExt::buffer_unordered` provides
  the required primitives.

### Resource accounting

`proc_cores` becomes the global maximum number of simultaneous Boogie processes.
Each root process receives `-vcsCores:1`; Boogie no longer owns package-level
parallelism. This prevents `queue_width * vcsCores` oversubscription.

Seed racing remains per root. When `num_instances > 1`, seed attempts also
acquire permits from the same global semaphore. The first successful attempt
completes the root job and drops the remaining futures; child processes use
`kill_on_drop(true)`. If sequential seed execution is requested, a root tries
its seeds one at a time without changing the global process limit.

Process isolation also removes solver warm state established by an earlier VC.
In one regression, the same VC and theory solved in 3.2 seconds after a preceding
VC but timed out alone at seed 1; isolated seeds 2--4 took 5.7, 1.7, and 0.7
seconds. Therefore, when no explicit seed race is configured, a deterministic
alternate seed begins after two thirds of the root's soft timeout. The
`--seed-handoff-ratio` option configures that fraction; zero disables the early
fallback. Successful fast roots run one Boogie process, and an INFO message
records each fallback launch. Stable-output tests prefer the primary result so
that a faster alternate does not change counterexamples.

The regression's intentionally false fold postcondition was also decomposed
with a stronger, valid `post assert`. This reduced five repeated primary-seed
runs to 3.2--4.5 seconds. Proof decomposition is the preferred local remedy;
the deterministic retry remains protection against isolation-induced solver
instability in arbitrary package VCs.

The BPL is already generated once per root and shared by seed attempts. Each
Boogie process nevertheless repeats parsing, inlining, passification, and VC
construction. Boogie's `/randomizeVcIterations` shares more frontend state, but
still regenerates the VC expression per seed; two iterations increased the
genesis root from 36.5 to 55.9 seconds. Reusing the final SMT query would require
solver-level retry support and is left as future work.

The soft VC timeout remains a Boogie procedure attribute. Each seed process gets
a watchdog deadline after acquiring a global process permit. An explicit hard
timeout takes precedence; otherwise the watchdog is at least five minutes and
at least four times the root's adjusted soft timeout. This deliberately leaves
room for Boogie parsing, type checking, inlining, and VC construction, which are
not covered by the solver-only soft timeout. A timed-out seed does not win the
race, so another deterministic seed can still complete the root. Timing out one
process does not cancel unrelated root jobs.

Embeddings with an outer request deadline also set a package deadline. The
pipeline stops admitting roots at that deadline, caps an admitted process by
the remaining time, and bounds aggregate diagnostics. This prevents a timed-out
request from continuing through one independent root after another.

### Raw execution versus diagnostics

Worker futures own only process-safe data: path, command-line options, root job
identifier, and timing metadata. They return raw stdout, stderr, exit status,
seed, and duration.

Boogie output parsing and diagnostic insertion happen as each process completes
on the model-owning thread, using the corresponding `CodeWriter` and
`GlobalEnv`. This provides:

- no `Send` or `Sync` requirement on the model or source maps;
- the same counterexample and trace rendering as the current wrapper;
- bounded retention of source maps and generated programs.

Normal runs report diagnostics in completion order. Stable-output tests use a
queue width of one to preserve root order while still exercising the same job
path. If several jobs have infrastructure failures, the lowest root ordinal is
returned.

Verification failures are accumulated across all jobs. A malformed BPL, missing
solver, or other infrastructure failure is reported separately from a failed
verification condition.

## Removal of Physical Sharding

Physical sharding existed to keep a package-wide BPL manageable and was also
used by the framework's daily prover test. Root-specific files provide the
memory boundary directly, while the bounded queue provides package-wide
parallelism. Retaining shard loops would only add sequential barriers and make
resource utilization depend on arbitrary hash buckets. The `--shards` and
`--only-shard` options are therefore removed; all selected roots enter one
queue.

## Decomposing a Large Root

Root isolation cannot reduce a single large verification procedure. The
`genesis::initialize_for_verification` root still took about 59 seconds after
isolation because it expanded every initialization phase into one VC.

The root is split at four explicit `opaque` boundaries. Each phase remains an
independent verification root, while its caller uses only its contract. An
opaque function with an incomplete `modifies` frame now produces a warning and
havocs every address of each unframed resource type its body can modify. This is
a sound but coarse summary; precise frames remain available when callers need
to preserve unrelated addresses.

This is an explicit specification choice, not a heuristic applied by root
isolation. With the boundaries and two required handoff postconditions, the
Genesis wrapper fell to about 13 seconds and the eight-process framework run
fell from about 172 to 126 seconds.

## Alternatives Tried

### Package-wide theory

The original encoding emits the package-wide monomorphization result and all
selected roots together. It minimizes parsing and process startup, and lets
Boogie parallelize VCs. It also exposes every VC to unrelated definitions and
quantifiers. This is the source of the observed package-only instability.

### Weakening or selectively disabling summaries

We considered omitting unsupported behavioral information, replacing it with
havoc, or suppressing precise facts when the immediate function did not appear
to use them. This is not compositional: a non-opaque function body can be
expanded in a caller or a caller's caller, where the information may be required.
The local function is therefore not a sound place to decide that a fact is
unnecessary. The performance design must preserve semantics instead of weakening
conditions.

### Procedure-local theory import

Direct assumptions and importer-procedure postconditions localized facts to an
entry procedure, but encoded ordinary definitions as quantified equivalences.
The resulting trigger behavior caused severe seed instability and timeouts. This
approach was rejected despite its attractive single-file structure.

### One selected theory per physical shard

The implemented first step unions struct slices for roots assigned to a shard and
emits ordinary validity/equality bodies only for that union. It successfully
removes definitions required solely by other shards, but unrelated roots in the
same shard still pollute one another.

`stake::join_validator_set` illustrates the remaining problem. In isolation it
verified in about 1--2 seconds. From the full shard payload, seed 0 took roughly
20 seconds and seed 1 timed out at 60 seconds, even when Boogie was instructed to
verify only that procedure. The surrounding definitional theory, not concurrent
VC execution, was sufficient to trigger the regression.

### Exact theory-key grouping

Grouping roots with identical `MonoSlice` values preserves ordinary definitions
and can reuse a file when slices match. On framework shard 2, however, 185 roots
produced 133 exact keys. With the current incomplete pruning, each generated file
was about 14.5 MiB. The projected output for that shard alone was approximately
1.9 GiB and required 133 Boogie startups. Exact keys therefore provide little
reuse in this workload.

A coarser, heuristic key would bound fragmentation only by reintroducing theory
pollution. It would also need a policy for deciding which definitions are safe to
co-locate. Per-root jobs provide a clear semantic boundary and let the process
queue address startup concurrency directly.

### One root by rerunning the whole prover pipeline

The benchmark and `--only` modes already demonstrate that independently building
one function produces a small, stable BPL. Rebuilding the compiler model and
bytecode pipeline for every root would repeat the most expensive front-end work,
consume substantially more memory under concurrency, and complicate diagnostic
aggregation. The selected design builds once and derives root views from retained
dependency provenance.

### Inlining semantic functions

Marking more Boogie functions inline changes term shape but does not remove
unrelated semantics. It can duplicate bodies and expose more quantifiers. It is
an optimization choice within one root program, not an isolation mechanism.

## Implementation

### Phase 1: Root-specific analysis

Run monomorphization analysis from one root over the existing transformed
targets, including generated behavioral and resource-access dependencies.

### Phase 2: Root-selected emission

Make each renderer consume a root slice explicitly. Generate one root BPL and
compare its declarations, definitions, and size with a fresh `--only` run for the
same function. Differences must be classified rather than silently accepted.

### Phase 3: Job production

Replace shard-wide and exact-theory-group loops with deterministic, bounded root
job production and remove the obsolete physical-sharding options.

### Phase 4: Bounded process execution

Refactor `ProverTaskRunner` into one invocation-wide Tokio queue. Set Boogie to
one VC core, enforce a global process permit count, preserve hard timeouts, and
retry isolated seed instability without exceeding the process limit.

### Phase 5: Result consumption

Separate raw process execution from output parsing. Consume each completed
result on the model thread, write its log, add source-mapped diagnostics, report
its timing, and clean artifacts before admitting another job.

### Phase 6: Validation and tuning

Run the Move Prover suite, compare results with the former sharded backend, and
benchmark the Aptos framework with queue widths 1, 2, 4, and 8. Measure
transformation time, generation time, solver time, peak memory, aggregate disk
output, process startup overhead, and seed stability.

The CLI benchmark uses this same queue with one process permit and records the
wall time of each actual root job. Roots for the same Move function are summed,
with `errors` taking precedence over `timeout` and `ok`. This replaces the old
benchmark loop, whose repeated mutation of one model could produce no
verification roots and report Boogie parsing time as successful verification.

Normal prover runs also print every root's solver wall time and status at INFO
level as its process finishes. This provides progress feedback and a low-overhead
comparison point without rerunning all roots sequentially in benchmark mode.

### Phase 7: Optional assertion-level jobs

If one root still contains independently unstable assertion splits, fork them in
the bytecode pipeline into explicit variants. Do not rely on extracting or
replaying Boogie's internal split representation.

## Correctness and Acceptance Criteria

The implementation must satisfy the following:

1. Every semantic item reachable from a root is defined in that root's BPL.
2. Missing provenance fails closed; there is no havoc fallback.
3. Generic instantiations, recursive components, function values, native models,
   invariants, and behavioral predicates are covered by tests.
4. One-root output verifies the same obligations as an existing `--only` run.
5. Queue width does not change verification results.
6. Diagnostic order is stable when stable output is requested.
7. Actual child-process concurrency never exceeds `proc_cores`.
8. Hard timeout and cancellation kill all seed children for the affected root.
9. The known package-only timeouts complete with the root-specific program.
10. Total runtime and memory improve on the former package sharding.
11. An embedding deadline bounds the whole root pipeline and its diagnostics.

## Open Questions

1. Which user axioms must remain package-global for backward compatibility?
2. Is assertion-level file isolation useful after root-level program isolation is
   in place?
