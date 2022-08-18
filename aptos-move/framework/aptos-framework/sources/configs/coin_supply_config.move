/// Provides configuration for upgrading total coin supply.
module aptos_framework::coin_supply_config {
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    /// Configuration that controls the behavior of `Supply`. If the field is set,
    /// users of `Supply` are allowed to upgrade to parallelizable implementation.
    struct SupplyConfig has key {
        allow_upgrades: bool,
    }

    /// Publishes supply configuration. Initially, upgrading is not allowed.
    public(friend) fun initialize(account: &signer) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, SupplyConfig { allow_upgrades: false });
    }

    /// This should be called by on-chain governance to update the config and allow
    // `Supply` upgradability.
    public fun allow_coin_supply_upgrades(account: &signer) acquires SupplyConfig {
        system_addresses::assert_aptos_framework(account);
        let allow_upgrades = &mut borrow_global_mut<SupplyConfig>(@aptos_framework).allow_upgrades;
        *allow_upgrades = true;
    }

    /// Returns true if `Supply` can be upgraded.
    public fun can_upgrade_coin_supply(): bool acquires SupplyConfig {
        borrow_global_mut<SupplyConfig>(@aptos_framework).allow_upgrades
    }
}
