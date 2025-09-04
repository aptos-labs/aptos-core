/// This module is the last part of our NFT Move tutorial. In this part, we are fleshing out the smart contracts so it
/// becomes production-ready. Specifically, we are going to go over:
/// - adding an `TokenMintingEvent` so we can emit a custom event to keep track of tokens minted from this module;
/// - introducing the concept of a proof challenge and how to use it to prevent bot spamming;
/// - adding unit tests to make sure that our code is working as expected.
///
/// Concept: signature validation
/// We use signature validation for many reasons. For example, in velor_framework::account::rotate_authentication_key(),
/// we asked for two signatures from the user proving that they intend and have the capability to rotate the account's
/// authentication key.
///
/// In this tutorial, we are using it for a different reason: we would like to make sure that only mint_event_ticket() requests
/// coming from our certified backend get processed.
/// Because we're validating the signature against the specified public key, only requests coming from the backend
/// with the corresponding private key can mint NFTs. All other requests will fail due to `EINVALID_PROOF_OF_KNOWLEDGE`.
/// This makes sure that a person cannot abuse and mint a lot of NFTs from this smart contract by spamming it, since they
/// wouldn't have passed the signature verification step.
///
/// Concept: event
/// Events are emitted during the execution of a transaction. Each Move module can define its own events and choose when
/// to emit the events upon execution of the module. In this module, we are adding a custom `TokenMintingEvent` to keep track
/// of the minted token_data_id and the token receiver's address.
/// For more information about events, see: https://velor.dev/concepts/events/.
///
/// Move unit tests
/// We added a few unit tests to make sure that our code is working as expected. For more information on how to write
/// Move unit tests, see: https://velor.dev/move/book/unit-testing
///
/// - How to interact with this module
/// 1. Configure the admin account name address in Move.toml file.
/// Go to `Move.toml` and replace `admin_addr = "0xbeef"` with the admin address we created in part 3.
///
/// 2. Publish the module under a resource account.
/// - 2.a Make sure you're in the right directory.
/// Run the following command in directory `velor-core/velor-move/move-examples/mint_nft/4-Getting-Production-Ready`.
/// - 2.b Run the following CLI command to publish the module under a resource account.
/// velor move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft --profile default --named-addresses source_addr=[default account's address]
///
/// 3. Call `mint_event_ticket()` with a valid signature to mint a token.
/// - 3.a Generate a keypair to validate the signature against.
/// velor key generate --key-type ed25519 --output-file output.key
/// - 3.b Update the public key stored within `ModuleData`.
/// velor move run --function-id [resource account's address]::create_nft_getting_production_ready::set_public_key --args hex:[public key we just generated] --profile admin
/// (To find the public key we just generated, run `nano output.key.pub`.)
/// example output:
/*
4-Getting-Production-Ready % velor move run --function-id f634035fea40e23c5ed8817f7e996d96372dd5dbd64e853fb3c108817d92dcb4::create_nft_getting_production_ready::set_public_key --args hex:5C0637A3865FCA80550502BC30C8E7B4CCA7C8AB3B4FFECEFC8C43F7D0D44DEE --profile admin
Do you want to submit a transaction for a range of [35200 - 52800] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
{
  "Result": {
    "transaction_hash": "0xd4ee713cca364dabd544d5f66252ceaa0b3ef3f0449209900495bffecd27926a",
    "gas_used": 352,
    "gas_unit_price": 100,
    "sender": "f42bcdc1fb9b8d4c0ac9e54568a53c8515d3d9afd7936484a923b0d7854e134f",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1669672770777276,
    "version": 13023905,
    "vm_status": "Executed successfully"
  }
}
*/
/// - 3.c Generate a valid signature.
///  Open up file `velor-core/velor-move/e2e-move-tests/src/tests/mint_nft.rs`.
///  In function `generate_nft_tutorial_part4_signature`, change the `resource_address`, `nft_receiver`, `admin_private_key`, and `receiver_account_sequence_number` variables to the actual values.
///  You can find the `admin_private_key` by running `nano output.key`, and the `receiver_account_sequence_number` by looking up the receiver's address on the Velor Explorer under tab `Info`.
///  Make sure you're in the right directory.
///  Run the following command in directory `velor-core/velor-move/e2e-move-tests`
///  Run `cargo test generate_nft_tutorial_part4_signature -- --nocapture` to generate a valid signature that we'll use in the next step.
/// - 3.d Call mint_event_ticket() with the signature we generated in the last step.
/// velor move run --function-id [resource account's address]::create_nft_getting_production_ready::mint_event_ticket --args hex:[signature generated in last step] --profile nft-receiver
/// example output:
/*
4-Getting-Production-Ready % velor move run --function-id f634035fea40e23c5ed8817f7e996d96372dd5dbd64e853fb3c108817d92dcb4::create_nft_getting_production_ready::mint_event_ticket --args hex:fc833512ad1c575850569d14f5e434b929a19eb491c08df9f6b91584a13551bdb95830081a429f148fddcb9ba201cf72a357957849046da0d60675ed034f580f --profile nft-receiver
Do you want to submit a transaction for a range of [600800 - 901200] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
{
  "Result": {
    "transaction_hash": "0x2cbc5f7444c381476bb69bfb40bb8f396875c0121d97b75e7b9b156ebef15f84",
    "gas_used": 6073,
    "gas_unit_price": 100,
    "sender": "7d69283af198b1265d17a305ff0cca6da1bcee64d499ce5b35b659098b3a82dc",
    "sequence_number": 4,
    "success": true,
    "timestamp_us": 1669673918958986,
    "version": 13046985,
    "vm_status": "Executed successfully"
  }
}
*/
///
/// This is the end of this NFT tutorial! Congrats on making it to the end. Please let us know if you have any questions / feedback by opening a github issue / feature request : )
module mint_nft::create_nft_getting_production_ready {
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use velor_framework::account;
    use velor_framework::event;
    use velor_framework::timestamp;
    use velor_std::ed25519;
    use velor_token::token::{Self, TokenDataId};
    use velor_framework::resource_account;
    #[test_only]
    use velor_framework::account::create_account_for_test;
    use velor_std::ed25519::ValidatedPublicKey;

    #[event]
    // This struct stores the token receiver's address and token_data_id in the event of token minting
    struct TokenMinting has drop, store {
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

    fun init_module(resource_signer: &signer) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        // This means that the supply of the token will not be tracked.
        let maximum_supply = 0;
        // This variable sets if we want to allow mutation for collection description, uri, and maximum.
        // Here, we are setting all of them to false, which means that we don't allow mutations to any CollectionData fields.
        let mutate_setting = vector<bool>[ false, false, false ];

        // Create the nft collection.
        token::create_collection(
            resource_signer,
            collection_name,
            description,
            collection_uri,
            maximum_supply,
            mutate_setting
        );

        // Create a token data id to specify the token to be minted.
        let token_data_id = token::create_tokendata(
            resource_signer,
            collection_name,
            token_name,
            string::utf8(b""),
            0,
            token_uri,
            signer::address_of(resource_signer),
            1,
            0,
            // This variable sets if we want to allow mutation for token maximum, uri, royalty, description, and properties.
            // Here we enable mutation for properties by setting the last boolean in the vector to true.
            token::create_token_mutability_config(
                &vector<bool>[ false, false, false, false, true ]
            ),
            // We can use property maps to record attributes related to the token.
            // In this example, we are using it to record the receiver's address.
            // We will mutate this field to record the user's address
            // when a user successfully mints a token in the `mint_nft()` function.
            vector<String>[string::utf8(b"given_to")],
            vector<vector<u8>>[b""],
            vector<String>[ string::utf8(b"address") ],
        );


        // store the token data id within the module, so we can refer to it later
        // when we're minting the NFT
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @source_addr);

        // hardcoded public key - we will update it to the real one by calling `set_public_key` from the admin account
        let pk_bytes = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(pk_bytes));
        move_to(resource_signer, ModuleData {
            public_key,
            signer_cap: resource_signer_cap,
            token_data_id,
            expiration_timestamp: 10000000000,
            minting_enabled: true,
        });
    }

    /// Mint an NFT to the receiver.
    /// `mint_proof_signature` should be the `MintProofChallenge` signed by the admin's private key
    /// `public_key_bytes` should be the public key of the admin
    public entry fun mint_event_ticket(receiver: &signer, mint_proof_signature: vector<u8>) acquires ModuleData {
        let receiver_addr = signer::address_of(receiver);

        // get the collection minter and check if the collection minting is disabled or expired
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        assert!(
            timestamp::now_seconds() < module_data.expiration_timestamp,
            error::permission_denied(ECOLLECTION_EXPIRED)
        );
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        // verify that the `mint_proof_signature` is valid against the admin's public key
        verify_proof_of_knowledge(
            receiver_addr,
            mint_proof_signature,
            module_data.token_data_id,
            module_data.public_key
        );

        // mint token to the receiver
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let token_id = token::mint_token(&resource_signer, module_data.token_data_id, 1);
        token::direct_transfer(&resource_signer, receiver, token_id, 1);

        event::emit(
            TokenMinting {
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

    /// Verify that the collection token minter intends to mint the given token_data_id to the receiver
    fun verify_proof_of_knowledge(
        receiver_addr: address,
        mint_proof_signature: vector<u8>,
        token_data_id: TokenDataId,
        public_key: ValidatedPublicKey
    ) {
        let sequence_number = account::get_sequence_number(receiver_addr);

        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: sequence_number,
            receiver_account_address: receiver_addr,
            token_data_id,
        };

        let signature = ed25519::new_signature_from_bytes(mint_proof_signature);
        let unvalidated_public_key = ed25519::public_key_to_unvalidated(&public_key);
        assert!(
            ed25519::signature_verify_strict_t(&signature, &unvalidated_public_key, proof_challenge),
            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)
        );
    }

    //
    // Tests
    //

    #[test_only]
    public fun set_up_test(
        origin_account: signer,
        resource_account: &signer,
        collection_token_minter_public_key: &ValidatedPublicKey,
        velor_framework: signer,
        nft_receiver: &signer,
        timestamp: u64
    ) acquires ModuleData {
        // set up global time for testing purpose
        timestamp::set_time_has_started_for_testing(&velor_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        create_account_for_test(signer::address_of(&origin_account));

        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(&origin_account, vector::empty<u8>(), vector::empty<u8>());

        init_module(resource_account);

        let admin = create_account_for_test(@admin_addr);
        let pk_bytes = ed25519::validated_public_key_to_bytes(collection_token_minter_public_key);
        set_public_key(&admin, pk_bytes);

        create_account_for_test(signer::address_of(nft_receiver));
    }

    #[test (
        origin_account = @0xcafe,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        nft_receiver = @0x123,
        nft_receiver2 = @0x234,
        velor_framework = @velor_framework
    )]
    public entry fun test_happy_path(
        origin_account: signer,
        resource_account: signer,
        nft_receiver: signer,
        nft_receiver2: signer,
        velor_framework: signer
    ) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, velor_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // mint nft to this nft receiver
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));

        // check that the nft_receiver has the token in their token store
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let resource_signer_addr = signer::address_of(&resource_signer);
        let token_id = token::create_token_id_raw(
            resource_signer_addr,
            string::utf8(b"Collection name"),
            string::utf8(b"Token name"),
            1
        );
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
        mint_event_ticket(&nft_receiver2, ed25519::signature_to_bytes(&sig2));

        //  check the property version is properly updated
        let token_id2 = token::create_token_id_raw(
            resource_signer_addr,
            string::utf8(b"Collection name"),
            string::utf8(b"Token name"),
            2
        );
        let new_token2 = token::withdraw_token(&nft_receiver2, token_id2, 1);
        token::deposit_token(&nft_receiver2, new_token2);
    }

    #[test (
        origin_account = @0xcafe,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        nft_receiver = @0x123,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50002, location = mint_nft::create_nft_getting_production_ready)]
    public entry fun test_minting_expired(
        origin_account: signer,
        resource_account: signer,
        nft_receiver: signer,
        velor_framework: signer
    ) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, velor_framework, &nft_receiver, 100000000001);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };
        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (
        origin_account = @0xcafe,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        admin = @admin_addr,
        nft_receiver = @0x123,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50002, location = mint_nft::create_nft_getting_production_ready)]
    public entry fun test_update_expiration_time(
        origin_account: signer,
        resource_account: signer,
        admin: signer,
        nft_receiver: signer,
        velor_framework: signer
    ) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, velor_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // set the expiration time of the minting to be earlier than the current time
        set_timestamp(&admin, 5);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (
        origin_account = @0xcafe,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        admin = @admin_addr,
        nft_receiver = @0x123,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x50003, location = mint_nft::create_nft_getting_production_ready)]
    public entry fun test_update_minting_enabled(
        origin_account: signer,
        resource_account: signer,
        admin: signer,
        nft_receiver: signer,
        velor_framework: signer
    ) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, velor_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            token_data_id: borrow_global<ModuleData>(@mint_nft).token_data_id,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // disable token minting
        set_minting_enabled(&admin, false);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (
        origin_account = @0xcafe,
        resource_account = @0xc3bb8488ab1a5815a9d543d7e41b0e0df46a7396f89b22821f07a4362f75ddc5,
        nft_receiver = @0x123,
        velor_framework = @velor_framework
    )]
    #[expected_failure(abort_code = 0x10006, location = mint_nft::create_nft_getting_production_ready)]
    public entry fun test_invalid_signature(
        origin_account: signer,
        resource_account: signer,
        nft_receiver: signer,
        velor_framework: signer
    ) acquires ModuleData {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, velor_framework, &nft_receiver, 10);
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

        mint_event_ticket(&nft_receiver, sig_bytes);
    }
}
