/// This module allows publishing to a resource account and retaining control its signer for future upgrades or
/// for other purposes such as creating an NFT collection. It also offer features around managing object and resource
/// account addresses so those can be easily accessed in other modules.
///
/// The deployment flow is as follows:
/// 1. Deploy the package, including this package_manager module, using the Velor CLI command create-resource-and-publish-package
/// with an appropriate seed. This will create a resource account and deploy the module. The deployer address also needs
/// to be specified in Move.toml.
/// 2. Make sure the created resource address is persisted in the Move.toml for future deployments and upgrades as the
/// CLI doesn't do so by default.
/// 3. During deployment, package_manager::init_module will be called and extract the SignerCapability from the newly
/// created resource account.
/// 4. All other modules from the same package that are friends can call package_manager to obtain the resource account
/// signer when needed. If cross-package access is needed, authorization can be granted via an address-based whitelist
/// instead of friendship, which is limited to the same package.
/// 5. If new modules need to be deployed or existing modules in this package need to be updated, an assigned admin
/// account (defaults to the deployer account) can call package_manager::publish_package to with the new code.
///
/// Other modules can store and obtain stored addreses by calling add_address or get_address. This is useful for
/// storing addresses of other modules in the same package or system addresses such as the NFT collection.
module package::package_manager {
    use velor_framework::account::{Self, SignerCapability};
    use velor_framework::resource_account;
    use velor_std::smart_table::{Self, SmartTable};
    use std::string::String;

    /// Stores permission config such as SignerCapability for controlling the resource account.
    struct PermissionConfig has key {
        /// Required to obtain the resource account signer.
        signer_cap: SignerCapability,
        /// Track the addresses created by the modules in this package.
        addresses: SmartTable<String, address>,
    }

    /// Initialize PermissionConfig to establish control over the resource account.
    /// This function is invoked only when this package is deployed the first time.
    fun init_module(package_signer: &signer) {
        let signer_cap = resource_account::retrieve_resource_account_cap(package_signer, @deployer);
        move_to(package_signer, PermissionConfig {
            addresses: smart_table::new<String, address>(),
            signer_cap,
        });
    }

    /// Can be called by friended modules to obtain the resource account signer.
    public(friend) fun get_signer(): signer acquires PermissionConfig {
        let signer_cap = &borrow_global<PermissionConfig>(@package).signer_cap;
        account::create_signer_with_capability(signer_cap)
    }

    /// Can be called by friended modules to keep track of a system address.
    public(friend) fun add_address(name: String, object: address) acquires PermissionConfig {
        let addresses = &mut borrow_global_mut<PermissionConfig>(@package).addresses;
        smart_table::add(addresses, name, object);
    }

    public fun address_exists(name: String): bool acquires PermissionConfig {
        smart_table::contains(&safe_permission_config().addresses, name)
    }

    public fun get_address(name: String): address acquires PermissionConfig {
        let addresses = &borrow_global<PermissionConfig>(@package).addresses;
        *smart_table::borrow(addresses, name)
    }

    inline fun safe_permission_config(): &PermissionConfig acquires PermissionConfig {
        borrow_global<PermissionConfig>(@package)
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
    friend package::package_manager_tests;
}
