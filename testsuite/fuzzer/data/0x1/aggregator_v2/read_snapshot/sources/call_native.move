module poc::read_snapshot {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner: &signer) {
        let snap = aggregator_v2::create_snapshot<u64>(789u64);
        let _value = aggregator_v2::read_snapshot<u64>(&snap);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
