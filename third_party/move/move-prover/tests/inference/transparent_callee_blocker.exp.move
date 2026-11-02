/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot complete `transparent_callee_blocker::caller` through a transparent callee which has neither a complete opaque contract nor a body describing its behavior exactly. Reasons:
  = transparent callee `0x42::transparent_callee_blocker::local_helper` is inside the editable WP scope and has neither a complete opaque contract nor a body which describes its behavior exactly (no loops, no global memory); WP cannot construct a complete caller specification
   ┌─ tests/inference/transparent_callee_blocker.move:17:5
   │
17 │ ╭     fun caller(): u64 {
18 │ │         let text = string::utf8(b"");
19 │ │         local_helper(text.length())
20 │ │     }
   │ ╰─────^
*/
