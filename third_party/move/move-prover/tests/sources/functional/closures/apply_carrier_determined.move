// Results of a call through a function value are bound to the target's `result_of` carrier
// where the target's specification determines them.
module 0x42::apply_carrier_determined {
    native fun same(x: u64): u64;
    spec same { pragma opaque; aborts_if false; ensures result == x; }

    // Binding is kept for a native whose specification determines the result.
    fun determined_native_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = same;
        (h(x), h(x))
    }
    spec determined_native_twice { ensures result_1 == result_2; }

    // Forwards a result of vector type. Trusted here; `apply_carrier_unread_memory` verifies it.
    fun apply_bytes(f: |vector<u8>| vector<u8> has copy + drop, b: vector<u8>): vector<u8> {
        f(b)
    }
    spec apply_bytes {
        pragma opaque, verify = false;
        aborts_if aborts_of<f>(b);
        ensures result == result_of<f>(b);
    }

    // A native the prelude models exactly is a function of its arguments.
    fun prelude_native_through_hof(b: vector<u8>): (vector<u8>, vector<u8>) {
        (apply_bytes(std::hash::sha2_256, b), std::hash::sha2_256(b))
    }
    spec prelude_native_through_hof { ensures result_1 == result_2; }
}
