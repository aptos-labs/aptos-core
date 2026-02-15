## Verification Timeout (attempt {{ agent.timeout_attempt }}/{{ agent.max_timeout_attempts }})

One or more functions **timed out** during verification. **Focus on fixing the timeouts first** —
other verification errors (if any) will be addressed separately afterward.

```
{{ agent.last_diagnostics }}
```

### Source Code

```move
{{ agent.current_source }}
```

{% if agent.timeout_attempt < agent.max_timeout_attempts %}
The SMT solver ran out of resources. Common causes include non-linear arithmetic or
existential quantifiers in the specifications, but the exact cause is not always certain.

**Please reformulate the specs for the timed-out function(s):**

1. **Eliminate non-linear arithmetic.** Replace expressions like `x * y` (where both are
   variables) with linear bounds, helper variables, or factor so at most one operand is
   a variable. For example, if you know `y <= N`, replace `ensures result == x * y` with
   `ensures result <= x * N` or introduce intermediate spec-level let bindings.
2. **Remove or simplify existential quantifiers.** Replace `exists x: T: P(x)` with
   a concrete witness or a bounded range.
3. **Split complex postconditions.** Break a single complex `ensures` into multiple
   simpler, independent `ensures` clauses.

Do NOT weaken specs — find an equivalent formulation the solver can handle, not a less
precise one. Do not just remove conditions from the spec, unless you can prove one condition is 
subsumed by another one.

Do NOT add `pragma verify = false` — keep trying to find a verifiable formulation.
{% else %}
This timeout appears not solvable. Add `pragma verify = false;` to the spec block of the 
effected functions.

**CRITICAL: You MUST keep all existing `ensures`, `aborts_if`, and other spec conditions
exactly as they are.** Do NOT remove, weaken, or empty any conditions. The only change is
adding `pragma verify = false;` and a comment explaining why. The specs document the
intended behavior even when the solver cannot verify them.

Use hedged language in timeout comments ("possibly due to", not "due to"):

```move
spec my_function {
    // Verification disabled: SMT solver timeout, possibly due to non-linear arithmetic.
    pragma verify = false;
    ensures result == x * y;
    aborts_if x * y > MAX_U64;
}
```

Only disable verification for functions that actually timed out — keep all other specs intact.
{% endif %}

**Reminders:**
- **No `old()` in `aborts_if` or `requires`** — pre-state clauses, `old()` is redundant.
- **No `old(local_var)` or `old(global<T>(..))` in loop invariants** — only function
  parameters may be wrapped in `old()`.
- **No `pragma aborts_if_is_partial`** — abort specs must be complete.

Return the complete corrected source.
