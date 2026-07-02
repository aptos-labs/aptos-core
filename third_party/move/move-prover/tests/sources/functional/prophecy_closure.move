// also_include_for: prophecy
module 0x42::prophecy_closure {
    // A closure with a &mut parameter, invoked through a function value.
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
}
