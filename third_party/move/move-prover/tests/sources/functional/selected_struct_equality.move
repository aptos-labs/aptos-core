// The program's `==` and `!=` on values of a declared struct compare what the
// runtime holds: two values equal in every runtime field are equal although
// their ghost fields differ, and function-valued fields compare as the VM
// compares closures. Values that differ in a runtime field stay unequal. The
// same holds for `vector::contains` and `vector::index_of`, which compare
// elements with the program's equality.
module 0x42::selected_struct_equality {
    use std::vector;

    struct G has copy, drop { v: u64 }
    spec G {
        ghost g: u64;
    }

    fun ghost_pair(): (G, G) {
        let a = G { v: 1 };
        let b = G { v: 1 };
        spec { update a.g = 1; };
        spec { update b.g = 2; };
        (a, b)
    }

    fun eq_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        a == b
    }
    spec eq_ghost_differs {
        ensures result;
    }

    fun eq_ghost_differs_false(): bool {
        let (a, b) = ghost_pair();
        a == b
    }
    spec eq_ghost_differs_false {
        ensures !result; // error: the runtime holds the values equal
    }

    fun ne_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        a != b
    }
    spec ne_ghost_differs {
        ensures !result;
    }

    fun ne_ghost_differs_true(): bool {
        let (a, b) = ghost_pair();
        a != b
    }
    spec ne_ghost_differs_true {
        ensures result; // error: the runtime holds the values equal
    }

    // Values whose runtime fields differ are unequal.
    fun eq_payload_differs(): bool {
        let a = G { v: 1 };
        let b = G { v: 2 };
        a == b
    }
    spec eq_payload_differs {
        ensures !result;
    }

    fun eq_payload_differs_true(): bool {
        let a = G { v: 1 };
        let b = G { v: 2 };
        a == b
    }
    spec eq_payload_differs_true {
        ensures result; // error: the values differ in `v`
    }

    fun contains_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        let v = vector::singleton(a);
        vector::contains(&v, &b)
    }
    spec contains_ghost_differs {
        ensures result;
    }

    fun contains_ghost_differs_false(): bool {
        let (a, b) = ghost_pair();
        let v = vector::singleton(a);
        vector::contains(&v, &b)
    }
    spec contains_ghost_differs_false {
        ensures !result; // error: the vector holds an equal value
    }

    fun index_of_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        let v = vector::singleton(a);
        let (found, _) = vector::index_of(&v, &b);
        found
    }
    spec index_of_ghost_differs {
        ensures result;
    }

    fun index_of_ghost_differs_false(): bool {
        let (a, b) = ghost_pair();
        let v = vector::singleton(a);
        let (found, _) = vector::index_of(&v, &b);
        found
    }
    spec index_of_ghost_differs_false {
        ensures !result; // error: the vector holds an equal value
    }

    // A function-valued field: closures over the same function whose
    // captures differ only in a ghost are equal.
    struct S has copy, drop { x: u64 }
    spec S {
        ghost sg: u64;
    }

    struct FunBox has copy, drop { f: |u64|u64 has copy + drop }

    fun mk(gv: u64): |u64|u64 has copy + drop {
        let s = S { x: 0 };
        spec { update s.sg = gv; };
        |y| y + s.x
    }

    fun eq_closure_field(): bool {
        FunBox { f: mk(1) } == FunBox { f: mk(2) }
    }
    spec eq_closure_field {
        ensures result;
    }

    fun ne_closure_field(): bool {
        FunBox { f: mk(1) } != FunBox { f: mk(2) }
    }
    spec ne_closure_field {
        ensures result; // error: the closures are equal
    }

    // Generic equality at an instantiation with the ghost-carrying struct.
    struct Wrap<T> has copy, drop { w: T }

    fun same<T: copy + drop>(a: T, b: T): bool { a == b }
    spec same {
        ensures result == (a == b);
    }

    fun same_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        same(a, b)
    }
    spec same_ghost_differs {
        ensures result;
    }

    fun same_ghost_differs_false(): bool {
        let (a, b) = ghost_pair();
        same(a, b)
    }
    spec same_ghost_differs_false {
        ensures !result; // error: the runtime holds the values equal
    }

    fun eq_wrap_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        Wrap { w: a } == Wrap { w: b }
    }
    spec eq_wrap_ghost_differs {
        ensures result;
    }

    fun ne_wrap_ghost_differs(): bool {
        let (a, b) = ghost_pair();
        Wrap { w: a } != Wrap { w: b }
    }
    spec ne_wrap_ghost_differs {
        ensures result; // error: the runtime holds the values equal
    }
}

// The comparing module declares no ghost; the struct's module does.
module 0x42::selected_struct_equality_maker {
    struct H has copy, drop { v: u64 }
    spec H {
        ghost h: u64;
    }

    public fun make(v: u64, gv: u64): H {
        let x = H { v };
        spec { update x.h = gv; };
        x
    }
    spec make {
        ensures result.v == v;
        ensures result.h == gv;
    }
}

module 0x42::selected_struct_equality_user {
    use 0x42::selected_struct_equality_maker;

    fun eq_imported(): bool {
        selected_struct_equality_maker::make(1, 1) == selected_struct_equality_maker::make(1, 2)
    }
    spec eq_imported {
        ensures result;
    }

    fun ne_imported(): bool {
        selected_struct_equality_maker::make(1, 1) != selected_struct_equality_maker::make(1, 2)
    }
    spec ne_imported {
        ensures result; // error: the runtime holds the values equal
    }
}
