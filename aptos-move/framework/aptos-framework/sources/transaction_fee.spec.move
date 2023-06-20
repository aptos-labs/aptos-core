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
        use std::signer;
        use aptos_framework::stake::ValidatorFees;
        use aptos_framework::aggregator_factory;
        use aptos_framework::system_addresses;

        aborts_if exists<CollectedFeesPerBlock>(@aptos_framework);
        aborts_if burn_percentage > 100;

        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<ValidatorFees>(aptos_addr);

        include system_addresses::AbortsIfNotAptosFramework {account: aptos_framework};
        include aggregator_factory::CreateAggregatorInternalAbortsIf;

        ensures exists<ValidatorFees>(aptos_addr);
    }

    spec upgrade_burn_percentage(aptos_framework: &signer, new_burn_percentage: u8) {
        use std::signer;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        // Percentage validation
        aborts_if new_burn_percentage > 100;
        // Signer validation
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        // Requirements of `process_collected_fees`
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        // The effect of upgrading the burn percentage
        ensures exists<CollectedFeesPerBlock>(@aptos_framework) ==>
            global<CollectedFeesPerBlock>(@aptos_framework).burn_percentage == new_burn_percentage;
    }

    spec register_proposer_for_fee_collection(proposer_addr: address) {
        aborts_if false;
        ensures is_fees_collection_enabled() ==>
            option::spec_borrow(global<CollectedFeesPerBlock>(@aptos_framework).proposer) == proposer_addr;
    }

    spec burn_coin_fraction(coin: &mut Coin<AptosCoin>, burn_percentage: u8) {
        use aptos_framework::optional_aggregator;
        use aptos_framework::aggregator;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        requires burn_percentage <= 100;
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        let amount_to_burn = (burn_percentage * coin::value(coin)) / 100;
        let maybe_supply = coin::get_coin_supply_opt<AptosCoin>();
        aborts_if amount_to_burn > 0 && option::is_some(maybe_supply) && optional_aggregator::is_parallelizable(option::borrow(maybe_supply))
            && aggregator::spec_aggregator_get_val(option::borrow(option::borrow(maybe_supply).aggregator)) <
            amount_to_burn;
        aborts_if option::is_some(maybe_supply) && !optional_aggregator::is_parallelizable(option::borrow(maybe_supply))
            && option::borrow(option::borrow(maybe_supply).integer).value <
            amount_to_burn;
        include (amount_to_burn > 0) ==> coin::AbortsIfNotExistCoinInfo<AptosCoin>;
    }

    spec fun collectedFeesAggregator(): AggregatableCoin<AptosCoin> {
        global<CollectedFeesPerBlock>(@aptos_framework).amount
    }

    spec schema RequiresCollectedFeesPerValueLeqBlockAptosSupply {
        use aptos_framework::optional_aggregator;
        use aptos_framework::aggregator;
        let maybe_supply = coin::get_coin_supply_opt<AptosCoin>();
        requires
            (is_fees_collection_enabled() && option::is_some(maybe_supply)) ==>
                (aggregator::spec_aggregator_get_val(global<CollectedFeesPerBlock>(@aptos_framework).amount.value) <=
                    optional_aggregator::optional_aggregator_value(option::spec_borrow(coin::get_coin_supply_opt<AptosCoin>())));
    }

    spec process_collected_fees() {
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include RequiresCollectedFeesPerValueLeqBlockAptosSupply;
    }

    /// `AptosCoinCapabilities` should be exists.
    spec burn_fee(account: address, fee: u64) {
        use aptos_std::type_info;
        use aptos_framework::optional_aggregator;
        use aptos_framework::coin::{CoinInfo, CoinStore};


        aborts_if !exists<AptosCoinCapabilities>(@aptos_framework);

        // This function essentially calls `coin::burn_coin`, monophormized for `AptosCoin`.
        let account_addr = account;
        let amount = fee;

        let aptos_addr = type_info::type_of<AptosCoin>().account_address;
        let coin_store = global<CoinStore<AptosCoin>>(account_addr);
        let post post_coin_store = global<CoinStore<AptosCoin>>(account_addr);

        modifies global<CoinInfo<AptosCoin>>(aptos_addr);
        modifies global<CoinStore<AptosCoin>>(account_addr);

        aborts_if amount != 0 && !(exists<CoinInfo<AptosCoin>>(aptos_addr)
            && exists<CoinStore<AptosCoin>>(account_addr));
        aborts_if coin_store.coin.value < amount;

        let maybe_supply = global<CoinInfo<AptosCoin>>(aptos_addr).supply;
        let supply = option::spec_borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        let post post_maybe_supply = global<CoinInfo<AptosCoin>>(aptos_addr).supply;
        let post post_supply = option::spec_borrow(post_maybe_supply);
        let post post_value = optional_aggregator::optional_aggregator_value(post_supply);

        aborts_if option::spec_is_some(maybe_supply) && value < amount;

        ensures post_coin_store.coin.value == coin_store.coin.value - amount;
        ensures if (option::spec_is_some(maybe_supply)) {
            post_value == value - amount
        } else {
            option::spec_is_none(post_maybe_supply)
        };
    }

    spec collect_fee(account: address, fee: u64) {
        use aptos_framework::aggregator;
        let collected_fees = global<CollectedFeesPerBlock>(@aptos_framework).amount;
        let aggr = collected_fees.value;
        aborts_if !exists<CollectedFeesPerBlock>(@aptos_framework);
        aborts_if fee > 0 && !exists<coin::CoinStore<AptosCoin>>(account);
        aborts_if fee > 0 && global<coin::CoinStore<AptosCoin>>(account).coin.value < fee;
        aborts_if fee > 0 && aggregator::spec_aggregator_get_val(aggr)
            + fee > aggregator::spec_get_limit(aggr);
        aborts_if fee > 0 && aggregator::spec_aggregator_get_val(aggr)
            + fee > MAX_U128;
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
