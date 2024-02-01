module 0x1::bls12381 {
    struct AggrOrMultiSignature has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct AggrPublicKeysWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct ProofOfPossession has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct PublicKey has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct Signature has copy, drop, store {
        bytes: vector<u8>,
    }
    
    public fun aggr_or_multi_signature_from_bytes(arg0: vector<u8>) : AggrOrMultiSignature {
        assert!(0x1::vector::length<u8>(&arg0) == 96, 0x1::error::invalid_argument(2));
        AggrOrMultiSignature{bytes: arg0}
    }
    
    public fun aggr_or_multi_signature_subgroup_check(arg0: &AggrOrMultiSignature) : bool {
        signature_subgroup_check_internal(arg0.bytes)
    }
    
    public fun aggr_or_multi_signature_to_bytes(arg0: &AggrOrMultiSignature) : vector<u8> {
        arg0.bytes
    }
    
    public fun aggregate_pubkey_to_bytes(arg0: &AggrPublicKeysWithPoP) : vector<u8> {
        arg0.bytes
    }
    
    public fun aggregate_pubkeys(arg0: vector<PublicKeyWithPoP>) : AggrPublicKeysWithPoP {
        let (v0, v1) = aggregate_pubkeys_internal(arg0);
        assert!(v1, 0x1::error::invalid_argument(1));
        AggrPublicKeysWithPoP{bytes: v0}
    }
    
    native fun aggregate_pubkeys_internal(arg0: vector<PublicKeyWithPoP>) : (vector<u8>, bool);
    public fun aggregate_signatures(arg0: vector<Signature>) : 0x1::option::Option<AggrOrMultiSignature> {
        let (v0, v1) = aggregate_signatures_internal(arg0);
        if (v1) {
            let v3 = AggrOrMultiSignature{bytes: v0};
            0x1::option::some<AggrOrMultiSignature>(v3)
        } else {
            0x1::option::none<AggrOrMultiSignature>()
        }
    }
    
    native fun aggregate_signatures_internal(arg0: vector<Signature>) : (vector<u8>, bool);
    public fun proof_of_possession_from_bytes(arg0: vector<u8>) : ProofOfPossession {
        ProofOfPossession{bytes: arg0}
    }
    
    public fun proof_of_possession_to_bytes(arg0: &ProofOfPossession) : vector<u8> {
        arg0.bytes
    }
    
    public fun public_key_from_bytes(arg0: vector<u8>) : 0x1::option::Option<PublicKey> {
        if (validate_pubkey_internal(arg0)) {
            let v1 = PublicKey{bytes: arg0};
            0x1::option::some<PublicKey>(v1)
        } else {
            0x1::option::none<PublicKey>()
        }
    }
    
    public fun public_key_from_bytes_with_pop(arg0: vector<u8>, arg1: &ProofOfPossession) : 0x1::option::Option<PublicKeyWithPoP> {
        if (verify_proof_of_possession_internal(arg0, arg1.bytes)) {
            let v1 = PublicKeyWithPoP{bytes: arg0};
            0x1::option::some<PublicKeyWithPoP>(v1)
        } else {
            0x1::option::none<PublicKeyWithPoP>()
        }
    }
    
    public fun public_key_to_bytes(arg0: &PublicKey) : vector<u8> {
        arg0.bytes
    }
    
    public fun public_key_with_pop_to_bytes(arg0: &PublicKeyWithPoP) : vector<u8> {
        arg0.bytes
    }
    
    public fun public_key_with_pop_to_normal(arg0: &PublicKeyWithPoP) : PublicKey {
        PublicKey{bytes: arg0.bytes}
    }
    
    public fun signature_from_bytes(arg0: vector<u8>) : Signature {
        Signature{bytes: arg0}
    }
    
    public fun signature_subgroup_check(arg0: &Signature) : bool {
        signature_subgroup_check_internal(arg0.bytes)
    }
    
    native fun signature_subgroup_check_internal(arg0: vector<u8>) : bool;
    public fun signature_to_bytes(arg0: &Signature) : vector<u8> {
        arg0.bytes
    }
    
    native fun validate_pubkey_internal(arg0: vector<u8>) : bool;
    public fun verify_aggregate_signature(arg0: &AggrOrMultiSignature, arg1: vector<PublicKeyWithPoP>, arg2: vector<vector<u8>>) : bool {
        verify_aggregate_signature_internal(arg0.bytes, arg1, arg2)
    }
    
    native fun verify_aggregate_signature_internal(arg0: vector<u8>, arg1: vector<PublicKeyWithPoP>, arg2: vector<vector<u8>>) : bool;
    public fun verify_multisignature(arg0: &AggrOrMultiSignature, arg1: &AggrPublicKeysWithPoP, arg2: vector<u8>) : bool {
        verify_multisignature_internal(arg0.bytes, arg1.bytes, arg2)
    }
    
    native fun verify_multisignature_internal(arg0: vector<u8>, arg1: vector<u8>, arg2: vector<u8>) : bool;
    public fun verify_normal_signature(arg0: &Signature, arg1: &PublicKey, arg2: vector<u8>) : bool {
        verify_normal_signature_internal(arg0.bytes, arg1.bytes, arg2)
    }
    
    native fun verify_normal_signature_internal(arg0: vector<u8>, arg1: vector<u8>, arg2: vector<u8>) : bool;
    native fun verify_proof_of_possession_internal(arg0: vector<u8>, arg1: vector<u8>) : bool;
    public fun verify_signature_share(arg0: &Signature, arg1: &PublicKeyWithPoP, arg2: vector<u8>) : bool {
        verify_signature_share_internal(arg0.bytes, arg1.bytes, arg2)
    }
    
    native fun verify_signature_share_internal(arg0: vector<u8>, arg1: vector<u8>, arg2: vector<u8>) : bool;
    // decompiled from Move bytecode v6
}
