#[test_only]
/// Tests can be added in a separate module file as well
///
/// Keep in mind that any private functions cannot be called in a separate module, and those tests will
/// have to be in the same module
module feature_sandbox::sandbox_messaging_tests {
    use std::signer;
    use std::unit_test;
    use std::vector;
    use std::string;

    use feature_sandbox::sandbox_messaging;

    inline fun get_account(): signer {
        vector::pop_back(&mut unit_test::create_signers_for_testing(1))
    }

    #[test]
    fun sender_can_set_message() {
        let account = get_account();
        let addr = signer::address_of(&account);
        aptos_framework::account::create_account_for_test(addr);
        sandbox_messaging::set_message(account,  string::utf8(b"Hello, Blockchain"));

        assert!(
          sandbox_messaging::get_message(addr) == string::utf8(b"Hello, Blockchain"),
          0
        );
    }
}
