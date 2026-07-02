// also_include_for: prophecy
module 0x42::prophecy_c4 {
    // A function with a &mut parameter.
    fun inc(x: &mut u64) {
        *x = *x + 1;
    }
    spec inc {
        ensures x == old(x) + 1;
    }

    // A caller passing a local borrow into inc.
    fun client(): u64 {
        let a = 5;
        inc(&mut a);
        a
    }
    spec client {
        ensures result == 6;
    }

    struct S has drop { f: u64, g: u64 }

    fun set(x: &mut u64, val: u64) {
        *x = val;
    }

    // Pass a field-derived borrow across a call; the other field is untouched.
    fun use_field_arg(): u64 {
        let s = S { f: 0, g: 7 };
        set(&mut s.f, 42);
        s.f + s.g
    }
    spec use_field_arg {
        ensures result == 49;
    }

    // Thread a &mut through two call levels.
    fun inc_twice(x: &mut u64) {
        inc(x);
        inc(x);
    }
    fun client2(): u64 {
        let a = 10;
        inc_twice(&mut a);
        a
    }
    spec client2 {
        ensures result == 12;
    }
}
