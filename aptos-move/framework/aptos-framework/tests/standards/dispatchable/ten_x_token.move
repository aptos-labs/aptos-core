#[test_only]
module 0xcafe::ten_x_token {
    use aptos_framework::fungible_asset;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::object::{ConstructorRef, Object};
    use aptos_framework::function_info;

    use std::option;
    use std::signer;
    use std::string;

    public fun initialize(account: &signer, constructor_ref: &ConstructorRef) {
        assert!(signer::address_of(account) == @0xcafe, 1);
        let value = function_info::new_function_info(
            account,
            string::utf8(b"ten_x_token"),
            string::utf8(b"derived_balance"),
        );
        dispatchable_fungible_asset::register_dispatch_functions(
            constructor_ref,
            option::none(),
            option::none(),
            option::some(value)
        );
    }

    public fun derived_balance<T: key>(store: Object<T>): u64 {
        // Derived value is always 10x!
        fungible_asset::balance(store) * 10
    }
}
