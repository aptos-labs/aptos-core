module 0x42::m {

    struct S { x: u64 }

    // Call styles

    fun receiver(self: S, y: u64): u64 {
        self.x + y
    }

    fun receiver_ref(self: &S, y: u64): u64 {
        self.x + y
    }

    fun receiver_ref_mut(self: &mut S, y: u64): u64 {
        self.x + y
    }

    inline fun inline_receiver_ref_mut(self: &mut S, y: u64): u64 {
        self.x + y
    }

    fun test_call_styles(s: S): u64 {
        s.receiver(1);
        s.receiver_ref(1);
        s.receiver_ref_mut(1);
        s.inline_receiver_ref_mut(1)
    }
}
