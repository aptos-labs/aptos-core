module nft::mint {
    use aptos_framework::account;
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;
    use aptos_std::ed25519;
    use aptos_std::multi_ed25519;
    use aptos_token::token::{Self, TokenDataId};
    
    use std::bcs;
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    struct CollectionTokenMinter has key {
        public_key: ed25519::ValidatedPublicKey,
        signer_cap: account::SignerCapability,
        expiration_timestamp: u64,
        minting_enabled: bool,
    }

    // prove that the user owns the public key and intends to acquire the NFT
    struct MintProofChallenge has drop {
        sequence_number: u64,
        token_data_id: TokenDataId,
    }

    const ED25519_SCHEME: u8 = 0;
    const MULTI_ED25519_SCHEME: u8 = 1;

    const ENOT_AUTHORIZED: u64 = 1;
    const EACCOUNT_DOES_NOT_EXIST: u64 = 2;
    const ECOLLECTION_EXPIRED: u64 = 3;
    const EMINTING_DISABLED: u64 = 4;
    const EWRONG_PUBLIC_KEY: u64 = 5;
    const EINVALID_SCHEME: u64 = 6;
    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 7;

    fun init_module(origin: &signer, collection_name: String, description: String, collection_uri: String, expiration_timestamp: u64, public_key_bytes: vector<u8>) {
        let (resource, signer_cap) = resource_account::create_resource_account(origin, vector::empty(), vector::empty());

        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        token::create_collection(&resource, collection_name, description, collection_uri, maximum_supply, mutate_setting);

        let public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(public_key_bytes));
        move_to(origin, CollectionTokenMinter {
            public_key,
            signer_cap,
            expiration_timestamp,
            minting_enabled: true
        });
    }

    public entry fun set_minting_enabled(minter: &signer, minting_enabled: bool) acquires CollectionTokenMinter {
        let minter_address = signer::address_of(minter);
        assert!(minter_address == @nft, error::permission_denied(ENOT_AUTHORIZED));
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(minter_address);
        collection_token_minter.minting_enabled = minting_enabled;
    }

    public entry fun set_timestamp(minter: &signer, expiration_timestamp: u64) acquires CollectionTokenMinter {
        let minter_address = signer::address_of(minter);
        assert!(minter_address == @nft, error::permission_denied(ENOT_AUTHORIZED));
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(minter_address);
        collection_token_minter.expiration_timestamp = expiration_timestamp;
    }

    public entry fun mint_NFT(receiver: &signer, mint_proof_signature: vector<u8>, token_data_id: TokenDataId, public_key_bytes: vector<u8>, account_scheme: u8) acquires CollectionTokenMinter {
        let receiver_addr = signer::address_of(receiver);
        assert!(account::exists_at(receiver_addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));

        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(@nft);
        assert!(timestamp::now_seconds() < collection_token_minter.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(collection_token_minter.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        verify_proof_of_knowledge(receiver_addr, mint_proof_signature, token_data_id, public_key_bytes, account_scheme);

        let resource_signer = account::create_signer_with_capability(&collection_token_minter.signer_cap);
        token::mint_token_to(&resource_signer, receiver_addr, token_data_id, 1);

        let (creator_address, collection, name) = token::get_token_data_id_fields(&token_data_id);
        token::mutate_token_properties(
            &resource_signer,
            receiver_addr,
            creator_address,
            collection,
            name,
            0,
            1,
            vector<String>[string::utf8(b"given_to")],
            vector<vector<u8>>[bcs::to_bytes(&receiver_addr)],
            vector<String>[string::utf8(b"address")],
        );
    }

    fun verify_proof_of_knowledge(receiver_addr: address, mint_proof_signature: vector<u8>, token_data_id: TokenDataId, public_key_bytes: vector<u8>, account_scheme: u8) {
        let sequence_number = account::get_sequence_number(receiver_addr);
        let auth_key = account::get_authentication_key(receiver_addr);

        let proof_challenge = MintProofChallenge {
            sequence_number,
            token_data_id
        };

        if (account_scheme == ED25519_SCHEME) {
            let pk = ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
            let expected_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&pk);
            assert!(auth_key == expected_auth_key, error::invalid_argument(EWRONG_PUBLIC_KEY));

            let signature = ed25519::new_signature_from_bytes(mint_proof_signature);
            assert!(ed25519::signature_verify_strict_t(&signature, &pk, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
        } else if (account_scheme == MULTI_ED25519_SCHEME) {
            let pk = multi_ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
            let expected_auth_key = multi_ed25519::unvalidated_public_key_to_authentication_key(&pk);
            assert!(auth_key == expected_auth_key, error::invalid_argument(EWRONG_PUBLIC_KEY));

            let signature = multi_ed25519::new_signature_from_bytes(mint_proof_signature);
            assert!(multi_ed25519::signature_verify_strict_t(&signature, &pk, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
        } else {
            abort EINVALID_SCHEME
        };
    }
}
