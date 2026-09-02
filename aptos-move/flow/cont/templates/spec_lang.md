{% if once(name="spec_lang") %}
## Move specification language

What follows is the working reference for this workflow, not a language tour.
For agents with internet access, the full Move Book is at
https://aptos.dev/en/build/smart-contracts/book.

### Function contracts

Attach conditions with `spec function_name { ... }`. If a function name is a
soft keyword, escape it as `spec @function_name { ... }`.

- `requires e` is a caller obligation evaluated in the pre-state.
- `aborts_if e` describes an allowed abort in the pre-state. With complete
  abort checking, the disjunction of all `aborts_if` clauses characterizes the
  function's abort behavior. No clauses means abort behavior is unspecified;
  use `aborts_if false` for a total function.
- `ensures e` is a normal-return guarantee evaluated in the post-state. Use
  `old(e)` for a pre-state value.
- `modifies global<T>(addr)` is a frame declaration for global state the
  function may change. An opaque function that can mutate global resources
  needs frames covering every such resource/address effect.

### Opaque and intrinsic functions

`pragma opaque` changes how callers are verified: callers use the function's
contract instead of its implementation. It does **not** disable verification of
the opaque function's own body. Preserve opaque pragmas while repairing their
contracts, and include complete result, abort, and global-state frame behavior.

`pragma intrinsic` identifies a function with built-in prover semantics. Do not
invent an opaque contract or body proof for an intrinsic merely because its Move
implementation is absent or unsuitable for ordinary verification.

### Specification expressions

- `result` denotes the return value in `ensures`.
- `global<T>(addr)` and `exists<T>(addr)` inspect global resources. A module's
  `spec_exists_at` wrapper should be modeled as the same existence fact; the
  prover is not subject to Move source visibility restrictions.
- Specifications use mathematical integers. Numeric bounds such as `MAX_U64`
  refer to Move values, while arithmetic in a spec expression is unbounded.
- Spec expressions operate on values, not references: use `v.field`, not `*v`
  or `&v`.

`old(e)` means the value at function entry. Do not use it in `requires` or
`aborts_if`, which are already pre-state expressions. In a loop invariant,
`old(x)` is valid only for a function parameter; save any other pre-loop value
in a local before the loop and mention that local directly.

### Loop invariants

Attach invariants to an ordinary loop:

```move
while (i < n) {
    // body
} spec {
    invariant i <= n;
    invariant acc == prefix_sum(values, i);
};
```

The prover checks invariant initialization, preservation, and the implication
from loop exit to the function contract.

For loops introduced by an inline higher-order iterator, use the prover's fold
logic and a `folds_of` invariant when its capture transformer is applicable.
Inside the iterator's loop invariant, `folds_of<f>(values, i)` summarizes a
unary callback over a prefix; `folds_of<f>(|j| (j, values[j]), i)` supplies an
explicit argument tuple. The predicate includes accumulated capture effects and
prefix no-abort behavior and is valid only as a loop invariant. If the fold
warning says it is not applicable, rewrite the iterator as an equivalent
ordinary loop and supply invariants. A simple ordinary accumulation loop may
conversely be clearer as an inline fold when the fold relation is expressible.

### Referring to callee behavior

For a non-inline named function or function value `f`, specifications can use:

- `requires_of<f>(args)`;
- `aborts_of<f>(args)`;
- `ensures_of<f>(args, result)`;
- `result_of<f>(args)`.

These predicates expose the callee's contract, including across modules. A
target specification may also call specification functions declared in its
dependencies, so retain the specification-level dependency closure as well as
the executable call closure.

### Separate specification files

A `.spec.move` file extends its corresponding module. Put helper functions,
lemmas, and module invariants in `spec module { ... }`; put conditions for a
Move function in `spec function_name { ... }`. There is no
`spec <module_name> { ... }` form.

### Inference markers

Use `[inferred]` on every inferred condition or invariant. WP may emit
`[inferred = vacuous]` when state is unconstrained and `[inferred = sathard]`
when a condition is likely to be difficult for SMT. Both mark unresolved
inference output.

### Reference

- [Move Specification Language](https://aptos.dev/en/build/smart-contracts/prover/spec-lang)
{% endif %}
