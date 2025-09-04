#[test_only]
module swap::package_manager_tests {
    use std::signer;
    use std::string;
    use swap::package_manager;

    #[test(deployer = @0xcafe)]
    public fun test_can_get_signer(deployer: &signer) {
        package_manager::initialize_for_test(deployer);
        let deployer_addr = signer::address_of(deployer);
        assert!(signer::address_of(&package_manager::get_signer()) == deployer_addr, 0);
    }

    #[test(deployer = @0xcafe)]
    public fun test_can_set_and_get_address(deployer: &signer) {
        package_manager::initialize_for_test(deployer);
        package_manager::add_address(string::utf8(b"test"), @0xdeadbeef);
        assert!(package_manager::get_address(string::utf8(b"test")) == @0xdeadbeef, 0);
    }
}
