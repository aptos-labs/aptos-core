module upgrade_tutorial::message_store {
    use std::error;
    use std::signer;
    use std::string;
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::event;

//:!:>resource
    struct MessageHolder has key {
        message: string::String,
        message_change_events: event::EventHandle<MessageChangeEvent>,
    }
//<:!:resource

    struct MessageChangeEvent has drop, store {
        from_message: string::String,
        to_message: string::String,
    }

    /// There is no message present
    const ENO_MESSAGE: u64 = 0;

    #[view]
    public fun get_message(addr: address): string::String acquires MessageHolder {
        assert!(exists<MessageHolder>(addr), error::not_found(ENO_MESSAGE));
        borrow_global<MessageHolder>(addr).message
    }

    public entry fun set_message(account: signer, message: string::String)
    acquires MessageHolder {
        let account_addr = signer::address_of(&account);
        let trimmed_message = trim_messsage(message);
        if (!exists<MessageHolder>(account_addr)) {
            move_to(&account, MessageHolder {
                message: trimmed_message,
                message_change_events: account::new_event_handle<MessageChangeEvent>(&account),
            })
        } else {
            let old_message_holder = borrow_global_mut<MessageHolder>(account_addr);
            let from_message = old_message_holder.message;
            event::emit_event(&mut old_message_holder.message_change_events, MessageChangeEvent {
                from_message,
                to_message: copy trimmed_message,
            });
            old_message_holder.message = trimmed_message;
        }
    }

    fun trim_messsage(message: string::String): string::String {
        let message_bytes = string::bytes(&message);
        let trimmed_message = vector::empty<u8>();
        let counter = 0;
        while (counter < 8) {
            vector::push_back<u8>(&mut trimmed_message, *vector::borrow(message_bytes, counter));
            counter = counter + 1;
        };
        string::utf8(trimmed_message)
    }

    #[test(account = @0x1)]
    public entry fun sender_can_set_message(account: signer) acquires MessageHolder {
        let addr = signer::address_of(&account);
        aptos_framework::account::create_account_for_test(addr);
        set_message(account,  string::utf8(b"Hello, Blockchain"));

        assert!(
          get_message(addr) == string::utf8(b"Hello, Blockchain"),
          ENO_MESSAGE
        );
    }
}
