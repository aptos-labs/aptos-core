// Declared frames and spec-variable updates can make a callee stateful even
// when usage analysis finds no global operation in executable code.
module 0x42::memory_free_declared_effects {
    struct R has key { value: u64 }

    spec module { global flag: num; }

    native fun set_flag();
    spec set_flag {
        pragma opaque;
        pragma inference = none;
        update flag = 1;
        aborts_if false;
    }

    fun update_after_publish(s: &signer) {
        move_to(s, R { value: 2 });
        set_flag();
    }
    spec update_after_publish(s: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(s)];
        ensures [inferred] S1.. |~ (ensures_of<set_flag>());
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ publish<R>(a, R{value: 2})
        };
        aborts_if [inferred] exists<R>(signer::address_of(s));
    }

}
/*
Verification: Succeeded.
*/
