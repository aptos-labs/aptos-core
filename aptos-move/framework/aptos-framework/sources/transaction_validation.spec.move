spec aptos_framework::transaction_validation {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Ensure caller is `aptos_framework`.
    /// Aborts if TransactionValidation already exists.
    spec initialize(
        aptos_framework: &signer,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
   ) {
        use std::signer;
        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<TransactionValidation>(addr);
   }

    /// Create a schema to reuse some code.
    /// Give some constraints that may abort according to the conditions.
    spec schema PrologueCommonAbortsIf {
        use aptos_framework::timestamp::{CurrentTimeMicroseconds};
        use aptos_framework::chain_id::{ChainId};
        use aptos_framework::account::{Account};
        use aptos_framework::coin::{CoinStore};
        sender: signer;
        gas_payer: address;
        txn_sequence_number: u64;
        txn_authentication_key: vector<u8>;
        txn_gas_price: u64;
        txn_max_gas_units: u64;
        txn_expiration_time: u64;
        chain_id: u8;

        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if !(timestamp::now_seconds() < txn_expiration_time);

        aborts_if !exists<ChainId>(@aptos_framework);
        aborts_if !(chain_id::get() == chain_id);
        let transaction_sender = signer::address_of(sender);
        aborts_if !account::exists_at(transaction_sender);
        aborts_if !(txn_sequence_number >= global<Account>(transaction_sender).sequence_number);
        aborts_if !(txn_authentication_key == global<Account>(transaction_sender).authentication_key);
        aborts_if !(txn_sequence_number < (1u64 << 63));

        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        aborts_if max_transaction_fee > MAX_U64;
        aborts_if !(txn_sequence_number == global<Account>(transaction_sender).sequence_number);
        aborts_if !exists<CoinStore<AptosCoin>>(gas_payer);
        aborts_if !(global<CoinStore<AptosCoin>>(gas_payer).coin.value >= max_transaction_fee);
    }

    spec prologue_common(
        sender: signer,
        gas_payer: address,
        txn_sequence_number: u64,
        txn_authentication_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        include PrologueCommonAbortsIf;
    }

    spec module_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        include PrologueCommonAbortsIf {
            gas_payer: signer::address_of(sender),
            txn_authentication_key: txn_public_key
        };
    }

    spec script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        _script_hash: vector<u8>,
    ) {
        include PrologueCommonAbortsIf {
            gas_payer: signer::address_of(sender),
            txn_authentication_key: txn_public_key
        };
    }

    spec schema MultiAgentPrologueCommonAbortsIf {
        secondary_signer_addresses: vector<address>;
        secondary_signer_public_key_hashes: vector<vector<u8>>;

        // Vectors to be `zipped with` should be of equal length.
        let num_secondary_signers = len(secondary_signer_addresses);
        aborts_if len(secondary_signer_public_key_hashes) != num_secondary_signers;

        // If any account does not exist, or public key hash does not match, abort.
        aborts_if exists i in 0..num_secondary_signers:
            !account::exists_at(secondary_signer_addresses[i])
                || secondary_signer_public_key_hashes[i] !=
                account::get_authentication_key(secondary_signer_addresses[i]);
    }

    spec multi_agent_common_prologue(
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
    ) {
        include MultiAgentPrologueCommonAbortsIf {
            secondary_signer_addresses,
            secondary_signer_public_key_hashes,
        };
    }

    /// Aborts if length of public key hashed vector
    /// not equal the number of singers.
    spec multi_agent_script_prologue (
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        pragma verify_duration_estimate = 120;
        let gas_payer = signer::address_of(sender);
        include PrologueCommonAbortsIf {
            gas_payer,
            txn_sequence_number,
            txn_authentication_key: txn_sender_public_key,
        };
        include MultiAgentPrologueCommonAbortsIf {
            secondary_signer_addresses,
            secondary_signer_public_key_hashes,
        };
    }

    spec fee_payer_script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
        fee_payer_address: address,
        fee_payer_public_key_hash: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        pragma verify_duration_estimate = 120;
        let gas_payer = fee_payer_address;
        include PrologueCommonAbortsIf {
            gas_payer,
            txn_sequence_number,
            txn_authentication_key: txn_sender_public_key,
        };
        include MultiAgentPrologueCommonAbortsIf {
            secondary_signer_addresses,
            secondary_signer_public_key_hashes,
        };

        aborts_if !account::exists_at(gas_payer);
        aborts_if !(fee_payer_public_key_hash == account::get_authentication_key(gas_payer));
        aborts_if !features::spec_fee_payer_enabled();
    }

        /// Abort according to the conditions.
    /// `AptosCoinCapabilities` and `CoinInfo` should exists.
    /// Skip transaction_fee::burn_fee verification.
    spec epilogue(
        account: signer,
        txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) {
        use std::option;
        use aptos_std::type_info;
        use aptos_framework::account::{Account};
        use aptos_framework::aggregator;
        use aptos_framework::aptos_coin::{AptosCoin};
        use aptos_framework::coin::{CoinStore, CoinInfo};
        use aptos_framework::optional_aggregator;
        use aptos_framework::transaction_fee::{AptosCoinCapabilities, CollectedFeesPerBlock};

        aborts_if !(txn_max_gas_units >= gas_units_remaining);
        let gas_used = txn_max_gas_units - gas_units_remaining;

        aborts_if !(txn_gas_price * gas_used <= MAX_U64);
        let transaction_fee_amount = txn_gas_price * gas_used;

        let addr = signer::address_of(account);
        aborts_if !exists<CoinStore<AptosCoin>>(addr);
        // Sufficiency of funds
        aborts_if !(global<CoinStore<AptosCoin>>(addr).coin.value >= transaction_fee_amount);

        aborts_if !exists<Account>(addr);
        aborts_if !(global<Account>(addr).sequence_number < MAX_U64);

        let pre_balance = global<coin::CoinStore<AptosCoin>>(addr).coin.value;
        let post balance = global<coin::CoinStore<AptosCoin>>(addr).coin.value;
        let pre_account = global<account::Account>(addr);
        let post account = global<account::Account>(addr);
        ensures balance == pre_balance - transaction_fee_amount;
        ensures account.sequence_number == pre_account.sequence_number + 1;


        // Bindings for `collect_fee` verification.
        let collected_fees = global<CollectedFeesPerBlock>(@aptos_framework).amount;
        let aggr = collected_fees.value;
        let aggr_val = aggregator::spec_aggregator_get_val(aggr);
        let aggr_lim = aggregator::spec_get_limit(aggr);
        let aptos_addr = type_info::type_of<AptosCoin>().account_address;
        // Bindings for `burn_fee` verification.
        let apt_addr = type_info::type_of<AptosCoin>().account_address;
        let maybe_apt_supply = global<CoinInfo<AptosCoin>>(apt_addr).supply;
        let apt_supply = option::spec_borrow(maybe_apt_supply);
        let apt_supply_value = optional_aggregator::optional_aggregator_value(apt_supply);
        // N.B.: Why can't `features::is_enabled`
        aborts_if if (features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES)) {
            !exists<CollectedFeesPerBlock>(@aptos_framework)
                || transaction_fee_amount > 0 &&
                    ( // `exists<CoinStore<AptosCoin>>(addr)` checked above.
                      // Sufficiency of funds is checked above.
                      aggr_val + transaction_fee_amount > aggr_lim
                        || aggr_val + transaction_fee_amount > MAX_U128)
        } else {
            // Existence of CoinStore in `addr` is checked above.
            // Sufficiency of funds is checked above.
            !exists<AptosCoinCapabilities>(@aptos_framework) ||
            // Existence of APT's CoinInfo
            transaction_fee_amount > 0 && !exists<CoinInfo<AptosCoin>>(aptos_addr) ||
            // Sufficiency of APT's supply
            option::spec_is_some(maybe_apt_supply) && apt_supply_value < transaction_fee_amount
        };
    }

    /// Abort according to the conditions.
    /// `AptosCoinCapabilities` and `CoinInfo` should exists.
    /// Skip transaction_fee::burn_fee verification.
    spec epilogue_gas_payer(
        account: signer,
        gas_payer: address,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) {
        use std::option;
        use aptos_std::type_info;
        use aptos_framework::account::{Account};
        use aptos_framework::aggregator;
        use aptos_framework::aptos_coin::{AptosCoin};
        use aptos_framework::coin::{CoinStore, CoinInfo};
        use aptos_framework::optional_aggregator;
        use aptos_framework::transaction_fee::{AptosCoinCapabilities, CollectedFeesPerBlock};

        aborts_if !(txn_max_gas_units >= gas_units_remaining);
        let gas_used = txn_max_gas_units - gas_units_remaining;

        aborts_if !(txn_gas_price * gas_used <= MAX_U64);
        let transaction_fee_amount = txn_gas_price * gas_used;

        let addr = signer::address_of(account);
        aborts_if !exists<CoinStore<AptosCoin>>(gas_payer);
        // Sufficiency of funds
        aborts_if !(global<CoinStore<AptosCoin>>(gas_payer).coin.value >= transaction_fee_amount);

        aborts_if !exists<Account>(addr);
        aborts_if !(global<Account>(addr).sequence_number < MAX_U64);

        let pre_balance = global<coin::CoinStore<AptosCoin>>(gas_payer).coin.value;
        let post balance = global<coin::CoinStore<AptosCoin>>(gas_payer).coin.value;
        let pre_account = global<account::Account>(addr);
        let post account = global<account::Account>(addr);
        ensures balance == pre_balance - transaction_fee_amount;
        ensures account.sequence_number == pre_account.sequence_number + 1;


        // Bindings for `collect_fee` verification.
        let collected_fees = global<CollectedFeesPerBlock>(@aptos_framework).amount;
        let aggr = collected_fees.value;
        let aggr_val = aggregator::spec_aggregator_get_val(aggr);
        let aggr_lim = aggregator::spec_get_limit(aggr);
        let aptos_addr = type_info::type_of<AptosCoin>().account_address;
        // Bindings for `burn_fee` verification.
        let apt_addr = type_info::type_of<AptosCoin>().account_address;
        let maybe_apt_supply = global<CoinInfo<AptosCoin>>(apt_addr).supply;
        let apt_supply = option::spec_borrow(maybe_apt_supply);
        let apt_supply_value = optional_aggregator::optional_aggregator_value(apt_supply);
        // N.B.: Why can't `features::is_enabled`
        aborts_if if (features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES)) {
            !exists<CollectedFeesPerBlock>(@aptos_framework)
                || transaction_fee_amount > 0 &&
                ( // `exists<CoinStore<AptosCoin>>(addr)` checked above.
                    // Sufficiency of funds is checked above.
                    aggr_val + transaction_fee_amount > aggr_lim
                        || aggr_val + transaction_fee_amount > MAX_U128)
        } else {
            // Existence of CoinStore in `addr` is checked above.
            // Sufficiency of funds is checked above.
            !exists<AptosCoinCapabilities>(@aptos_framework) ||
                // Existence of APT's CoinInfo
                transaction_fee_amount > 0 && !exists<CoinInfo<AptosCoin>>(aptos_addr) ||
                // Sufficiency of APT's supply
                option::spec_is_some(maybe_apt_supply) && apt_supply_value < transaction_fee_amount
        };
    }
}
