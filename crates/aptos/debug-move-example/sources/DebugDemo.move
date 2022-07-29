module DebugDemo::Message {
    use std::string;
    use std::signer;
    use aptos_std::debug;

    struct MessageHolder has key {
        message: string::String,
    }


    public entry fun set_message(account: signer, message_bytes: vector<u8>)
    acquires MessageHolder {
        debug::print_stack_trace();
        let message = string::utf8(message_bytes);
        let account_addr = signer::address_of(&account);
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
    public entry fun sender_can_set_message(account: signer) acquires MessageHolder {
        let addr = signer::address_of(&account);
        debug::print<address>(&addr);
        set_message(account,  b"Hello, Blockchain");
    }
}
