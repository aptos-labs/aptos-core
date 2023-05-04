/// This module is the last part of our NFT Move tutorial. In this part, we are fleshing out the smart contracts so it
/// becomes production-ready. Specifically, we are going to go over:
/// - adding an `TokenMintingEvent` so we can emit a custom event to keep track of tokens minted from this module;
/// - introducing the concept of a proof challenge and how to use it to prevent bot spamming;
/// - adding unit tests to make sure that our code is working as expected.
///
/// Concept: signature validation
/// We use signature validation for many reasons. For example, in aptos_framework::account::rotate_authentication_key(),
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
/// For more information about events, see: https://aptos.dev/concepts/events/.
///
/// Move unit tests
/// We added a few unit tests to make sure that our code is working as expected. For more information on how to write
/// Move unit tests, see: https://aptos.dev/guides/move-guides/book/unit-testing
///
/// How to interact with this module:
/// 1.  Create and configure an admin account (in addition to the source account and nft-receiver account that we created in the earlier parts).
///     run `aptos init --profile admin` to create an admin account
///     go to Move.toml and replace `admin_addr = 0xcafe` with the actual admin address we just created
///
/// 2.a Ensure your terminal is in the correct directory:
///         `aptos-core/aptos-move/move-examples/mint_nft_v2/4-Getting-Production-Ready`
/// 2.b Publish the module under a resource account with the following command:
///         `aptos move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft_v2 --profile default --named-addresses source_addr=default`
///     Sample output is below:
///         aptos move create-resource-account-and-publish-package --seed 3 --address-name mint_nft_v2 --profile default --named-addresses source_addr=default
///         Compiling, may take a little while to download git dependencies...
///         INCLUDING DEPENDENCY AptosFramework
///         INCLUDING DEPENDENCY AptosStdlib
///         INCLUDING DEPENDENCY AptosTokenObjects
///         INCLUDING DEPENDENCY MoveStdlib
///         BUILDING Examples
///         Do you want to publish this package under the resource account's address a2534c87a046bc9dfe768a5f6d05ca9e5d5de7cc99aad60435f2325f6e9cf84c? [yes/no] >
///         yes
///         package size 6826 bytes
///         Do you want to submit a transaction for a range of [540000 - 810000] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": "Success"
///         }
///
///     Note the resource account address in the above output: a2534c87a046bc9dfe768a5f6d05ca9e5d5de7cc99aad60435f2325f6e9cf84c
///
///
/// 3. Call `mint_event_ticket()` with a valid signature to mint a token.
///     a. Get the public key of the admin account you created earlier. Look for the `public_key` of the `admin` profile:
///         `cat ~/.aptos/config.yaml`
///         Make sure you don't include the `0x` at the beginning of the address when supplying it to the function as a hex argument.
///     b. Update the public key stored within `ModuleData`.
///         aptos move run --function-id [resource account's address]::create_nft_getting_production_ready::set_public_key --args hex:[public key] --profile admin
///     c. Generate a valid signature.
///         Open up file `aptos-core/aptos-move/e2e-move-tests/src/tests/mint_nft_v2.rs`.
///         In function `generate_nft_tutorial_part4_signature`, change the `resource_address`, `nft_receiver`, `admin_private_key`, and `receiver_account_sequence_number` variables to the actual values.
///         You can find the `admin_private_key` by running `cat ~/.aptos/config.yaml`, and the `receiver_account_sequence_number` by looking up the receiver's address on the Aptos Explorer under tab `Info`.
///         Make sure you're in the right directory.
///         Run the following command in directory `aptos-core/aptos-move/e2e-move-tests`
///         Run `cargo test generate_nft_tutorial_part4_signature -- --nocapture` to generate a valid signature that we'll use in the next step.
///     d. Call mint_event_ticket() with the signature we generated in the last step.
///         aptos move run --function-id [resource account's address]::create_nft_getting_production_ready::mint_event_ticket --args hex:[signature generated in last step] --profile nft-receiver
///
///     Sample output is below:
///         aptos move run --function-id a2534c87a046bc9dfe768a5f6d05ca9e5d5de7cc99aad60435f2325f6e9cf84c::create_nft_getting_production_ready::mint_event_ticket --args hex:debdf8ae46162ba07721e1b03ed108d5453c2ba36d28ae8418f8363c489196de7e0404b3fa9bb8e56260903fed8be166138d145bb721155c7fa94551de853202 --profile nft-receiver
///         Do you want to submit a transaction for a range of [53000 - 79500] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0xcf6b13f085c05fd7699b14d98dac739c01be084201b467d3f84d0a98c010966a",
///             "gas_used": 531,
///             "gas_unit_price": 100,
///             "sender": "e9907369a82cc0d5b93c77e867e36b7e412912ed4825e17b3ca49541888cae67",
///             "sequence_number": 4,
///             "success": true,
///             "timestamp_us": 1683088148662007,
///             "version": 3719147,
///             "vm_status": "Executed successfully"
///           }
///         }
///     View the transaction on the explorer here:
///     https://explorer.aptoslabs.com/txn/0xcf6b13f085c05fd7699b14d98dac739c01be084201b467d3f84d0a98c010966a?network=devnet
///
/// 4. If you want to view the token and collection object addresses and their respective owners, you can query the REST API with the CURL command:
///     `curl --request GET --url https://fullnode.devnet.aptoslabs.com/v1/accounts/[resource_account_address]/resource/[resource_account_address]::create_nft_getting_production_ready::ObjectAddresses`
///     Make sure the address is prepended with `0x` or the query won't work correctly.
///     within that, get the `collection_object_address` and the `last_minted_token_object_address`, then query the resources at those addresses to view all object data:
///     `curl --request GET --url https://fullnode.devnet.aptoslabs.com/v1/accounts/[collection_object_address]/resources`
///     `curl --request GET --url https://fullnode.devnet.aptoslabs.com/v1/accounts/[last_minted_token_object_address]/resources`
///
/// This is the end of this NFT tutorial! Congrats on making it to the end. Please let us know if you have any questions / feedback by opening a github issue / feature request : )
module mint_nft_v2::create_nft_getting_production_ready {
    use std::error;
    use std::signer;
    use std::bcs;
    use std::object;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use aptos_framework::account;
    use aptos_framework::event::{EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::ed25519;
    use aptos_framework::resource_account;
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    use aptos_std::ed25519::ValidatedPublicKey;

    use aptos_token_objects::aptos_token::{Self, AptosToken};
    use aptos_token_objects::collection;


    // This struct stores the token receiver's address and token_data_id in the event of token minting
    struct TokenMintingEvent has drop, store {
        token_receiver_address: address,
        collection_name: String,
        creator: address,
        token_name: String,
    }

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        public_key: ed25519::ValidatedPublicKey,
        signer_cap: account::SignerCapability,
        collection_name: String,
        creator: address,
        token_description: String,
        token_name: String,
        token_uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        expiration_timestamp: u64,
        minting_enabled: bool,
        token_minting_events: EventHandle<TokenMintingEvent>,
    }

    struct ObjectAddresses has key {
        collection_object_address: address,
        last_minted_token_object_address: Option<address>,
    }

    // This struct stores the challenge message that proves that the resource signer wants to mint this token
    // to the receiver. This struct will need to be signed by the resource signer to pass the verification.
    struct MintProofChallenge has drop {
        receiver_account_sequence_number: u64,
        receiver_account_address: address,
        collection_name: String,
        creator: address,
        token_name: String,
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
        let maximum_supply = 1000;

        aptos_token::create_collection(
            resource_signer,
            description,
            maximum_supply,
            collection_name,
            collection_uri,
            false, // mutable_description
            false, // mutable_royalty
            false, // mutable_uri
            false, // mutable_token_description
            false, // mutable_token_name
            true, // mutable_token_properties
            false, // mutable_token_uri
            false, // tokens_burnable_by_creator
            false, // tokens_freezable_by_creator
            5, // royalty_numerator
            100, // royalty_denominator
        );

        // Retrieve the resource signer's signer capability and store it within the `ModuleData`.
        // Note that by calling `resource_account::retrieve_resource_account_cap` to retrieve the resource account's signer capability,
        // we rotate the resource account's authentication key to 0 and give up our control over the resource account. Before calling this function,
        // the resource account has the same authentication key as the source account so we had control over the resource account.
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @source_addr);
        let resource_address = signer::address_of(resource_signer);
        let collection_object_address = collection::create_collection_address(&resource_address, &collection_name);

        // hardcoded public key - we will update it to the real one by calling `set_public_key` from the admin account
        let pk_bytes = x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18";
        let public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(pk_bytes));
        move_to(resource_signer, ModuleData {
            public_key: public_key,
            signer_cap: resource_signer_cap,
            collection_name,
            creator: resource_address,
            token_description: string::utf8(b""),
            token_name,
            token_uri,
            property_keys: vector<String>[string::utf8(b"given_to")],
            property_types: vector<String>[ string::utf8(b"address") ],
            property_values: vector<vector<u8>>[bcs::to_bytes(&@source_addr)],
            minting_enabled: true,
            expiration_timestamp: 10000000000,
            token_minting_events: account::new_event_handle<TokenMintingEvent>(resource_signer),
        });

        //Store the collection object address with a currently empty most recently minted token object address for later.
        move_to(resource_signer, ObjectAddresses {
            collection_object_address,
            last_minted_token_object_address: option::none(),
        });
    }

    /// Mint an NFT to the receiver.
    /// `mint_proof_signature` should be the `MintProofChallenge` signed by the admin's private key
    /// `public_key_bytes` should be the public key of the admin
    public entry fun mint_event_ticket(receiver: &signer, mint_proof_signature: vector<u8>) acquires ModuleData, ObjectAddresses {
        // get the collection minter and check if the collection minting is disabled or expired
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        assert!(timestamp::now_seconds() < module_data.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        let receiver_addr = signer::address_of(receiver);
        // verify that the `mint_proof_signature` is valid against the admin's public key
        verify_proof_of_knowledge(receiver_addr,
            mint_proof_signature,
            module_data.collection_name,
            @mint_nft_v2,
            module_data.token_name,
            module_data.public_key);

        // mint token to the receiver
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let resource_address = signer::address_of(&resource_signer);
        let token_creation_num = account::get_guid_next_creation_num(resource_address);

        aptos_token::mint(
            &resource_signer,
            module_data.collection_name,
            module_data.token_description,
            module_data.token_name,
            module_data.token_uri,
            module_data.property_keys,
            module_data.property_types,
            module_data.property_values,
        );

        let token_object_address = object::create_guid_object_address(resource_address, token_creation_num);
        let token_object = object::address_to_object<AptosToken>(object::create_guid_object_address(resource_address, token_creation_num));
        object::transfer(&resource_signer, token_object, receiver_addr);

        let object_addresses = borrow_global_mut<ObjectAddresses>(@mint_nft_v2);
        if (option::is_some(&object_addresses.last_minted_token_object_address)) {
            option::swap(&mut object_addresses.last_minted_token_object_address, token_object_address);
        } else {
            option::fill(&mut object_addresses.last_minted_token_object_address, token_object_address);
        };

        // update "given_to" to the value of the new receiver.
        aptos_token::update_property(
            &resource_signer,
            token_object,
            string::utf8(b"given_to"),
            string::utf8(b"address"),
            bcs::to_bytes(&receiver_addr),
        );
    }

    /// Set if minting is enabled for this minting contract
    public entry fun set_minting_enabled(caller: &signer, minting_enabled: bool) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        module_data.minting_enabled = minting_enabled;
    }

    /// Set the expiration timestamp of this minting contract
    public entry fun set_timestamp(caller: &signer, expiration_timestamp: u64) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        module_data.expiration_timestamp = expiration_timestamp;
    }

    /// Set the public key of this minting contract
    public entry fun set_public_key(caller: &signer, pk_bytes: vector<u8>) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        module_data.public_key = std::option::extract(&mut ed25519::new_validated_public_key_from_bytes(pk_bytes));
    }

    /// Verify that the collection token minter intends to mint the given token_data_id to the receiver
    fun verify_proof_of_knowledge(
        receiver_addr: address,
        mint_proof_signature: vector<u8>,
        collection_name: String,
        creator: address,
        token_name: String,
        public_key: ValidatedPublicKey,
    ) {
        let sequence_number = account::get_sequence_number(receiver_addr);

        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: sequence_number,
            receiver_account_address: receiver_addr,
            collection_name,
            creator,
            token_name,
        };

        let signature = ed25519::new_signature_from_bytes(mint_proof_signature);
        let unvalidated_public_key = ed25519::public_key_to_unvalidated(&public_key);
        assert!(ed25519::signature_verify_strict_t(&signature, &unvalidated_public_key, proof_challenge), error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE));
    }

    //
    // Tests
    //

    // unit tests will only work if you specify `mint_nft_v2` and `source_addr` such that `mint_nft_v2` is
    // the generated resource address with an empty seed combined with `source_addr
    #[test_only]
    public fun set_up_test(
        origin_account: signer,
        resource_account: &signer,
        collection_token_minter_public_key: &ValidatedPublicKey,
        aptos_framework: signer,
        nft_receiver: &signer,
        timestamp: u64
    ) acquires ModuleData {
        // set up global time for testing purpose
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);

        create_account_for_test(signer::address_of(&origin_account));

        // for clarity's sake, this is how you can re-generate the resource address with a seed to view later
        let seed = vector::empty<u8>();
        let resource_address = std::account::create_resource_address(&signer::address_of(&origin_account), seed);
        assert!(@mint_nft_v2 == resource_address, 0);
        // create a resource account from the origin account, mocking the module publishing process
        resource_account::create_resource_account(&origin_account, seed, vector::empty<u8>());

        init_module(resource_account);

        let admin = create_account_for_test(@admin_addr);
        let pk_bytes = ed25519::validated_public_key_to_bytes(collection_token_minter_public_key);
        set_public_key(&admin, pk_bytes);

        create_account_for_test(signer::address_of(nft_receiver));
    }

    // because this contract uses hard coded addresses for @mint_nft_v2 resource access, you must supply this to the unit test command:
    // mint_nft_v2=0xa8988f85709a0b4ad2c8e5d28a39131fc1d4ad0cccd3f4da0083d6f5b5410df2
    #[test (origin_account = @0x1234, resource_account = @mint_nft_v2, nft_receiver = @nft_receiver, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50002, location = mint_nft_v2::create_nft_getting_production_ready)]
    public entry fun test_minting_expired(origin_account: signer, resource_account: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData, ObjectAddresses {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 100000000001);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            collection_name: borrow_global<ModuleData>(@mint_nft_v2).collection_name,
            creator: borrow_global<ModuleData>(@mint_nft_v2).creator,
            token_name: borrow_global<ModuleData>(@mint_nft_v2).token_name,
        };
        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0x1234, resource_account = @mint_nft_v2, nft_receiver = @nft_receiver, aptos_framework = @aptos_framework)]
    public entry fun test_minting_success(origin_account: signer, resource_account: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData, ObjectAddresses {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 100000);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            collection_name: borrow_global<ModuleData>(@mint_nft_v2).collection_name,
            creator: borrow_global<ModuleData>(@mint_nft_v2).creator,
            token_name: borrow_global<ModuleData>(@mint_nft_v2).token_name,
        };
        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0x1234, resource_account = @mint_nft_v2, admin = @admin_addr, nft_receiver = @nft_receiver, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50002, location = mint_nft_v2::create_nft_getting_production_ready)]
    public entry fun test_update_expiration_time(origin_account: signer, resource_account: signer, admin: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData, ObjectAddresses {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            collection_name: borrow_global<ModuleData>(@mint_nft_v2).collection_name,
            creator: borrow_global<ModuleData>(@mint_nft_v2).creator,
            token_name: borrow_global<ModuleData>(@mint_nft_v2).token_name,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // set the expiration time of the minting to be earlier than the current time
        set_timestamp(&admin, 5);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0x1234, resource_account = @mint_nft_v2, admin = @admin_addr, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50003, location = mint_nft_v2::create_nft_getting_production_ready)]
    public entry fun test_update_minting_enabled(origin_account: signer, resource_account: signer, admin: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData, ObjectAddresses {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            collection_name: borrow_global<ModuleData>(@mint_nft_v2).collection_name,
            creator: borrow_global<ModuleData>(@mint_nft_v2).creator,
            token_name: borrow_global<ModuleData>(@mint_nft_v2).token_name,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);

        // disable token minting
        set_minting_enabled(&admin, false);
        mint_event_ticket(&nft_receiver, ed25519::signature_to_bytes(&sig));
    }

    #[test (origin_account = @0x1234, resource_account = @mint_nft_v2, nft_receiver = @0x123, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10006, location = mint_nft_v2::create_nft_getting_production_ready)]
    public entry fun test_invalid_signature(origin_account: signer, resource_account: signer, nft_receiver: signer, aptos_framework: signer) acquires ModuleData, ObjectAddresses {
        let (admin_sk, admin_pk) = ed25519::generate_keys();
        set_up_test(origin_account, &resource_account, &admin_pk, aptos_framework, &nft_receiver, 10);
        let receiver_addr = signer::address_of(&nft_receiver);
        let proof_challenge = MintProofChallenge {
            receiver_account_sequence_number: account::get_sequence_number(receiver_addr),
            receiver_account_address: receiver_addr,
            collection_name: borrow_global<ModuleData>(@mint_nft_v2).collection_name,
            creator: borrow_global<ModuleData>(@mint_nft_v2).creator,
            token_name: borrow_global<ModuleData>(@mint_nft_v2).token_name,
        };

        let sig = ed25519::sign_struct(&admin_sk, proof_challenge);
        let sig_bytes = ed25519::signature_to_bytes(&sig);

        // Pollute signature.
        let first_sig_byte = vector::borrow_mut(&mut sig_bytes, 0);
        *first_sig_byte = *first_sig_byte + 1;

        mint_event_ticket(&nft_receiver, sig_bytes);
    }
}
