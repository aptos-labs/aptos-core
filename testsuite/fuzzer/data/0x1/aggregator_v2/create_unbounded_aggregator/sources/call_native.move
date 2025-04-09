module poc::create_unbounded_aggregator {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner:&signer) {
        let _agg = aggregator_v2::create_unbounded_aggregator<u64>();
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
