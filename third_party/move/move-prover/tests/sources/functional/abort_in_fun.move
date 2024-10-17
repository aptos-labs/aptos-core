module 0x42::TestAbortInFunction {

    fun aborts_with(x: u64, y: u64): u64 {
        if (x == 1) {
            abort 2
        } else if (y == 2) {
            abort 3
        } else {
            x
        }
    }
    spec aborts_with {
        aborts_if x == 1 with 2;
        aborts_if y == 2 with 3;
        ensures result == x;
    }

    fun call_aborts_with(): u64 {
        aborts_with(2, 3)
    }

    spec call_aborts_with {
        ensures result == aborts_with(2, 3);
    }

    fun abort_generic<Element: copy + drop>(x: Element, y: Element): Element {
        if (x == y) {
            abort 0
        } else {
            x
        }
    }

    fun call_aborts_generic(): u64 {
        abort_generic(2, 3)
    }

    spec call_aborts_generic {
        ensures result == abort_generic(2, 3);
    }

    struct S<Element: copy + drop> has copy, drop {
        value: Element
    }

    fun abort_generic_struct<Element: copy + drop>(x: S<Element>, y: S<Element>): S<Element> {
        if (x == y) {
            abort 0
        } else {
            x
        }
    }

    fun spec_abort_generic_struct<Element: copy + drop>(x: S<Element>, y: S<Element>): S<Element> {
        if (x == y) {
            abort 0
        } else {
            x
        }
    }

    fun call_abort_generic_struct<Element: copy + drop>(x: Element, y: Element): Element {
        let sx = S {
            value: x
        };
        let sy = S {
            value: y
        };
        abort_generic_struct(sx, sy).value
    }

    spec call_abort_generic_struct {
        aborts_if x == y;
        ensures result == call_abort_generic_struct(x, y);
    }

    struct T has copy, drop {
        v: u64
    }

    spec T {
        pragma bv=b"0";
    }

    fun call_abort_generic_struct_concrete(x: u64, y: u64, test_assert1: bool): T {
        let sx = S {
            value: T {
                v: x
            }
        };
        let sy = S {
            value: T {
                v: y
            }
        };
        assert!(test_assert1, 0);
        abort_generic_struct(sx, sy).value
    }

    spec call_abort_generic_struct_concrete {
        aborts_if x == y;
        aborts_if !test_assert1;
        ensures result == call_abort_generic_struct_concrete(x, y, test_assert1);
        ensures result == spec_call_abort_generic_struct_concrete(x, y);
    }

    spec fun spec_call_abort_generic_struct_concrete(x: u64, y: u64): T {
        let sx = S {
            value: T {
                v: x
            }
        };
        let sy = S {
            value: T {
                v: y
            }
        };
        spec_abort_generic_struct(sx, sy).value
    }

}
