// exclude_for: cvc5
// Regression: a function type that is referenced in a type (here inside
// Option) without any closure of that type ever being constructed gets an
// empty Boogie datatype with a dummy constructor; the constructor name was
// emitted malformed ($dummy'..()'()) and caused a Boogie parse error.
module 0x42::fun_type_unused_ctor {
    use std::option::{Self, Option};

    struct A has drop { f: Option<|u64|u64 has drop> }
    struct B has drop { g: Option<|u64|bool has drop> }

    public fun mk(): (A, B) {
        (A { f: option::none() }, B { g: option::none() })
    }

    spec mk {
        aborts_if false;
    }
}
