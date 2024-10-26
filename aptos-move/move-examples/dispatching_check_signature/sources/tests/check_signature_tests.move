#[test_only]
module dispatching_check_signature::check_signature_tests {
    use std::vector;
    use std::hash;
    use std::signer;
    
    use dispatching_check_signature::storage;
    use dispatching_check_signature::check_signature;
    use dispatching_check_signature::check_signature_example;

    #[test(resource_account = @0x101)]
    public fun test_verify_success(resource_account: &signer) {
        storage::init_module_for_testing(resource_account);
        check_signature_example::init_module_for_testing(resource_account);

        let signer_address = signer::address_of(resource_account);
        
        let digest_hash = vector::empty<u8>();
        vector::append(&mut digest_hash, hash::sha3_256(b"test_hash"));
        
        let signature = vector::empty<u8>();
        vector::append(&mut signature, hash::sha3_256(b"test_signature"));
        
        let is_valid = check_signature::check_signature(signer_address, digest_hash, signature);
        assert!(is_valid, 1001);
    }

    #[test(resource_account = @0x101)]
    public fun test_verify_failure(resource_account: &signer) {
        storage::init_module_for_testing(resource_account);
        check_signature_example::init_module_for_testing(resource_account);

        let signer_address = signer::address_of(resource_account);
        
        let digest_hash = vector::empty<u8>();
        vector::append(&mut digest_hash, hash::sha3_256(b"test_hash_failure"));
        
        let signature = vector::empty<u8>();
        vector::append(&mut signature, hash::sha3_256(b"test_signature_failure"));
        
        let is_valid = check_signature::check_signature(signer_address, digest_hash, signature);
        assert!(!is_valid, 1002);
    }
}