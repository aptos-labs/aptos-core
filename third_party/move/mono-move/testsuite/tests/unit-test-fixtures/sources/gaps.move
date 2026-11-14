/// A test MonoMove cannot run: it has no aggregator V1 natives.
module fixtures::gaps {
    use aptos_framework::aggregator;
    use aptos_framework::aggregator_factory;

    #[test(fx = @aptos_framework)]
    fun creates_aggregator_v1(fx: &signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(fx);
        let agg = aggregator_factory::create_aggregator_for_test();
        aggregator::add(&mut agg, 1);
        aggregator::destroy(agg);
    }
}
