module poc::derive_string_concat {
    use velor_framework::aggregator_v2;
    use std::string;

    public entry fun main(_owner: &signer) {
        let snap = aggregator_v2::create_snapshot<u64>(101u64);
        let before = string::utf8(b"Value is: ");
        let after = string::utf8(b" units.");
        let _derived_snap = aggregator_v2::derive_string_concat<u64>(before, &snap, after);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
