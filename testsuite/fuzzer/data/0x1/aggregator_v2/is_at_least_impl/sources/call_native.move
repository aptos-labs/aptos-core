module poc::is_at_least_impl {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner: &signer) {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add<u64>(&mut agg, 50u64);
        let _at_least_40 = aggregator_v2::is_at_least<u64>(&agg, 40u64);
        let _at_least_60 = aggregator_v2::is_at_least<u64>(&agg, 60u64);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
