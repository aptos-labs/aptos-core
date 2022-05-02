module DebugDemo::Message {
    use Std::ASCII;
    use Std::Signer;
    use Std::Debug;

    struct MessageHolder has key {
        message: ASCII::String,
    }


    public(script) fun set_message(account: signer, message_bytes: vector<u8>)
    acquires MessageHolder {
        Debug::print_stack_trace();
        let message = ASCII::string(message_bytes);
        let account_addr = Signer::address_of(&account);
        if (!exists<MessageHolder>(account_addr)) {
            move_to(&account, MessageHolder {
                message,
            })
        } else {
            let old_message_holder = borrow_global_mut<MessageHolder>(account_addr);
            old_message_holder.message = message;
        }
    }

    #[test(account = @0x1)]
    public(script) fun sender_can_set_message(account: signer) acquires MessageHolder {
        let addr = Signer::address_of(&account);
        Debug::print<address>(&addr);
        set_message(account,  b"Hello, Blockchain");
    }
}
