// exclude_for: cvc5
module 0x42::bv_internal_invalid {

    fun not_opaque(x: u64): u64 {
        x ^ 1
    }
    spec not_opaque {
        pragma bv_internal;
    }

    fun bitwise_spec(x: u64): u64 {
        x | 2
    }
    spec bitwise_spec {
        pragma opaque;
        pragma bv_internal;
        ensures result == (x | 2);
    }

    fun bitwise_spec_fun(x: u64): u64 {
        x | 4
    }
    spec bitwise_spec_fun {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == or_four(x);
    }
    spec fun or_four(x: u64): u64 {
        x | 4
    }

    struct Wrap has drop {
        v: u64
    }

    fun escapes(x: u64): Wrap {
        Wrap { v: x & 3 }
    }
    spec escapes {
        pragma opaque;
        pragma bv_internal;
    }
}
