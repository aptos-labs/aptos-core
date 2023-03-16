/// An example module similar to the hello blockchain demo
///
/// This module is to show off all features that can be used in Move in a simple way.
///
/// Here is an example of a doc comment.  The doc comments will be used when running the doc generator,
/// and at this location it will document for the module.
module feature_sandbox::sandbox_messaging {
    // Imports can be used as an individual functions and in groups
    use std::error::{Self, not_found, permission_denied};

    // Imports can be used for structs
    use std::string::String;

    // Imports can also be used for modules
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::event;

    // Imports can be test only
    #[test_only]
    use std::string::utf8;

    // Friends can be declared as well
    friend feature_sandbox::sandbox_friend;

    // Errors should start with an E and have their error messages printed in an abort in doc comment form
    /// There is no message holder present at address
    const ENO_MESSAGE_HOLDER: u64 = 1;
    /// The action is an admin action, and the caller is not the admin
    const ENOT_ADMIN: u64 = 2;

    /// Doc comments on structs should work
    struct MessageHolder<phantom T: drop + store> has key {
        /// The message being stored in the holder
        message: String,
        /// An event handle for Aptos events associated with the message
        message_change_events: event::EventHandle<T>,
    }

    /// An event for when the holding message changes
    struct MessageChangeEvent has drop, store {
        /// The message that previously existed in the MessageHolder
        old_message: String,
        /// The message that is replacing the previous message in the MessageHolder
        new_message: String,
    }

    // All annotations must go above doc comments
    // This annotation denotes that the function is a view function.  This means that the function can be called from
    // the view function API on the full node, and provides you a way to read data in Move through the API layer.
    #[view]
    /// Retrieves the message from the struct
    ///
    /// Public view functions should work fine
    public fun get_message(address: address): String acquires MessageHolder {
        assert!(exists<MessageHolder<MessageChangeEvent>>(address), error::not_found(ENO_MESSAGE_HOLDER));
        *&borrow_global<MessageHolder<MessageChangeEvent>>(address).message
    }

    #[view]
    /// Functions can be friend functions.  These are able to be called from other modules that are declared friends.
    ///
    /// Additionally, here's an example of returning a tuple.  Multiple values can be returned at once.
    public(friend) fun get_message_and_revision(account: address): (u64, String) acquires MessageHolder {
        assert!(exists<MessageHolder<MessageChangeEvent>>(account), not_found(ENO_MESSAGE_HOLDER));
        let holder = borrow_global<MessageHolder<MessageChangeEvent>>(account);
        (event::counter(&holder.message_change_events), holder.message)
    }

    #[view]
    /// Checks if the admin account is the given address
    ///
    /// Private functions (without public or friend), can only be called within the same module.
    ///
    /// View functions can also apply to private functions.  View functions must return a value.
    fun check_if_admin(address: address): bool {
        assert!(address == @feature_sandbox, permission_denied(ENOT_ADMIN));
        true
    }

    /// An admin private entry function for setting messages of accounts that have already created message holders
    ///
    /// Private entry functions allow you to create externally callable functions in a transaction that cannot
    /// be called from another module.  Entry functions cannot take structs as arguments, and cannot return values.
    ///
    /// This allows only the admin (the deployer) to override messages in the account
    entry fun set_message_admin(
        account: signer,
        message_address: address,
        message: String
    ) acquires MessageHolder {
        let account_addr = signer::address_of(&account);
        check_if_admin(account_addr);
        assert!(exists<MessageHolder<MessageChangeEvent>>(message_address), not_found(ENO_MESSAGE_HOLDER));
        set_message_inline(message_address, message);
    }

    /// Sets a message for the signer's account
    ///
    /// Public entry functiosn allow you to create externally callable functions that can be called from other modules.
    /// Entry functions cannot take structs as arguments, and cannot return values.
    ///
    /// Only the owner of the account holding the resource can update the message.
    public entry fun set_message(account: signer, message: String)
    acquires MessageHolder {
        let account_addr = signer::address_of(&account);

        // Create the message holder if it doesn't exist yet
        if (!exists<MessageHolder<MessageChangeEvent>>(account_addr)) {
            move_to(&account, MessageHolder {
                message,
                message_change_events: account::new_event_handle<MessageChangeEvent>(&account),
            })
        } else {
            // Otherwise just update the existing message
            set_message_inline(account_addr, message);
        }
    }

    /// Updates an existing message in the message holder
    ///
    /// Inline function doesn't need a reutrn value
    inline fun set_message_inline(address: address, message: String) acquires MessageHolder {
        let old_message_holder = get_message_holder_inline(address);
        let old_message = *&old_message_holder.message;
        event::emit_event(&mut old_message_holder.message_change_events, MessageChangeEvent {
            old_message,
            new_message: copy message,
        });
        old_message_holder.message = message;
    }

    /// Inline functions also can return references
    ///
    /// Acquires is not required
    inline fun get_message_holder_inline(address: address): &mut MessageHolder<MessageChangeEvent> {
         borrow_global_mut<MessageHolder<MessageChangeEvent>>(address)
    }

    #[test(account = @0x1)]
    /// Tests can be in the module, and you can provide test addresses in the test annotation
    ///
    /// Each account in the test annotation must be also specified in the function signature as an argument.
    fun sender_can_set_message(account: signer) acquires MessageHolder {
        let message = b"Hello, Blockchain";

        // We can an account for testing like so
        let addr = signer::address_of(&account);
        aptos_framework::account::create_account_for_test(addr);

        // And then run any functions want
        set_message(account, utf8(message));

        // Including assertions
        assert!(
            get_message(addr) == utf8(message),
            0
        );
    }

    #[test(account = @0x1, admin = @feature_sandbox)]
    /// Additionally, tests can have more than one signer for testing complex traits between multiple accounts.
    fun admin_can_set_message(account: signer, admin: signer) acquires MessageHolder {
        let orig_message = b"Hello, Blockchain";
        let admin_message = b"REDACTED";
        let addr = signer::address_of(&account);
        aptos_framework::account::create_account_for_test(addr);
        set_message(account, utf8(orig_message));

        assert!(
            get_message(addr) == utf8(orig_message),
            0
        );
        set_message_admin(admin, addr, utf8(admin_message));

        assert!(
            get_message(addr) == utf8(admin_message),
            0
        );
    }
}
