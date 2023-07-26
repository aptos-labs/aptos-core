module no_code_mint::package_manager {
    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::resource_account;
    use aptos_std::smart_table::{Self, SmartTable};
    use std::string::String;
    use aptos_std::code;
    use std::error;
    use std::signer;
    friend no_code_mint::mint_machine;
    friend no_code_mint::whitelist;

    /// The signer is not authorized to deploy this module.
    const ENOT_AUTHORIZED: u64 = 0;

    /// Stores permission config such as SignerCapability for controlling the resource account.
    struct PermissionConfig has key {
        /// Required to obtain the resource account signer.
        signer_cap: SignerCapability,
        /// Track the addresses created by the modules in this package.
        addresses: SmartTable<String, address>,
    }

    /// Initialize PermissionConfig to establish control over the resource account.
    /// This function is invoked only when this package is deployed the first time.
    fun init_module(resource_signer: &signer) {
        let signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @deployer);
        move_to(resource_signer, PermissionConfig {
            addresses: smart_table::new<String, address>(),
            signer_cap,
        });
    }

    public entry fun publish_package(
        deployer: &signer,
        package_metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires PermissionConfig {
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));
        code::publish_package_txn(&get_signer(), package_metadata, code);
    }

    /// Can be called by friended modules to obtain the resource account signer.
    public(friend) fun get_signer(): signer acquires PermissionConfig {
        let signer_cap = &borrow_global<PermissionConfig>(@no_code_mint).signer_cap;
        account::create_signer_with_capability(signer_cap)
    }

    /// Can be called by friended modules to keep track of a system address.
    public(friend) fun add_named_address(name: String, object: address) acquires PermissionConfig {
        let addresses = &mut borrow_global_mut<PermissionConfig>(@no_code_mint).addresses;
        smart_table::add(addresses, name, object);
    }

    /// Can be called by friended modules to keep track of a system address.
    public(friend) fun remove_named_address(name: String) acquires PermissionConfig {
        let addresses = &mut borrow_global_mut<PermissionConfig>(@no_code_mint).addresses;
        smart_table::remove(addresses, name);
    }

    public fun named_address_exists(name: String): bool acquires PermissionConfig {
        smart_table::contains(&safe_permission_config().addresses, name)
    }

    #[view]
    public fun get_named_address(name: String): address acquires PermissionConfig {
        let addresses = &borrow_global<PermissionConfig>(@no_code_mint).addresses;
        *smart_table::borrow(addresses, name)
    }

    inline fun safe_permission_config(): &PermissionConfig acquires PermissionConfig {
        borrow_global<PermissionConfig>(@no_code_mint)
    }

    public(friend) inline fun assert_deployer(deployer: &signer) {
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));
    }

    public(friend) inline fun assert_deployer_and_get_signer(deployer: &signer): signer {
        assert_deployer(deployer);
        get_signer()
    }

    #[test_only]
    public fun init_module_for_test(resource_signer: &signer) {
        init_module(resource_signer);
    }

}
