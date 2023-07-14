#[test_only]
module vote_lockup::test_helpers {
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use vote_lockup::epoch;
    use vote_lockup::package_manager;
    use vote_lockup::vote;

    public fun set_up() {
        timestamp::set_time_has_started_for_testing(&account::create_signer_for_test(@0x1));
        epoch::fast_forward(100);
        package_manager::initialize_for_test(&account::create_signer_for_test(@0xcafe));
        vote::initialize();
    }
}
