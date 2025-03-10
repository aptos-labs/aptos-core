/// This module provides utils to add and emit new token events that are not in token.move
module aptos_token::token_event_store {
    use std::string::String;
    use std::signer;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use std::option::Option;
    use aptos_std::any::Any;
    use std::option;
    use aptos_token::property_map::PropertyValue;

    friend aptos_token::token;

    //
    // Collection mutation events
    //

    /// Event emitted when collection description is mutated
    struct CollectionDescriptionMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        old_description: String,
        new_description: String,
    }

    #[event]
    /// Event emitted when collection description is mutated
    struct CollectionDescriptionMutate has drop, store {
        creator_addr: address,
        collection_name: String,
        old_description: String,
        new_description: String,
    }

    /// Event emitted when collection uri is mutated
    struct CollectionUriMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        old_uri: String,
        new_uri: String,
    }

    #[event]
    /// Event emitted when collection uri is mutated
    struct CollectionUriMutate has drop, store {
        creator_addr: address,
        collection_name: String,
        old_uri: String,
        new_uri: String,
    }

    /// Event emitted when the collection maximum is mutated
    struct CollectionMaxiumMutateEvent has drop, store {
        creator_addr: address,
        collection_name: String,
        old_maximum: u64,
        new_maximum: u64,
    }

    #[event]
    /// Event emitted when the collection maximum is mutated
    struct CollectionMaximumMutate has drop, store {
        creator_addr: address,
        collection_name: String,
        old_maximum: u64,
        new_maximum: u64,
    }

    //
    // Token transfer related events
    //

    /// Event emitted when an user opt-in the direct transfer
    struct OptInTransferEvent has drop, store {
        /// True if the user opt in, false if the user opt-out
        opt_in: bool
    }

    #[event]
    /// Event emitted when an user opt-in the direct transfer
    struct OptInTransfer has drop, store {
        account_address: address,
        /// True if the user opt in, false if the user opt-out
        opt_in: bool
    }

    //
    // Token mutation events
    //

    /// Event emitted when the tokendata uri mutates
    struct UriMutationEvent has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_uri: String,
        new_uri: String,
    }

    #[event]
    /// Event emitted when the tokendata uri mutates
    struct UriMutation has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_uri: String,
        new_uri: String,
    }

    /// Event emitted when mutating the default the token properties stored at tokendata
    struct DefaultPropertyMutateEvent has drop, store {
        creator: address,
        collection: String,
        token: String,
        keys: vector<String>,
        /// we allow upsert so the old values might be none
        old_values: vector<Option<PropertyValue>>,
        new_values: vector<PropertyValue>,
    }

    #[event]
    /// Event emitted when mutating the default the token properties stored at tokendata
    struct DefaultPropertyMutate has drop, store {
        creator: address,
        collection: String,
        token: String,
        keys: vector<String>,
        /// we allow upsert so the old values might be none
        old_values: vector<Option<PropertyValue>>,
        new_values: vector<PropertyValue>,
    }

    /// Event emitted when the tokendata description is mutated
    struct DescriptionMutateEvent has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_description: String,
        new_description: String,
    }

    #[event]
    /// Event emitted when the tokendata description is mutated
    struct DescriptionMutate has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_description: String,
        new_description: String,
    }

    /// Event emitted when the token royalty is mutated
    struct RoyaltyMutateEvent has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: address,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: address,
    }

    #[event]
    /// Event emitted when the token royalty is mutated
    struct RoyaltyMutate has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: address,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: address,
    }

    /// Event emitted when the token maximum is mutated
    struct MaxiumMutateEvent has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_maximum: u64,
        new_maximum: u64,
    }

    #[event]
    /// Event emitted when the token maximum is mutated
    struct MaximumMutate has drop, store {
        creator: address,
        collection: String,
        token: String,
        old_maximum: u64,
        new_maximum: u64,
    }

    struct TokenEventStoreV1 has key {
        /// collection mutation events
        collection_uri_mutate_events: EventHandle<CollectionUriMutateEvent>,
        collection_maximum_mutate_events: EventHandle<CollectionMaxiumMutateEvent>,
        collection_description_mutate_events: EventHandle<CollectionDescriptionMutateEvent>,
        /// token transfer opt-in event
        opt_in_events: EventHandle<OptInTransferEvent>,
        /// token mutation events
        uri_mutate_events: EventHandle<UriMutationEvent>,
        default_property_mutate_events: EventHandle<DefaultPropertyMutateEvent>,
        description_mutate_events: EventHandle<DescriptionMutateEvent>,
        royalty_mutate_events: EventHandle<RoyaltyMutateEvent>,
        maximum_mutate_events: EventHandle<MaxiumMutateEvent>,
        /// This is for adding new events in future
        extension: Option<Any>,
    }

    fun initialize_token_event_store(acct: &signer){
        if (!exists<TokenEventStoreV1>(signer::address_of(acct))) {
            move_to(acct, TokenEventStoreV1 {
                collection_uri_mutate_events: account::new_event_handle<CollectionUriMutateEvent>(acct),
                collection_maximum_mutate_events: account::new_event_handle<CollectionMaxiumMutateEvent>(acct),
                collection_description_mutate_events: account::new_event_handle<CollectionDescriptionMutateEvent>(acct),
                opt_in_events: account::new_event_handle<OptInTransferEvent>(acct),
                uri_mutate_events: account::new_event_handle<UriMutationEvent>(acct),
                default_property_mutate_events: account::new_event_handle<DefaultPropertyMutateEvent>(acct),
                description_mutate_events: account::new_event_handle<DescriptionMutateEvent>(acct),
                royalty_mutate_events: account::new_event_handle<RoyaltyMutateEvent>(acct),
                maximum_mutate_events: account::new_event_handle<MaxiumMutateEvent>(acct),
                extension: option::none<Any>(),
            });
        };
    }

    /// Emit the collection uri mutation event
    friend fun emit_collection_uri_mutate_event(creator: &signer, collection: String, old_uri: String, new_uri: String) acquires TokenEventStoreV1 {
        let event = CollectionUriMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            old_uri,
            new_uri,
        };
        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[signer::address_of(creator)];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                CollectionUriMutate {
                    creator_addr: signer::address_of(creator),
                    collection_name: collection,
                    old_uri,
                    new_uri,
                }
            );
        } else {
            event::emit_event<CollectionUriMutateEvent>(
                &mut token_event_store.collection_uri_mutate_events,
                event,
            );
        };
    }

    /// Emit the collection description mutation event
    friend fun emit_collection_description_mutate_event(creator: &signer, collection: String, old_description: String, new_description: String) acquires TokenEventStoreV1 {
        let event = CollectionDescriptionMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            old_description,
            new_description,
        };
        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[signer::address_of(creator)];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                CollectionDescriptionMutate {
                    creator_addr: signer::address_of(creator),
                    collection_name: collection,
                    old_description,
                    new_description,
                }
            );
        } else {
            event::emit_event<CollectionDescriptionMutateEvent>(
                &mut token_event_store.collection_description_mutate_events,
                event,
            );
        }
    }

    /// Emit the collection maximum mutation event
    friend fun emit_collection_maximum_mutate_event(creator: &signer, collection: String, old_maximum: u64, new_maximum: u64) acquires TokenEventStoreV1 {
        let event = CollectionMaxiumMutateEvent {
            creator_addr: signer::address_of(creator),
            collection_name: collection,
            old_maximum,
            new_maximum,
        };
        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[signer::address_of(creator)];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                CollectionMaximumMutate {
                    creator_addr: signer::address_of(creator),
                    collection_name: collection,
                    old_maximum,
                    new_maximum,
                }
            );
        } else {
            event::emit_event<CollectionMaxiumMutateEvent>(
                &mut token_event_store.collection_maximum_mutate_events,
                event,
            );
        };
    }

    /// Emit the direct opt-in event
    friend fun emit_token_opt_in_event(account: &signer, opt_in: bool) acquires TokenEventStoreV1 {
        let opt_in_event = OptInTransferEvent {
          opt_in,
        };
        initialize_token_event_store(account);
        let token_event_store = &mut TokenEventStoreV1[signer::address_of(account)];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                OptInTransfer {
                    account_address: signer::address_of(account),
                    opt_in,
                });
        } else {
            event::emit_event<OptInTransferEvent>(
                &mut token_event_store.opt_in_events,
                opt_in_event,
            );
        }
    }

    /// Emit URI mutation event
    friend fun emit_token_uri_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_uri: String,
        new_uri: String,
    ) acquires TokenEventStoreV1 {
        let creator_addr = signer::address_of(creator);

        let event = UriMutationEvent {
            creator: creator_addr,
            collection,
            token,
            old_uri,
            new_uri,
        };

        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[creator_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                UriMutation {
                    creator: creator_addr,
                    collection,
                    token,
                    old_uri,
                    new_uri,
                });
        } else {
            event::emit_event<UriMutationEvent>(
                &mut token_event_store.uri_mutate_events,
                event,
            );
        };
    }

    /// Emit tokendata property map mutation event
    friend fun emit_default_property_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        keys: vector<String>,
        old_values: vector<Option<PropertyValue>>,
        new_values: vector<PropertyValue>,
    ) acquires TokenEventStoreV1 {
        let creator_addr = signer::address_of(creator);

        let event = DefaultPropertyMutateEvent {
            creator: creator_addr,
            collection,
            token,
            keys,
            old_values,
            new_values,
        };

        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[creator_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                DefaultPropertyMutate {
                    creator: creator_addr,
                    collection,
                    token,
                    keys,
                    old_values,
                    new_values,
                });
        } else {
            event::emit_event<DefaultPropertyMutateEvent>(
                &mut token_event_store.default_property_mutate_events,
                event,
            );
        };
    }

    /// Emit description mutation event
    friend fun emit_token_descrition_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_description: String,
        new_description: String,
    ) acquires TokenEventStoreV1 {
        let creator_addr = signer::address_of(creator);

        let event = DescriptionMutateEvent {
            creator: creator_addr,
            collection,
            token,
            old_description,
            new_description,
        };

        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[creator_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                DescriptionMutate {
                    creator: creator_addr,
                    collection,
                    token,
                    old_description,
                    new_description,
                });
        } else {
            event::emit_event<DescriptionMutateEvent>(
                &mut token_event_store.description_mutate_events,
                event,
            );
        };
    }

    /// Emit royalty mutation event
    friend fun emit_token_royalty_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: address,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: address,
    ) acquires TokenEventStoreV1 {
        let creator_addr = signer::address_of(creator);
        let event = RoyaltyMutateEvent {
            creator: creator_addr,
            collection,
            token,
            old_royalty_numerator,
            old_royalty_denominator,
            old_royalty_payee_addr,
            new_royalty_numerator,
            new_royalty_denominator,
            new_royalty_payee_addr,
        };

        initialize_token_event_store(creator);
        let token_event_store = &mut TokenEventStoreV1[creator_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                RoyaltyMutate {
                    creator: creator_addr,
                    collection,
                    token,
                    old_royalty_numerator,
                    old_royalty_denominator,
                    old_royalty_payee_addr,
                    new_royalty_numerator,
                    new_royalty_denominator,
                    new_royalty_payee_addr,
                });
        } else {
            event::emit_event<RoyaltyMutateEvent>(
                &mut token_event_store.royalty_mutate_events,
                event,
            );
        };
    }

    /// Emit maximum mutation event
    friend fun emit_token_maximum_mutate_event(
        creator: &signer,
        collection: String,
        token: String,
        old_maximum: u64,
        new_maximum: u64,
    ) acquires TokenEventStoreV1 {
        let creator_addr = signer::address_of(creator);

        let event = MaxiumMutateEvent {
            creator: creator_addr,
            collection,
            token,
            old_maximum,
            new_maximum,
        };

        initialize_token_event_store(creator);
        let token_event_store =  &mut TokenEventStoreV1[creator_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                MaximumMutate {
                    creator: creator_addr,
                    collection,
                    token,
                    old_maximum,
                    new_maximum,
                });
        } else {
            event::emit_event<MaxiumMutateEvent>(
                &mut token_event_store.maximum_mutate_events,
                event,
            );
        };
    }

    #[deprecated]
    #[event]
    struct CollectionMaxiumMutate has drop, store {
        creator_addr: address,
        collection_name: String,
        old_maximum: u64,
        new_maximum: u64,
    }
}
