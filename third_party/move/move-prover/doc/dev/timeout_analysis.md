# Solver Timeout Analysis

## Status

Implemented from the revised plan. The initial design was validated against
the current one-verification-root job pipeline and the pinned Boogie 3.5.6 /
Z3 4.13.0 toolchain before implementation.

## Motivation

A verification timeout currently reports the affected source location and the
configured timeout, but gives no evidence about what occupied the solver. The
most useful low-volume evidence Z3 exposes is:

- aggregate solver statistics from `-st`;
- per-quantifier instantiation counts from `smt.qi.profile=true`;
- periodic cumulative quantifier counts from `smt.qi.profile_freq` when the
  solver does not shut down gracefully.

The feature should surface that evidence in the existing timeout diagnostic.
It must not change default verification, exceed the prover's process limit or
request deadline, or turn an optional analysis failure into a verifier failure.

## User interface and scope

Add `timeout_analysis: bool` to `BoogieOptions`, exposed as
`--timeout-analysis` and defaulting to `false`.

The first version is Z3-only. Combining `timeout_analysis` with `use_cvc5` is a
configuration error. Supporting other solvers requires a separate parser and
evidence model.

MoveFlow integration is deliberately outside this change. MoveFlow owns its
tool schema, caching policy, and request budget, and is being changed
independently.

## Execution model

```text
selected Boogie seed
    |
    +-- seed-qualified -proverLog captures every internal VC
    |
    +-- success / assertion failure --------------------> normal result
    |
    `-- soft timeout or out-of-resource
            |
            +-- find this seed's timed-out SMT captures
            +-- create instrumented replay copies
            +-- acquire the shared prover-process semaphore
            +-- run bounded Z3 replays
            `-- return raw replay evidence with Boogie output
                            |
                            `-- parse and source-resolve on the model thread
                                and attach notes to the timeout diagnostic
```

The replay is post-processing of the seed selected by the existing seed race.
It is not performed by every losing seed. It acquires the same semaphore as
Boogie, so total child-process concurrency remains bounded by `proc_cores`.
The model and `CodeWriter` remain on the result-consumption thread; worker tasks
return only process-safe raw evidence.

Only solver soft timeouts and solver out-of-resource results with a usable SMT
capture are analyzed. A killed Boogie process, package deadline, missing
capture, malformed capture, failed Z3 launch, or failed replay leaves the
original timeout result intact. A concise `analysis unavailable` or
`analysis incomplete` note may be attached, but analysis never creates an
internal prover error.

## SMT capture and discovery

When timeout analysis is enabled, each Boogie seed receives a distinct log
pattern next to its BPL:

```text
<bpl-without-extension>.seed-<seed>.@PROC@.smt
```

The seed component prevents the primary and fallback processes from
overwriting one another. Boogie can also expand one procedure into internal
assertion splits such as `_split0` and `_split1`; therefore the implementation
does not reconstruct one filename from the displayed procedure name. It scans
only the selected seed's fixed prefix, reads `:boogie-vc-id` from each capture,
and selects captures whose terminal Boogie status is timed out or out of
resource. This also avoids depending on a reimplementation of Boogie's filename
sanitization.

The generated command-line capture option is placed after custom Boogie flags
so a custom `-proverLog` cannot silently disable analysis. `generate_smt` keeps
its existing behavior when analysis is off; when both options are on, the
seed-qualified analysis captures are the retained SMT artifacts.

Captures and replay output are placed with the BPL and follow
`keep_artifacts`. Normal prover runs already use a temporary directory when
artifacts are not kept. Explicit cleanup remains best-effort and never removes
an unrelated path.

## Faithful, bounded replay

The captured SMT contains the effective random seed, eager/lazy quantifier
thresholds, resource limit, and other options selected by Boogie. Repeating
those settings separately on the Z3 command line is both redundant and unsafe:
SMT `set-option` commands can override command-line settings.

For each timed-out capture, create a replay copy which preserves the original
query and inserts these options immediately before each `check-sat` or
`check-sat-assuming`:

```smt2
(set-option :smt.qi.profile true)
(set-option :smt.qi.profile_freq 1000)
(set-option :timeout <analysis-budget-ms>)
```

Run the configured Z3 executable with `-st` on the copy. Standard output and
standard error are redirected to files so periodic profile records survive a
forced kill. The captured file, rather than separately reconstructed options,
provides the selected seed and QI thresholds.

The nominal analysis budget is the verification root's actual soft timeout,
including a function-level timeout override. It is shortened to fit the
remaining package deadline. If no positive budget remains, analysis is skipped.
A process watchdog fires two seconds after the inserted soft timeout, or at the
package deadline if sooner. On watchdog expiry the child is killed and waited
for, then any flushed periodic profile output is parsed as incomplete evidence.

The timeout shown in the base diagnostic is also changed to the actual root
timeout instead of always showing the global default.

## Quantifier identities and source resolution

Z3 does not merge different quantifiers which share a `:qid`; it can print
multiple indistinguishable profile rows with independent cumulative counts.
Every source emission therefore receives a unique explicit qid within a
generated BPL. Boogie can still duplicate one quantified expression while
lowering a VC, preserving the same qid on both copies.

When timeout analysis is enabled, the backend emits unique qids for:

- user-written `forall` and `exists` expressions, using a deterministic
  emission ordinal plus their Move source location;
- explicit recursive-spec-function defining axioms, using their source node,
  source-level qualified name, and an emission ordinal. The renderer uses the
  node to report the declaration location without exposing the Boogie name.

For a raw qid, the parser retains the maximum cumulative count observed. This
is a safe lower bound when Boogie duplicated the expression and also avoids
double-counting periodic cumulative profile rows. After resolution, distinct
raw qids which describe the same Move quantifier location are aggregated for
display. Explicit qids are emitted only under `timeout_analysis`, so the
default solver input is unchanged.

Unannotated Boogie quantifiers use auto qids containing a BPL line and column.
The fallback resolver:

1. parses the position from the right-hand side of the qid;
2. maps it through `CodeWriter::get_output_byte_index` and
   `get_source_location` when the position belongs to generated Move code;
3. otherwise scans backward in the BPL for the enclosing `axiom`, `function`,
   or `procedure`, including a nearby descriptive comment when present;
4. retains the raw qid if neither resolution is reliable.

Fallback output does not invent semantic names which are absent from the BPL.

## Evidence model and rendering

Solver statistics are observations from an instrumented replay, not proof of a
counterfactual cause. The first version therefore avoids unsupported
`dominated by` claims.

The diagnostic reports:

- aggregate `quant-instantiations` when Z3 reports it;
- up to five top resolved quantifiers;
- nonlinear-arithmetic evidence only from an explicit allowlist:
  `arith-nla-*`, `arith-grobner-*`, and `nlsat-*` counters;
- one factual signature: quantifier activity, nonlinear arithmetic activity,
  mixed activity, or no clear signature;
- whether evidence is incomplete because only periodic output survived.

Generic `arith-*` counters are not treated as nonlinear evidence because linear
arithmetic emits them as well.

Example:

```text
error: verification out of resources/timeout (timeout set to 40s)
   = timeout analysis replay: quantifier activity (48,231 observed quantifier instantiations)
   = top quantifier activity:
     definition of spec function 0x1::staking_contract::spec_fold at sources/staking_contract.move:397 — 31,204+ observed instantiations
     forall at sources/staking_contract.move:412 — 9,876+ observed instantiations
     vector-array-theory.bpl: select/store axiom — 3,102+ observed instantiations
```

An embedding may add repair guidance separately. The core prover reports only
evidence it can substantiate.

### Stable test output

Counts, top-N membership, completion, and even activity signatures can vary
between solver runs. Sorting and redacting counts is insufficient. With
`stable_test_output`, the prover emits only a deterministic note that timeout
analysis was requested and its runtime evidence was redacted. Detailed parsing
and rendering are tested from recorded output.

## Implementation layout

- `options.rs`: option, validation, and seed-qualified capture construction.
- `prover_task_runner.rs`: selected-result post-processing under the shared
  process semaphore; raw capture discovery and bounded Z3 execution.
- `timeout_analysis.rs`: replay instrumentation, raw-output parsing, evidence
  aggregation, fallback qid resolution, and rendering.
- `boogie_wrapper.rs`: parse procedure identities in supported Boogie timeout
  formats and attach analysis notes to inconclusive diagnostics.
- `spec_translator.rs`: conditional unique qids.
- `src/lib.rs`: retain the actual root timeout in job metadata.

## Tests

Required deterministic coverage:

- option defaults, Z3/CVC5 validation, and seed-qualified command construction;
- capture discovery for ordinary and `_splitN` files;
- replay instrumentation after resets and before every solver check;
- parsing complete statistics, periodic-only output, malformed rows, duplicate
  profile rows, and multiple unique qids resolving to one source location;
- explicit qid generation for a recursive spec-function axiom and a user
  quantifier;
- BPL position fallback resolution against fabricated generated/prelude text;
- stable-output redaction and live rendering from recorded evidence;
- analysis launch failure and exhausted-deadline behavior do not mask the base
  timeout. Watchdog-kill behavior remains a manual smoke test because it is
  wall-clock dependent.

Wall-clock forced timeouts are not part of normal CI because they are inherently
machine-dependent. A manual or ignored smoke test may run against known
timing-out packages with the pinned Boogie and Z3 versions.

## Non-goals

- CVC5 timeout profiling.
- Full Z3 `trace=true` logs and offline axiom-profiler integration.
- Automatic repair or timeout tuning.
- A calibrated causal classifier.
- MoveFlow tool, cache, feedback-level, or experiment changes.
