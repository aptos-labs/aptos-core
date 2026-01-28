module 0x99::FunctionCall {

    struct S has key {
        val: u64,
    }

    fun foo_ref(x: &u64): u64 {
        *x + 1
    }

    fun foo_mut_ref(x: &mut u64): u64 {
        *x + 1
    }

    fun foo_global(account: &signer, x: &u64): u64 {
        move_to(account, S { val: 42 });
        *x + 1
    }

    // `foo_mut_ref(y)` cannot be reused because `ref` is a mutable reference
    fun bar_mut_ref(y: u64): u64 {
        let ref = &mut y;
        foo_mut_ref(ref) + foo_mut_ref(ref)
    }

    // `foo_global(y)` cannot be reused because `foo_global` access global storage
    fun bar_global(account: &signer, y: u64): u64 {
        let ref = &y;
        foo_global(account, ref) + foo_global(account, ref)
    }
}
