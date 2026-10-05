/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot infer a specification for `index_writes_unsupported::set_elem`: the reference returned by `index_writes_unsupported::elem` points to a vector element or map entry which the callee selects, so WP cannot phrase a write through it.
   ┌─ tests/inference/index_writes_unsupported.move:15:10
   │
15 │         *elem(v, i) = x;
   │          ^^^^^^^^^^

error: WP cannot infer a specification for `index_writes_unsupported::set_picked`: the reference returned by `index_writes_unsupported::pick` may point into one of several places, and the callee decides which one, so WP cannot tell which of them a write through it changes.
   ┌─ tests/inference/index_writes_unsupported.move:23:10
   │
23 │         *pick(s, first) = x;
   │          ^^^^^^^^^^^^^^
*/
