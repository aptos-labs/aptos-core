module poc::string_concat {
    use aptos_framework::aggregator_v2;
    use std::string;

    public entry fun main(_owner: &signer) {
        let snap = aggregator_v2::create_snapshot<u64>(100u64);
        let before = string::utf8(b"before-");
        let after = string::utf8(b"-after");
        let _string_snap = aggregator_v2::string_concat<u64>(before, &snap, after);
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code = 196617)]
    fun a(owner:&signer){
        main(owner);
    }
}
