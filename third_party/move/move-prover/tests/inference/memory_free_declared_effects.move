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
}
