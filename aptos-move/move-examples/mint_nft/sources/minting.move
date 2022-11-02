/// This module is an example of how one can create a NFT collection from a resource account
/// and allow users to mint from the NFT collection.
/// Check aptos-move/move-e2e-tests/src/tests/mint_nft.rs for an e2e example.
///
/// - Initialization of this module
/// Let's say we have an original account at address `0xcafe`. We can use it to call the smart contract function
/// `create_resource_account_and_publish_package(origin, vector::empty<>(), ...)` - this will create a resource address at
/// `0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5`. The module `mint_nft` will be published under the
/// resource account's address.
///
/// - Accounts of this module
/// > source account: the account used to create the resource account.
/// > resource account: resource account is the account in charge of creating the collection and minting the tokens. A resource account is
/// meant to programmatically sign for transactions so nobody has the private key of the resource account. The signer capability of
/// the resource account is store in `ModuleData` to programmatically create a NFT collection and mint tokens.
/// > admin account: admin account is the account in charge of updating the config of this contract. The admin account can update the
/// expiration time, minting_enabled flag, and the public key of the admin account.
///
/// - When using this module, we expect the flow to look like:
/// 1. call create_resource_account_and_publish_package() (in a script or cli) to publish this module under the resource account's address.
/// init_module() will be called automatically as part of publishing the package. In init_module(), we set up the NFT collection to mint.
/// 2. call mint_nft() from a nft receiver's account: this will check if this token minting is still valid, verify the `MintProofChallenge` struct
/// against the admin's public key, and mint a token to the `receiver` upon successful verification. We will also emit an event
/// and mutate the token property (update the token version) upon successful token transfer.
/// 3. (optional) the admin account can update `expiration_timestamp`, `minting_enabled`, and `public_key` when needed.
///
/// - How to run this on cli
/// 1. Create three accounts: source-account (default), admin-account, and nft-receiver account
/// aptos init (create source account)
/// aptos init --profile admin-account (create admin-account)
/// aptos init --profile nft-receiver (create nft-receiver account)
/// 2. Fund all accounts
/// aptos account fund-with-faucet --account default (repeat multiple times for the source account, because publishing a module costs more gas)
/// aptos account fund-with-faucet --account admin-account
/// aptos account fund-with-faucet --account nft-receiver
/// 3. Run create_resource_account_and_publish_package to publish this contract under the resource account's address
/// (need to change the named address in Move.toml file to the actual values first. also, the seed here is just an example)
/// cargo run -p aptos -- move create-resource-account-and-publish-package --seed hex_array:4321 --address-name mint_nft --profile default
/// 4. Update the admin's public key from the admin's account
/// aptos move run --function-id [resource account's address]::minting::set_public_key --profile admin-account --args hex:[admin account's public key]
/// for example: aptos move run --function-id 9a0e3291258d2a3d7698fe850509d37bc8ae29d83b9f9796dea188fe9a7b5cd3::minting::set_public_key --profile admin-account --args hex:E563FA6BC769ACD4EA99F7206156F1EACE2129DB56AEEC00D8E5DE992ADC1495
/// 5. Update the timestamp of the colleciton from the admin's account
/// aptos move run --function-id [resource account's address]::minting::set_timestamp --args u64:1000000000000000 --profile admin-account
/// 6. Call `mint_nft` from the nft-receiver's account
/// 6.1 Generate a valid signature.
///     Go to aptos-core/aptos/move-e2e-tests/src/tests/mint_nft.rs
///     In function `sample_tutorial_signature`, change the `resource_address`, `nft_receiver`, `admin_private_key` to the actual values.
///     run `cargo test sample_tutorial_signature -- --nocapture` to generate a valid signature that we'll use in the next step.
/// 6.2 Run `mint_nft` from the nft-receiver's account
///     aptos move run --function-id [resource account's address]::minting::mint_nft --args hex:[valid signature] --profile nft-receiver
///     for example: aptos move run --function-id 9a0e3291258d2a3d7698fe850509d37bc8ae29d83b9f9796dea188fe9a7b5cd3::minting::mint_nft --args hex:cf32699cf84a5390e021b8e775ff7627ee8c7a0c049bd3d774ea7880632a5638a200852f167ccb12590436bedd5b0d86187871f4836e3a0c47c55e8f5709440c --profile nft-receiver
/// 7. (Optional) Go to devnet explorer and check out the resources of these accounts
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
    use aptos_framework::resource_account;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    use aptos_std::ed25519::ValidatedPublicKey;

    // This struct stores the token receiver's address and token_data_id in the event of token minting
    struct TokenMintingEvent has drop, store {
        token_receiver_address: address,
        token_data_id: TokenDataId,
    }

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        public_key: ed25519::ValidatedPublicKey,
        signer_cap: account::SignerCapability,
        token_data_id: TokenDataId,
        expiration_timestamp: u64,
        minting_enabled: bool,
        token_minting_events: EventHandle<TokenMintingEvent>,
    }

    // This struct stores the challenge message that proves that the resource signer wants to mint this token
    // to the receiver. This struct will need to be signed by the resource signer to pass the verification.
    struct MintProofChallenge has drop {
        receiver_account_sequence_number: u64,
        receiver_account_address: address,
        token_data_id: TokenDataId,
    }

    /// Action not authorized because the signer is not the admin of this module
    const ENOT_AUTHORIZED: u64 = 1;
    /// The collection minting is expired
    const ECOLLECTION_EXPIRED: u64 = 2;
    /// The collection minting is disabled
    const EMINTING_DISABLED: u64 = 3;
    /// Specified public key is not the same as the admin's public key
    const EWRONG_PUBLIC_KEY: u64 = 4;
    /// Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)
    const EINVALID_SCHEME: u64 = 5;
    /// Specified proof of knowledge required to prove ownership of a public key is invalid
    const EINVALID_PROOF_OF_KNOWLEDGE: u64 = 6;

    /// Initialize this module: create a resource account, a collection, and a token data id
    fun init_module(resource_account: &signer) {
        // NOTE: This is just an example PK; please replace this with your desired admin PK.
        let hardcoded_pk = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        init_module_with_admin_public_key(resource_account, hardcoded_pk);
    }

    fun init_module_with_admin_public_key(resource_account: &signer, pk_bytes: vector<u8>) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        let expiration_timestamp = 1000000;

        // change source_addr to the actual account that created the resource account
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_account, @source_addr);
        let resource_signer = account::create_signer_with_capability(&resource_signer_cap);

        // create the nft collection
        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        let resource_account_address = signer::address_of(&resource_signer);
        token::create_collection(&resource_signer, collection_name, description, collection_uri, maximum_supply, mutate_setting);

        // create a token data id to specify which token will be minted
        let token_data_id = token::create_tokendata(
            &resource_signer,
            collection_name,
            token_name,
            string::utf8(b""),
            0,
            token_uri,
            resource_account_address,
            1,
            0,
            // we don't allow any mutation to the token
            token::create_token_mutability_config(
                &vector<bool>[ false, false, false, false, true ]
            ),
            vector::empty<String>(),
            vector::empty<vector<u8>>(),
            vector::empty<String>(),
        );

        let public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(pk_bytes));

        move_to(resource_account, ModuleData {
            public_key,
            signer_cap: resource_signer_cap,
            token_data_id,
            expiration_timestamp,
            minting_enabled: true,
            token_minting_events: account::new_event_handle<TokenMintingEvent>(&resource_signer),
        });
    }

    /// Set if minting is enabled for this minting contract
    public entry fun set_minting_enabled(caller: &signer, minting_enabled: bool) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        module_data.minting_enabled = minting_enabled;
    }

    /// Set the expiration timestamp of this minting contract
    public entry fun set_timestamp(caller: &signer, expiration_timestamp: u64) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        module_data.expiration_timestamp = expiration_timestamp;
    }

    /// Set the public key of this minting contract
    public entry fun set_public_key(caller: &signer, pk_bytes: vector<u8>) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        module_data.public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(pk_bytes));
    }

    /// Mint an NFT to the receiver.
    /// `mint_proof_signature` should be the `MintProofChallenge` signed by the admin's private key
    /// `public_key_bytes` should be the public key of the admin
    public entry fun mint_nft(receiver: &signer, mint_proof_signature: vector<u8>) acquires ModuleData {
        let receiver_addr = signer::address_of(receiver);

        // get the collection minter and check if the collection minting is disabled or expired
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        assert!(timestamp::now_seconds() < module_data.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        // verify that the `mint_proof_signature` is valid against the admin's public key
        verify_proof_of_knowledge(receiver_addr, mint_proof_signature, module_data.token_data_id, module_data.public_key);

        // mint token to the receiver
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let token_id = token::mint_token(&resource_signer, module_data.token_data_id, 1);
        token::direct_transfer(&resource_signer, receiver, token_id, 1);

        event::emit_event<TokenMintingEvent>(
            &mut module_data.token_minting_events,
            TokenMintingEvent {
                token_receiver_address: receiver_addr,
                token_data_id: module_data.token_data_id,
            }
        );

        // mutate the token properties to update the property version of this token
        let (creator_address, collection, name) = token::get_token_data_id_fields(&module_data.token_data_id);
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

    /// Verify that the collection token minter intends to mint the given token_data_id to the receiver
    fun verify_proof_of_knowledge(receiver_addr: address, mint_proof_signature: vector<u8>, token_data_id: TokenDataId, public_key: ValidatedPublicKey) {
        let sequence_number = account::get_sequence_number(receiver_addr);

        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: sequence_number,
            receiver_account_address: receiver_addr,
            token_data_id,
        };

        let signature = ed25519::new_signature_from_bytes(mint_proof_signature);
        let unvalidated_public_key = ed25519::public_key_to_unvalidated(&public_key);
        assert!(ed25519::signature_verify_strict_t(&signature, &unvalidated_public_key, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
    }

    //
    // Tests
    //

    #[test_only]
    public fun set_up_test(
        origin_account: signer,
        resource_account: &signer,
        collection_token_minter_public_key: &ValidatedPublicKey,
        aptos_framework: signer,
        nft_receiver: &signer,
        timestamp: u64
    ) {
        // set up global time for testing purpose
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        create_account_for_test(signer::address_of(&origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(&origin_account, vector::empty<u8>(), vector::empty<u8>());

        let pk_bytes = ed25519::validated_public_key_to_bytes(collection_token_minter_public_key);
        init_module_with_admin_public_key(resource_account, pk_bytes);

        create_account_for_test(signer::address_of(nft_receiver));

        create_account_for_test(@admin_addr);
    }

    #[test (origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, nft_receiver = @0x123, nft_receiver2 = @0x234, aptos_framework = @aptos_framework)]
    public entry fun test_happy_path(origin_account: signer, resource_account: signer, nft_receiver: signer, nft_receiver2: signer, aptos_framework: signer) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // mint nft to this nft receiver
        mint_nft(&nft_receiver, ed25519::signature_to_bytes(&sig));

        // check that the nft_receiver has the token in their token store
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let resource_signer_addr = signer::address_of(&resource_signer);
        let token_id = token::create_token_id_raw(resource_signer_addr, string::utf8(b"Collection name"), string::utf8(b"Token name"), 1);
        let new_token = token::withdraw_token(&nft_receiver, token_id, 1);

        // put the token back since a token isn't droppable
        token::deposit_token(&nft_receiver, new_token);

        // mint the second NFT
        let receiver_addr_2 = signer::address_of(&nft_receiver2);
        create_account_for_test(receiver_addr_2);

        let proof_challenge_2 = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr_2),
            receiver_account_address: receiver_addr_2,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig2 = ed25519::sign_struct(&admin_sk, proof_challenge_2);
        mint_nft(&nft_receiver2, ed25519::signature_to_bytes(&sig2));

        //  check the property version is properly updated
        let token_id2 = token::create_token_id_raw(resource_signer_addr, string::utf8(b"Collection name"), string::utf8(b"Token name"), 2);
        let new_token2 = token::withdraw_token(&nft_receiver2, token_id2, 1);
        token::deposit_token(&nft_receiver2, new_token2);
    }

    #[test (origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50002)]
    public entry fun test_minting_expired(origin_account: signer, resource_account: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10000000);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };
        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        mint_nft(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin = @admin_addr, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50002)]
    public entry fun test_update_expiration_time(origin_account: signer, resource_account: signer, admin: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // set the expiration time of the minting to be earlier than the current time
        set_timestamp(&admin, 5);
        mint_nft(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, admin = @admin_addr, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50003)]
    public entry fun test_update_minting_enabled(origin_account: signer, resource_account: signer, admin: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // disable token minting
        set_minting_enabled(&admin, false);
        mint_nft(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0xcafe, resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10006)]
    public entry fun test_invalid_signature(origin_account: signer, resource_account: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        let sig_bytes = ed25519::signature_to_bytes(&sig);

        // Pollute signature.
        let first_sig_byte = vector::borrow_mut(&mut sig_bytes, 0);
        *first_sig_byte = *first_sig_byte + 1;

        mint_nft(&nft_receiver, sig_bytes);
    }
}
