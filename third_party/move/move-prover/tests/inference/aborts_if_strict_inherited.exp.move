/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP could not characterize the aborts of `aborts_if_strict_inherited::caller` exactly, and an exact abort characterization is required. Resolve the reasons below and rerun WP. Reasons:
  = callee `0x42::aborts_if_strict_inherited::halve` has no trusted complete abort summary
   ┌─ tests/inference/aborts_if_strict_inherited.move:14:5
   │
14 │ ╭     fun caller(x: u64): u64 {
15 │ │         halve(x)
16 │ │     }
   │ ╰─────^
*/
