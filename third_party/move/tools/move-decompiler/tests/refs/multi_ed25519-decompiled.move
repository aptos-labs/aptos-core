module 0x1::multi_ed25519 {
    struct Signature has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct UnvalidatedPublicKey has copy, drop, store {
        bytes: vector<u8>,
    }
    
    struct ValidatedPublicKey has copy, drop, store {
        bytes: vector<u8>,
    }
    
    public fun check_and_get_threshold(arg0: vector<u8>) : 0x1::option::Option<u8> {
        let v0 = 0x1::vector::length<u8>(&arg0);
        if (v0 == 0) {
            return 0x1::option::none<u8>()
        };
        let v1 = v0 / 32;
        let v2 = *0x1::vector::borrow<u8>(&arg0, v0 - 1);
        if (v1 == 0 || v1 > 32 || v0 % 32 != 1) {
            return 0x1::option::none<u8>()
        };
        if (v2 == 0 || v2 > (v1 as u8)) {
            return 0x1::option::none<u8>()
        };
        0x1::option::some<u8>(v2)
    }
    
    public fun new_signature_from_bytes(arg0: vector<u8>) : Signature {
        assert!(0x1::vector::length<u8>(&arg0) % 64 == 4, 0x1::error::invalid_argument(2));
        Signature{bytes: arg0}
    }
    
    public fun new_unvalidated_public_key_from_bytes(arg0: vector<u8>) : UnvalidatedPublicKey {
        let v0 = 0x1::vector::length<u8>(&arg0);
        assert!(v0 / 32 <= 32, 0x1::error::invalid_argument(1));
        assert!(v0 % 32 == 1, 0x1::error::invalid_argument(1));
        UnvalidatedPublicKey{bytes: arg0}
    }
    
    public fun new_validated_public_key_from_bytes(arg0: vector<u8>) : 0x1::option::Option<ValidatedPublicKey> {
        if (0x1::vector::length<u8>(&arg0) % 32 == 1 && public_key_validate_internal(arg0)) {
            let v1 = ValidatedPublicKey{bytes: arg0};
            0x1::option::some<ValidatedPublicKey>(v1)
        } else {
            0x1::option::none<ValidatedPublicKey>()
        }
    }
    
    public fun new_validated_public_key_from_bytes_v2(arg0: vector<u8>) : 0x1::option::Option<ValidatedPublicKey> {
        if (!0x1::features::multi_ed25519_pk_validate_v2_enabled()) {
            abort 0x1::error::invalid_state(4)
        };
        if (public_key_validate_v2_internal(arg0)) {
            let v1 = ValidatedPublicKey{bytes: arg0};
            0x1::option::some<ValidatedPublicKey>(v1)
        } else {
            0x1::option::none<ValidatedPublicKey>()
        }
    }
    
    fun public_key_bytes_to_authentication_key(arg0: vector<u8>) : vector<u8> {
        0x1::vector::push_back<u8>(&mut arg0, 1);
        0x1::hash::sha3_256(arg0)
    }
    
    public fun public_key_into_unvalidated(arg0: ValidatedPublicKey) : UnvalidatedPublicKey {
        UnvalidatedPublicKey{bytes: arg0.bytes}
    }
    
    public fun public_key_to_unvalidated(arg0: &ValidatedPublicKey) : UnvalidatedPublicKey {
        UnvalidatedPublicKey{bytes: arg0.bytes}
    }
    
    public fun public_key_validate(arg0: &UnvalidatedPublicKey) : 0x1::option::Option<ValidatedPublicKey> {
        new_validated_public_key_from_bytes(arg0.bytes)
    }
    
    native fun public_key_validate_internal(arg0: vector<u8>) : bool;
    public fun public_key_validate_v2(arg0: &UnvalidatedPublicKey) : 0x1::option::Option<ValidatedPublicKey> {
        new_validated_public_key_from_bytes_v2(arg0.bytes)
    }
    
    native fun public_key_validate_v2_internal(arg0: vector<u8>) : bool;
    public fun signature_to_bytes(arg0: &Signature) : vector<u8> {
        arg0.bytes
    }
    
    public fun signature_verify_strict(arg0: &Signature, arg1: &UnvalidatedPublicKey, arg2: vector<u8>) : bool {
        signature_verify_strict_internal(arg0.bytes, arg1.bytes, arg2)
    }
    
    native fun signature_verify_strict_internal(arg0: vector<u8>, arg1: vector<u8>, arg2: vector<u8>) : bool;
    public fun signature_verify_strict_t<T0: drop>(arg0: &Signature, arg1: &UnvalidatedPublicKey, arg2: T0) : bool {
        let v0 = 0x1::ed25519::new_signed_message<T0>(arg2);
        let v1 = 0x1::bcs::to_bytes<0x1::ed25519::SignedMessage<T0>>(&v0);
        signature_verify_strict_internal(arg0.bytes, arg1.bytes, v1)
    }
    
    public fun unvalidated_public_key_num_sub_pks(arg0: &UnvalidatedPublicKey) : u8 {
        (0x1::vector::length<u8>(&arg0.bytes) / 32) as u8
    }
    
    public fun unvalidated_public_key_threshold(arg0: &UnvalidatedPublicKey) : 0x1::option::Option<u8> {
        check_and_get_threshold(arg0.bytes)
    }
    
    public fun unvalidated_public_key_to_authentication_key(arg0: &UnvalidatedPublicKey) : vector<u8> {
        public_key_bytes_to_authentication_key(arg0.bytes)
    }
    
    public fun unvalidated_public_key_to_bytes(arg0: &UnvalidatedPublicKey) : vector<u8> {
        arg0.bytes
    }
    
    public fun validated_public_key_num_sub_pks(arg0: &ValidatedPublicKey) : u8 {
        (0x1::vector::length<u8>(&arg0.bytes) / 32) as u8
    }
    
    public fun validated_public_key_threshold(arg0: &ValidatedPublicKey) : u8 {
        *0x1::vector::borrow<u8>(&arg0.bytes, 0x1::vector::length<u8>(&arg0.bytes) - 1)
    }
    
    public fun validated_public_key_to_authentication_key(arg0: &ValidatedPublicKey) : vector<u8> {
        public_key_bytes_to_authentication_key(arg0.bytes)
    }
    
    public fun validated_public_key_to_bytes(arg0: &ValidatedPublicKey) : vector<u8> {
        arg0.bytes
    }
    
    // decompiled from Move bytecode v6
}
