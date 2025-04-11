module poc::try_sub {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner:&signer) {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add<u64>(&mut agg, 20u64);
        aggregator_v2::try_sub<u64>(&mut agg, 10u64);
        aggregator_v2::try_sub<u64>(&mut agg, 11u64);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
