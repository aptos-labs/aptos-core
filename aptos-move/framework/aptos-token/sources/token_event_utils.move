module aptos_token::token_event_utils {
    use std::string::{utf8, String};
    use std::signer;
    use aptos_token::property_map::{Self, PropertyMap, PropertyValue};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;

    friend aptos_token::token;

    const TOKEN_TRANSFER_EVENT_NAME: vector<u8> = b"token_transfer";
    const TOKEN_TRANSFER_EVENT_SENDER: vector<u8> = b"sender";
    const TOKEN_TRANSFER_EVENT_RECEIVER: vector<u8> = b"receiver";
    const TOKEN_TRANSFER_EVENT_TOKEN_ID: vector<u8> = b"token_id";
    const TOKEN_TRANSFER_EVENT_TOKEN_AMOUNT: vector<u8> = b"amount";


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

    public(friend) fun emit_token_transfer_event(account: &signer, from: address, to: address, token_id: PropertyValue, amount: u64) acquires TokenEventStore {
        let pm = property_map::empty();
        property_map::add(
            &mut pm,
            utf8(TOKEN_TRANSFER_EVENT_SENDER), property_map::create_property_value<address>(&from)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_TRANSFER_EVENT_RECEIVER), property_map::create_property_value<address>(&to)
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_TRANSFER_EVENT_TOKEN_ID), token_id
        );
        property_map::add(
            &mut pm,
            utf8(TOKEN_TRANSFER_EVENT_TOKEN_AMOUNT), property_map::create_property_value<u64>(&amount)
        );

        let gte = GeneralTokenEvent {
            token_event_name: utf8(TOKEN_TRANSFER_EVENT_NAME),
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