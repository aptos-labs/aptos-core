## Verification Failure (attempt {{ agent.iteration }}/{{ agent.max_iterations }})

Fix the verification failures in the last Move source.

### Diagnosis

```
{{ agent.last_diagnostics }}
```

### How to Fix Common Failures

- **"abort not covered"** — The function can abort on a path that has no matching
  `aborts_if`. Trace the code path indicated in the diagnostics to find the operation
  that aborts (arithmetic overflow, missing resource, vector index, explicit `abort`),
  then add the corresponding `aborts_if` condition.
- **"post-condition does not hold"** — An `ensures` clause is wrong. The prover may
  show a counterexample with concrete variable values — use those to understand which
  case the spec fails on, then weaken or correct the condition.
- **"precondition does not hold" (at call site)** — A function call's `requires` is
  not satisfied. Either strengthen the caller's own `requires` to propagate the
  constraint, or add a guard / `aborts_if` that covers the case.
- **Multiple related errors** — Fix one error at a time, starting with the earliest
  in the function body. Later errors are often consequences of earlier ones.
- **Do not weaken specs unnecessarily.** Prefer adding missing conditions over
  removing correct ones. Only weaken an `ensures` if the original is genuinely wrong,
  not just because the prover can't prove it yet.
- Do NOT add `pragma verify = false`.

**Reminders:**
- **No `old()` in `aborts_if` or `requires`** — pre-state clauses, `old()` is redundant.
- **No `old(local_var)` or `old(global<T>(..))` in loop invariants** — only function
  parameters may be wrapped in `old()`.
- **No `pragma aborts_if_is_partial`** — abort specs must be complete.

Return the corrected source.
