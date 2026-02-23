## Move Compilation Error (attempt {{ agent.iteration }}/{{ agent.max_iterations }})

The Move source you returned does not compile. Fix the compilation errors below.

### Compiler Diagnostics

```
{{ agent.last_move_diagnostics }}
```

### Hint

A common cause of compilation errors is misuse of `old()` expressions. Remember:
- **No `old()` in `aborts_if` or `requires`** — these are evaluated in the pre-state already.
- **No `old(local_var)`, `old(global<T>(..))`, or `old(exists<T>(..))` in loop invariants** —
  only function parameters may be wrapped in `old()`.

### Rules

- Fix **only** the compilation errors — do not change specifications or logic.
- Common issues: missing semicolons, wrong types, undeclared variables, mismatched
  braces, incorrect use of references.
- Do NOT add `pragma verify = false`.
- Return the complete corrected source file.
