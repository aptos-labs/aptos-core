module 0x1::ristretto255_elgamal {
    struct Ciphertext has drop {
        left: 0x1::ristretto255::RistrettoPoint,
        right: 0x1::ristretto255::RistrettoPoint,
    }
    
    struct CompressedCiphertext has copy, drop, store {
        left: 0x1::ristretto255::CompressedRistretto,
        right: 0x1::ristretto255::CompressedRistretto,
    }
    
    struct CompressedPubkey has copy, drop, store {
        point: 0x1::ristretto255::CompressedRistretto,
    }
    
    public fun ciphertext_add(arg0: &Ciphertext, arg1: &Ciphertext) : Ciphertext {
        let v0 = 0x1::ristretto255::point_add(&arg0.right, &arg1.right);
        Ciphertext{
            left  : 0x1::ristretto255::point_add(&arg0.left, &arg1.left), 
            right : v0,
        }
    }
    
    public fun ciphertext_add_assign(arg0: &mut Ciphertext, arg1: &Ciphertext) {
        0x1::ristretto255::point_add_assign(&mut arg0.left, &arg1.left);
        0x1::ristretto255::point_add_assign(&mut arg0.right, &arg1.right);
    }
    
    public fun ciphertext_as_points(arg0: &Ciphertext) : (&0x1::ristretto255::RistrettoPoint, &0x1::ristretto255::RistrettoPoint) {
        (&arg0.left, &arg0.right)
    }
    
    public fun ciphertext_clone(arg0: &Ciphertext) : Ciphertext {
        let v0 = 0x1::ristretto255::point_clone(&arg0.right);
        Ciphertext{
            left  : 0x1::ristretto255::point_clone(&arg0.left), 
            right : v0,
        }
    }
    
    public fun ciphertext_equals(arg0: &Ciphertext, arg1: &Ciphertext) : bool {
        let v0 = 0x1::ristretto255::point_equals(&arg0.left, &arg1.left);
        v0 && 0x1::ristretto255::point_equals(&arg0.right, &arg1.right)
    }
    
    public fun ciphertext_from_compressed_points(arg0: 0x1::ristretto255::CompressedRistretto, arg1: 0x1::ristretto255::CompressedRistretto) : CompressedCiphertext {
        CompressedCiphertext{
            left  : arg0, 
            right : arg1,
        }
    }
    
    public fun ciphertext_from_points(arg0: 0x1::ristretto255::RistrettoPoint, arg1: 0x1::ristretto255::RistrettoPoint) : Ciphertext {
        Ciphertext{
            left  : arg0, 
            right : arg1,
        }
    }
    
    public fun ciphertext_into_points(arg0: Ciphertext) : (0x1::ristretto255::RistrettoPoint, 0x1::ristretto255::RistrettoPoint) {
        let Ciphertext {
            left  : v0,
            right : v1,
        } = arg0;
        (v0, v1)
    }
    
    public fun ciphertext_sub(arg0: &Ciphertext, arg1: &Ciphertext) : Ciphertext {
        let v0 = 0x1::ristretto255::point_sub(&arg0.right, &arg1.right);
        Ciphertext{
            left  : 0x1::ristretto255::point_sub(&arg0.left, &arg1.left), 
            right : v0,
        }
    }
    
    public fun ciphertext_sub_assign(arg0: &mut Ciphertext, arg1: &Ciphertext) {
        0x1::ristretto255::point_sub_assign(&mut arg0.left, &arg1.left);
        0x1::ristretto255::point_sub_assign(&mut arg0.right, &arg1.right);
    }
    
    public fun ciphertext_to_bytes(arg0: &Ciphertext) : vector<u8> {
        let v0 = 0x1::ristretto255::point_compress(&arg0.left);
        let v1 = 0x1::ristretto255::point_compress(&arg0.right);
        let v2 = 0x1::vector::empty<u8>();
        0x1::vector::append<u8>(&mut v2, 0x1::ristretto255::point_to_bytes(&v0));
        0x1::vector::append<u8>(&mut v2, 0x1::ristretto255::point_to_bytes(&v1));
        v2
    }
    
    public fun compress_ciphertext(arg0: &Ciphertext) : CompressedCiphertext {
        let v0 = 0x1::ristretto255::point_compress(&arg0.right);
        CompressedCiphertext{
            left  : 0x1::ristretto255::point_compress(&arg0.left), 
            right : v0,
        }
    }
    
    public fun decompress_ciphertext(arg0: &CompressedCiphertext) : Ciphertext {
        let v0 = 0x1::ristretto255::point_decompress(&arg0.right);
        Ciphertext{
            left  : 0x1::ristretto255::point_decompress(&arg0.left), 
            right : v0,
        }
    }
    
    public fun get_value_component(arg0: &Ciphertext) : &0x1::ristretto255::RistrettoPoint {
        &arg0.left
    }
    
    public fun new_ciphertext_from_bytes(arg0: vector<u8>) : 0x1::option::Option<Ciphertext> {
        if (0x1::vector::length<u8>(&arg0) != 64) {
            return 0x1::option::none<Ciphertext>()
        };
        let v0 = 0x1::ristretto255::new_point_from_bytes(arg0);
        let v1 = 0x1::ristretto255::new_point_from_bytes(0x1::vector::trim<u8>(&mut arg0, 32));
        let v2 = 0x1::option::is_some<0x1::ristretto255::RistrettoPoint>(&mut v0);
        if (v2 && 0x1::option::is_some<0x1::ristretto255::RistrettoPoint>(&mut v1)) {
            let v4 = 0x1::option::extract<0x1::ristretto255::RistrettoPoint>(&mut v0);
            let v5 = Ciphertext{
                left  : v4, 
                right : 0x1::option::extract<0x1::ristretto255::RistrettoPoint>(&mut v1),
            };
            0x1::option::some<Ciphertext>(v5)
        } else {
            0x1::option::none<Ciphertext>()
        }
    }
    
    public fun new_ciphertext_no_randomness(arg0: &0x1::ristretto255::Scalar) : Ciphertext {
        Ciphertext{
            left  : 0x1::ristretto255::basepoint_mul(arg0), 
            right : 0x1::ristretto255::point_identity(),
        }
    }
    
    public fun new_pubkey_from_bytes(arg0: vector<u8>) : 0x1::option::Option<CompressedPubkey> {
        let v0 = 0x1::ristretto255::new_compressed_point_from_bytes(arg0);
        if (0x1::option::is_some<0x1::ristretto255::CompressedRistretto>(&mut v0)) {
            let v2 = CompressedPubkey{point: 0x1::option::extract<0x1::ristretto255::CompressedRistretto>(&mut v0)};
            0x1::option::some<CompressedPubkey>(v2)
        } else {
            0x1::option::none<CompressedPubkey>()
        }
    }
    
    public fun pubkey_to_bytes(arg0: &CompressedPubkey) : vector<u8> {
        0x1::ristretto255::compressed_point_to_bytes(arg0.point)
    }
    
    public fun pubkey_to_compressed_point(arg0: &CompressedPubkey) : 0x1::ristretto255::CompressedRistretto {
        arg0.point
    }
    
    public fun pubkey_to_point(arg0: &CompressedPubkey) : 0x1::ristretto255::RistrettoPoint {
        0x1::ristretto255::point_decompress(&arg0.point)
    }
    
    // decompiled from Move bytecode v6
}
