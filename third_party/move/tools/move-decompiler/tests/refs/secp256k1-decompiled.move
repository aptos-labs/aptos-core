module 0x1::secp256k1 {
    struct ECDSARawPublicKey has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct ECDSASignature has copy, drop, store {
        bytes: vector<u8>,
    }
    
    public fun ecdsa_raw_public_key_from_64_bytes(arg0: vector<u8>) : ECDSARawPublicKey {
        assert!(0x1::vector::length<u8>(&arg0) == 64, 0x1::error::invalid_argument(1));
        ECDSARawPublicKey{bytes: arg0}
    }
    
    public fun ecdsa_raw_public_key_to_bytes(arg0: &ECDSARawPublicKey) : vector<u8> {
        arg0.bytes
    }
    
    public fun ecdsa_recover(arg0: vector<u8>, arg1: u8, arg2: &ECDSASignature) : 0x1::option::Option<ECDSARawPublicKey> {
        let (v0, v1) = ecdsa_recover_internal(arg0, arg1, arg2.bytes);
        if (v1) {
            0x1::option::some<ECDSARawPublicKey>(ecdsa_raw_public_key_from_64_bytes(v0))
        } else {
            0x1::option::none<ECDSARawPublicKey>()
        }
    }
    
    native fun ecdsa_recover_internal(arg0: vector<u8>, arg1: u8, arg2: vector<u8>) : (vector<u8>, bool);
    public fun ecdsa_signature_from_bytes(arg0: vector<u8>) : ECDSASignature {
        assert!(0x1::vector::length<u8>(&arg0) == 64, 0x1::error::invalid_argument(1));
        ECDSASignature{bytes: arg0}
    }
    
    public fun ecdsa_signature_to_bytes(arg0: &ECDSASignature) : vector<u8> {
        arg0.bytes
    }
    
    // decompiled from Move bytecode v6
}
