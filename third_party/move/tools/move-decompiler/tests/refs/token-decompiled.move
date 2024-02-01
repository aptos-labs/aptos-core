module 0x1337::token {
    struct BurnTokenEvent has drop, store {
        id: TokenId,
        amount: u64,
    }
    
    struct CollectionData has store {
        description: 0x1::string::String,
        name: 0x1::string::String,
        uri: 0x1::string::String,
        supply: u64,
        maximum: u64,
        mutability_config: CollectionMutabilityConfig,
    }
    
    struct CollectionMutabilityConfig has copy, drop, store {
        description: bool,
        uri: bool,
        maximum: bool,
    }
    
    struct Collections has key {
        collection_data: 0x1::table::Table<0x1::string::String, CollectionData>,
        token_data: 0x1::table::Table<TokenDataId, TokenData>,
        create_collection_events: 0x1::event::EventHandle<CreateCollectionEvent>,
        create_token_data_events: 0x1::event::EventHandle<CreateTokenDataEvent>,
        mint_token_events: 0x1::event::EventHandle<MintTokenEvent>,
    }
    
    struct CreateCollectionEvent has drop, store {
        creator: address,
        collection_name: 0x1::string::String,
        uri: 0x1::string::String,
        description: 0x1::string::String,
        maximum: u64,
    }
    
    struct CreateTokenDataEvent has drop, store {
        id: TokenDataId,
        description: 0x1::string::String,
        maximum: u64,
        uri: 0x1::string::String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        name: 0x1::string::String,
        mutability_config: TokenMutabilityConfig,
        property_keys: vector<0x1::string::String>,
        property_values: vector<vector<u8>>,
        property_types: vector<0x1::string::String>,
    }
    
    struct DepositEvent has drop, store {
        id: TokenId,
        amount: u64,
    }
    
    struct MintTokenEvent has drop, store {
        id: TokenDataId,
        amount: u64,
    }
    
    struct MutateTokenPropertyMapEvent has drop, store {
        old_id: TokenId,
        new_id: TokenId,
        keys: vector<0x1::string::String>,
        values: vector<vector<u8>>,
        types: vector<0x1::string::String>,
    }
    
    struct Royalty has copy, drop, store {
        royalty_points_numerator: u64,
        royalty_points_denominator: u64,
        payee_address: address,
    }
    
    struct Token has store {
        id: TokenId,
        amount: u64,
        token_properties: 0x1337::property_map::PropertyMap,
    }
    
    struct TokenData has store {
        maximum: u64,
        largest_property_version: u64,
        supply: u64,
        uri: 0x1::string::String,
        royalty: Royalty,
        name: 0x1::string::String,
        description: 0x1::string::String,
        default_properties: 0x1337::property_map::PropertyMap,
        mutability_config: TokenMutabilityConfig,
    }
    
    struct TokenDataId has copy, drop, store {
        creator: address,
        collection: 0x1::string::String,
        name: 0x1::string::String,
    }
    
    struct TokenId has copy, drop, store {
        token_data_id: TokenDataId,
        property_version: u64,
    }
    
    struct TokenMutabilityConfig has copy, drop, store {
        maximum: bool,
        uri: bool,
        royalty: bool,
        description: bool,
        properties: bool,
    }
    
    struct TokenStore has key {
        tokens: 0x1::table::Table<TokenId, Token>,
        direct_transfer: bool,
        deposit_events: 0x1::event::EventHandle<DepositEvent>,
        withdraw_events: 0x1::event::EventHandle<WithdrawEvent>,
        burn_events: 0x1::event::EventHandle<BurnTokenEvent>,
        mutate_token_property_events: 0x1::event::EventHandle<MutateTokenPropertyMapEvent>,
    }
    
    struct WithdrawCapability has drop, store {
        token_owner: address,
        token_id: TokenId,
        amount: u64,
        expiration_sec: u64,
    }
    
    struct WithdrawEvent has drop, store {
        id: TokenId,
        amount: u64,
    }
    
    fun assert_collection_exists(arg0: address, arg1: 0x1::string::String) acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = borrow_global<Collections>(arg0);
        let v1 = 0x1::table::contains<0x1::string::String, CollectionData>(&v0.collection_data, arg1);
        assert!(v1, 0x1::error::not_found(2));
    }
    
    fun assert_non_standard_reserved_property(arg0: &vector<0x1::string::String>) {
        let v0 = 0;
        while (v0 < 0x1::vector::length<0x1::string::String>(arg0)) {
            let v1 = 0x1::vector::borrow<0x1::string::String>(arg0, v0);
            if (0x1::string::length(v1) >= 6) {
                let v2 = *v1;
                let v3 = 0x1::string::sub_string(&v2, 0, 6) != 0x1::string::utf8(b"TOKEN_");
                assert!(v3, 0x1::error::permission_denied(40));
            };
            v0 = v0 + 1;
        };
    }
    
    fun assert_tokendata_exists(arg0: &signer, arg1: TokenDataId) acquires Collections {
        let v0 = arg1.creator;
        assert!(0x1::signer::address_of(arg0) == v0, 0x1::error::permission_denied(14));
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &mut borrow_global_mut<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg1), 0x1::error::not_found(10));
    }
    
    public fun balance_of(arg0: address, arg1: TokenId) : u64 acquires TokenStore {
        if (!exists<TokenStore>(arg0)) {
            return 0
        };
        let v0 = borrow_global<TokenStore>(arg0);
        if (0x1::table::contains<TokenId, Token>(&v0.tokens, arg1)) {
            0x1::table::borrow<TokenId, Token>(&v0.tokens, arg1).amount
        } else {
            0
        }
    }
    
    public entry fun burn(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: u64) acquires Collections, TokenStore {
        assert!(arg5 > 0, 0x1::error::invalid_argument(29));
        let v0 = create_token_id_raw(arg1, arg2, arg3, arg4);
        let v1 = v0.token_data_id.creator;
        assert!(exists<Collections>(v1), 0x1::error::not_found(1));
        let v2 = borrow_global_mut<Collections>(v1);
        let v3 = 0x1::table::contains<TokenDataId, TokenData>(&v2.token_data, v0.token_data_id);
        assert!(v3, 0x1::error::not_found(10));
        let v4 = 0x1::table::borrow_mut<TokenDataId, TokenData>(&mut v2.token_data, v0.token_data_id);
        let v5 = 0x1::string::utf8(b"TOKEN_BURNABLE_BY_OWNER");
        let v6 = 0x1337::property_map::contains_key(&v4.default_properties, &v5);
        assert!(v6, 0x1::error::permission_denied(30));
        let v7 = 0x1::string::utf8(b"TOKEN_BURNABLE_BY_OWNER");
        let v8 = 0x1337::property_map::read_bool(&v4.default_properties, &v7);
        assert!(v8, 0x1::error::permission_denied(30));
        let v9 = withdraw_token(arg0, v0, arg5);
        let Token {
            id               : _,
            amount           : v11,
            token_properties : _,
        } = v9;
        let v13 = &mut borrow_global_mut<TokenStore>(0x1::signer::address_of(arg0)).burn_events;
        let v14 = BurnTokenEvent{
            id     : v0, 
            amount : v11,
        };
        0x1::event::emit_event<BurnTokenEvent>(v13, v14);
        let v15 = 0x1::table::borrow_mut<TokenDataId, TokenData>(&mut v2.token_data, v0.token_data_id);
        if (v15.maximum > 0) {
            v15.supply = v15.supply - v11;
            if (v15.supply == 0) {
                destroy_token_data(0x1::table::remove<TokenDataId, TokenData>(&mut v2.token_data, v0.token_data_id));
                let v16 = v0.token_data_id.collection;
                let v17 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(&mut v2.collection_data, v16);
                if (v17.maximum > 0) {
                    v17.supply = v17.supply - 1;
                    if (v17.supply == 0) {
                        let v18 = 0x1::table::remove<0x1::string::String, CollectionData>(&mut v2.collection_data, v17.name);
                        destroy_collection_data(v18);
                    };
                };
            };
        };
    }
    
    public entry fun burn_by_creator(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: u64) acquires Collections, TokenStore {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(arg5 > 0, 0x1::error::invalid_argument(29));
        let v1 = create_token_id_raw(v0, arg2, arg3, arg4);
        assert!(exists<Collections>(v1.token_data_id.creator), 0x1::error::not_found(1));
        let v2 = borrow_global_mut<Collections>(v0);
        let v3 = 0x1::table::contains<TokenDataId, TokenData>(&v2.token_data, v1.token_data_id);
        assert!(v3, 0x1::error::not_found(10));
        let v4 = 0x1::table::borrow_mut<TokenDataId, TokenData>(&mut v2.token_data, v1.token_data_id);
        let v5 = 0x1::string::utf8(b"TOKEN_BURNABLE_BY_CREATOR");
        let v6 = 0x1337::property_map::contains_key(&v4.default_properties, &v5);
        assert!(v6, 0x1::error::permission_denied(31));
        let v7 = 0x1::string::utf8(b"TOKEN_BURNABLE_BY_CREATOR");
        let v8 = 0x1337::property_map::read_bool(&v4.default_properties, &v7);
        assert!(v8, 0x1::error::permission_denied(31));
        let v9 = withdraw_with_event_internal(arg1, v1, arg5);
        let Token {
            id               : _,
            amount           : v11,
            token_properties : _,
        } = v9;
        let v13 = BurnTokenEvent{
            id     : v1, 
            amount : v11,
        };
        0x1::event::emit_event<BurnTokenEvent>(&mut borrow_global_mut<TokenStore>(arg1).burn_events, v13);
        if (v4.maximum > 0) {
            v4.supply = v4.supply - v11;
            if (v4.supply == 0) {
                destroy_token_data(0x1::table::remove<TokenDataId, TokenData>(&mut v2.token_data, v1.token_data_id));
                let v14 = v1.token_data_id.collection;
                let v15 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(&mut v2.collection_data, v14);
                if (v15.maximum > 0) {
                    v15.supply = v15.supply - 1;
                    if (v15.supply == 0) {
                        let v16 = 0x1::table::remove<0x1::string::String, CollectionData>(&mut v2.collection_data, v15.name);
                        destroy_collection_data(v16);
                    };
                };
            };
        };
    }
    
    public fun check_collection_exists(arg0: address, arg1: 0x1::string::String) : bool acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).collection_data;
        0x1::table::contains<0x1::string::String, CollectionData>(v0, arg1)
    }
    
    public fun check_tokendata_exists(arg0: address, arg1: 0x1::string::String, arg2: 0x1::string::String) : bool acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).token_data;
        0x1::table::contains<TokenDataId, TokenData>(v0, create_token_data_id(arg0, arg1, arg2))
    }
    
    public fun create_collection(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: vector<bool>) acquires Collections {
        assert!(0x1::string::length(&arg1) <= 128, 0x1::error::invalid_argument(25));
        assert!(0x1::string::length(&arg3) <= 512, 0x1::error::invalid_argument(27));
        let v0 = 0x1::signer::address_of(arg0);
        if (!exists<Collections>(v0)) {
            let v1 = 0x1::table::new<0x1::string::String, CollectionData>();
            let v2 = 0x1::table::new<TokenDataId, TokenData>();
            let v3 = 0x1::account::new_event_handle<CreateCollectionEvent>(arg0);
            let v4 = 0x1::account::new_event_handle<CreateTokenDataEvent>(arg0);
            let v5 = 0x1::account::new_event_handle<MintTokenEvent>(arg0);
            let v6 = Collections{
                collection_data          : v1, 
                token_data               : v2, 
                create_collection_events : v3, 
                create_token_data_events : v4, 
                mint_token_events        : v5,
            };
            move_to<Collections>(arg0, v6);
        };
        let v7 = &mut borrow_global_mut<Collections>(v0).collection_data;
        assert!(!0x1::table::contains<0x1::string::String, CollectionData>(v7, arg1), 0x1::error::already_exists(3));
        let v8 = create_collection_mutability_config(&arg5);
        let v9 = CollectionData{
            description       : arg2, 
            name              : arg1, 
            uri               : arg3, 
            supply            : 0, 
            maximum           : arg4, 
            mutability_config : v8,
        };
        0x1::table::add<0x1::string::String, CollectionData>(v7, arg1, v9);
        let v10 = CreateCollectionEvent{
            creator         : v0, 
            collection_name : arg1, 
            uri             : arg3, 
            description     : arg2, 
            maximum         : arg4,
        };
        0x1::event::emit_event<CreateCollectionEvent>(&mut borrow_global_mut<Collections>(v0).create_collection_events, v10);
    }
    
    public fun create_collection_mutability_config(arg0: &vector<bool>) : CollectionMutabilityConfig {
        let v0 = *0x1::vector::borrow<bool>(arg0, 1);
        let v1 = *0x1::vector::borrow<bool>(arg0, 2);
        CollectionMutabilityConfig{
            description : *0x1::vector::borrow<bool>(arg0, 0), 
            uri         : v0, 
            maximum     : v1,
        }
    }
    
    public entry fun create_collection_script(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: vector<bool>) acquires Collections {
        create_collection(arg0, arg1, arg2, arg3, arg4, arg5);
    }
    
    public fun create_royalty(arg0: u64, arg1: u64, arg2: address) : Royalty {
        assert!(arg0 <= arg1, 0x1::error::invalid_argument(34));
        assert!(0x1::account::exists_at(arg2), 0x1::error::invalid_argument(35));
        Royalty{
            royalty_points_numerator   : arg0, 
            royalty_points_denominator : arg1, 
            payee_address              : arg2,
        }
    }
    
    public fun create_token_data_id(arg0: address, arg1: 0x1::string::String, arg2: 0x1::string::String) : TokenDataId {
        assert!(0x1::string::length(&arg1) <= 128, 0x1::error::invalid_argument(25));
        assert!(0x1::string::length(&arg2) <= 128, 0x1::error::invalid_argument(26));
        TokenDataId{
            creator    : arg0, 
            collection : arg1, 
            name       : arg2,
        }
    }
    
    public fun create_token_id(arg0: TokenDataId, arg1: u64) : TokenId {
        TokenId{
            token_data_id    : arg0, 
            property_version : arg1,
        }
    }
    
    public fun create_token_id_raw(arg0: address, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u64) : TokenId {
        TokenId{
            token_data_id    : create_token_data_id(arg0, arg1, arg2), 
            property_version : arg3,
        }
    }
    
    public fun create_token_mutability_config(arg0: &vector<bool>) : TokenMutabilityConfig {
        let v0 = *0x1::vector::borrow<bool>(arg0, 0);
        let v1 = *0x1::vector::borrow<bool>(arg0, 1);
        let v2 = *0x1::vector::borrow<bool>(arg0, 2);
        let v3 = *0x1::vector::borrow<bool>(arg0, 3);
        let v4 = *0x1::vector::borrow<bool>(arg0, 4);
        TokenMutabilityConfig{
            maximum     : v0, 
            uri         : v1, 
            royalty     : v2, 
            description : v3, 
            properties  : v4,
        }
    }
    
    public entry fun create_token_script(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: u64, arg6: 0x1::string::String, arg7: address, arg8: u64, arg9: u64, arg10: vector<bool>, arg11: vector<0x1::string::String>, arg12: vector<vector<u8>>, arg13: vector<0x1::string::String>) acquires Collections, TokenStore {
        let v0 = create_token_mutability_config(&arg10);
        let v1 = create_tokendata(arg0, arg1, arg2, arg3, arg5, arg6, arg7, arg8, arg9, v0, arg11, arg12, arg13);
        mint_token(arg0, v1, arg4);
    }
    
    public fun create_tokendata(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: 0x1::string::String, arg6: address, arg7: u64, arg8: u64, arg9: TokenMutabilityConfig, arg10: vector<0x1::string::String>, arg11: vector<vector<u8>>, arg12: vector<0x1::string::String>) : TokenDataId acquires Collections {
        assert!(0x1::string::length(&arg2) <= 128, 0x1::error::invalid_argument(26));
        assert!(0x1::string::length(&arg1) <= 128, 0x1::error::invalid_argument(25));
        assert!(0x1::string::length(&arg5) <= 512, 0x1::error::invalid_argument(27));
        assert!(arg8 <= arg7, 0x1::error::invalid_argument(34));
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = borrow_global_mut<Collections>(v0);
        let v2 = create_token_data_id(v0, arg1, arg2);
        assert!(0x1::table::contains<0x1::string::String, CollectionData>(&v1.collection_data, v2.collection), 0x1::error::not_found(2));
        assert!(!0x1::table::contains<TokenDataId, TokenData>(&v1.token_data, v2), 0x1::error::already_exists(9));
        let v3 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(&mut v1.collection_data, v2.collection);
        if (v3.maximum > 0) {
            v3.supply = v3.supply + 1;
            assert!(v3.maximum >= v3.supply, 0x1::error::invalid_argument(4));
        };
        let v4 = TokenData{
            maximum                  : arg4, 
            largest_property_version : 0, 
            supply                   : 0, 
            uri                      : arg5, 
            royalty                  : create_royalty(arg8, arg7, arg6), 
            name                     : arg2, 
            description              : arg3, 
            default_properties       : 0x1337::property_map::new(arg10, arg11, arg12), 
            mutability_config        : arg9,
        };
        0x1::table::add<TokenDataId, TokenData>(&mut v1.token_data, v2, v4);
        let v5 = arg5;
        let v6 = arg2;
        let v7 = CreateTokenDataEvent{
            id                         : v2, 
            description                : arg3, 
            maximum                    : arg4, 
            uri                        : v5, 
            royalty_payee_address      : arg6, 
            royalty_points_denominator : arg7, 
            royalty_points_numerator   : arg8, 
            name                       : v6, 
            mutability_config          : arg9, 
            property_keys              : arg10, 
            property_values            : arg11, 
            property_types             : arg12,
        };
        0x1::event::emit_event<CreateTokenDataEvent>(&mut v1.create_token_data_events, v7);
        v2
    }
    
    public fun create_withdraw_capability(arg0: &signer, arg1: TokenId, arg2: u64, arg3: u64) : WithdrawCapability {
        let v0 = 0x1::signer::address_of(arg0);
        WithdrawCapability{
            token_owner    : v0, 
            token_id       : arg1, 
            amount         : arg2, 
            expiration_sec : arg3,
        }
    }
    
    public fun deposit_token(arg0: &signer, arg1: Token) acquires TokenStore {
        initialize_token_store(arg0);
        direct_deposit(0x1::signer::address_of(arg0), arg1);
    }
    
    fun destroy_collection_data(arg0: CollectionData) {
        let CollectionData {
            description       : _,
            name              : _,
            uri               : _,
            supply            : _,
            maximum           : _,
            mutability_config : _,
        } = arg0;
    }
    
    fun destroy_token_data(arg0: TokenData) {
        let TokenData {
            maximum                  : _,
            largest_property_version : _,
            supply                   : _,
            uri                      : _,
            royalty                  : _,
            name                     : _,
            description              : _,
            default_properties       : _,
            mutability_config        : _,
        } = arg0;
    }
    
    fun direct_deposit(arg0: address, arg1: Token) acquires TokenStore {
        assert!(arg1.amount > 0, 0x1::error::invalid_argument(33));
        let v0 = borrow_global_mut<TokenStore>(arg0);
        let v1 = DepositEvent{
            id     : arg1.id, 
            amount : arg1.amount,
        };
        0x1::event::emit_event<DepositEvent>(&mut v0.deposit_events, v1);
        assert!(exists<TokenStore>(arg0), 0x1::error::not_found(11));
        if (!0x1::table::contains<TokenId, Token>(&v0.tokens, arg1.id)) {
            0x1::table::add<TokenId, Token>(&mut v0.tokens, arg1.id, arg1);
        } else {
            merge(0x1::table::borrow_mut<TokenId, Token>(&mut v0.tokens, arg1.id), arg1);
        };
    }
    
    public fun direct_deposit_with_opt_in(arg0: address, arg1: Token) acquires TokenStore {
        assert!(borrow_global<TokenStore>(arg0).direct_transfer, 0x1::error::permission_denied(16));
        direct_deposit(arg0, arg1);
    }
    
    public fun direct_transfer(arg0: &signer, arg1: &signer, arg2: TokenId, arg3: u64) acquires TokenStore {
        let v0 = withdraw_token(arg0, arg2, arg3);
        deposit_token(arg1, v0);
    }
    
    public entry fun direct_transfer_script(arg0: &signer, arg1: &signer, arg2: address, arg3: 0x1::string::String, arg4: 0x1::string::String, arg5: u64, arg6: u64) acquires TokenStore {
        direct_transfer(arg0, arg1, create_token_id_raw(arg2, arg3, arg4, arg5), arg6);
    }
    
    public fun get_collection_description(arg0: address, arg1: 0x1::string::String) : 0x1::string::String acquires Collections {
        assert_collection_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg0).collection_data;
        0x1::table::borrow_mut<0x1::string::String, CollectionData>(v0, arg1).description
    }
    
    public fun get_collection_maximum(arg0: address, arg1: 0x1::string::String) : u64 acquires Collections {
        assert_collection_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg0).collection_data;
        0x1::table::borrow_mut<0x1::string::String, CollectionData>(v0, arg1).maximum
    }
    
    public fun get_collection_mutability_config(arg0: address, arg1: 0x1::string::String) : CollectionMutabilityConfig acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).collection_data;
        let v1 = 0x1::table::contains<0x1::string::String, CollectionData>(v0, arg1);
        assert!(v1, 0x1::error::not_found(2));
        0x1::table::borrow<0x1::string::String, CollectionData>(v0, arg1).mutability_config
    }
    
    public fun get_collection_mutability_description(arg0: &CollectionMutabilityConfig) : bool {
        arg0.description
    }
    
    public fun get_collection_mutability_maximum(arg0: &CollectionMutabilityConfig) : bool {
        arg0.maximum
    }
    
    public fun get_collection_mutability_uri(arg0: &CollectionMutabilityConfig) : bool {
        arg0.uri
    }
    
    public fun get_collection_supply(arg0: address, arg1: 0x1::string::String) : 0x1::option::Option<u64> acquires Collections {
        assert_collection_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg0).collection_data;
        let v1 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(v0, arg1);
        if (v1.maximum > 0) {
            0x1::option::some<u64>(v1.supply)
        } else {
            0x1::option::none<u64>()
        }
    }
    
    public fun get_collection_uri(arg0: address, arg1: 0x1::string::String) : 0x1::string::String acquires Collections {
        assert_collection_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg0).collection_data;
        0x1::table::borrow_mut<0x1::string::String, CollectionData>(v0, arg1).uri
    }
    
    public fun get_direct_transfer(arg0: address) : bool acquires TokenStore {
        if (!exists<TokenStore>(arg0)) {
            return false
        };
        borrow_global<TokenStore>(arg0).direct_transfer
    }
    
    public fun get_property_map(arg0: address, arg1: TokenId) : 0x1337::property_map::PropertyMap acquires Collections, TokenStore {
        let v0 = balance_of(arg0, arg1);
        assert!(v0 > 0, 0x1::error::not_found(5));
        if (arg1.property_version == 0) {
            let v2 = &borrow_global<Collections>(arg1.token_data_id.creator).token_data;
            let v3 = 0x1::table::contains<TokenDataId, TokenData>(v2, arg1.token_data_id);
            assert!(v3, 0x1::error::not_found(10));
            0x1::table::borrow<TokenDataId, TokenData>(v2, arg1.token_data_id).default_properties
        } else {
            0x1::table::borrow<TokenId, Token>(&borrow_global<TokenStore>(arg0).tokens, arg1).token_properties
        }
    }
    
    public fun get_royalty(arg0: TokenId) : Royalty acquires Collections {
        get_tokendata_royalty(arg0.token_data_id)
    }
    
    public fun get_royalty_denominator(arg0: &Royalty) : u64 {
        arg0.royalty_points_denominator
    }
    
    public fun get_royalty_numerator(arg0: &Royalty) : u64 {
        arg0.royalty_points_numerator
    }
    
    public fun get_royalty_payee(arg0: &Royalty) : address {
        arg0.payee_address
    }
    
    public fun get_token_amount(arg0: &Token) : u64 {
        arg0.amount
    }
    
    public fun get_token_data_id_fields(arg0: &TokenDataId) : (address, 0x1::string::String, 0x1::string::String) {
        (arg0.creator, arg0.collection, arg0.name)
    }
    
    public fun get_token_id(arg0: &Token) : TokenId {
        arg0.id
    }
    
    public fun get_token_id_fields(arg0: &TokenId) : (address, 0x1::string::String, 0x1::string::String, u64) {
        let v0 = arg0.token_data_id.collection;
        (arg0.token_data_id.creator, v0, arg0.token_data_id.name, arg0.property_version)
    }
    
    public fun get_token_mutability_default_properties(arg0: &TokenMutabilityConfig) : bool {
        arg0.properties
    }
    
    public fun get_token_mutability_description(arg0: &TokenMutabilityConfig) : bool {
        arg0.description
    }
    
    public fun get_token_mutability_maximum(arg0: &TokenMutabilityConfig) : bool {
        arg0.maximum
    }
    
    public fun get_token_mutability_royalty(arg0: &TokenMutabilityConfig) : bool {
        arg0.royalty
    }
    
    public fun get_token_mutability_uri(arg0: &TokenMutabilityConfig) : bool {
        arg0.uri
    }
    
    public fun get_token_supply(arg0: address, arg1: TokenDataId) : 0x1::option::Option<u64> acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v0, arg1), 0x1::error::not_found(10));
        let v1 = 0x1::table::borrow<TokenDataId, TokenData>(v0, arg1);
        if (v1.maximum > 0) {
            0x1::option::some<u64>(v1.supply)
        } else {
            0x1::option::none<u64>()
        }
    }
    
    public fun get_tokendata_description(arg0: TokenDataId) : 0x1::string::String acquires Collections {
        let v0 = arg0.creator;
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &borrow_global<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg0), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v1, arg0).description
    }
    
    public fun get_tokendata_id(arg0: TokenId) : TokenDataId {
        arg0.token_data_id
    }
    
    public fun get_tokendata_largest_property_version(arg0: address, arg1: TokenDataId) : u64 acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v0, arg1), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v0, arg1).largest_property_version
    }
    
    public fun get_tokendata_maximum(arg0: TokenDataId) : u64 acquires Collections {
        let v0 = arg0.creator;
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &borrow_global<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg0), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v1, arg0).maximum
    }
    
    public fun get_tokendata_mutability_config(arg0: TokenDataId) : TokenMutabilityConfig acquires Collections {
        let v0 = arg0.creator;
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &borrow_global<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg0), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v1, arg0).mutability_config
    }
    
    public fun get_tokendata_royalty(arg0: TokenDataId) : Royalty acquires Collections {
        let v0 = arg0.creator;
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &borrow_global<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg0), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v1, arg0).royalty
    }
    
    public fun get_tokendata_uri(arg0: address, arg1: TokenDataId) : 0x1::string::String acquires Collections {
        assert!(exists<Collections>(arg0), 0x1::error::not_found(1));
        let v0 = &borrow_global<Collections>(arg0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v0, arg1), 0x1::error::not_found(10));
        0x1::table::borrow<TokenDataId, TokenData>(v0, arg1).uri
    }
    
    public fun has_token_store(arg0: address) : bool {
        exists<TokenStore>(arg0)
    }
    
    public fun initialize_token(arg0: &signer, arg1: TokenId) {
        abort 0
    }
    
    public entry fun initialize_token_script(arg0: &signer) {
        abort 0
    }
    
    public fun initialize_token_store(arg0: &signer) {
        if (!exists<TokenStore>(0x1::signer::address_of(arg0))) {
            let v0 = 0x1::table::new<TokenId, Token>();
            let v1 = 0x1::account::new_event_handle<DepositEvent>(arg0);
            let v2 = 0x1::account::new_event_handle<WithdrawEvent>(arg0);
            let v3 = 0x1::account::new_event_handle<BurnTokenEvent>(arg0);
            let v4 = 0x1::account::new_event_handle<MutateTokenPropertyMapEvent>(arg0);
            let v5 = TokenStore{
                tokens                       : v0, 
                direct_transfer              : false, 
                deposit_events               : v1, 
                withdraw_events              : v2, 
                burn_events                  : v3, 
                mutate_token_property_events : v4,
            };
            move_to<TokenStore>(arg0, v5);
        };
    }
    
    public fun merge(arg0: &mut Token, arg1: Token) {
        assert!(&arg0.id == &arg1.id, 0x1::error::invalid_argument(6));
        arg0.amount = arg0.amount + arg1.amount;
        let Token {
            id               : _,
            amount           : _,
            token_properties : _,
        } = arg1;
    }
    
    public entry fun mint_script(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64) acquires Collections, TokenStore {
        let v0 = create_token_data_id(arg1, arg2, arg3);
        assert!(v0.creator == 0x1::signer::address_of(arg0), 0x1::error::permission_denied(19));
        mint_token(arg0, v0, arg4);
    }
    
    public fun mint_token(arg0: &signer, arg1: TokenDataId, arg2: u64) : TokenId acquires Collections, TokenStore {
        assert!(arg1.creator == 0x1::signer::address_of(arg0), 0x1::error::permission_denied(19));
        let v0 = arg1.creator;
        let v1 = &mut borrow_global_mut<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg1), 0x1::error::not_found(10));
        let v2 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v1, arg1);
        if (v2.maximum > 0) {
            assert!(v2.supply + arg2 <= v2.maximum, 0x1::error::invalid_argument(7));
            v2.supply = v2.supply + arg2;
        };
        let v3 = create_token_id(arg1, 0);
        let v4 = &mut borrow_global_mut<Collections>(v0).mint_token_events;
        let v5 = MintTokenEvent{
            id     : arg1, 
            amount : arg2,
        };
        0x1::event::emit_event<MintTokenEvent>(v4, v5);
        let v6 = Token{
            id               : v3, 
            amount           : arg2, 
            token_properties : 0x1337::property_map::empty(),
        };
        deposit_token(arg0, v6);
        v3
    }
    
    public fun mint_token_to(arg0: &signer, arg1: address, arg2: TokenDataId, arg3: u64) acquires Collections, TokenStore {
        assert!(exists<TokenStore>(arg1), 0x1::error::not_found(11));
        assert!(borrow_global<TokenStore>(arg1).direct_transfer, 0x1::error::permission_denied(16));
        assert!(arg2.creator == 0x1::signer::address_of(arg0), 0x1::error::permission_denied(19));
        let v0 = arg2.creator;
        let v1 = &mut borrow_global_mut<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg2), 0x1::error::not_found(10));
        let v2 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v1, arg2);
        if (v2.maximum > 0) {
            assert!(v2.supply + arg3 <= v2.maximum, 0x1::error::invalid_argument(7));
            v2.supply = v2.supply + arg3;
        };
        let v3 = &mut borrow_global_mut<Collections>(v0).mint_token_events;
        let v4 = MintTokenEvent{
            id     : arg2, 
            amount : arg3,
        };
        0x1::event::emit_event<MintTokenEvent>(v3, v4);
        let v5 = Token{
            id               : create_token_id(arg2, 0), 
            amount           : arg3, 
            token_properties : 0x1337::property_map::empty(),
        };
        direct_deposit(arg1, v5);
    }
    
    public fun mutate_collection_description(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String) acquires Collections {
        let v0 = 0x1::signer::address_of(arg0);
        assert_collection_exists(v0, arg1);
        let v1 = &mut borrow_global_mut<Collections>(v0).collection_data;
        let v2 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(v1, arg1);
        assert!(v2.mutability_config.description, 0x1::error::permission_denied(13));
        let v3 = v2.description;
        0x1337::token_event_store::emit_collection_description_mutate_event(arg0, arg1, v3, arg2);
        v2.description = arg2;
    }
    
    public fun mutate_collection_maximum(arg0: &signer, arg1: 0x1::string::String, arg2: u64) acquires Collections {
        let v0 = 0x1::signer::address_of(arg0);
        assert_collection_exists(v0, arg1);
        let v1 = &mut borrow_global_mut<Collections>(v0).collection_data;
        let v2 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(v1, arg1);
        assert!(v2.maximum != 0 && arg2 != 0, 0x1::error::invalid_argument(36));
        assert!(arg2 >= v2.supply, 0x1::error::invalid_argument(36));
        assert!(v2.mutability_config.maximum, 0x1::error::permission_denied(13));
        0x1337::token_event_store::emit_collection_maximum_mutate_event(arg0, arg1, v2.maximum, arg2);
        v2.maximum = arg2;
    }
    
    public fun mutate_collection_uri(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String) acquires Collections {
        assert!(0x1::string::length(&arg2) <= 512, 0x1::error::invalid_argument(27));
        let v0 = 0x1::signer::address_of(arg0);
        assert_collection_exists(v0, arg1);
        let v1 = &mut borrow_global_mut<Collections>(v0).collection_data;
        let v2 = 0x1::table::borrow_mut<0x1::string::String, CollectionData>(v1, arg1);
        assert!(v2.mutability_config.uri, 0x1::error::permission_denied(13));
        0x1337::token_event_store::emit_collection_uri_mutate_event(arg0, arg1, v2.uri, arg2);
        v2.uri = arg2;
    }
    
    public fun mutate_one_token(arg0: &signer, arg1: address, arg2: TokenId, arg3: vector<0x1::string::String>, arg4: vector<vector<u8>>, arg5: vector<0x1::string::String>) : TokenId acquires Collections, TokenStore {
        let v0 = arg2.token_data_id.creator;
        assert!(0x1::signer::address_of(arg0) == v0, 0x1::error::permission_denied(14));
        assert!(exists<Collections>(v0), 0x1::error::not_found(1));
        let v1 = &mut borrow_global_mut<Collections>(v0).token_data;
        assert!(0x1::table::contains<TokenDataId, TokenData>(v1, arg2.token_data_id), 0x1::error::not_found(10));
        let v2 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v1, arg2.token_data_id);
        if (!v2.mutability_config.properties) {
            let v3 = 0x1::string::utf8(b"TOKEN_PROPERTY_MUTATBLE");
            assert!(0x1337::property_map::contains_key(&v2.default_properties, &v3), 0x1::error::permission_denied(13));
            let v4 = 0x1::string::utf8(b"TOKEN_PROPERTY_MUTATBLE");
            assert!(0x1337::property_map::read_bool(&v2.default_properties, &v4), 0x1::error::permission_denied(13));
        };
        if (arg2.property_version == 0) {
            let v6 = withdraw_with_event_internal(arg1, arg2, 1);
            let v7 = v2.largest_property_version + 1;
            let v8 = create_token_id(arg2.token_data_id, v7);
            let v9 = Token{
                id               : v8, 
                amount           : 1, 
                token_properties : v2.default_properties,
            };
            direct_deposit(arg1, v9);
            update_token_property_internal(arg1, v8, arg3, arg4, arg5);
            let v10 = MutateTokenPropertyMapEvent{
                old_id : arg2, 
                new_id : v8, 
                keys   : arg3, 
                values : arg4, 
                types  : arg5,
            };
            0x1::event::emit_event<MutateTokenPropertyMapEvent>(&mut borrow_global_mut<TokenStore>(arg1).mutate_token_property_events, v10);
            v2.largest_property_version = v7;
            let Token {
                id               : _,
                amount           : _,
                token_properties : _,
            } = v6;
            v8
        } else {
            update_token_property_internal(arg1, arg2, arg3, arg4, arg5);
            let v14 = MutateTokenPropertyMapEvent{
                old_id : arg2, 
                new_id : arg2, 
                keys   : arg3, 
                values : arg4, 
                types  : arg5,
            };
            0x1::event::emit_event<MutateTokenPropertyMapEvent>(&mut borrow_global_mut<TokenStore>(arg1).mutate_token_property_events, v14);
            arg2
        }
    }
    
    public entry fun mutate_token_properties(arg0: &signer, arg1: address, arg2: address, arg3: 0x1::string::String, arg4: 0x1::string::String, arg5: u64, arg6: u64, arg7: vector<0x1::string::String>, arg8: vector<vector<u8>>, arg9: vector<0x1::string::String>) acquires Collections, TokenStore {
        assert!(0x1::signer::address_of(arg0) == arg2, 0x1::error::not_found(14));
        let v0 = 0;
        while (v0 < arg6) {
            mutate_one_token(arg0, arg1, create_token_id_raw(arg2, arg3, arg4, arg5), arg7, arg8, arg9);
            v0 = v0 + 1;
        };
    }
    
    public fun mutate_tokendata_description(arg0: &signer, arg1: TokenDataId, arg2: 0x1::string::String) acquires Collections {
        assert_tokendata_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg1.creator).token_data;
        let v1 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v0, arg1);
        assert!(v1.mutability_config.description, 0x1::error::permission_denied(13));
        let v2 = arg1.collection;
        let v3 = v1.description;
        0x1337::token_event_store::emit_token_descrition_mutate_event(arg0, v2, arg1.name, v3, arg2);
        v1.description = arg2;
    }
    
    public fun mutate_tokendata_maximum(arg0: &signer, arg1: TokenDataId, arg2: u64) acquires Collections {
        assert_tokendata_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg1.creator).token_data;
        let v1 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v0, arg1);
        assert!(v1.maximum != 0 && arg2 != 0, 0x1::error::invalid_argument(36));
        assert!(arg2 >= v1.supply, 0x1::error::invalid_argument(36));
        assert!(v1.mutability_config.maximum, 0x1::error::permission_denied(13));
        let v2 = arg1.collection;
        0x1337::token_event_store::emit_token_maximum_mutate_event(arg0, v2, arg1.name, v1.maximum, arg2);
        v1.maximum = arg2;
    }
    
    public fun mutate_tokendata_property(arg0: &signer, arg1: TokenDataId, arg2: vector<0x1::string::String>, arg3: vector<vector<u8>>, arg4: vector<0x1::string::String>) acquires Collections {
        assert_tokendata_exists(arg0, arg1);
        let v0 = 0x1::vector::length<0x1::string::String>(&arg2);
        assert!(v0 == 0x1::vector::length<vector<u8>>(&arg3), 0x1::error::invalid_state(37));
        assert!(v0 == 0x1::vector::length<0x1::string::String>(&arg4), 0x1::error::invalid_state(37));
        let v1 = &mut borrow_global_mut<Collections>(arg1.creator).token_data;
        let v2 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v1, arg1);
        assert!(v2.mutability_config.properties, 0x1::error::permission_denied(13));
        let v3 = 0;
        let v4 = 0x1::vector::empty<0x1::option::Option<0x1337::property_map::PropertyValue>>();
        let v5 = 0x1::vector::empty<0x1337::property_map::PropertyValue>();
        assert_non_standard_reserved_property(&arg2);
        while (v3 < 0x1::vector::length<0x1::string::String>(&arg2)) {
            let v6 = 0x1::vector::borrow<0x1::string::String>(&arg2, v3);
            let v7 = if (0x1337::property_map::contains_key(&v2.default_properties, v6)) {
                0x1::option::some<0x1337::property_map::PropertyValue>(*0x1337::property_map::borrow(&v2.default_properties, v6))
            } else {
                0x1::option::none<0x1337::property_map::PropertyValue>()
            };
            let v8 = v7;
            0x1::vector::push_back<0x1::option::Option<0x1337::property_map::PropertyValue>>(&mut v4, v8);
            let v9 = *0x1::vector::borrow<0x1::string::String>(&arg4, v3);
            let v10 = 0x1337::property_map::create_property_value_raw(*0x1::vector::borrow<vector<u8>>(&arg3, v3), v9);
            0x1::vector::push_back<0x1337::property_map::PropertyValue>(&mut v5, v10);
            if (0x1::option::is_some<0x1337::property_map::PropertyValue>(&v8)) {
                0x1337::property_map::update_property_value(&mut v2.default_properties, v6, v10);
            } else {
                0x1337::property_map::add(&mut v2.default_properties, *v6, v10);
            };
            v3 = v3 + 1;
        };
        let v11 = arg1.collection;
        0x1337::token_event_store::emit_default_property_mutate_event(arg0, v11, arg1.name, arg2, v4, v5);
    }
    
    public fun mutate_tokendata_royalty(arg0: &signer, arg1: TokenDataId, arg2: Royalty) acquires Collections {
        assert_tokendata_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg1.creator).token_data;
        let v1 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v0, arg1);
        assert!(v1.mutability_config.royalty, 0x1::error::permission_denied(13));
        let v2 = arg1.collection;
        let v3 = arg1.name;
        let v4 = v1.royalty.royalty_points_numerator;
        let v5 = v1.royalty.royalty_points_denominator;
        let v6 = v1.royalty.payee_address;
        let v7 = arg2.royalty_points_numerator;
        let v8 = arg2.royalty_points_denominator;
        let v9 = arg2.payee_address;
        0x1337::token_event_store::emit_token_royalty_mutate_event(arg0, v2, v3, v4, v5, v6, v7, v8, v9);
        v1.royalty = arg2;
    }
    
    public fun mutate_tokendata_uri(arg0: &signer, arg1: TokenDataId, arg2: 0x1::string::String) acquires Collections {
        assert!(0x1::string::length(&arg2) <= 512, 0x1::error::invalid_argument(27));
        assert_tokendata_exists(arg0, arg1);
        let v0 = &mut borrow_global_mut<Collections>(arg1.creator).token_data;
        let v1 = 0x1::table::borrow_mut<TokenDataId, TokenData>(v0, arg1);
        assert!(v1.mutability_config.uri, 0x1::error::permission_denied(13));
        let v2 = arg1.collection;
        0x1337::token_event_store::emit_token_uri_mutate_event(arg0, v2, arg1.name, v1.uri, arg2);
        v1.uri = arg2;
    }
    
    public entry fun opt_in_direct_transfer(arg0: &signer, arg1: bool) acquires TokenStore {
        initialize_token_store(arg0);
        borrow_global_mut<TokenStore>(0x1::signer::address_of(arg0)).direct_transfer = arg1;
        0x1337::token_event_store::emit_token_opt_in_event(arg0, arg1);
    }
    
    public fun partial_withdraw_with_capability(arg0: WithdrawCapability, arg1: u64) : (Token, 0x1::option::Option<WithdrawCapability>) acquires TokenStore {
        assert!(0x1::timestamp::now_seconds() <= arg0.expiration_sec, 0x1::error::invalid_argument(39));
        assert!(arg1 <= arg0.amount, 0x1::error::invalid_argument(38));
        let v0 = if (arg1 == arg0.amount) {
            0x1::option::none<WithdrawCapability>()
        } else {
            let v1 = arg0.token_owner;
            let v2 = arg0.amount - arg1;
            let v3 = arg0.expiration_sec;
            let v4 = WithdrawCapability{
                token_owner    : v1, 
                token_id       : arg0.token_id, 
                amount         : v2, 
                expiration_sec : v3,
            };
            0x1::option::some<WithdrawCapability>(v4)
        };
        let v5 = withdraw_with_event_internal(arg0.token_owner, arg0.token_id, arg1);
        (v5, v0)
    }
    
    public fun split(arg0: &mut Token, arg1: u64) : Token {
        assert!(arg0.id.property_version == 0, 0x1::error::invalid_state(18));
        assert!(arg0.amount > arg1, 0x1::error::invalid_argument(12));
        assert!(arg1 > 0, 0x1::error::invalid_argument(33));
        arg0.amount = arg0.amount - arg1;
        Token{
            id               : arg0.id, 
            amount           : arg1, 
            token_properties : 0x1337::property_map::empty(),
        }
    }
    
    public fun token_id(arg0: &Token) : &TokenId {
        &arg0.id
    }
    
    public fun transfer(arg0: &signer, arg1: TokenId, arg2: address, arg3: u64) acquires TokenStore {
        assert!(borrow_global<TokenStore>(arg2).direct_transfer, 0x1::error::permission_denied(16));
        let v0 = withdraw_token(arg0, arg1, arg3);
        direct_deposit(arg2, v0);
    }
    
    public entry fun transfer_with_opt_in(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: address, arg6: u64) acquires TokenStore {
        transfer(arg0, create_token_id_raw(arg1, arg2, arg3, arg4), arg5, arg6);
    }
    
    fun update_token_property_internal(arg0: address, arg1: TokenId, arg2: vector<0x1::string::String>, arg3: vector<vector<u8>>, arg4: vector<0x1::string::String>) acquires TokenStore {
        let v0 = &mut borrow_global_mut<TokenStore>(arg0).tokens;
        assert!(0x1::table::contains<TokenId, Token>(v0, arg1), 0x1::error::not_found(15));
        let v1 = &mut 0x1::table::borrow_mut<TokenId, Token>(v0, arg1).token_properties;
        assert_non_standard_reserved_property(&arg2);
        0x1337::property_map::update_property_map(v1, arg2, arg3, arg4);
    }
    
    public fun withdraw_token(arg0: &signer, arg1: TokenId, arg2: u64) : Token acquires TokenStore {
        withdraw_with_event_internal(0x1::signer::address_of(arg0), arg1, arg2)
    }
    
    public fun withdraw_with_capability(arg0: WithdrawCapability) : Token acquires TokenStore {
        assert!(0x1::timestamp::now_seconds() <= arg0.expiration_sec, 0x1::error::invalid_argument(39));
        withdraw_with_event_internal(arg0.token_owner, arg0.token_id, arg0.amount)
    }
    
    fun withdraw_with_event_internal(arg0: address, arg1: TokenId, arg2: u64) : Token acquires TokenStore {
        assert!(arg2 > 0, 0x1::error::invalid_argument(17));
        let v0 = balance_of(arg0, arg1);
        assert!(v0 >= arg2, 0x1::error::invalid_argument(5));
        assert!(exists<TokenStore>(arg0), 0x1::error::not_found(11));
        let v1 = WithdrawEvent{
            id     : arg1, 
            amount : arg2,
        };
        0x1::event::emit_event<WithdrawEvent>(&mut borrow_global_mut<TokenStore>(arg0).withdraw_events, v1);
        let v2 = &mut borrow_global_mut<TokenStore>(arg0).tokens;
        assert!(0x1::table::contains<TokenId, Token>(v2, arg1), 0x1::error::not_found(15));
        let v3 = &mut 0x1::table::borrow_mut<TokenId, Token>(v2, arg1).amount;
        if (*v3 > arg2) {
            *v3 = *v3 - arg2;
            Token{id: arg1, amount: arg2, token_properties: 0x1337::property_map::empty()}
        } else {
            0x1::table::remove<TokenId, Token>(v2, arg1)
        }
    }
    
    // decompiled from Move bytecode v6
}
