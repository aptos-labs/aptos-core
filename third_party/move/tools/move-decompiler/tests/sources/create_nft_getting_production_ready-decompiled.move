module 0x12::create_nft_getting_production_ready {
    struct MintProofChallenge has drop {
        receiver_account_sequence_number: u64,
        receiver_account_address: address,
        token_data_id: 0x1337::token::TokenDataId,
    }
    
    struct ModuleData has key {
        public_key: 0x1::ed25519::ValidatedPublicKey,
        signer_cap: 0x1::account::SignerCapability,
        token_data_id: 0x1337::token::TokenDataId,
        expiration_timestamp: u64,
        minting_enabled: bool,
        token_minting_events: 0x1::event::EventHandle<TokenMintingEvent>,
    }
    
    struct TokenMintingEvent has drop, store {
        token_receiver_address: address,
        token_data_id: 0x1337::token::TokenDataId,
    }
    
    fun init_module(arg0: &signer) {
        let v0 = 0x1::string::utf8(b"Collection name");
        0x1337::token::create_collection(arg0, v0, 0x1::string::utf8(b"Description"), 0x1::string::utf8(b"Collection uri"), 0, vector[false, false, false]);
        let v1 = vector[false, false, false, false, true];
        let v2 = 0x1::vector::empty<0x1::string::String>();
        0x1::vector::push_back<0x1::string::String>(&mut v2, 0x1::string::utf8(b"given_to"));
        let v3 = 0x1::vector::empty<0x1::string::String>();
        0x1::vector::push_back<0x1::string::String>(&mut v3, 0x1::string::utf8(b"address"));
        let v4 = 0x1::ed25519::new_validated_public_key_from_bytes(x"f66bf0ce5ceb582b93d6780820c2025b9967aedaa259bdbb9f3d0297eced0e18");
        let v5 = ModuleData{
            public_key           : 0x1::option::extract<0x1::ed25519::ValidatedPublicKey>(&mut v4), 
            signer_cap           : 0x1::resource_account::retrieve_resource_account_cap(arg0, @0x2345), 
            token_data_id        : 0x1337::token::create_tokendata(arg0, v0, 0x1::string::utf8(b"Token name"), 0x1::string::utf8(b""), 0, 0x1::string::utf8(b"Token uri"), 0x1::signer::address_of(arg0), 1, 0, 0x1337::token::create_token_mutability_config(&v1), v2, vector[b""], v3), 
            expiration_timestamp : 10000000000, 
            minting_enabled      : true, 
            token_minting_events : 0x1::account::new_event_handle<TokenMintingEvent>(arg0),
        };
        move_to<ModuleData>(arg0, v5);
    }
    
    public entry fun mint_event_ticket(arg0: &signer, arg1: vector<u8>) acquires ModuleData {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global_mut<ModuleData>(@0x1234);
        assert!(0x1::timestamp::now_seconds() < v1.expiration_timestamp, 0x1::error::permission_denied(2));
        assert!(v1.minting_enabled, 0x1::error::permission_denied(3));
        verify_proof_of_knowledge(v0, arg1, v1.token_data_id, v1.public_key);
        let v2 = 0x1::account::create_signer_with_capability(&v1.signer_cap);
        0x1337::token::direct_transfer(&v2, arg0, 0x1337::token::mint_token(&v2, v1.token_data_id, 1), 1);
        let v3 = TokenMintingEvent{
            token_receiver_address : v0, 
            token_data_id          : v1.token_data_id,
        };
        0x1::event::emit_event<TokenMintingEvent>(&mut v1.token_minting_events, v3);
        let (v4, v5, v6) = 0x1337::token::get_token_data_id_fields(&v1.token_data_id);
        0x1337::token::mutate_token_properties(&v2, v0, v4, v5, v6, 0, 1, 0x1::vector::empty<0x1::string::String>(), 0x1::vector::empty<vector<u8>>(), 0x1::vector::empty<0x1::string::String>());
    }
    
    public entry fun set_minting_enabled(arg0: &signer, arg1: bool) acquires ModuleData {
        assert!(0x1::signer::address_of(arg0) == @0xbeef, 0x1::error::permission_denied(1));
        borrow_global_mut<ModuleData>(@0x1234).minting_enabled = arg1;
    }
    
    public entry fun set_public_key(arg0: &signer, arg1: vector<u8>) acquires ModuleData {
        assert!(0x1::signer::address_of(arg0) == @0xbeef, 0x1::error::permission_denied(1));
        let v0 = 0x1::ed25519::new_validated_public_key_from_bytes(arg1);
        borrow_global_mut<ModuleData>(@0x1234).public_key = 0x1::option::extract<0x1::ed25519::ValidatedPublicKey>(&mut v0);
    }
    
    public entry fun set_timestamp(arg0: &signer, arg1: u64) acquires ModuleData {
        assert!(0x1::signer::address_of(arg0) == @0xbeef, 0x1::error::permission_denied(1));
        borrow_global_mut<ModuleData>(@0x1234).expiration_timestamp = arg1;
    }
    
    fun verify_proof_of_knowledge(arg0: address, arg1: vector<u8>, arg2: 0x1337::token::TokenDataId, arg3: 0x1::ed25519::ValidatedPublicKey) {
        let v0 = MintProofChallenge{
            receiver_account_sequence_number : 0x1::account::get_sequence_number(arg0), 
            receiver_account_address         : arg0, 
            token_data_id                    : arg2,
        };
        let v1 = 0x1::ed25519::new_signature_from_bytes(arg1);
        let v2 = 0x1::ed25519::public_key_to_unvalidated(&arg3);
        assert!(0x1::ed25519::signature_verify_strict_t<MintProofChallenge>(&v1, &v2, v0), 0x1::error::invalid_argument(6));
    }
    
    // decompiled from Move bytecode v6
}
