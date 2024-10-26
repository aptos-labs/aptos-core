module dispatching_check_signature::check_signature_example {
    use std::option;
    use std::hash;
    use std::vector;
    use aptos_framework::object::{Self, ExtendRef, Object};
  
    use dispatching_check_signature::check_signature;
    use dispatching_check_signature::storage;

    const VERIFY_SUCCESS: u128 = 0;
    const VERIFY_FAILURE: u128 = 1;
    
    const E_INVALID_DATA_LENGTH: u64 = 0;
    
    struct SignatureExampleConfig has key {
        obj_ref: ExtendRef,
    }

    fun init_module(publisher: &signer) {
        let constructor_ref = object::create_object(@dispatching_check_signature);

        move_to(publisher, SignatureExampleConfig {
            obj_ref: object::generate_extend_ref(&constructor_ref),
        });
        register(publisher);
    }

    fun register(signer: &signer) {
        check_signature::register_dispatchable(signer);
    }

    public fun verify<T: key>(_metadata: Object<T>): option::Option<u128> {
        let data = storage::retrieve(@dispatching_check_signature);
        let (signature, digest_hash) = extract_data(data);
        if (signature == hash::sha3_256(b"test_signature") && digest_hash == hash::sha3_256(b"test_hash")) {
            option::some(VERIFY_SUCCESS)
        } else {
            option::some(VERIFY_FAILURE)
        }
    }

    #[view]
    public fun extract_data(data: vector<u8>): (vector<u8>, vector<u8>) {
        assert!(vector::length(&data) == 64, E_INVALID_DATA_LENGTH);
        let digest_hash = vector::slice(&data, 0, 32);
        let signature = vector::slice(&data, 32, vector::length(&data));
        (signature, digest_hash)
    }

    #[view]
    fun object_signer(): signer acquires SignatureExampleConfig {
        let config = borrow_global<SignatureExampleConfig>(@dispatching_check_signature);
        object::generate_signer_for_extending(&config.obj_ref)
    }

    #[test_only]
    public fun init_module_for_testing(publisher: &signer) {
        init_module(publisher);
    }
}