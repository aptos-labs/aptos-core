module poc::create_user_derived_object_address_impl {
    use velor_framework::object;
    use std::signer;

    public entry fun main(owner: &signer) {
        let source_addr = signer::address_of(owner);
        let _derived_addr = object::create_user_derived_object_address(source_addr, @0x123);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
