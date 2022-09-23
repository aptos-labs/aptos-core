/// This module is an example of how one can create a NFT collection from a resource account
/// and allow users to mint from the NFT collection.
/// Check aptos/move-e2e-tests/src/tests /mint.nft.rs for an e2e example.
///
/// The basic flow: the module publisher will call `init_module` to initialize the module, and then a nft receiver
/// can call `mint_nft` to mint a token. If needed, we can update the expiration time of minting using `set_timestamp`
/// and update if we want to enable token minting using `set_minting_enabled`.
///
/// More details:
/// init_module()
/// The module publisher will call `init_module` to create a resource account and store the resource account's
/// address in the publisher's `ModuleData`.
/// A NFT collection and the first token_data_id to be minted will be created using this resource account.
/// We then store all the relevant information about this token in `CollectionTokenMinter`.
///
/// mint_nft()
/// `mint_nft` will be called when a user wants to mint a token from this token data id. The user needs to provide
/// a `mint_proof_signature` (their signature on a MintProofChallenge) to prove that they intend to claim the token.
/// After function `verify_proof_of_knowledge` verifies that the signature is valid, the signer of the resource account
/// will transfer the given token_id to the user.
/// After a token is minted, we call `mutate_token_properties` to update the property version of this token.

module mint_nft::minting {
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::ed25519;
    use aptos_token::token::{Self, TokenDataId};

    // This struct stores the token receiver's address and token_data_id in the event of token minting
    struct TokenMintingEvent has drop, store {
        token_receiver_address: address,
        token_data_id: TokenDataId,
    }

    // This struct stores an NFT collection's relevant information
    struct CollectionTokenMinter has key {
        public_key: ed25519::ValidatedPublicKey,
        signer_cap: account::SignerCapability,
        token_data_id: TokenDataId,
        expiration_timestamp: u64,
        minting_enabled: bool,
        token_minting_events: EventHandle<TokenMintingEvent>,
    }

    // This struct stores the challenge message that proves that the user owns the public key and intends to claims this specific NFT
    struct MintProofChallenge has drop {
        sequence_number: u64,
        token_data_id: TokenDataId,
    }

    struct ModuleData has key {
        resource_account_address: address,
    }

    /// Action not authorized because the signer is not the owner of this module
    const ENOT_AUTHORIZED: u64 = 1;
    /// The collection minting is expired
    const ECOLLECTION_EXPIRED: u64 = 2;
    /// The collection minting is disabled
    const EMINTING_DISABLED: u64 = 3;
    /// Specified public key is not correct
    const EWRONG_PUBLIC_KEY: u64 = 4;
    /// Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)
    const EINVALID_SCHEME: u64 = 5;
    /// Specified proof of knowledge required to prove ownership of a public key is invalid
    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 6;

    /// Initialize this module: create a resource account, a collection, and a token data id
    fun init_module(origin: &signer) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri= string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        let expiration_timestamp = 1000000;
        let public_key_bytes = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";

        let (resource, signer_cap) = account::create_resource_account(origin, vector::empty());

        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        let resource_account_address = signer::address_of(&resource);

        token::create_collection(&resource, collection_name, description, collection_uri, maximum_supply, mutate_setting);
        // create a token data id to specify which token will be minted
        let token_data_id = token::create_tokendata(
            &resource,
            collection_name,
            token_name,
            string::utf8(b""),
            0,
            token_uri,
            resource_account_address,
            0,
            0,
            // we don't allow any mutation to the token
            token::create_token_mutability_config(
                &vector<bool>[ false, false, false, false, true ]
            ),
            vector::empty<String>(),
            vector::empty<vector<u8>>(),
            vector::empty<String>(),
        );

        let public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(public_key_bytes));

        move_to(origin, CollectionTokenMinter {
            public_key,
            signer_cap,
            token_data_id,
            expiration_timestamp,
            minting_enabled: true,
            token_minting_events: account::new_event_handle<TokenMintingEvent>(&resource),
        });

        move_to(origin, ModuleData {
            resource_account_address,
        });
    }

    /// Set if minting is enabled for this collection token minter
    public entry fun set_minting_enabled(minter: &signer, minting_enabled: bool) acquires CollectionTokenMinter {
        let minter_address = signer::address_of(minter);
        assert!(minter_address == @mint_nft, error::permission_denied(ENOT_AUTHORIZED));
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(minter_address);
        collection_token_minter.minting_enabled = minting_enabled;
    }

    /// Set the expiration timestamp of this collection token minter
    public entry fun set_timestamp(minter: &signer, expiration_timestamp: u64) acquires CollectionTokenMinter {
        let minter_address = signer::address_of(minter);
        assert!(minter_address == @mint_nft, error::permission_denied(ENOT_AUTHORIZED));
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(minter_address);
        collection_token_minter.expiration_timestamp = expiration_timestamp;
    }

    /// Mint an NFT to the receiver.
    /// `mint_proof_signature` is the `MintProofChallenge` signed by the receiver's private key
    /// `public_key_bytes` is the public key of the receiver
    /// `account_scheme` is the account scheme of the receiver (should be 0/ed25519 or 1/multi_ed25519)
    public entry fun mint_nft(receiver: &signer, mint_proof_signature: vector<u8>, public_key_bytes: vector<u8>) acquires CollectionTokenMinter {
        let receiver_addr = signer::address_of(receiver);

        // get the collection minter and check if the collection minting is disabled or expired
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(@mint_nft);

        assert!(timestamp::now_seconds() < collection_token_minter.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(collection_token_minter.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        // verify that the `mint_proof_signature` is valid
        verify_proof_of_knowledge(receiver_addr, mint_proof_signature, collection_token_minter.token_data_id, public_key_bytes);

        // mint token to the receiver
        let resource_signer = account::create_signer_with_capability(&collection_token_minter.signer_cap);
        let token_id = token::mint_token(&resource_signer, collection_token_minter.token_data_id, 1);
        token::direct_transfer(&resource_signer, receiver, token_id, 1);

        event::emit_event<TokenMintingEvent> (
            &mut collection_token_minter.token_minting_events,
            TokenMintingEvent {
                token_receiver_address: receiver_addr,
                token_data_id: collection_token_minter.token_data_id,
            }
        );

        // record that the token is given to the user
        let (creator_address, collection, name) = token::get_token_data_id_fields(&collection_token_minter.token_data_id);
        token::mutate_token_properties(
            &resource_signer,
            receiver_addr,
            creator_address,
            collection,
            name,
            0,
            1,
            vector::empty<String>(),
            vector::empty<vector<u8>>(),
            vector::empty<String>(),
        );
    }

    /// Verify that the user owns the public key and intends to claim this NFT
    fun verify_proof_of_knowledge(receiver_addr: address, mint_proof_signature: vector<u8>, token_data_id: TokenDataId, public_key_bytes: vector<u8>) {
        let sequence_number = account::get_sequence_number(receiver_addr);
        let auth_key = account::get_authentication_key(receiver_addr);

        let proof_challenge = MintProofChallenge {
            sequence_number,
            token_data_id
        };

        let pk = ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
        let expected_auth_key = ed25519::unvalidated_public_key_to_authentication_key(&pk);
        assert!(auth_key == expected_auth_key, error::invalid_argument(EWRONG_PUBLIC_KEY));

        let signature = ed25519::new_signature_from_bytes(mint_proof_signature);
        assert!(ed25519::signature_verify_strict_t(&signature, &pk, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
    }

    #[test_only]
    const VALID_SIGNATURE: vector<u8> = x"4714d8aa98706998940a5fd568542dcd9e21f488704f95bcd963904a66bbe7d22a299b0796af1f1c4ce09c1da32ba28706fa6492380dae4ac5d07e3c5857220b";
    const VALID_SIGNATURE2: vector<u8> = x"bb66fef1d5537752cde3a326ea0d567612f3612e4cc3c7e3c58dc1a92e19ade6396adafcb799e25f61e7872d3eff5207afaa55f1069898c49e765547d3eba10e";
    const INVALID_SIGNATURE: vector<u8> = x"3714d8aa98706998940a5fd568542dcd9e21f488704f95bcd963904a66bbe7d22a299b0796af1f1c4ce09c1da32ba28706fa6492380dae4ac5d07e3c5857220b";
    const PUBLIC_KEY: vector<u8> = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
    const PUBLIC_KEY2: vector<u8> = x"b1197cecf9aba50ee1590fbc5f8e33e83116c6958068c6db906bb21e8f3b2f61";

    #[test_only]
    public fun set_up_test(collection_token_minter: &signer, aptos_framework: signer, timestamp: u64) {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        account::create_account_for_test(signer::address_of(collection_token_minter));

        init_module(collection_token_minter);
    }

    #[test (collection_token_minter = @0xcafe, aptos_framework = @aptos_framework)]
    public entry fun test_happy_path(collection_token_minter: signer, aptos_framework: signer) acquires CollectionTokenMinter {
        set_up_test(&collection_token_minter, aptos_framework, 10);
        let nft_receiver = account::create_account_from_ed25519_public_key(PUBLIC_KEY);
        mint_nft(&nft_receiver, VALID_SIGNATURE, PUBLIC_KEY);

        // check that the nft_receiver has the token in their token store
        let collection_token_minter = borrow_global_mut<CollectionTokenMinter>(@mint_nft);
        let resource_signer = account::create_signer_with_capability(&collection_token_minter.signer_cap);
        let token_id = token::create_token_id_raw(signer::address_of(&resource_signer), string::utf8(b"Collection name"), string::utf8(b"Token name"), 1);
        let new_token = token::withdraw_token(&nft_receiver, token_id, 1);
        token::deposit_token(&nft_receiver, new_token);

        // mint the second NFT and check the property version is updated
        let nft_receiver2 = account::create_account_from_ed25519_public_key(PUBLIC_KEY2);
        mint_nft(&nft_receiver2, VALID_SIGNATURE2, PUBLIC_KEY2);
        let token_id2 = token::create_token_id_raw(signer::address_of(&resource_signer), string::utf8(b"Collection name"), string::utf8(b"Token name"), 2);
        let new_token2 = token::withdraw_token(&nft_receiver2, token_id2, 1);
        token::deposit_token(&nft_receiver2, new_token2);
    }

    #[test (collection_token_minter = @0xcafe, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 327682)]
    public entry fun test_minting_expired(collection_token_minter: signer, aptos_framework: signer) acquires CollectionTokenMinter {
        set_up_test(&collection_token_minter, aptos_framework, 10000000);
        let nft_receiver = account::create_account_from_ed25519_public_key(PUBLIC_KEY);
        mint_nft(&nft_receiver, VALID_SIGNATURE, PUBLIC_KEY);
    }

    #[test (collection_token_minter = @0xcafe, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 327682)]
    public entry fun test_update_expiration_time(collection_token_minter: signer, aptos_framework: signer) acquires CollectionTokenMinter {
        set_up_test(&collection_token_minter, aptos_framework, 10);
        set_timestamp(&collection_token_minter, 5);
        let nft_receiver = account::create_account_from_ed25519_public_key(PUBLIC_KEY);
        mint_nft(&nft_receiver, VALID_SIGNATURE, PUBLIC_KEY);
    }

    #[test (collection_token_minter = @0xcafe, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 327683)]
    public entry fun test_update_minting_enabled(collection_token_minter: signer, aptos_framework: signer) acquires CollectionTokenMinter {
        set_up_test(&collection_token_minter, aptos_framework, 10);
        set_minting_enabled(&collection_token_minter, false);
        let nft_receiver = account::create_account_from_ed25519_public_key(PUBLIC_KEY);
        mint_nft(&nft_receiver, VALID_SIGNATURE, PUBLIC_KEY);
    }

    #[test (collection_token_minter = @0xcafe, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 65542)]
    public entry fun test_invalid_signature(collection_token_minter: signer, aptos_framework: signer) acquires CollectionTokenMinter {
        set_up_test(&collection_token_minter, aptos_framework, 10);
        let nft_receiver = account::create_account_from_ed25519_public_key(PUBLIC_KEY);
        mint_nft(&nft_receiver, INVALID_SIGNATURE, PUBLIC_KEY);
    }
}
