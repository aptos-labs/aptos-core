/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot complete `transparent_callee_blocker::caller` while a transparent callee lacks a complete opaque contract. Repair the named callee boundary before changing or rerunning the caller. Reasons:
  = transparent callee `0x42::transparent_callee_blocker::local_helper` is inside the editable WP scope and has neither a complete opaque contract nor a body which describes its behavior exactly (no loops, no global memory); infer and verify an opaque specification for that callee first, then rerun WP for the caller
   ┌─ tests/inference/transparent_callee_blocker.move:17:5
   │
17 │ ╭     fun caller(): u64 {
18 │ │         let text = string::utf8(b"");
19 │ │         local_helper(text.length())
20 │ │     }
   │ ╰─────^
*/
