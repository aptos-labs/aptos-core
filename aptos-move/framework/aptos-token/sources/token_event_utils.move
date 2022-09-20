module aptos_token::token_event_utils {
    use std::string::{utf8, String};
    use std::signer;
    use aptos_token::property_map::{Self, PropertyMap};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;

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
    // Token URI mutation event
    //
    const TOKEN_URI_MUTATION_EVENT_NAME: vector<u8> = b"token_uri_mutation";
    const TOKEN_URI_MUTATION_EVENT_NEW_URI: vector<u8> = b"new_uri";

    //
    // TokenData property mutation event
    //
    const TOKENDATA_PROPERTY_MUTATION_EVENT_NAME: vector<u8> = b"tokendata_propertymap_mutation";
    const TOKENDATA_PROPERTY_MUTATION_EVENT_NEW_PROPERTYMAP: vector<u8> = b"new_property_map";


    /// General token event used for representing all new token events
   /// The event is identified using event_name. Attributes of the event are stored in a PropertyMap
    struct GeneralTokenEvent has drop, store {
        token_event_name: String,
        event_attributes: PropertyMap,
    }

    struct TokenEventStore has key {
        events: EventHandle<GeneralTokenEvent>,
    }

    fun initialize_token_event_store(acct: &signer){
        if (!exists<TokenEventStore>(signer::address_of(acct))) {
            move_to(acct, TokenEventStore {
                events: account::new_event_handle<GeneralTokenEvent>(acct),
            });
        };
    }

    /// Emit the direct opt-in event
    public(friend) fun emit_token_opt_in_event(account: &signer, opt_in: bool) acquires TokenEventStore {
        let pm = property_map::empty();
        property_map::add(
            &mut pm,
            utf8(TOEKN_OPT_IN_EVENT_IN), property_map::create_property_value<bool>(&opt_in)
        );

        let gte = GeneralTokenEvent {
            token_event_name: utf8(TOEKN_OPT_IN_EVENT_NAME),
            event_attributes: pm,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<GeneralTokenEvent>(
            &mut token_event_store.events,
            gte,
        );
    }

    /// Emit URI mutation event
    public(friend) fun emit_token_uri_mutate_event(
        account: &signer,
        new_uri: String,
        collection: String,
        token_name: String,
    ) acquires TokenEventStore {
        let pm = property_map::empty();
        property_map::add(
            &mut pm,
            utf8(TOKEN_URI_MUTATION_EVENT_NEW_URI), property_map::create_property_value<String>(&new_uri)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_COLLECTION), property_map::create_property_value<String>(&collection)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_NAME), property_map::create_property_value<String>(&token_name)
        );

        let gte = GeneralTokenEvent {
            token_event_name: utf8(TOKEN_URI_MUTATION_EVENT_NAME),
            event_attributes: pm,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<GeneralTokenEvent>(
            &mut token_event_store.events,
            gte,
        );
    }

    /// Emit tokendata property map mutation event
    public(friend) fun emit_tokendata_property_mutate_event(
        account: &signer,
        new_propertymap: PropertyMap,
        creator: address,
        collection: String,
        token_name: String,
    ) acquires TokenEventStore {
        let pm = property_map::empty();
        property_map::add(
            &mut pm,
            utf8(TOKENDATA_PROPERTY_MUTATION_EVENT_NEW_PROPERTYMAP), property_map::create_property_value<PropertyMap>(&new_propertymap)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_CREATOR), property_map::create_property_value<address>(&creator)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_COLLECTION), property_map::create_property_value<String>(&collection)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_NAME), property_map::create_property_value<String>(&token_name)
        );


        let gte = GeneralTokenEvent {
            token_event_name: utf8(TOKENDATA_PROPERTY_MUTATION_EVENT_NAME),
            event_attributes: pm,
        };

        initialize_token_event_store(account);
        let token_event_store = borrow_global_mut<TokenEventStore>(signer::address_of(account));
        event::emit_event<GeneralTokenEvent>(
            &mut token_event_store.events,
            gte,
        );
    }
}