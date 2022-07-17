#[test_only]
module HelloBlockchain::MessageTests {
    use std::signer;
    use std::unit_test;
    use std::vector;
    use std::ascii;

    use HelloBlockchain::Message;

    fun get_account(): signer {
        vector::pop_back(&mut unit_test::create_signers_for_testing(1))
    }

    #[test]
    public entry fun sender_can_set_message() {
        let account = get_account();
        let addr = signer::address_of(&account);
        Message::set_message(account,  b"Hello, Blockchain");

        assert!(
          Message::get_message(addr) == ascii::string(b"Hello, Blockchain"),
          0
        );
    }
}
