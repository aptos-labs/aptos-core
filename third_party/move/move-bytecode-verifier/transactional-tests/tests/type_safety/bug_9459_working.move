//# publish
module 0x1337::reproduce {
    use std::option::{Self, Option};
    #[test_only]
    use velor_framework::account;

    const ESpaceAlreadyMarked: u64 = 0;

    public entry fun init(_account: &signer) {
        let space: Option<u8> = option::none();
        assert!(
            option::is_none(&mut space),
            ESpaceAlreadyMarked
        );
    }

    #[test]
    fun test() {
        let account = account::create_account_for_test(@0x1337);
        init(&account);
    }
}
