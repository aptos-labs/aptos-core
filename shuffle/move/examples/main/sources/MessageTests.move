#[test_only]
module Sender::MessageTests {
    use Sender::Message;
    use Std::Signer;
    use Std::UnitTest;
    use Std::Vector;
    use Std::ASCII;

    fun get_account(): signer {
        Vector::pop_back(&mut UnitTest::create_signers_for_testing(1))
    }

    #[test]
    public(script) fun sender_can_set_message() {
        let account = get_account();
        let addr = Signer::address_of(&account);
        Message::set_message(account,  b"Hello Blockchain");

        assert!(
          Message::get_message(addr) == ASCII::string(b"Hello Blockchain"),
          0
        );
    }
}
