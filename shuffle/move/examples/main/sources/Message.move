module Sender::Message {
    use Std::Errors;
    use Std::Event;
    use Std::Signer;

    struct MessageHolder has key {
        message: vector<u8>,
        message_change_events: Event::EventHandle<MessageChangeEvent>,
    }
    struct MessageChangeEvent has drop, store {
        from_message: vector<u8>,
        to_message: vector<u8>,
    }

    /// There is no message present
    const ENO_MESSAGE: u64 = 0;

    public fun get_message(addr: address): vector<u8> acquires MessageHolder {
        assert!(exists<MessageHolder>(addr), Errors::not_published(ENO_MESSAGE));
        *&borrow_global<MessageHolder>(addr).message
    }

    public(script) fun set_message(account: signer, message: vector<u8>)
    acquires MessageHolder {
        let account_addr = Signer::address_of(&account);
        if (!exists<MessageHolder>(account_addr)) {
            move_to(&account, MessageHolder {
                message,
                message_change_events: Event::new_event_handle<MessageChangeEvent>(&account),
            })
        } else {
            let old_message_holder = borrow_global_mut<MessageHolder>(account_addr);
            let from_message = *&old_message_holder.message;
            Event::emit_event(&mut old_message_holder.message_change_events, MessageChangeEvent {
                from_message,
                to_message: copy message,
            });
            old_message_holder.message = message;
        }
    }
}
