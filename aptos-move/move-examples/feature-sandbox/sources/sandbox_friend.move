module feature_sandbox::sandbox_friend {

    use std::string::String;
    use feature_sandbox::sandbox_messaging;

     #[view]
     // View functions should be able to call friends
     fun call_friend(account: address): (u64, String) {
        sandbox_messaging::get_message_and_revision(account)
    }

    /// Does nothing, just allows for testing friend entry functions
    public(friend) entry fun friend_entry() {
        // This function, actually does nothing, just for testing
    }
}