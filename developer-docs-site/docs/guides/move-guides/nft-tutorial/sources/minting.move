module mint_nft::minting {
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_token::token::{Self, TokenDataId};
    use aptos_framework::resource_account;

    // This struct stores the token receiver's address and token_data_id in the event of token minting
    struct TokenMintingEvent has drop, store {
        token_receiver_address: address,
        token_data_id: TokenDataId,
    }

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        counter: u64,
        signer_cap: account::SignerCapability,
        minting_enabled: bool,
        token_minting_events: EventHandle<TokenMintingEvent>,
    }

    /// Action not authorized because the signer is not the admin of this module
    const ENOT_AUTHORIZED: u64 = 1;
    /// The collection minting is disabled
    const EMINTING_DISABLED: u64 = 2;

    const COLLECTION_NAME: vector<u8> = b"Move Workshop 1";
    const TOKEN_NAME_PREFIX: vector<u8> = b"Aptos Penguin";

    fun init_module(resource_account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_account, @source_addr);
        let resource_signer = account::create_signer_with_capability(&resource_signer_cap);

        // create the nft collection
        let collection = string::utf8(COLLECTION_NAME);
        let description = string::utf8(b"this is for our first move workshop!!");
        let collection_uri = string::utf8(b"N/A");
        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        token::create_collection(&resource_signer, collection, description, collection_uri, maximum_supply, mutate_setting);

        move_to(resource_account, ModuleData {
            counter: 1,
            signer_cap: resource_signer_cap,
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

    /// Mint an NFT to the receiver.
    /// `mint_proof_signature` should be the `MintProofChallenge` signed by the admin's private key
    /// `public_key_bytes` should be the public key of the admin
    public entry fun mint_nft(receiver: &signer) acquires ModuleData {
        let receiver_addr = signer::address_of(receiver);

        // get the collection minter and check if the collection minting is disabled or expired
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

        let collection = string::utf8(COLLECTION_NAME);
        let token_name = string::utf8(TOKEN_NAME_PREFIX);
        string::append_utf8(&mut token_name, b": ");
        let num = u64_to_string(module_data.counter);
        string::append(&mut token_name, num);

        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);

        let token_data_id = token::create_tokendata(
            &resource_signer,
            collection,
            token_name,
            string::utf8(b"Penguins for Our First Move Workshop!"),
            0,
            string::utf8(b"https://slwdaeeko5tz5hx46c6zwqhmh3c6je4sbdbjsdjzbntme5dxarxa.arweave.net/kuwwEIp3Z56e_PC9m0DsPsXkk5IIwpkNOQtmwnR3BG4"),
            @mint_nft,
            1,
            0,
            token::create_token_mutability_config(&vector<bool>[ false, true, false, false, true ]),
            vector::empty<String>(),
            vector::empty<vector<u8>>(),
            vector::empty<String>(),
        );

        let token_id = token::mint_token(&resource_signer, token_data_id, 1);
        token::direct_transfer(&resource_signer, receiver, token_id, 1);

        event::emit_event<TokenMintingEvent>(
            &mut module_data.token_minting_events,
            TokenMintingEvent {
                token_receiver_address: receiver_addr,
                token_data_id,
            }
        );

        module_data.counter = module_data.counter + 1;
    }

    fun u64_to_string(value: u64): string::String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let buffer = vector::empty<u8>();
        while (value != 0) {
            vector::push_back(&mut buffer, ((48 + value % 10) as u8));
            value = value / 10;
        };
        vector::reverse(&mut buffer);
        string::utf8(buffer)
    }
}
