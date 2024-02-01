module 0x1::ristretto255_bulletproofs {
    struct RangeProof has copy, drop, store {
        bytes: vector<u8>,
    }
    
    public fun get_max_range_bits() : u64 {
        64
    }
    
    public fun range_proof_from_bytes(arg0: vector<u8>) : RangeProof {
        RangeProof{bytes: arg0}
    }
    
    public fun range_proof_to_bytes(arg0: &RangeProof) : vector<u8> {
        arg0.bytes
    }
    
    public fun verify_range_proof(arg0: &0x1::ristretto255::RistrettoPoint, arg1: &0x1::ristretto255::RistrettoPoint, arg2: &0x1::ristretto255::RistrettoPoint, arg3: &RangeProof, arg4: u64, arg5: vector<u8>) : bool {
        assert!(0x1::features::bulletproofs_enabled(), 0x1::error::invalid_state(4));
        let v0 = 0x1::ristretto255::point_compress(arg0);
        let v1 = 0x1::ristretto255::point_to_bytes(&v0);
        verify_range_proof_internal(v1, arg1, arg2, arg3.bytes, arg4, arg5)
    }
    
    native fun verify_range_proof_internal(arg0: vector<u8>, arg1: &0x1::ristretto255::RistrettoPoint, arg2: &0x1::ristretto255::RistrettoPoint, arg3: vector<u8>, arg4: u64, arg5: vector<u8>) : bool;
    public fun verify_range_proof_pedersen(arg0: &0x1::ristretto255_pedersen::Commitment, arg1: &RangeProof, arg2: u64, arg3: vector<u8>) : bool {
        assert!(0x1::features::bulletproofs_enabled(), 0x1::error::invalid_state(4));
        let v0 = 0x1::ristretto255_pedersen::commitment_as_compressed_point(arg0);
        let v1 = 0x1::ristretto255::point_to_bytes(&v0);
        let v2 = 0x1::ristretto255::basepoint();
        let v3 = 0x1::ristretto255::hash_to_point_base();
        verify_range_proof_internal(v1, &v2, &v3, arg1.bytes, arg2, arg3)
    }
    
    // decompiled from Move bytecode v6
}
