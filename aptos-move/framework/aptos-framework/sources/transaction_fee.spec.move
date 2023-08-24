spec aptos_framework::transaction_fee {
    spec module {
        use aptos_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;
        // property 1: Given the blockchain is in an operating state, it guarantees that the Aptos framework signer may burn Aptos coins.
        invariant [suspendable] chain_status::is_operating() ==> exists<AptosCoinCapabilities>(@aptos_framework);
    }

    spec CollectedFeesPerBlock {
        // property 4: The percentage of the burnt collected fee is always a value from 0 to 100.
        invariant burn_percentage <= 100;
    }

    spec initialize_fee_collection_and_distribution(aptos_framework: &signer, burn_percentage: u8) {
        use std::signer;
        use aptos_framework::stake::ValidatorFees;
        use aptos_framework::aggregator_factory;
        use aptos_framework::system_addresses;

        // property 2: The initialization function may only be called once.
        aborts_if exists<CollectedFeesPerBlock>(@aptos_framework);
        aborts_if burn_percentage > 100;

        let aptos_addr = signer::address_of(aptos_framework);
        // property 3: Only the admin address is authorized to call the initialization function.
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<ValidatorFees>(aptos_addr);

        include system_addresses::AbortsIfNotAptosFramework {account: aptos_framework};
        include aggregator_factory::CreateAggregatorInternalAbortsIf;
        aborts_if exists<CollectedFeesPerBlock>(aptos_addr);

        ensures exists<ValidatorFees>(aptos_addr);
        ensures exists<CollectedFeesPerBlock>(aptos_addr);
    }

    spec upgrade_burn_percentage(aptos_framework: &signer, new_burn_percentage: u8) {
        use std::signer;

        // Percentage validation
        aborts_if new_burn_percentage > 100;
        // Signer validation
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);

        // property 5: Prior to upgrading the burn percentage, it must process all the fees collected up to that point.
        // property 6: Ensure the presence of the resource.
        // Requirements and ensures conditions of `process_collected_fees`
        include ProcessCollectedFeesRequiresAndEnsures;

        // The effect of upgrading the burn percentage
        ensures exists<CollectedFeesPerBlock>(@aptos_framework) ==>
            global<CollectedFeesPerBlock>(@aptos_framework).burn_percentage == new_burn_percentage;
    }

    spec register_proposer_for_fee_collection(proposer_addr: address) {
        aborts_if false;
        // property 6: Ensure the presence of the resource.
        ensures is_fees_collection_enabled() ==>
            option::spec_borrow(global<CollectedFeesPerBlock>(@aptos_framework).proposer) == proposer_addr;
    }

    spec burn_coin_fraction(coin: &mut Coin<AptosCoin>, burn_percentage: u8) {
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;

        requires burn_percentage <= 100;
        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);

        let amount_to_burn = (burn_percentage * coin::value(coin)) / 100;
        // include (amount_to_burn > 0) ==> coin::AbortsIfNotExistCoinInfo<AptosCoin>;
        include amount_to_burn > 0 ==> coin::AbortsIfAggregator<AptosCoin>{ coin: Coin<AptosCoin>{ value: amount_to_burn } };
        ensures coin.value == old(coin).value - amount_to_burn;
    }

    spec fun collectedFeesAggregator(): AggregatableCoin<AptosCoin> {
        global<CollectedFeesPerBlock>(@aptos_framework).amount
    }

    spec schema RequiresCollectedFeesPerValueLeqBlockAptosSupply {
        use aptos_framework::optional_aggregator;
        use aptos_framework::aggregator;
        let maybe_supply = coin::get_coin_supply_opt<AptosCoin>();
        // property 6: Ensure the presence of the resource.
        requires
            (is_fees_collection_enabled() && option::is_some(maybe_supply)) ==>
                (aggregator::spec_aggregator_get_val(global<CollectedFeesPerBlock>(@aptos_framework).amount.value) <=
                    optional_aggregator::optional_aggregator_value(option::spec_borrow(coin::get_coin_supply_opt<AptosCoin>())));
    }

    spec schema ProcessCollectedFeesRequiresAndEnsures {
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::aggregator;
        use aptos_std::table;

        requires exists<AptosCoinCapabilities>(@aptos_framework);
        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);
        include RequiresCollectedFeesPerValueLeqBlockAptosSupply;

        aborts_if false;

        let collected_fees = global<CollectedFeesPerBlock>(@aptos_framework);
        let post post_collected_fees = global<CollectedFeesPerBlock>(@aptos_framework);
        let pre_amount = aggregator::spec_aggregator_get_val(collected_fees.amount.value);
        let post post_amount = aggregator::spec_aggregator_get_val(post_collected_fees.amount.value);
        let fees_table = global<stake::ValidatorFees>(@aptos_framework).fees_table;
        let post post_fees_table = global<stake::ValidatorFees>(@aptos_framework).fees_table;
        let proposer = option::spec_borrow(collected_fees.proposer);
        let fee_to_add = pre_amount - pre_amount * collected_fees.burn_percentage / 100;
        ensures is_fees_collection_enabled() ==> option::spec_is_none(post_collected_fees.proposer) && post_amount == 0;
        ensures is_fees_collection_enabled() && aggregator::spec_read(collected_fees.amount.value) > 0 &&
            option::spec_is_some(collected_fees.proposer) ==>
            if (proposer != @vm_reserved) {
                if (table::spec_contains(fees_table, proposer)) {
                    table::spec_get(post_fees_table, proposer).value == table::spec_get(fees_table, proposer).value + fee_to_add
                } else {
                table::spec_get(post_fees_table, proposer).value == fee_to_add
                }
            } else {
                option::spec_is_none(post_collected_fees.proposer) && post_amount == 0
            };
    }

    spec process_collected_fees() {
        include ProcessCollectedFeesRequiresAndEnsures;
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
        let coin_store = global<coin::CoinStore<AptosCoin>>(account);
        aborts_if !exists<CollectedFeesPerBlock>(@aptos_framework);
        aborts_if fee > 0 && !exists<coin::CoinStore<AptosCoin>>(account);
        aborts_if fee > 0 && coin_store.coin.value < fee;
        aborts_if fee > 0 && aggregator::spec_aggregator_get_val(aggr)
            + fee > aggregator::spec_get_limit(aggr);
        aborts_if fee > 0 && aggregator::spec_aggregator_get_val(aggr)
            + fee > MAX_U128;

        let post post_coin_store = global<coin::CoinStore<AptosCoin>>(account);
        let post post_collected_fees = global<CollectedFeesPerBlock>(@aptos_framework).amount;
        ensures post_coin_store.coin.value == coin_store.coin.value - fee;
        ensures aggregator::spec_aggregator_get_val(post_collected_fees.value) == aggregator::spec_aggregator_get_val(aggr) + fee;
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
