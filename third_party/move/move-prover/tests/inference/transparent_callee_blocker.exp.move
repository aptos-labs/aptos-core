/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot complete `transparent_callee_blocker::caller` while a transparent callee lacks a complete opaque contract. Repair the named callee boundary before changing or rerunning the caller. Reasons:
  = transparent callee `0x1::string::length` is outside the editable WP scope and has no complete opaque contract; WP cannot construct a complete caller specification. The package or corpus must provide and verify a complete opaque contract for that callee before the caller is rerun
  = transparent callee `0x42::transparent_callee_blocker::local_helper` is inside the editable WP scope but has no complete opaque contract; infer and verify an opaque specification for that callee first, then rerun WP for the caller
   ┌─ tests/inference/transparent_callee_blocker.move:12:5
   │
12 │ ╭     fun caller(): u64 {
13 │ │         let text = string::utf8(b"");
14 │ │         local_helper(text.length())
15 │ │     }
   │ ╰─────^
*/
