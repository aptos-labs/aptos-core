module 0x1337::token_event_store {
    struct CollectionDescriptionMutateEvent has drop, store {
        creator_addr: address,
        collection_name: 0x1::string::String,
        old_description: 0x1::string::String,
        new_description: 0x1::string::String,
    }
    
    struct CollectionMaxiumMutateEvent has drop, store {
        creator_addr: address,
        collection_name: 0x1::string::String,
        old_maximum: u64,
        new_maximum: u64,
    }
    
    struct CollectionUriMutateEvent has drop, store {
        creator_addr: address,
        collection_name: 0x1::string::String,
        old_uri: 0x1::string::String,
        new_uri: 0x1::string::String,
    }
    
    struct DefaultPropertyMutateEvent has drop, store {
        creator: address,
        collection: 0x1::string::String,
        token: 0x1::string::String,
        keys: vector<0x1::string::String>,
        old_values: vector<0x1::option::Option<0x1337::property_map::PropertyValue>>,
        new_values: vector<0x1337::property_map::PropertyValue>,
    }
    
    struct DescriptionMutateEvent has drop, store {
        creator: address,
        collection: 0x1::string::String,
        token: 0x1::string::String,
        old_description: 0x1::string::String,
        new_description: 0x1::string::String,
    }
    
    struct MaxiumMutateEvent has drop, store {
        creator: address,
        collection: 0x1::string::String,
        token: 0x1::string::String,
        old_maximum: u64,
        new_maximum: u64,
    }
    
    struct OptInTransferEvent has drop, store {
        opt_in: bool,
    }
    
    struct RoyaltyMutateEvent has drop, store {
        creator: address,
        collection: 0x1::string::String,
        token: 0x1::string::String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: address,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: address,
    }
    
    struct TokenEventStoreV1 has key {
        collection_uri_mutate_events: 0x1::event::EventHandle<CollectionUriMutateEvent>,
        collection_maximum_mutate_events: 0x1::event::EventHandle<CollectionMaxiumMutateEvent>,
        collection_description_mutate_events: 0x1::event::EventHandle<CollectionDescriptionMutateEvent>,
        opt_in_events: 0x1::event::EventHandle<OptInTransferEvent>,
        uri_mutate_events: 0x1::event::EventHandle<UriMutationEvent>,
        default_property_mutate_events: 0x1::event::EventHandle<DefaultPropertyMutateEvent>,
        description_mutate_events: 0x1::event::EventHandle<DescriptionMutateEvent>,
        royalty_mutate_events: 0x1::event::EventHandle<RoyaltyMutateEvent>,
        maximum_mutate_events: 0x1::event::EventHandle<MaxiumMutateEvent>,
        extension: 0x1::option::Option<0x1::any::Any>,
    }
    
    struct UriMutationEvent has drop, store {
        creator: address,
        collection: 0x1::string::String,
        token: 0x1::string::String,
        old_uri: 0x1::string::String,
        new_uri: 0x1::string::String,
    }
    
    public(friend) fun emit_collection_description_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = CollectionDescriptionMutateEvent{
            creator_addr    : v0, 
            collection_name : arg1, 
            old_description : arg2, 
            new_description : arg3,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(0x1::signer::address_of(arg0)).collection_description_mutate_events;
        0x1::event::emit_event<CollectionDescriptionMutateEvent>(v2, v1);
    }
    
    public(friend) fun emit_collection_maximum_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: u64, arg3: u64) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = CollectionMaxiumMutateEvent{
            creator_addr    : v0, 
            collection_name : arg1, 
            old_maximum     : arg2, 
            new_maximum     : arg3,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(0x1::signer::address_of(arg0)).collection_maximum_mutate_events;
        0x1::event::emit_event<CollectionMaxiumMutateEvent>(v2, v1);
    }
    
    public(friend) fun emit_collection_uri_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = CollectionUriMutateEvent{
            creator_addr    : v0, 
            collection_name : arg1, 
            old_uri         : arg2, 
            new_uri         : arg3,
        };
        initialize_token_event_store(arg0);
        let v2 = borrow_global_mut<TokenEventStoreV1>(0x1::signer::address_of(arg0));
        0x1::event::emit_event<CollectionUriMutateEvent>(&mut v2.collection_uri_mutate_events, v1);
    }
    
    public(friend) fun emit_default_property_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: vector<0x1::string::String>, arg4: vector<0x1::option::Option<0x1337::property_map::PropertyValue>>, arg5: vector<0x1337::property_map::PropertyValue>) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = DefaultPropertyMutateEvent{
            creator    : v0, 
            collection : arg1, 
            token      : arg2, 
            keys       : arg3, 
            old_values : arg4, 
            new_values : arg5,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(v0).default_property_mutate_events;
        0x1::event::emit_event<DefaultPropertyMutateEvent>(v2, v1);
    }
    
    public(friend) fun emit_token_descrition_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: 0x1::string::String) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = DescriptionMutateEvent{
            creator         : v0, 
            collection      : arg1, 
            token           : arg2, 
            old_description : arg3, 
            new_description : arg4,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(v0).description_mutate_events;
        0x1::event::emit_event<DescriptionMutateEvent>(v2, v1);
    }
    
    public(friend) fun emit_token_maximum_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u64, arg4: u64) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = MaxiumMutateEvent{
            creator     : v0, 
            collection  : arg1, 
            token       : arg2, 
            old_maximum : arg3, 
            new_maximum : arg4,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(v0).maximum_mutate_events;
        0x1::event::emit_event<MaxiumMutateEvent>(v2, v1);
    }
    
    public(friend) fun emit_token_opt_in_event(arg0: &signer, arg1: bool) acquires TokenEventStoreV1 {
        let v0 = OptInTransferEvent{opt_in: arg1};
        initialize_token_event_store(arg0);
        let v1 = &mut borrow_global_mut<TokenEventStoreV1>(0x1::signer::address_of(arg0)).opt_in_events;
        0x1::event::emit_event<OptInTransferEvent>(v1, v0);
    }
    
    public(friend) fun emit_token_royalty_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u64, arg4: u64, arg5: address, arg6: u64, arg7: u64, arg8: address) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = RoyaltyMutateEvent{
            creator                 : v0, 
            collection              : arg1, 
            token                   : arg2, 
            old_royalty_numerator   : arg3, 
            old_royalty_denominator : arg4, 
            old_royalty_payee_addr  : arg5, 
            new_royalty_numerator   : arg6, 
            new_royalty_denominator : arg7, 
            new_royalty_payee_addr  : arg8,
        };
        initialize_token_event_store(arg0);
        0x1::event::emit_event<RoyaltyMutateEvent>(&mut borrow_global_mut<TokenEventStoreV1>(v0).royalty_mutate_events, v1);
    }
    
    public(friend) fun emit_token_uri_mutate_event(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: 0x1::string::String) acquires TokenEventStoreV1 {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = UriMutationEvent{
            creator    : v0, 
            collection : arg1, 
            token      : arg2, 
            old_uri    : arg3, 
            new_uri    : arg4,
        };
        initialize_token_event_store(arg0);
        let v2 = &mut borrow_global_mut<TokenEventStoreV1>(v0).uri_mutate_events;
        0x1::event::emit_event<UriMutationEvent>(v2, v1);
    }
    
    fun initialize_token_event_store(arg0: &signer) {
        if (!exists<TokenEventStoreV1>(0x1::signer::address_of(arg0))) {
            let v0 = 0x1::account::new_event_handle<CollectionUriMutateEvent>(arg0);
            let v1 = 0x1::account::new_event_handle<CollectionMaxiumMutateEvent>(arg0);
            let v2 = 0x1::account::new_event_handle<CollectionDescriptionMutateEvent>(arg0);
            let v3 = 0x1::account::new_event_handle<OptInTransferEvent>(arg0);
            let v4 = 0x1::account::new_event_handle<UriMutationEvent>(arg0);
            let v5 = 0x1::account::new_event_handle<DefaultPropertyMutateEvent>(arg0);
            let v6 = 0x1::account::new_event_handle<DescriptionMutateEvent>(arg0);
            let v7 = 0x1::account::new_event_handle<RoyaltyMutateEvent>(arg0);
            let v8 = 0x1::account::new_event_handle<MaxiumMutateEvent>(arg0);
            let v9 = 0x1::option::none<0x1::any::Any>();
            let v10 = TokenEventStoreV1{
                collection_uri_mutate_events         : v0, 
                collection_maximum_mutate_events     : v1, 
                collection_description_mutate_events : v2, 
                opt_in_events                        : v3, 
                uri_mutate_events                    : v4, 
                default_property_mutate_events       : v5, 
                description_mutate_events            : v6, 
                royalty_mutate_events                : v7, 
                maximum_mutate_events                : v8, 
                extension                            : v9,
            };
            move_to<TokenEventStoreV1>(arg0, v10);
        };
    }
    
    // decompiled from Move bytecode v6
}
