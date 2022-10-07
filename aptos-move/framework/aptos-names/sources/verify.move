module aptos_names::verify {
    use std::string;
    use std::vector;
    use aptos_framework::chain_id;
    use aptos_std::ed25519;
    use aptos_names::config;

    friend aptos_names::domains;

    struct RegisterDomainProofChallenge has drop {
        sequence_number: u64,
        register_address: address,
        domain_name: string::String,
        chain_id: u8,
    }

    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 1;

    public(friend) fun verify_register_domain_signature(signature: vector<u8>, sequence_number: u64, account_address: address, domain_name: string::String) {
        let chain_id = chain_id::get();
        let register_domain_proof_challenge = RegisterDomainProofChallenge {
            sequence_number,
            register_address: account_address,
            domain_name,
            chain_id
        };

        let admin_public_key_bytes = config::admin_public_key();
        // the number at index 0 is the size of the vector, so we need to remove it to get a valid 32-byte public key
        vector::remove(&mut admin_public_key_bytes, 0);
        let admin_public_key = ed25519::new_unvalidated_public_key_from_bytes(admin_public_key_bytes);
        let sig = ed25519::new_signature_from_bytes(signature);
        assert!(ed25519::signature_verify_strict_t(&sig, &admin_public_key, register_domain_proof_challenge), std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
    }
}
