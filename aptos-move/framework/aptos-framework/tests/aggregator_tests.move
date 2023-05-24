#[test_only]
module aptos_framework::aggregator_tests {

    use aptos_framework::aggregator;
    use aptos_framework::aggregator_factory;

    #[test(account = @aptos_framework)]
    fun test_can_add_and_sub_and_read(account: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&account);
        let aggregator = aggregator_factory::create_aggregator(&account, 1000);

        aggregator::add(&mut aggregator, 12);
        assert!(aggregator::read(&aggregator) == 12, 0);

        aggregator::add(&mut aggregator, 3);
        assert!(aggregator::read(&aggregator) == 15, 0);

        aggregator::add(&mut aggregator, 3);
        aggregator::add(&mut aggregator, 2);
        aggregator::sub(&mut aggregator, 20);
        assert!(aggregator::read(&aggregator) == 0, 0);

        aggregator::add(&mut aggregator, 1000);
        aggregator::sub(&mut aggregator, 1000);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    #[expected_failure(abort_code = 0x020001, location = aptos_framework::aggregator)]
    fun test_overflow(account: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&account);
        let aggregator = aggregator_factory::create_aggregator(&account, 10);

        // Overflow!
        aggregator::add(&mut aggregator, 12);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    fun test_try_overflow(account: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&account);
        let aggregator = aggregator_factory::create_aggregator(&account, 10);
        assert!(aggregator::try_add(&mut aggregator, 5) == true, 0);
        assert!(aggregator::try_sub(&mut aggregator, 3) == true, 1);

        // Overflow!
        assert!(aggregator::try_add(&mut aggregator, 9) == false, 2);

        aggregator::destroy(aggregator);
    }

    #[test(account = @aptos_framework)]
    fun test_underflow(account: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&account);
        let aggregator = aggregator_factory::create_aggregator(&account, 10);
        assert!(aggregator::try_add(&mut aggregator, 5) == true, 0);
        assert!(aggregator::try_sub(&mut aggregator, 3) == true, 1);

        // Underflow!
        assert!(aggregator::try_sub(&mut aggregator, 4) == false, 2);

        aggregator::destroy(aggregator);
    }
}
