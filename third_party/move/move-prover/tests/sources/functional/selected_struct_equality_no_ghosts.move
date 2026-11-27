// In a unit that declares no ghost, equality at a type parameter is the
// carrier's: a generic comparison of declared values keeps its exact
// specification (the control of `selected_struct_equality.move`).
module 0x42::selected_struct_equality_no_ghosts {
    struct Box<T> has copy, drop { b: T }

    fun box_eq<T: copy + drop>(a: Box<T>, b: Box<T>): bool { a == b }
    spec box_eq {
        ensures result == (a == b);
    }

    fun box_ne<T: copy + drop>(a: Box<T>, b: Box<T>): bool { a != b }
    spec box_ne {
        ensures result == (a != b);
    }

    fun box_eq_same(): bool { box_eq(Box { b: 1u64 }, Box { b: 1u64 }) }
    spec box_eq_same {
        ensures result;
    }

    fun box_eq_differs(): bool { box_eq(Box { b: 1u64 }, Box { b: 2u64 }) }
    spec box_eq_differs {
        ensures result; // error: the boxes differ
    }
}
