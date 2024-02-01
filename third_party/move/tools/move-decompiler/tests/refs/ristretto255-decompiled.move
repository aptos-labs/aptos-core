module 0x1::ristretto255 {
    struct CompressedRistretto has copy, drop, store {
        data: vector<u8>,
    }
    
    struct RistrettoPoint has drop {
        handle: u64,
    }
    
    struct Scalar has copy, drop, store {
        data: vector<u8>,
    }
    
    public fun basepoint() : RistrettoPoint {
        let (v0, _) = point_decompress_internal(x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76");
        RistrettoPoint{handle: v0}
    }
    
    public fun basepoint_compressed() : CompressedRistretto {
        CompressedRistretto{data: x"e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76"}
    }
    
    public fun basepoint_double_mul(arg0: &Scalar, arg1: &RistrettoPoint, arg2: &Scalar) : RistrettoPoint {
        RistrettoPoint{handle: basepoint_double_mul_internal(arg0.data, arg1, arg2.data)}
    }
    
    native fun basepoint_double_mul_internal(arg0: vector<u8>, arg1: &RistrettoPoint, arg2: vector<u8>) : u64;
    public fun basepoint_mul(arg0: &Scalar) : RistrettoPoint {
        RistrettoPoint{handle: basepoint_mul_internal(arg0.data)}
    }
    
    native fun basepoint_mul_internal(arg0: vector<u8>) : u64;
    public fun compressed_point_to_bytes(arg0: CompressedRistretto) : vector<u8> {
        arg0.data
    }
    
    public fun double_scalar_mul(arg0: &Scalar, arg1: &RistrettoPoint, arg2: &Scalar, arg3: &RistrettoPoint) : RistrettoPoint {
        if (!0x1::features::bulletproofs_enabled()) {
            abort 0x1::error::invalid_state(5)
        };
        RistrettoPoint{handle: double_scalar_mul_internal(arg1.handle, arg3.handle, arg0.data, arg2.data)}
    }
    
    native fun double_scalar_mul_internal(arg0: u64, arg1: u64, arg2: vector<u8>, arg3: vector<u8>) : u64;
    public fun hash_to_point_base() : RistrettoPoint {
        let v0 = CompressedRistretto{data: x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134"};
        point_decompress(&v0)
    }
    
    public fun multi_scalar_mul(arg0: &vector<RistrettoPoint>, arg1: &vector<Scalar>) : RistrettoPoint {
        assert!(!0x1::vector::is_empty<RistrettoPoint>(arg0), 0x1::error::invalid_argument(2));
        assert!(!0x1::vector::is_empty<Scalar>(arg1), 0x1::error::invalid_argument(3));
        let v0 = 0x1::vector::length<RistrettoPoint>(arg0) == 0x1::vector::length<Scalar>(arg1);
        assert!(v0, 0x1::error::invalid_argument(1));
        RistrettoPoint{handle: multi_scalar_mul_internal<RistrettoPoint, Scalar>(arg0, arg1)}
    }
    
    native fun multi_scalar_mul_internal<T0, T1>(arg0: &vector<T0>, arg1: &vector<T1>) : u64;
    public fun new_compressed_point_from_bytes(arg0: vector<u8>) : 0x1::option::Option<CompressedRistretto> {
        if (point_is_canonical_internal(arg0)) {
            let v1 = CompressedRistretto{data: arg0};
            0x1::option::some<CompressedRistretto>(v1)
        } else {
            0x1::option::none<CompressedRistretto>()
        }
    }
    
    public fun new_point_from_64_uniform_bytes(arg0: vector<u8>) : 0x1::option::Option<RistrettoPoint> {
        if (0x1::vector::length<u8>(&arg0) == 64) {
            let v1 = RistrettoPoint{handle: new_point_from_64_uniform_bytes_internal(arg0)};
            0x1::option::some<RistrettoPoint>(v1)
        } else {
            0x1::option::none<RistrettoPoint>()
        }
    }
    
    native fun new_point_from_64_uniform_bytes_internal(arg0: vector<u8>) : u64;
    public fun new_point_from_bytes(arg0: vector<u8>) : 0x1::option::Option<RistrettoPoint> {
        let (v0, v1) = point_decompress_internal(arg0);
        if (v1) {
            let v3 = RistrettoPoint{handle: v0};
            0x1::option::some<RistrettoPoint>(v3)
        } else {
            0x1::option::none<RistrettoPoint>()
        }
    }
    
    public fun new_point_from_sha2_512(arg0: vector<u8>) : RistrettoPoint {
        RistrettoPoint{handle: new_point_from_sha512_internal(arg0)}
    }
    
    public fun new_point_from_sha512(arg0: vector<u8>) : RistrettoPoint {
        new_point_from_sha2_512(arg0)
    }
    
    native fun new_point_from_sha512_internal(arg0: vector<u8>) : u64;
    public fun new_scalar_from_bytes(arg0: vector<u8>) : 0x1::option::Option<Scalar> {
        if (scalar_is_canonical_internal(arg0)) {
            let v1 = Scalar{data: arg0};
            0x1::option::some<Scalar>(v1)
        } else {
            0x1::option::none<Scalar>()
        }
    }
    
    public fun new_scalar_from_sha2_512(arg0: vector<u8>) : Scalar {
        Scalar{data: scalar_from_sha512_internal(arg0)}
    }
    
    public fun new_scalar_from_sha512(arg0: vector<u8>) : Scalar {
        new_scalar_from_sha2_512(arg0)
    }
    
    public fun new_scalar_from_u128(arg0: u128) : Scalar {
        Scalar{data: scalar_from_u128_internal(arg0)}
    }
    
    public fun new_scalar_from_u32(arg0: u32) : Scalar {
        Scalar{data: scalar_from_u64_internal(arg0 as u64)}
    }
    
    public fun new_scalar_from_u64(arg0: u64) : Scalar {
        Scalar{data: scalar_from_u64_internal(arg0)}
    }
    
    public fun new_scalar_from_u8(arg0: u8) : Scalar {
        let v0 = scalar_zero();
        *0x1::vector::borrow_mut<u8>(&mut v0.data, 0) = arg0;
        v0
    }
    
    public fun new_scalar_reduced_from_32_bytes(arg0: vector<u8>) : 0x1::option::Option<Scalar> {
        if (0x1::vector::length<u8>(&arg0) == 32) {
            let v1 = Scalar{data: scalar_reduced_from_32_bytes_internal(arg0)};
            0x1::option::some<Scalar>(v1)
        } else {
            0x1::option::none<Scalar>()
        }
    }
    
    public fun new_scalar_uniform_from_64_bytes(arg0: vector<u8>) : 0x1::option::Option<Scalar> {
        if (0x1::vector::length<u8>(&arg0) == 64) {
            let v1 = Scalar{data: scalar_uniform_from_64_bytes_internal(arg0)};
            0x1::option::some<Scalar>(v1)
        } else {
            0x1::option::none<Scalar>()
        }
    }
    
    public fun point_add(arg0: &RistrettoPoint, arg1: &RistrettoPoint) : RistrettoPoint {
        RistrettoPoint{handle: point_add_internal(arg0, arg1, false)}
    }
    
    public fun point_add_assign(arg0: &mut RistrettoPoint, arg1: &RistrettoPoint) : &mut RistrettoPoint {
        point_add_internal(arg0, arg1, true);
        arg0
    }
    
    native fun point_add_internal(arg0: &RistrettoPoint, arg1: &RistrettoPoint, arg2: bool) : u64;
    public fun point_clone(arg0: &RistrettoPoint) : RistrettoPoint {
        if (!0x1::features::bulletproofs_enabled()) {
            abort 0x1::error::invalid_state(5)
        };
        RistrettoPoint{handle: point_clone_internal(arg0.handle)}
    }
    
    native fun point_clone_internal(arg0: u64) : u64;
    public fun point_compress(arg0: &RistrettoPoint) : CompressedRistretto {
        CompressedRistretto{data: point_compress_internal(arg0)}
    }
    
    native fun point_compress_internal(arg0: &RistrettoPoint) : vector<u8>;
    public fun point_decompress(arg0: &CompressedRistretto) : RistrettoPoint {
        let (v0, _) = point_decompress_internal(arg0.data);
        RistrettoPoint{handle: v0}
    }
    
    native fun point_decompress_internal(arg0: vector<u8>) : (u64, bool);
    native public fun point_equals(arg0: &RistrettoPoint, arg1: &RistrettoPoint) : bool;
    public fun point_identity() : RistrettoPoint {
        RistrettoPoint{handle: point_identity_internal()}
    }
    
    public fun point_identity_compressed() : CompressedRistretto {
        CompressedRistretto{data: x"0000000000000000000000000000000000000000000000000000000000000000"}
    }
    
    native fun point_identity_internal() : u64;
    native fun point_is_canonical_internal(arg0: vector<u8>) : bool;
    public fun point_mul(arg0: &RistrettoPoint, arg1: &Scalar) : RistrettoPoint {
        RistrettoPoint{handle: point_mul_internal(arg0, arg1.data, false)}
    }
    
    public fun point_mul_assign(arg0: &mut RistrettoPoint, arg1: &Scalar) : &mut RistrettoPoint {
        point_mul_internal(arg0, arg1.data, true);
        arg0
    }
    
    native fun point_mul_internal(arg0: &RistrettoPoint, arg1: vector<u8>, arg2: bool) : u64;
    public fun point_neg(arg0: &RistrettoPoint) : RistrettoPoint {
        RistrettoPoint{handle: point_neg_internal(arg0, false)}
    }
    
    public fun point_neg_assign(arg0: &mut RistrettoPoint) : &mut RistrettoPoint {
        point_neg_internal(arg0, true);
        arg0
    }
    
    native fun point_neg_internal(arg0: &RistrettoPoint, arg1: bool) : u64;
    public fun point_sub(arg0: &RistrettoPoint, arg1: &RistrettoPoint) : RistrettoPoint {
        RistrettoPoint{handle: point_sub_internal(arg0, arg1, false)}
    }
    
    public fun point_sub_assign(arg0: &mut RistrettoPoint, arg1: &RistrettoPoint) : &mut RistrettoPoint {
        point_sub_internal(arg0, arg1, true);
        arg0
    }
    
    native fun point_sub_internal(arg0: &RistrettoPoint, arg1: &RistrettoPoint, arg2: bool) : u64;
    public fun point_to_bytes(arg0: &CompressedRistretto) : vector<u8> {
        arg0.data
    }
    
    public fun scalar_add(arg0: &Scalar, arg1: &Scalar) : Scalar {
        Scalar{data: scalar_add_internal(arg0.data, arg1.data)}
    }
    
    public fun scalar_add_assign(arg0: &mut Scalar, arg1: &Scalar) : &mut Scalar {
        let v0 = scalar_add(arg0, arg1);
        arg0.data = v0.data;
        arg0
    }
    
    native fun scalar_add_internal(arg0: vector<u8>, arg1: vector<u8>) : vector<u8>;
    public fun scalar_equals(arg0: &Scalar, arg1: &Scalar) : bool {
        arg0.data == arg1.data
    }
    
    native fun scalar_from_sha512_internal(arg0: vector<u8>) : vector<u8>;
    native fun scalar_from_u128_internal(arg0: u128) : vector<u8>;
    native fun scalar_from_u64_internal(arg0: u64) : vector<u8>;
    public fun scalar_invert(arg0: &Scalar) : 0x1::option::Option<Scalar> {
        if (scalar_is_zero(arg0)) {
            0x1::option::none<Scalar>()
        } else {
            let v1 = Scalar{data: scalar_invert_internal(arg0.data)};
            0x1::option::some<Scalar>(v1)
        }
    }
    
    native fun scalar_invert_internal(arg0: vector<u8>) : vector<u8>;
    native fun scalar_is_canonical_internal(arg0: vector<u8>) : bool;
    public fun scalar_is_one(arg0: &Scalar) : bool {
        arg0.data == x"0100000000000000000000000000000000000000000000000000000000000000"
    }
    
    public fun scalar_is_zero(arg0: &Scalar) : bool {
        arg0.data == x"0000000000000000000000000000000000000000000000000000000000000000"
    }
    
    public fun scalar_mul(arg0: &Scalar, arg1: &Scalar) : Scalar {
        Scalar{data: scalar_mul_internal(arg0.data, arg1.data)}
    }
    
    public fun scalar_mul_assign(arg0: &mut Scalar, arg1: &Scalar) : &mut Scalar {
        let v0 = scalar_mul(arg0, arg1);
        arg0.data = v0.data;
        arg0
    }
    
    native fun scalar_mul_internal(arg0: vector<u8>, arg1: vector<u8>) : vector<u8>;
    public fun scalar_neg(arg0: &Scalar) : Scalar {
        Scalar{data: scalar_neg_internal(arg0.data)}
    }
    
    public fun scalar_neg_assign(arg0: &mut Scalar) : &mut Scalar {
        let v0 = scalar_neg(arg0);
        arg0.data = v0.data;
        arg0
    }
    
    native fun scalar_neg_internal(arg0: vector<u8>) : vector<u8>;
    public fun scalar_one() : Scalar {
        Scalar{data: x"0100000000000000000000000000000000000000000000000000000000000000"}
    }
    
    native fun scalar_reduced_from_32_bytes_internal(arg0: vector<u8>) : vector<u8>;
    public fun scalar_sub(arg0: &Scalar, arg1: &Scalar) : Scalar {
        Scalar{data: scalar_sub_internal(arg0.data, arg1.data)}
    }
    
    public fun scalar_sub_assign(arg0: &mut Scalar, arg1: &Scalar) : &mut Scalar {
        let v0 = scalar_sub(arg0, arg1);
        arg0.data = v0.data;
        arg0
    }
    
    native fun scalar_sub_internal(arg0: vector<u8>, arg1: vector<u8>) : vector<u8>;
    public fun scalar_to_bytes(arg0: &Scalar) : vector<u8> {
        arg0.data
    }
    
    native fun scalar_uniform_from_64_bytes_internal(arg0: vector<u8>) : vector<u8>;
    public fun scalar_zero() : Scalar {
        Scalar{data: x"0000000000000000000000000000000000000000000000000000000000000000"}
    }
    
    // decompiled from Move bytecode v6
}
