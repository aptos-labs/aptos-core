module hello_blockchain::message {
    use std::error;
    use std::signer;
    use std::string;
    use aptos_framework::event;
    #[test_only]
    use std::debug;

    //
    // Each account that uses this module will store one MessageHolder.
    // This resource permanently lives under the user's account address.
    //
    struct MessageHolder has key {
        message: string::String,
    }

    //
    // This event will be emitted whenever a message is changed.
    // Events allow off-chain systems (indexers / wallets / UIs) to track updates.
    //
    #[event]
    struct MessageChange has drop, store {
        account: address,
        from_message: string::String,
        to_message: string::String,
    }

    // Error code used when trying to read a message that doesn't exist.
    const ENO_MESSAGE: u64 = 0;

    //
    // View function: returns the stored message for an account.
    // Does NOT modify storage.
    //
    #[view]
    public fun get_message(addr: address): string::String acquires MessageHolder {
        // Ensure the user has initialized their message.
        assert!(exists<MessageHolder>(addr), error::not_found(ENO_MESSAGE));

        // Return the current stored message.
        borrow_global<MessageHolder>(addr).message
    }

    //
    // Main function to create or update a message.
    // Called by users via transaction.
    //
    public entry fun set_message(account: signer, message: string::String)
    acquires MessageHolder {

        // Get the address of the signer (sender of transaction).
        let account_addr = signer::address_of(&account);

        // If this is the first time user interacts → create MessageHolder.
        if (!exists<MessageHolder>(account_addr)) {
            move_to(&account, MessageHolder { message })
        } else {
            // Borrow the existing message and update it.
            let old_message_holder = borrow_global_mut<MessageHolder>(account_addr);
            let from_message = old_message_holder.message;

            // Emit event so off-chain systems can detect the update.
            event::emit(MessageChange {
                account: account_addr,
                from_message,
                to_message: copy message,
            });

            // Write the new message.
            old_message_holder.message = message;
        }
    }

    //
    // Unit test that runs only in Move testing environment.
    //
    #[test(account = @0x1)]
    public entry fun sender_can_set_message(account: signer) acquires MessageHolder {
        let msg: string::String = string::utf8(b"Running test for sender_can_set_message...");
        debug::print(&msg);

        let addr = signer::address_of(&account);

        // Create the testing account
        aptos_framework::account::create_account_for_test(addr);

        // Call the entry function
        set_message(account, string::utf8(b"Hello, Blockchain"));

        // Verify the expected result
        assert!(
            get_message(addr) == string::utf8(b"Hello, Blockchain"),
            ENO_MESSAGE
        );
    }
}
