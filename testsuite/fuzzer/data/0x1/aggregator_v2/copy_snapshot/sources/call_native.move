module poc::copy_snapshot {
    use aptos_framework::aggregator_v2;

    public entry fun main(_owner: &signer) {
        let snap = aggregator_v2::create_snapshot<u64>(100u64);
        let _copied_snap = aggregator_v2::copy_snapshot<u64>(&snap);
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code = 196617)]
    fun a(owner:&signer){
        main(owner);
    }
}
