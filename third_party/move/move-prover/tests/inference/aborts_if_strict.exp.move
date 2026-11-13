/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
warning: WP inferred a `vacuous` condition, but this function has no loop without an invariant. A callee that returns `&mut` into state its contract does not model leaves the write-back through that reference unconstrained; give that callee an intrinsic model or an exact write-back contract before relying on the inferred specification.
   ┌─ tests/inference/aborts_if_strict.move:11:5
   │
11 │ ╭     fun bump(a: address, k: u64) {
12 │ │         let i = 0;
13 │ │         while (i < k) {
14 │ │             Counter[a].n += 1;
   · │
18 │ │         };
19 │ │     }
   │ ╰─────^

error: WP could not characterize the aborts of `aborts_if_strict::bump` exactly, and an exact abort characterization is required. Resolve the reasons below and rerun WP. Reasons:
  = an abort condition did not survive a memory-havocking loop
   ┌─ tests/inference/aborts_if_strict.move:11:5
   │
11 │ ╭     fun bump(a: address, k: u64) {
12 │ │         let i = 0;
13 │ │         while (i < k) {
14 │ │             Counter[a].n += 1;
   · │
18 │ │         };
19 │ │     }
   │ ╰─────^
*/
