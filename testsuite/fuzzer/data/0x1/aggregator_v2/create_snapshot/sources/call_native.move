module poc::create_snapshot {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner: &signer) {
        let _snap = aggregator_v2::create_snapshot<u64>(999u64);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
