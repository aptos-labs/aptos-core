spec aptos_framework::transaction_fee {
    spec module {
        use aptos_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;

        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
    }

    spec CollectedFeesPerBlock {
        invariant burn_percentage <= 100;
    }

    spec initialize_fee_collection_and_distribution(aptos_framework: &signer, burn_percentage: u8) {
        // TODO: monomorphization issue. duplicated boogie procedures.
        pragma verify=false;
    }

    spec upgrade_burn_percentage(aptos_framework: &signer, new_burn_percentage: u8) {
        // TODO: missing aborts_if spec
        pragma verify=false;
    }

    spec register_proposer_for_fee_collection(proposer_addr: address) {
        aborts_if false;
        ensures is_fees_collection_enabled() ==>
            option::spec_borrow(global<CollectedFeesPerBlock>(@aptos_framework).proposer) == proposer_addr;
    }

    spec burn_coin_fraction(coin: &mut Coin<AptosCoin>, burn_percentage: u8) {
        let amount_to_burn = (burn_percentage * coin::value(coin)) / 100;
        aborts_if burn_percentage > 100;
        aborts_if (amount_to_burn > 0) && !exists<AptosCoinCapabilities>(@aptos_framework);
        include (amount_to_burn > 0) ==> coin::AbortsIfNotExistCoinInfo<AptosCoin>;
    }

    spec process_collected_fees() {
        // TODO: missing aborts_if spec
        pragma verify=false;
    }

    /// `AptosCoinCapabilities` should be exists.
    spec burn_fee(account: address, fee: u64) {
        // TODO: call burn_from, complex aborts conditions.
        pragma aborts_if_is_partial;
        aborts_if !exists<AptosCoinCapabilities>(@aptos_framework);
    }

    spec collect_fee(account: address, fee: u64) {
        aborts_if !exists<CollectedFeesPerBlock>(@aptos_framework);
        aborts_if fee > 0 && !exists<coin::CoinStore<AptosCoin>>(account);
        aborts_if fee > 0 && global<coin::CoinStore<AptosCoin>>(account).coin.value < fee;
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
