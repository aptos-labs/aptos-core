// exclude_for: cvc5
module 0x42::bv_internal {

    spec module {
        axiom forall a: u16, b: u16 where a != b: spec_swap_bytes(a) != spec_swap_bytes(b);
    }

    spec fun spec_swap_bytes(x: u16): u16;

    fun swap_bytes(x: u16): u16 {
        let v = x;
        v = ((v & 0xff) << 8) | ((v >> 8) & 0xff);
        v
    }
    spec swap_bytes {
        pragma opaque;
        pragma bv_internal;
        pragma bv = b"0";
        pragma bv_ret = b"0";
        ensures [abstract] result == spec_swap_bytes(x);
    }

    fun masked(x: u16): u16 {
        x & 0xff
    }
    spec masked {
        pragma opaque;
        pragma bv_internal;
        ensures (result as num) <= (x as num);
        ensures (result as u64) <= 0xff;
    }

    fun swap_bytes_checked(x: u16): u16 {
        assert!(x != 0, 1);
        let v = x;
        v = ((v & 0xff) << 8) | ((v >> 8) & 0xff);
        v
    }
    spec swap_bytes_checked {
        pragma opaque;
        pragma bv_internal;
        aborts_if x == 0;
        ensures [abstract] result == spec_swap_bytes(x);
    }

    struct Holder has drop {
        v: u16
    }

    fun consume(a: u16): u16 {
        if (a == 0) return 0;
        let r = swap_bytes_checked(a);
        let h = Holder { v: r };
        if (h.v < 0xffff) { h.v + 1 } else { 0 }
    }
    spec consume {
        aborts_if false;
        ensures a != 0 && spec_swap_bytes(a) < 0xffff ==> result == spec_swap_bytes(a) + 1;
    }

    fun bv_ret_caller(x: u16): u16 {
        swap_bytes(x)
    }
    spec bv_ret_caller {
        pragma bv_ret = b"0";
    }

    fun makes_bitwise_vector(x: u8): vector<u8> {
        vector[x & 1]
    }
    spec makes_bitwise_vector {
        ensures result != vector[];
    }

    fun bitwise_vector(x: u16): vector<u16> {
        vector[x & 1]
    }
    spec bitwise_vector {
        ensures contains(result, x & 1) || !contains(result, x & 1);
    }

    fun boundary_contains(v: vector<u16>, x: u16): u16 {
        let _ = v;
        x | 0
    }
    spec boundary_contains {
        pragma opaque;
        pragma bv_internal;
        ensures contains(v, result) || !contains(v, result);
    }

    fun inverted(x: u16): u16 {
        x ^ 0xffff
    }
    spec inverted {
        pragma opaque;
        pragma bv_internal;
        ensures [concrete] result == x ^ 0xffff;
        ensures [abstract] result == spec_inverted(x);
    }
    spec fun spec_inverted(x: u16): u16;

    fun uses_inverted(x: u16): u16 {
        inverted(x)
    }
    spec uses_inverted {
        ensures result == spec_inverted(x);
    }

    spec fun spec_concrete_bv(x: u16): u16;

    fun concrete_bv_body(x: u16): u16 {
        (x ^ 0xffff) & 0x0ff0
    }
    spec concrete_bv_body {
        pragma opaque;
        pragma bv_internal;
        pragma bv = b"0";
        pragma bv_ret = b"0";
        ensures [concrete] result == (x ^ 0xffff) & 0x0ff0;
        ensures [abstract] result == spec_concrete_bv(x);
    }

    fun uses_concrete_bv(x: u16): u16 {
        concrete_bv_body(x)
    }
    spec uses_concrete_bv {
        ensures result == spec_concrete_bv(x);
    }

    fun proved_masked(x: u16): u16 {
        x & 3
    }
    spec proved_masked {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == spec_swap_bytes(x);
    } proof {
        assert x & 3 == x & 3;
        post assert result == x & 3;
    }

    fun masked_opaque(x: u16): u16 {
        x & 3
    }
    spec masked_opaque {
        pragma opaque;
        ensures result == x & 3;
    }

    fun uses_bitwise_callee(x: u16): u16 {
        let m = masked_opaque(x);
        m | 0
    }
    spec uses_bitwise_callee {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == spec_swap_bytes(x);
    }

    fun distinct_ids(a: u16, b: u16): bool {
        swap_bytes(a) != swap_bytes(b)
    }
    spec distinct_ids {
        ensures a != b ==> result == true;
    }

    spec fun spec_sum_bytes(data: vector<u8>): u64;
    spec fun spec_pack_bytes(data: vector<u8>): u64;

    fun sum_bytes(data: &vector<u8>): u64 {
        let value = 0u64;
        for (i in 0..4u64) {
            value = (value << 8) | (data[i] as u64);
        };
        value
    }
    spec sum_bytes {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == spec_sum_bytes(data);
    }

    fun pack_bytes(data: &vector<u8>): u64 {
        let value = 0u64;
        let i = 0;
        while ({
            spec {
                invariant value | 0 == value;
            };
            i < 4
        }) {
            value = (value << 8) | (data[i] as u64);
            i = i + 1;
        };
        value
    }
    spec pack_bytes {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == spec_pack_bytes(data);
    }

    fun packed_is_stable(data: &vector<u8>): bool {
        pack_bytes(data) == pack_bytes(data)
    }
    spec packed_is_stable {
        ensures result == true;
    }

    fun bitwise_caller(y: u16): u16 {
        let x = y & 0xff;
        let r = swap_bytes(x);
        if (r < 0xffff) { r + 1 } else { 0 }
    }
    spec bitwise_caller {
        ensures result <= 0xffff;
    }

    spec fun spec_u8(x: u8): u8;

    fun aborts_concrete(x: u8): u8 {
        assert!(x & 1 == 0, 1);
        x | 0
    }
    spec aborts_concrete {
        pragma opaque;
        pragma bv_internal;
        aborts_if [concrete] x & 1 != 0;
        ensures [abstract] result == spec_u8(x);
    }

    fun bv2int_boundary(x: u8): u8 {
        x | 0
    }
    spec bv2int_boundary {
        pragma opaque;
        pragma bv_internal;
        ensures bv2int(result) >= 0;
        ensures [abstract] result == spec_u8(x);
    }

    fun aborts_boundary(x: u8): u8 {
        assert!(x < 255, 1);
        x | 0
    }
    spec aborts_boundary {
        pragma opaque;
        pragma bv_internal;
        aborts_if x + 1 > 255;
        ensures [abstract] result == spec_u8(x);
    }

    fun uses_aborts_boundary(x: u8): u8 {
        aborts_boundary(x)
    }
    spec uses_aborts_boundary {
        aborts_if x + 1 > 255;
    }

    spec fun spec_generic<T>(x: T): T;

    fun arith_opaque(x: u64): u64 {
        x + 0
    }
    spec arith_opaque {
        pragma opaque;
        ensures [abstract] result == spec_generic(x);
    }

    fun bitwise_severed(x: u8): u8 {
        x | 0
    }
    spec bitwise_severed {
        pragma opaque;
        pragma bv_internal;
        ensures [abstract] result == spec_generic(x);
    }

    fun severed_vector(x: u8): vector<u8> {
        vector[x & 1]
    }
    spec severed_vector {
        pragma opaque;
        pragma bv_internal;
        ensures result == result;
    }

    public fun public_masked(x: u16): u16 {
        x & 0xff
    }
    spec public_masked {
        pragma opaque;
        pragma bv_internal;
        ensures (result as u64) <= 0xff;
    }
}

module 0x42::bv_internal_client {
    use 0x42::bv_internal;

    fun cross_module_caller(x: u16): u16 {
        let r = bv_internal::public_masked(x);
        if (r < 0xffff) { r + 1 } else { 0 }
    }
    spec cross_module_caller {
        ensures result <= 0x100;
    }
}
