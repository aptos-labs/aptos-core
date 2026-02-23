## Loop Invariant Probing (Iteration {{ agent.iteration }}/{{ agent.max_iterations }})

The source below contains loops that lack invariants. Without loop invariants, the prover
*havocs* all loop-modified variables, causing the weakest-precondition analysis to produce
`[inferred = vacuous]` conditions — specifications that are trivially true and useless for
verification.

The source already contains some specifications -- those written by the user and derived
from WP analysis (marked with `inferred` property).

**Your task:**

1. Add loop invariants to every loop that lacks them. **Mark every loop invariant you add
   with `[inferred = ai]`**, e.g. `invariant [inferred = ai] i <= len(v);`.
2. **Remove all WP-inferred conditions** — any `ensures`, `aborts_if`, or other spec
   clauses annotated with `[inferred]` or `[inferred = vacuous]`. After you add
   invariants, WP inference will be re-run and will produce fresh, non-vacuous conditions.
3. Do NOT modify any user-written specifications (those without `[inferred …]`) or
   function bodies.

### Move Loop Invariant Syntax

Loop invariants are written in a `spec` block immediately after the loop body:

```move
while (cond) {
    // loop body
} spec {
    invariant [inferred = ai] <expr>;
};
```

### What Makes a Good Loop Invariant

A loop invariant must:
1. **Hold before the first iteration** (when the loop variable is at its initial value).
2. **Be preserved by each iteration** (if it holds before the iteration, it holds after).
3. **Relate loop state to function parameters** — express how loop-modified variables
   relate to inputs and constants (e.g., `i <= n`, `sum == i * (i - 1) / 2`).

Common patterns:
- **Bound invariants:** `i <= len(v)`, `counter <= max_count`
- **Accumulator invariants:** `sum == i * step`, `result == old_val + i * delta`
- **Structural invariants:** `len(result_vec) == i`, `forall j in 0..i: P(result_vec[j])`

### Source Code

```move
{{ agent.current_source }}
```

**Do NOT call the `verify` tool in this step.** Since you are removing inferred conditions
and adding invariants, the spec is not yet in a verifiable state. After you return the
updated source, WP inference will be re-run automatically to produce fresh conditions,
and verification will happen in a later step.

**Reminders:**
- **No `old()` in `aborts_if` or `requires`** — pre-state clauses, `old()` is redundant.
- **No `old(local_var)` or `old(global<T>(..))` in loop invariants** — only function
  parameters may be wrapped in `old()`.
- **No `pragma aborts_if_is_partial`** — abort specs must be complete.

Return the complete source with loop invariants added and all `[inferred …]` conditions
removed. Keep all user-written specs and function bodies unchanged. Do not add any specs
besides loop invariants. Do NOT add `pragma verify = false`.
