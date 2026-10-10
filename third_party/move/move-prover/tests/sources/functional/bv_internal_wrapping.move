// exclude_for: cvc5
module 0x42::bv_internal_wrapping {

    fun f(x: u8): u8 {
        x | 0
    }
    spec f {
        pragma opaque;
        pragma bv_internal;
        ensures result == (x as num) + 256;
    }

    fun g(x: u8): u8 {
        x | 0
    }
    spec g {
        pragma opaque;
        pragma bv_internal;
        ensures (result as num) * 2 == (x as num) * 2;
    }
}
