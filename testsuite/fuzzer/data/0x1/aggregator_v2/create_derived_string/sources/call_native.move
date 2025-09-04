module poc::create_derived_string {
    use velor_framework::aggregator_v2;
    use std::string;

    public entry fun main(_owner: &signer) {
        let _derived_snap = aggregator_v2::create_derived_string(string::utf8(b"world"));
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
