module poc::try_add {
    use velor_framework::aggregator_v2;

    const U64_MAX: u64 = 18446744073709551615u64;

    public entry fun main(_owner: &signer) {
        let agg1 = aggregator_v2::create_unbounded_aggregator<u64>();
        let success1 = aggregator_v2::try_add<u64>(&mut agg1, 10u64);
        assert!(success1, 1);
        let val1 = aggregator_v2::read(&agg1);
        assert!(val1 == 10u64, 2);

        aggregator_v2::try_add<u64>(&mut agg1, U64_MAX);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
