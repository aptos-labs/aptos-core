//# publish
module 0x1337::reproduce {
    use std::option::{Self, Option};
    #[test_only]
    use velor_framework::account;

    const ESpaceAlreadyMarked: u64 = 0;

    public entry fun init(_account: &signer) {
        let space: Option<u8> = option::none();
        check_if_space_is_open(&mut space);
    }

    inline fun check_if_space_is_open(space: &Option<u8>) {
        // TODO: Ensure given space is not already marked. If it is, abort with code:
        //          ESpaceAlreadyMarked
        assert!(
            option::is_none(space),
            ESpaceAlreadyMarked
        );
    }

    #[test]
    fun test() {
        let account = account::create_account_for_test(@0x1337);
        init(&account);
    }
}
