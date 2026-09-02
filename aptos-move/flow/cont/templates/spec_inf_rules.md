{# Shared inference decisions and safeguards. Keep treatment-specific tool
   orchestration in spec_inf_tasks.md. #}
{% if once(name="spec_inf_rules") %}

### Tools this task needs

Specification inference is a compile-and-prove loop, not a test loop:

- `{{ tool(name="move_spec_check") }}` decides whether the work is
  done, and is the only thing that does.
- `{{ tool(name="move_package_verify") }}` proves a scope while the contract is
  still taking shape.
- `{{ tool(name="move_package_status") }}`,
  `{{ tool(name="move_package_manifest") }}` and
  `{{ tool(name="move_package_query") }}` answer questions about the package.

Nothing else is part of this task. In particular, running the package's unit
tests says nothing about whether a specification holds: the prover reasons over
all inputs, so a passing test neither supports a contract nor locates a missing
condition, and the run leaves a coverage map in the package.

### Scope and evidence

- Work only in the requested function or module. Skip `#[test]` and
  `#[test_only]` functions unless the user asks for test specifications.
- Read the implementation, existing specifications, and relevant callee
  contracts before writing conditions. Use `function_usage` for executable
  calls and closure captures instead of inferring dependencies from imports.
- Preserve user-written specifications. Report a conflict with the
  implementation instead of silently changing their meaning.

### Completion criteria

Describe all behavior visible to a caller:

- normal results and mutated reference values;
- every direct and transitive abort, including arithmetic, bounds, and resource
  access;
- global-state changes and their `modifies` frames;
- genuine API preconditions; and
- loop invariants needed to prove the body.

Check boundary cases explicitly. Similar control flow can still differ on empty
input or a single element, and arithmetic can abort without an `assert!` in the
source.

Give the specification you author for the target `pragma opaque`. That pragma is
the claim the whole task is judged on: it tells the prover a caller may be
verified against the contract alone, without ever reading the body. A contract
that omits a result, an abort, or a frame is therefore not merely incomplete, it
is wrong — callers will be verified against a promise the implementation does not
keep. Write the contract so that claim holds, rather than dropping the pragma to
make a partial contract defensible. `pragma opaque` does not suppress
verification of the function's own body, so an opaque contract still has to be
proved against the implementation.

Do not add `pragma opaque` to a helper outside what is being checked. Only the
functions in scope are proved, so an opaque contract on a helper would be
assumed at the target's call sites without ever being verified — and the check
rejects it for that reason. A helper left transparent needs no contract at all:
the prover reads its body, so the target is proved against what the helper
really does.

Do not weaken the contract to make verification pass. Never remove or narrow a
behavioral condition, invent a restrictive `requires`, enable partial abort
coverage, omit a frame, or skip verification. Replace a condition only with a
semantically equivalent, complete form.

### Loop abstractions

Derive an invariant from the implementation and the fact needed at loop exit.
It must hold initially, survive one iteration, and constrain every relevant
loop-modified value. A loop reported as needing an invariant comes with bounded
loop-head facts for its first iterations; generalize those into a predicate that
holds at entry and survives one back-edge. Common shapes include:

- **Accumulation:** relate the accumulator to the processed prefix, often with
  a recursive helper or a processed-plus-remaining conservation relation.
- **Search:** record the index bounds and what is known about the processed
  prefix, including first-match or no-match facts required by the result.
- **Quantified traversal:** restrict the final quantifier to the prefix already
  visited.
- **Stateful traversal:** relate modified references or resources to their
  pre-loop state and state the necessary frame facts.

For an inline higher-order iterator, use `folds_of` when its capture transformer
expresses the exact accumulated effect. If the prover says the fold is
inapplicable, use an equivalent ordinary loop with explicit invariants.

An inferred `vacuous` or `sathard` clause is an unresolved obligation, not a
clause to delete. Diagnose its source:

- loop havoc requires a stronger loop abstraction;
- a hard quantifier or nonlinear expression requires an equivalent,
  solver-friendly representation; and
- an unconstrained `result_of`, `ensures_of`, or `aborts_of` carrier requires a
  stronger callee or function-value contract.

### Dependencies and abstraction

An ordinary non-inline callee needs a contract strong enough for the target.
Verify an opaque callee's body against that contract once; callers then consume
only its result, abort, and frame behavior. Retain specification functions used
by the target even when they are outside the executable call graph.

Do not synthesize ordinary contracts for `pragma intrinsic` functions; the
prover supplies their semantics.

### Output discipline

- Mark every authored condition and invariant `[inferred]`; never mark existing
  user-written clauses.
- Follow the package's inline or `.spec.move` placement convention. Loop
  invariants always remain beside their executable loop.
- Avoid equivalent duplicates and empty spec blocks. Document non-obvious
  helpers and lemmas.
- Finish with formatted, compiler-clean files and no unresolved `vacuous`,
  `sathard`, uninvariant-loop, or inapplicable-fold diagnostic in scope.

{% endif %}
