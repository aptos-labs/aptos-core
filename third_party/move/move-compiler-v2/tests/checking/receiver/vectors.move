// Do not include stdlib for this test:
// no-stdlib

module 0x1::vector { // must be this module

    fun receiver<T>(self: vector<T>, _y: T) {
    }

    fun receiver_ref<T>(self: &vector<T>, _y: T) {
    }

    fun receiver_ref_mut<T>(self: &mut vector<T>, _y: T) {
    }

    fun test_call_styles(s: vector<u64>, x: u64) {
        s.receiver(x);
        s.receiver_ref(x);
        s.receiver_ref_mut(x);
    }
}
