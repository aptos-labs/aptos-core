spec aptos_framework::transaction_fee {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// `AptosCoinCapabilities` should be exists.
    spec burn_fee(account: address, fee: u64) {
        // TODO: call burn_from, complex aborts conditions.
        pragma aborts_if_is_partial;
        aborts_if !exists<AptosCoinCapabilities>(@aptos_framework);
    }

    /// Ensure caller is admin.
    /// Aborts if `AptosCoinCapabilities` already exists.
    spec store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<AptosCoinCapabilities>(addr);
        ensures exists<AptosCoinCapabilities>(addr);
    }
}
