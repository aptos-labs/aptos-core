module 0x1::ristretto255_pedersen {
    struct Commitment has drop {
        point: 0x1::ristretto255::RistrettoPoint,
    }
    
    public fun commitment_add(arg0: &Commitment, arg1: &Commitment) : Commitment {
        Commitment{point: 0x1::ristretto255::point_add(&arg0.point, &arg1.point)}
    }
    
    public fun commitment_add_assign(arg0: &mut Commitment, arg1: &Commitment) {
        0x1::ristretto255::point_add_assign(&mut arg0.point, &arg1.point);
    }
    
    public fun commitment_as_compressed_point(arg0: &Commitment) : 0x1::ristretto255::CompressedRistretto {
        0x1::ristretto255::point_compress(&arg0.point)
    }
    
    public fun commitment_as_point(arg0: &Commitment) : &0x1::ristretto255::RistrettoPoint {
        &arg0.point
    }
    
    public fun commitment_clone(arg0: &Commitment) : Commitment {
        Commitment{point: 0x1::ristretto255::point_clone(&arg0.point)}
    }
    
    public fun commitment_equals(arg0: &Commitment, arg1: &Commitment) : bool {
        0x1::ristretto255::point_equals(&arg0.point, &arg1.point)
    }
    
    public fun commitment_from_compressed(arg0: &0x1::ristretto255::CompressedRistretto) : Commitment {
        Commitment{point: 0x1::ristretto255::point_decompress(arg0)}
    }
    
    public fun commitment_from_point(arg0: 0x1::ristretto255::RistrettoPoint) : Commitment {
        Commitment{point: arg0}
    }
    
    public fun commitment_into_compressed_point(arg0: Commitment) : 0x1::ristretto255::CompressedRistretto {
        0x1::ristretto255::point_compress(&arg0.point)
    }
    
    public fun commitment_into_point(arg0: Commitment) : 0x1::ristretto255::RistrettoPoint {
        let Commitment { point: v0 } = arg0;
        v0
    }
    
    public fun commitment_sub(arg0: &Commitment, arg1: &Commitment) : Commitment {
        Commitment{point: 0x1::ristretto255::point_sub(&arg0.point, &arg1.point)}
    }
    
    public fun commitment_sub_assign(arg0: &mut Commitment, arg1: &Commitment) {
        0x1::ristretto255::point_sub_assign(&mut arg0.point, &arg1.point);
    }
    
    public fun commitment_to_bytes(arg0: &Commitment) : vector<u8> {
        let v0 = 0x1::ristretto255::point_compress(&arg0.point);
        0x1::ristretto255::point_to_bytes(&v0)
    }
    
    public fun new_commitment(arg0: &0x1::ristretto255::Scalar, arg1: &0x1::ristretto255::RistrettoPoint, arg2: &0x1::ristretto255::Scalar, arg3: &0x1::ristretto255::RistrettoPoint) : Commitment {
        Commitment{point: 0x1::ristretto255::double_scalar_mul(arg0, arg1, arg2, arg3)}
    }
    
    public fun new_commitment_for_bulletproof(arg0: &0x1::ristretto255::Scalar, arg1: &0x1::ristretto255::Scalar) : Commitment {
        let v0 = 0x1::ristretto255::new_point_from_bytes(x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134");
        let v1 = 0x1::option::extract<0x1::ristretto255::RistrettoPoint>(&mut v0);
        Commitment{point: 0x1::ristretto255::basepoint_double_mul(arg1, &v1, arg0)}
    }
    
    public fun new_commitment_from_bytes(arg0: vector<u8>) : 0x1::option::Option<Commitment> {
        let v0 = 0x1::ristretto255::new_point_from_bytes(arg0);
        if (0x1::option::is_some<0x1::ristretto255::RistrettoPoint>(&mut v0)) {
            let v2 = Commitment{point: 0x1::option::extract<0x1::ristretto255::RistrettoPoint>(&mut v0)};
            0x1::option::some<Commitment>(v2)
        } else {
            0x1::option::none<Commitment>()
        }
    }
    
    public fun new_commitment_with_basepoint(arg0: &0x1::ristretto255::Scalar, arg1: &0x1::ristretto255::Scalar, arg2: &0x1::ristretto255::RistrettoPoint) : Commitment {
        Commitment{point: 0x1::ristretto255::basepoint_double_mul(arg1, arg2, arg0)}
    }
    
    public fun randomness_base_for_bulletproof() : 0x1::ristretto255::RistrettoPoint {
        let v0 = 0x1::ristretto255::new_point_from_bytes(x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134");
        0x1::option::extract<0x1::ristretto255::RistrettoPoint>(&mut v0)
    }
    
    // decompiled from Move bytecode v6
}
