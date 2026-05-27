module 0x42::test {

    // Immut refs

    fun exec_ref(f: |&u64|u64, x: u64): u64 {
        f(&x)
    }

    fun call_exec_ref(x: u64): u64 {
         exec_ref(|y| x + *y, 3)
    }
    spec call_exec_ref {
        ensures result == x + 3;
    }

    // Mut refs

    fun exec_mut_ref(f: |&mut u64|, x: u64): u64 {
        f(&mut x);
        x
    }

    fun call_exec_mut_ref(x: u64): u64 {
        exec_mut_ref(|y| *y = *y + 2, x)
    }
    spec call_exec_mut_ref {
        ensures result == x + 2;
    }

    // Mut ref derivation (expected to fail)

    struct S has drop { a: u64, b: u64 }

    fun borrow_a(s: &mut S): &mut u64 {
        &mut s.a
    }

    fun borrow_b(s: &mut S): &mut u64 {
        &mut s.b
    }

    fun update_indirect(s: &mut S, borrow: |&mut S|&mut u64, new_value: u64) {
        *borrow(s) = new_value
    }

    fun update_a(): u64 {
        let s = S{a: 0, b: 0};
        update_indirect(&mut s, borrow_a, 33);
        s.a
    }
    spec update_a  {
        // Currently we cannot verify code where borrow paths are derived via function values.
        // This is since the reference elimination approach.
        // TODO(#17904): there are potential ways to improve this, but ultimately, this does not fit with
        //   our reference elimination approach, which requires a statically known borrow path.
        ensures result == 33; // expected to fail
    }
}
