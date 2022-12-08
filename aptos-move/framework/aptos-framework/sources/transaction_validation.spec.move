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
        aborts_if !(txn_sequence_number < MAX_U64);

        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        aborts_if max_transaction_fee > MAX_U64;
        aborts_if !(txn_sequence_number == global<Account>(transaction_sender).sequence_number);
        aborts_if !exists<CoinStore<AptosCoin>>(transaction_sender);
        aborts_if !(global<CoinStore<AptosCoin>>(transaction_sender).coin.value >= max_transaction_fee);
    }

    spec prologue_common(
        sender: signer,
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
            txn_authentication_key: txn_public_key
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
        /// TODO: complex while loop condition.
        pragma aborts_if_is_partial;

        include PrologueCommonAbortsIf {
            txn_authentication_key: txn_sender_public_key
        };
        let num_secondary_signers = len(secondary_signer_addresses);
        aborts_if !(len(secondary_signer_public_key_hashes) == num_secondary_signers);
    }

    /// Abort according to the conditions.
    /// `AptosCoinCapabilities` and `CoinInfo` should exists.
    /// Skip transaction_fee::burn_fee verification.
    spec epilogue(
        account: signer,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) {
        use aptos_framework::coin::{CoinStore};
        use aptos_framework::account::{Account};
        use aptos_framework::aptos_coin::{AptosCoin};
        // TODO: call burn_fee, complex aborts conditions.
        pragma aborts_if_is_partial;

        aborts_if !(txn_max_gas_units >= gas_units_remaining);
        let gas_used = txn_max_gas_units - gas_units_remaining;

        aborts_if !(txn_gas_price * gas_used <= MAX_U64);
        let transaction_fee_amount = txn_gas_price * gas_used;

        let addr = signer::address_of(account);
        aborts_if !exists<CoinStore<AptosCoin>>(addr);
        aborts_if !(global<CoinStore<AptosCoin>>(addr).coin.value >= transaction_fee_amount);

        aborts_if !exists<Account>(addr);
        aborts_if !(global<Account>(addr).sequence_number < MAX_U64);

        let pre_balance = global<coin::CoinStore<AptosCoin>>(addr).coin.value;
        let post balance = global<coin::CoinStore<AptosCoin>>(addr).coin.value;
        let pre_account = global<account::Account>(addr);
        let post account = global<account::Account>(addr);
        ensures balance == pre_balance - transaction_fee_amount;
        ensures account.sequence_number == pre_account.sequence_number + 1;
    }
}
