module 0x1::crypto_algebra {
    struct Element<phantom T0> has copy, drop {
        handle: u64,
    }
    
    fun abort_unless_cryptography_algebra_natives_enabled() {
        assert!(0x1::features::cryptography_algebra_enabled(), 0x1::error::not_implemented(0));
    }
    
    public fun add<T0>(arg0: &Element<T0>, arg1: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: add_internal<T0>(arg0.handle, arg1.handle)}
    }
    
    native fun add_internal<T0>(arg0: u64, arg1: u64) : u64;
    public fun deserialize<T0, T1>(arg0: &vector<u8>) : 0x1::option::Option<Element<T0>> {
        abort_unless_cryptography_algebra_natives_enabled();
        let (v0, v1) = deserialize_internal<T0, T1>(arg0);
        if (v0) {
            let v3 = Element<T0>{handle: v1};
            0x1::option::some<Element<T0>>(v3)
        } else {
            0x1::option::none<Element<T0>>()
        }
    }
    
    native fun deserialize_internal<T0, T1>(arg0: &vector<u8>) : (bool, u64);
    public fun div<T0>(arg0: &Element<T0>, arg1: &Element<T0>) : 0x1::option::Option<Element<T0>> {
        abort_unless_cryptography_algebra_natives_enabled();
        let (v0, v1) = div_internal<T0>(arg0.handle, arg1.handle);
        if (v0) {
            let v3 = Element<T0>{handle: v1};
            0x1::option::some<Element<T0>>(v3)
        } else {
            0x1::option::none<Element<T0>>()
        }
    }
    
    native fun div_internal<T0>(arg0: u64, arg1: u64) : (bool, u64);
    public fun double<T0>(arg0: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: double_internal<T0>(arg0.handle)}
    }
    
    native fun double_internal<T0>(arg0: u64) : u64;
    public fun downcast<T0, T1>(arg0: &Element<T0>) : 0x1::option::Option<Element<T1>> {
        abort_unless_cryptography_algebra_natives_enabled();
        let (v0, v1) = downcast_internal<T0, T1>(arg0.handle);
        if (v0) {
            let v3 = Element<T1>{handle: v1};
            0x1::option::some<Element<T1>>(v3)
        } else {
            0x1::option::none<Element<T1>>()
        }
    }
    
    native fun downcast_internal<T0, T1>(arg0: u64) : (bool, u64);
    public fun eq<T0>(arg0: &Element<T0>, arg1: &Element<T0>) : bool {
        abort_unless_cryptography_algebra_natives_enabled();
        eq_internal<T0>(arg0.handle, arg1.handle)
    }
    
    native fun eq_internal<T0>(arg0: u64, arg1: u64) : bool;
    public fun from_u64<T0>(arg0: u64) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: from_u64_internal<T0>(arg0)}
    }
    
    native fun from_u64_internal<T0>(arg0: u64) : u64;
    fun handles_from_elements<T0>(arg0: &vector<Element<T0>>) : vector<u64> {
        let v0 = 0x1::vector::empty<u64>();
        let v1 = 0;
        while (v1 < 0x1::vector::length<Element<T0>>(arg0)) {
            0x1::vector::push_back<u64>(&mut v0, 0x1::vector::borrow<Element<T0>>(arg0, v1).handle);
            v1 = v1 + 1;
        };
        v0
    }
    
    public fun hash_to<T0, T1>(arg0: &vector<u8>, arg1: &vector<u8>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: hash_to_internal<T0, T1>(arg0, arg1)}
    }
    
    native fun hash_to_internal<T0, T1>(arg0: &vector<u8>, arg1: &vector<u8>) : u64;
    public fun inv<T0>(arg0: &Element<T0>) : 0x1::option::Option<Element<T0>> {
        abort_unless_cryptography_algebra_natives_enabled();
        let (v0, v1) = inv_internal<T0>(arg0.handle);
        if (v0) {
            let v3 = Element<T0>{handle: v1};
            0x1::option::some<Element<T0>>(v3)
        } else {
            0x1::option::none<Element<T0>>()
        }
    }
    
    native fun inv_internal<T0>(arg0: u64) : (bool, u64);
    public fun mul<T0>(arg0: &Element<T0>, arg1: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: mul_internal<T0>(arg0.handle, arg1.handle)}
    }
    
    native fun mul_internal<T0>(arg0: u64, arg1: u64) : u64;
    public fun multi_pairing<T0, T1, T2>(arg0: &vector<Element<T0>>, arg1: &vector<Element<T1>>) : Element<T2> {
        abort_unless_cryptography_algebra_natives_enabled();
        let v0 = multi_pairing_internal<T0, T1, T2>(handles_from_elements<T0>(arg0), handles_from_elements<T1>(arg1));
        Element<T2>{handle: v0}
    }
    
    native fun multi_pairing_internal<T0, T1, T2>(arg0: vector<u64>, arg1: vector<u64>) : u64;
    public fun multi_scalar_mul<T0, T1>(arg0: &vector<Element<T0>>, arg1: &vector<Element<T1>>) : Element<T0> {
        let v0 = multi_scalar_mul_internal<T0, T1>(handles_from_elements<T0>(arg0), handles_from_elements<T1>(arg1));
        Element<T0>{handle: v0}
    }
    
    native fun multi_scalar_mul_internal<T0, T1>(arg0: vector<u64>, arg1: vector<u64>) : u64;
    public fun neg<T0>(arg0: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: neg_internal<T0>(arg0.handle)}
    }
    
    native fun neg_internal<T0>(arg0: u64) : u64;
    public fun one<T0>() : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: one_internal<T0>()}
    }
    
    native fun one_internal<T0>() : u64;
    public fun order<T0>() : vector<u8> {
        abort_unless_cryptography_algebra_natives_enabled();
        order_internal<T0>()
    }
    
    native fun order_internal<T0>() : vector<u8>;
    public fun pairing<T0, T1, T2>(arg0: &Element<T0>, arg1: &Element<T1>) : Element<T2> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T2>{handle: pairing_internal<T0, T1, T2>(arg0.handle, arg1.handle)}
    }
    
    native fun pairing_internal<T0, T1, T2>(arg0: u64, arg1: u64) : u64;
    public fun scalar_mul<T0, T1>(arg0: &Element<T0>, arg1: &Element<T1>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: scalar_mul_internal<T0, T1>(arg0.handle, arg1.handle)}
    }
    
    native fun scalar_mul_internal<T0, T1>(arg0: u64, arg1: u64) : u64;
    public fun serialize<T0, T1>(arg0: &Element<T0>) : vector<u8> {
        abort_unless_cryptography_algebra_natives_enabled();
        serialize_internal<T0, T1>(arg0.handle)
    }
    
    native fun serialize_internal<T0, T1>(arg0: u64) : vector<u8>;
    public fun sqr<T0>(arg0: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: sqr_internal<T0>(arg0.handle)}
    }
    
    native fun sqr_internal<T0>(arg0: u64) : u64;
    public fun sub<T0>(arg0: &Element<T0>, arg1: &Element<T0>) : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: sub_internal<T0>(arg0.handle, arg1.handle)}
    }
    
    native fun sub_internal<T0>(arg0: u64, arg1: u64) : u64;
    public fun upcast<T0, T1>(arg0: &Element<T0>) : Element<T1> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T1>{handle: upcast_internal<T0, T1>(arg0.handle)}
    }
    
    native fun upcast_internal<T0, T1>(arg0: u64) : u64;
    public fun zero<T0>() : Element<T0> {
        abort_unless_cryptography_algebra_natives_enabled();
        Element<T0>{handle: zero_internal<T0>()}
    }
    
    native fun zero_internal<T0>() : u64;
    // decompiled from Move bytecode v6
}
