module swap::package_manager {
    use velor_framework::account::{Self, SignerCapability};
    use velor_framework::resource_account;
    use velor_std::smart_table::{Self, SmartTable};
    use std::string::String;

    friend swap::coin_wrapper;
    friend swap::liquidity_pool;
    friend swap::router;

    /// Stores permission config such as SignerCapability for controlling the resource account.
    struct PermissionConfig has key {
        /// Required to obtain the resource account signer.
        signer_cap: SignerCapability,
        /// Track the addresses created by the modules in this package.
        addresses: SmartTable<String, address>,
    }

    /// Initialize PermissionConfig to establish control over the resource account.
    /// This function is invoked only when this package is deployed the first time.
    fun init_module(swap_signer: &signer) {
        let signer_cap = resource_account::retrieve_resource_account_cap(swap_signer, @deployer);
        move_to(swap_signer, PermissionConfig {
            addresses: smart_table::new<String, address>(),
            signer_cap,
        });
    }

    /// Can be called by friended modules to obtain the resource account signer.
    public(friend) fun get_signer(): signer acquires PermissionConfig {
        let signer_cap = &borrow_global<PermissionConfig>(@swap).signer_cap;
        account::create_signer_with_capability(signer_cap)
    }

    /// Can be called by friended modules to keep track of a system address.
    public(friend) fun add_address(name: String, object: address) acquires PermissionConfig {
        let addresses = &mut borrow_global_mut<PermissionConfig>(@swap).addresses;
        smart_table::add(addresses, name, object);
    }

    public fun address_exists(name: String): bool acquires PermissionConfig {
        smart_table::contains(&safe_permission_config().addresses, name)
    }

    public fun get_address(name: String): address acquires PermissionConfig {
        let addresses = &borrow_global<PermissionConfig>(@swap).addresses;
        *smart_table::borrow(addresses, name)
    }

    inline fun safe_permission_config(): &PermissionConfig acquires PermissionConfig {
        borrow_global<PermissionConfig>(@swap)
    }

    #[test_only]
    public fun initialize_for_test(deployer: &signer) {
        let deployer_addr = std::signer::address_of(deployer);
        if (!exists<PermissionConfig>(deployer_addr)) {
            velor_framework::timestamp::set_time_has_started_for_testing(&account::create_signer_for_test(@0x1));

            account::create_account_for_test(deployer_addr);
            move_to(deployer, PermissionConfig {
                addresses: smart_table::new<String, address>(),
                signer_cap: account::create_test_signer_cap(deployer_addr),
            });
        };
    }

    #[test_only]
    friend swap::package_manager_tests;
}
