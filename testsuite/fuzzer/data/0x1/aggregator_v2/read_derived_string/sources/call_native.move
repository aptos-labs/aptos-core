module poc::read_derived_string {
    use velor_framework::aggregator_v2;
    use std::string;

    public entry fun main(_owner: &signer) {
        let derived_snap = aggregator_v2::create_derived_string(string::utf8(b"hello"));
        let _value = aggregator_v2::read_derived_string(&derived_snap);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
