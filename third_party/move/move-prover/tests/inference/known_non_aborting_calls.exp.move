/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot complete `known_non_aborting_calls::constructs_empty_string` while a transparent callee lacks a complete opaque contract. Repair the named callee boundary before changing or rerunning the caller. Reasons:
  = transparent callee `0x1::string::length` is outside the editable WP scope and has no complete opaque contract; WP cannot construct a complete caller specification. The package or corpus must provide and verify a complete opaque contract for that callee before the caller is rerun
   ┌─ tests/inference/known_non_aborting_calls.move:12:5
   │
12 │ ╭     fun constructs_empty_string(): bool {
13 │ │         string::utf8(b"").length() == 0
14 │ │     }
   │ ╰─────^
*/
