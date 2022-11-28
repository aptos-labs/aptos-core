module aptos_token::token_event_utils {
    use std::string::{utf8, String};
    use std::signer;
    use aptos_token::property_map::{Self, PropertyMap};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use aptos_token::token::{Self, TokenDataId, Royalty, TokenId, CollectionMutabilityConfig};
    use std::option::Option;
    use aptos_std::any::Any;
    use std::option;

    friend aptos_token::token;

    const TOKEN_CREATOR: vector<u8> = b"creator";
    const TOKEN_COLLECTION: vector<u8> = b"collection";
    const TOKEN_NAME: vector<u8> = b"token_name";

    //
    // Token opt_in direct transfer
    //
    const TOEKN_OPT_IN_EVENT_NAME: vector<u8> = b"token_transfer_opt_in";
    const TOEKN_OPT_IN_EVENT_IN: vector<u8> = b"opt_in";

    //
    // Token Mutation Events
    //

    // Token URI mutation event
    const TOKEN_URI_MUTATION_EVENT_NAME: vector<u8> = b"token_uri_mutation";
    const TOKEN_URI_MUTATION_EVENT_NEW_URI: vector<u8> = b"new_uri";

    // TokenData property mutation event
    const TOKENDATA_PROPERTY_MUTATION_EVENT_NAME: vector<u8> = b"tokendata_propertymap_mutation";
    const TOKENDATA_PROPERTY_MUTATION_EVENT_NEW_PROPERTYMAP: vector<u8> = b"new_property_map";

    //
    // Collection mutation events
    //

    /// Event emitted when collection description is mutated
    struct CollectionDescriptionMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        /// new description
        description: String,
    }

    /// Event emitted when collection uri is mutated
    struct CollectionUriMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        /// new uri
        uri: String,
    }

    /// Event emitted when the collection maximum is mutated
    struct CollectionMaxiumMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        /// new maximum
        maximum: u64,
    }

    //
    // Token transfer related events
    //

    /// Event emitted when an user opt-in the direct transfer
    struct OptInTransferEvent has drop, store {
        /// True if the user opt in, false if the user opt-out
        opt_in: bool
    }

    //
    // Token mutation events
    //

    /// Event emitted when the tokendata uri mutates
    struct UriMutationEvent has drop, store {
        /// TokenData Id that has its uri mutated
        token_data_id: TokenDataId,
        /// new URI
        new_uri: String,
    }

    /// Event emitted when mutating the default the token properties stored at tokendata
    struct DefaultPropertyMutateEvent has drop, store {
        token_data_id: TokenDataId,
        new_keys: vector<String>,
        new_values: vector<vector<u8>>,
        new_types: vector<String>,
    }

    /// Event emitted when mutating the token properties stored at each token
    struct TokenPropertyMutateEvent has drop, store {
        token_id: TokenId,
        new_keys: vector<String>,
        new_values: vector<vector<u8>>,
        new_types: vector<String>,
    }

    /// Event emitted when the tokendata description is mutated
    struct DescriptionMutateEvent has drop, store {
        token_data_id: TokenDataId,
        description: String,
    }

    /// Event emitted when the token royalty is mutated
    struct RoyaltyMutateEvent has drop, store {
        token_data_id: TokenDataId,
        royalty: Royalty,
    }

    /// Event emitted when the token maximum is mutated
    struct MaxiumMutateEvent has drop, store {
        token_data_id: TokenDataId,
        maximum: u64,
    }

    struct TokenEventStore has key {
        /// collection mutation events
        collection_uri_mutate_events: EventHandle<CollectionUriMutateEvent>,
        collection_maximum_mutate_events: EventHandle<CollectionMaxiumMutateEvent>,
        collection_description_mutate_events: EventHandle<CollectionDescriptionMutateEvent>,
        /// token transfer opt-in event
        opt_in_events: EventHandle<OptInTransferEvent>,
        /// token mutation events
        uri_mutate_events: EventHandle<UriMutationEvent>,
        default_property_mutate_events: EventHandle<DefaultPropertyMutateEvent>,
        token_property_mutate_events: EventHandle<TokenPropertyMutateEvent>,
        description_mutate_events: EventHandle<DescriptionMutateEvent>,
        royalty_mutate_events: EventHandle<RoyaltyMutateEvent>,
        maximum_mutate_events: EventHandle<MaxiumMutateEvent>,
        /// This is for adding new events in future
        extention: Option<Any>,
    }

    fun initialize_token_event_store(acct: &signer){
        if (!exists<TokenEventStore>(signer::address_of(acct))) {
            move_to(acct, TokenEventStore {
                collection_uri_mutate_events: account::new_event_handle<CollectionUriMutateEvent>(acct),
                collection_maximum_mutate_events: account::new_event_handle<CollectionMaxiumMutateEvent>(acct),
                collection_description_mutate_events: account::new_event_handle<CollectionDescriptionMutateEvent>(acct),
                opt_in_events: account::new_event_handle<OptInTransferEvent>(acct),
                uri_mutate_events: account::new_event_handle<UriMutationEvent>(acct),
                default_property_mutate_events: account::new_event_handle<DefaultPropertyMutateEvent>(acct),
                token_property_mutate_events: account::new_event_handle<TokenPropertyMutateEvent>(acct),
                description_mutate_events: account::new_event_handle<DescriptionMutateEvent>(acct),
                royalty_mutate_events: account::new_event_handle<RoyaltyMutateEvent>(acct),
                maximum_mutate_events: account::new_event_handle<MaxiumMutateEvent>(acct),
                extention: option::none<Any>(),
            });
        };
    }

    /// Emit the collection uri mutation event
    public(friend) fun emit_collection_uri_mutate_event(creator: &signer, collection: String, uri: String) acquires TokenEventStore {
        let event = CollectionUriMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            uri,
        };
        initialize_token_event_store(creator);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(creator));
        event::emit_event<CollectionUriMutateEvent>(
            &mut token_event_store.collection_uri_mutate_events,
            event,
        );
    }

    /// Emit the collection description mutation event
    public(friend) fun emit_collection_description_mutate_event(creator: &signer, collection: String, description: String) acquires TokenEventStore {
        let event = CollectionDescriptionMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            description,
        };
        initialize_token_event_store(creator);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(creator));
        event::emit_event<CollectionDescriptionMutateEvent>(
            &mut token_event_store.collection_description_mutate_events,
            event,
        );
    }

    /// Emit the collection maximum mutation event
    public(friend) fun emit_collection_maximum_mutate_event(creator: &signer, collection: String, maximum: u64) acquires TokenEventStore {
        let event = CollectionMaxiumMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            maximum,
        };
        initialize_token_event_store(creator);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(creator));
        event::emit_event<CollectionMaxiumMutateEvent>(
            &mut token_event_store.collection_maximum_mutate_events,
            event,
        );
    }

    /// Emit the direct opt-in event
    public(friend) fun emit_token_opt_in_event(account: &signer, opt_in: bool) acquires TokenEventStore {
        let opt_in_event = OptInTransferEvent {
          opt_in,
        };
        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<OptInTransferEvent>(
            &mut token_event_store.opt_in_events,
            opt_in_event,
        );
    }

    /// Emit URI mutation event
    public(friend) fun emit_token_uri_mutate_event(
        creator: &signer,
        new_uri: String,
        collection: String,
        token_name: String,
    ) acquires TokenEventStore {
        let event = UriMutationEvent {
            /// TokenData Id that has its uri mutated
            token_data_id: token::create_token_data_id(signer::address_of(creator), collection, token_name),
            /// new URI
            new_uri,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<UriMutationEvent>(
            &mut token_event_store.uri_mutate_events,
            event,
        );
    }

    /// Emit tokendata property map mutation event
    public(friend) fun emit_default_property_mutate_event(
        creator: &signer,
        collection: String,
        token_name: String,
        new_keys: vector<String>,
        new_values: vector<vector<u8>>,
        new_types: vector<String>,
    ) acquires TokenEventStore {
        let event = DefaultPropertyMutateEvent {
            /// TokenData Id that has its uri mutated
            token_data_id: token::create_token_data_id(signer::address_of(creator), collection, token_name),
            new_keys,
            new_values,
            new_types,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<DefaultPropertyMutateEvent>(
            &mut token_event_store.default_property_mutate_events,
            event,
        );
    }
    /// Emit tokendata property map mutation event
    public(friend) fun emit_token_property_mutate_event(
        creator: &signer,
        collection: String,
        token_name: String,
        property_version: u64,
        new_keys: vector<String>,
        new_values: vector<vector<u8>>,
        new_types: vector<String>,
    ) acquires TokenEventStore {
        let event = TokenPropertyMutateEvent {
            token_id: token::create_token_id_raw(signer::address_of(creator), collection, token_name, property_version),
            new_keys,
            new_values,
            new_types,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<TokenPropertyMutateEvent>(
            &mut token_event_store.token_property_mutate_events,
            event,
        );
    }

    /// Emit description mutation event
    public(friend) fun emit_token_descrition_mutate_event(
        creator: &signer,
        collection: String,
        token_name: String,
        description: String,
    ) acquires TokenEventStore {
        let event = DescriptionMutateEvent {
            token_data_id: token::create_token_data_id(signer::address_of(creator), collection, token_name),
            description,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<DescriptionMutateEvent>(
            &mut token_event_store.description_mutate_events,
            event,
        );
    }

    /// Emit royalty mutation event
    public(friend) fun emit_token_royalty_mutate_event(
        creator: &signer,
        collection: String,
        token_name: String,
        royalty: Royalty,
    ) acquires TokenEventStore {
        let event = RoyaltyMutateEvent {
            token_data_id: token::create_token_data_id(signer::address_of(creator), collection, token_name),
            royalty,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<RoyaltyMutateEvent>(
            &mut token_event_store.royalty_mutate_events,
            event,
        );
    }

    /// Emit maximum mutation event
    public(friend) fun emit_token_maximum_mutate_event(
        creator: &signer,
        collection: String,
        token_name: String,
        maximum: Royalty,
    ) acquires TokenEventStore {
        let event = MaxiumMutateEvent {
            token_data_id: token::create_token_data_id(signer::address_of(creator), collection, token_name),
            maximum,
        };

        initialize_token_event_store(account);
        let token_event_store =  borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<MaxiumMutateEvent>(
            &mut token_event_store.maximum_mutate_events,
            event,
        );
    }
}
