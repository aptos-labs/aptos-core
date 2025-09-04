module poc::read {
    use velor_framework::aggregator_v2;

    public entry fun main(_owner: &signer) {
        let agg = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator_v2::try_add<u64>(&mut agg, 123u64);
        let _value = aggregator_v2::read<u64>(&agg);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
