## Spec Simplification (iteration {{ agent.iteration }}/{{ agent.max_iterations }})

Your task is to simplify the specifications in the source below.

### Goals

1. **Eliminate `[inferred = sathard]` conditions.** These contain quantifiers over loop
   variables (`exists` in `aborts_if`, `forall` in `ensures`) that will cause SMT solver
   timeouts. Replace each with an equivalent **non-quantified** expression:
   - If a loop accumulates a value monotonically (e.g., summing), then
     `exists i in 0..n: f(i) > MAX` simplifies to checking the final value `f(n) > MAX`.
   - `forall i: (i == n ==> P(i))` simplifies to `P(n)` — substitute the determined value.
   - In general, if the quantified variable is fully determined by constraints in the
     antecedent/body, substitute the determined value and drop the quantifier.
2. **Remove redundant conditions** that are implied by others or by language guarantees
   (e.g., `ensures result >= 0` for unsigned types).
3. **Simplify complex expressions** where a clearer, equivalent formulation exists.
4. **Consolidate** multiple conditions that can be expressed as one.
5. **Improve readability** — prefer clear names and straightforward logic over
   technically precise but opaque formulations. Use several simple `ensures` clauses
   rather than one complex conjunction. Use spec helpers (`spec fun`) for reusable
   predicates.
6. **Avoid non-linear arithmetic** — do not introduce multiplications of two variables
   (`x * y`); prefer linear bounds or factor so at most one operand is a variable.
7. **Remove `[inferred]` and `[inferred = sathard]` markers** from conditions you keep,
   and mark anything you rewrite with `[inferred = ai]`.

Do NOT change function bodies. Do NOT weaken specs — every condition you remove must
be genuinely redundant, not just hard to verify. Do NOT add `pragma verify = false`.

### Source Code

```move
{{ agent.current_source }}
```

**Reminders:**
- **No `old()` in `aborts_if` or `requires`** — pre-state clauses, `old()` is redundant.
- **No `old(local_var)` or `old(global<T>(..))` in loop invariants** — only function
  parameters may be wrapped in `old()`.
- **No `pragma aborts_if_is_partial`** — abort specs must be complete.

Return the complete simplified source.
