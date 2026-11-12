spec aptos_framework::transaction_validation {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The sender of a transaction should have sufficient coin balance to pay the transaction fee.
    /// Criticality: High
    /// Implementation: The prologue_common function asserts that the transaction sender has enough coin balance to be
    /// paid as the max_transaction_fee.
    /// Enforcement: Formally verified via [high-level-req-1](PrologueCommonAbortsIf). Moreover, the native transaction validation patterns have been manually audited.
    ///
    /// No.: 2
    /// Requirement: All secondary signer addresses are verified to be authentic through a validation process.
    /// Criticality: Critical
    /// Implementation: The function multi_agent_script_prologue ensures that each secondary signer address undergoes
    /// authentication validation, including verification of account existence and authentication key matching,
    /// confirming their authenticity.
    /// Enforcement: Audited that each secondary signer's account exists and its authentication key matches. Moreover, the native transaction validation patterns have been manually audited.
    ///
    /// No.: 3
    /// Requirement: After successful execution, base the transaction fee on the configuration set by the features library.
    /// Criticality: High
    /// Implementation: The epilogue function collects the transaction fee for either redistribution or burning based on
    /// the feature::collect_and_distribute_gas_fees result.
    /// Enforcement: The fee distribution mechanism no longer exists; the epilogue burns the fee net of the storage refund. Moreover, the native transaction validation patterns have been manually audited.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec grant_gas_permission {
        aborts_if true;
    }

    spec revoke_gas_permission {
        aborts_if true;
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

        ensures exists<TransactionValidation>(addr);
    }

    /// The checks of `prologue_common`: expiration, chain id, the sender's authentication key, replay protection
    /// and the gas payer's balance.
    spec schema PrologueCommonAbortsIf {
        use aptos_framework::timestamp::{CurrentTimeMicroseconds};
        use aptos_framework::chain_id::{ChainId};
        sender: &signer;
        gas_payer: &signer;
        replay_protector: ReplayProtector;
        txn_authentication_key: Option<vector<u8>>;
        txn_gas_price: u64;
        txn_max_gas_units: u64;
        txn_expiration_time: u64;
        chain_id: u8;
        is_simulation: bool;

        aborts_if !exists<CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if !(timestamp::now_seconds() < txn_expiration_time);

        aborts_if !exists<ChainId>(@aptos_framework);
        aborts_if !(chain_id::get() == chain_id);
        let transaction_sender = signer::address_of(sender);
        let gas_payer_addr = signer::address_of(gas_payer);

        // A sponsored first transaction is checked against the default key of an account without `Account`
        // resource, which is what `spec_get_authentication_key` returns for it.
        let check_auth_key = !(is_simulation && (option::is_none(txn_authentication_key)
            || len(option::borrow(txn_authentication_key)) == 0));
        let sponsored_new_account = transaction_sender != gas_payer_addr
            && !account::spec_exists_at(transaction_sender)
            && features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION);
        aborts_if check_auth_key && option::is_some(txn_authentication_key)
            && !sponsored_new_account && !account::spec_exists_at(transaction_sender);
        aborts_if check_auth_key && option::is_some(txn_authentication_key)
            && option::borrow(txn_authentication_key) != account::spec_get_authentication_key(transaction_sender);
        aborts_if check_auth_key && option::is_none(txn_authentication_key)
            && !features::spec_is_enabled(features::DERIVABLE_ACCOUNT_ABSTRACTION)
            && !features::spec_is_enabled(features::ACCOUNT_ABSTRACTION);

        include (replay_protector is ReplayProtector::SequenceNumber) ==> CheckReplayProtectionRegularTxnAbortsIf {
            sender_address: transaction_sender,
            gas_payer_address: gas_payer_addr,
            txn_sequence_number: replay_protector.0,
        };
        aborts_if (replay_protector is ReplayProtector::Nonce)
            && txn_expiration_time > timestamp::spec_now_seconds() + MAX_EXP_TIME_SECONDS_FOR_ORDERLESS_TXNS;
        aborts_if (replay_protector is ReplayProtector::Nonce)
            && nonce_validation::spec_is_nonce_in_use(transaction_sender, replay_protector.0, txn_expiration_time);

        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        aborts_if max_transaction_fee > MAX_U64;
        // property 1: The sender of a transaction should have sufficient coin balance to pay the transaction fee.
        /// [high-level-req-1]
        aborts_if !(is_simulation && gas_payer_addr == @0x0) && !spec_can_pay_gas(gas_payer_addr, max_transaction_fee);
    }

    /// The APT primary store of `gas_payer` holds at least `amount`; see `aptos_account::is_fungible_balance_at_least`.
    spec fun spec_can_pay_gas(gas_payer: address, amount: u64): bool {
        use aptos_framework::fungible_asset;
        fungible_asset::spec_is_address_balance_at_least(aptos_account::spec_apt_store_address(gas_payer), amount)
    }

    spec prologue_common(
        sender: &signer,
        gas_payer: &signer,
        replay_protector: ReplayProtector,
        txn_authentication_key: Option<vector<u8>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
        txn_limits_request: Option<UserTxnLimitsRequest>,
    ) {
        // Partial: the aborts of the nonce history update, the transaction limits check and the account abstraction
        // authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf;
    }

    spec check_for_replay_protection_orderless_txn(
        sender: address,
        nonce: u64,
        txn_expiration_time: u64,
    ) {
        // Partial: the aborts of the nonce history update are not specified.
        pragma aborts_if_is_partial;
        aborts_if txn_expiration_time > timestamp::spec_now_seconds() + MAX_EXP_TIME_SECONDS_FOR_ORDERLESS_TXNS;
        aborts_if nonce_validation::spec_is_nonce_in_use(sender, nonce, txn_expiration_time);
    }

    spec check_for_replay_protection_regular_txn(
        sender_address: address,
        gas_payer_address: address,
        txn_sequence_number: u64,
    ) {
        include CheckReplayProtectionRegularTxnAbortsIf;
    }

    /// Unless this is a sponsored first transaction of an account without an `Account` resource, the account
    /// must exist and the sequence number must match the account's.
    spec schema CheckReplayProtectionRegularTxnAbortsIf {
        use aptos_framework::account::{Account};
        sender_address: address;
        gas_payer_address: address;
        txn_sequence_number: u64;
        let checked = sender_address == gas_payer_address
            || account::spec_exists_at(sender_address)
            || !features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION)
            || txn_sequence_number > 0;
        let account_sequence_number =
            if (exists<Account>(sender_address)) global<Account>(sender_address).sequence_number else 0;
        aborts_if checked && !account::spec_exists_at(sender_address);
        aborts_if checked && txn_sequence_number >= 9223372036854775808;
        aborts_if checked && txn_sequence_number != account_sequence_number;
    }

    spec script_prologue_extended(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        _script_hash: vector<u8>,
        is_simulation: bool,
    ) {
        include PrologueCommonAbortsIf {
            gas_payer: sender,
            txn_authentication_key: option::spec_some(txn_public_key),
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
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
            gas_payer: sender,
            txn_authentication_key: option::spec_some(txn_public_key),
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            is_simulation: false,
        };
    }

    spec schema MultiAgentPrologueCommonAbortsIf {
        secondary_signer_addresses: vector<address>;
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>;
        is_simulation: bool;

        // Vectors to be `zipped with` should be of equal length.
        let num_secondary_signers = len(secondary_signer_addresses);
        aborts_if len(secondary_signer_public_key_hashes) != num_secondary_signers;

        // If any account does not exist, or public key hash does not match, abort.
        // property 2: All secondary signer addresses are verified to be authentic through a validation process.
        aborts_if exists i in 0..num_secondary_signers:
            !account::spec_exists_at(secondary_signer_addresses[i]);
        aborts_if exists i in 0..num_secondary_signers:
            !can_skip(features::spec_simulation_enhancement_enabled(), is_simulation, secondary_signer_public_key_hashes[i]) &&
                option::is_some(secondary_signer_public_key_hashes[i]) && option::borrow(
                secondary_signer_public_key_hashes[i]
            ) !=
                    account::spec_get_authentication_key(secondary_signer_addresses[i]);
        // By the end, all secondary signers account should exist and public key hash should match.
        ensures forall i in 0..num_secondary_signers:
            account::spec_exists_at(secondary_signer_addresses[i]);
        ensures forall i in 0..num_secondary_signers:
            option::is_none(secondary_signer_public_key_hashes[i]) || option::borrow(
                secondary_signer_public_key_hashes[i]
            ) ==
                account::spec_get_authentication_key(secondary_signer_addresses[i])
                || can_skip(features::spec_simulation_enhancement_enabled(), is_simulation, secondary_signer_public_key_hashes[i]);
    }

    spec fun can_skip(feature_flag: bool, is_simulation: bool, auth_key: Option<vector<u8>>): bool {
        features::spec_simulation_enhancement_enabled() && is_simulation && option::is_none(auth_key)
    }

    spec multi_agent_common_prologue(
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>,
        is_simulation: bool,
    ) {
        // Partial: the per-signer existence and authentication key checks are not specified.
        pragma aborts_if_is_partial;
        aborts_if len(secondary_signer_public_key_hashes) != len(secondary_signer_addresses);
        // include MultiAgentPrologueCommonAbortsIf {
        //     secondary_signer_addresses,
        //     secondary_signer_public_key_hashes,
        //     is_simulation,
        // };
    }

    spec multi_agent_script_prologue_extended(
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
    ) {
        // Partial: the secondary signer checks are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: sender,
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: option::spec_some(txn_sender_public_key),
        };
        // include PrologueCommonAbortsIf {
        //     gas_payer,
        //     txn_sequence_number,
        //     txn_authentication_key: txn_sender_public_key,
        // };
        // include MultiAgentPrologueCommonAbortsIf {
        //     secondary_signer_addresses,
        //     vector::map(secondary_signer_public_key_hashes, |x| option::spec_some(x)),
        //     is_simulation,
        // };
    }

    spec multi_agent_script_prologue(
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
        // Partial: the secondary signer checks are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: sender,
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: option::spec_some(txn_sender_public_key),
            is_simulation: false,
        };
    }

    spec fee_payer_script_prologue_extended(
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
        is_simulation: bool,
    ) {
        pragma aborts_if_is_partial;
        pragma verify_duration_estimate = 120;

        aborts_if !features::spec_is_enabled(features::FEE_PAYER_ENABLED);
        let gas_payer = create_signer::spec_create_signer(fee_payer_address);
        include PrologueCommonAbortsIf {
            gas_payer,
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: option::spec_some(txn_sender_public_key),
        };
        // include MultiAgentPrologueCommonAbortsIf {
        //     secondary_signer_addresses,
        //     secondary_signer_public_key_hashes,
        //     is_simulation,
        // };

        aborts_if !account::spec_exists_at(fee_payer_address);
        aborts_if !(fee_payer_public_key_hash == account::spec_get_authentication_key(fee_payer_address));
        aborts_if !features::spec_fee_payer_enabled();
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
        // Partial: the secondary signer checks are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: create_signer::spec_create_signer(fee_payer_address),
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: option::spec_some(txn_sender_public_key),
            is_simulation: false,
        };
        aborts_if !account::spec_exists_at(fee_payer_address);
        aborts_if fee_payer_public_key_hash != account::spec_get_authentication_key(fee_payer_address);
    }

    spec epilogue_extended(
        account: signer,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf { gas_payer: signer::address_of(account) };
        include IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    spec epilogue(
        account: signer,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf { gas_payer: signer::address_of(account), is_simulation: false };
        include IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    /// Charges the gas fee net of the storage refund to `gas_payer` and increments the sender's sequence number.
    spec epilogue_gas_payer_extended(
        account: signer,
        gas_payer: address,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf;
        include IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    /// Checks the gas used, then burns the fee net of the storage refund from the APT store of `gas_payer`, or
    /// mints the excess refund into it.
    spec schema EpilogueGasChargeAbortsIf {
        use aptos_framework::transaction_fee::{
            AptosFABurnCapabilities, AptosFAMintCapabilities, AptosCoinMintCapability
        };
        gas_payer: address;
        storage_fee_refunded: u64;
        txn_gas_price: u64;
        txn_max_gas_units: u64;
        gas_units_remaining: u64;
        is_simulation: bool;

        aborts_if txn_max_gas_units < gas_units_remaining;
        let gas_used = txn_max_gas_units - gas_units_remaining;
        aborts_if txn_gas_price * gas_used > MAX_U64;
        let transaction_fee_amount = txn_gas_price * gas_used;
        let pays_gas = !(is_simulation && gas_payer == @0x0);
        aborts_if pays_gas && !spec_can_pay_gas(gas_payer, transaction_fee_amount);

        let burns = pays_gas && transaction_fee_amount > storage_fee_refunded;
        aborts_if burns && !exists<AptosFABurnCapabilities>(@aptos_framework);
        include burns ==> aptos_account::BurnFromAptStore {
            ref: global<AptosFABurnCapabilities>(@aptos_framework).burn_ref,
            account: gas_payer,
            amount: transaction_fee_amount - storage_fee_refunded,
        };

        let mints = pays_gas && transaction_fee_amount < storage_fee_refunded;
        aborts_if mints && !exists<AptosCoinMintCapability>(@aptos_framework)
            && !(features::spec_is_enabled(features::GAS_REFUND_FA_MINT)
                && exists<AptosFAMintCapabilities>(@aptos_framework));
    }

    spec schema IncrementSequenceNumber {
        use aptos_framework::account::{Account};
        addr: address;
        let sequence_number = if (exists<Account>(addr)) global<Account>(addr).sequence_number else 0;
        include account::EnsureResourceExistsAbortsIf;
        aborts_if sequence_number == MAX_U64;
        ensures exists<Account>(addr);
        ensures global<Account>(addr).sequence_number == sequence_number + 1;
    }

    spec epilogue_gas_payer(
        account: signer,
        gas_payer: address,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf { is_simulation: false };
        include IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    spec unified_prologue(
        sender: signer,
        txn_sender_public_key: Option<vector<u8>>,
        txn_sequence_number: u64,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
    ) {
        // Partial: the secondary signer checks and the account abstraction authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: sender,
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: txn_sender_public_key,
        };
        aborts_if len(secondary_signer_public_key_hashes) != len(secondary_signer_addresses);
    }

    spec unified_prologue_fee_payer(
        sender: signer,
        fee_payer: signer,
        txn_sender_public_key: Option<vector<u8>>,
        fee_payer_public_key_hash: Option<vector<u8>>,
        txn_sequence_number: u64,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
    ) {
        // Partial: the secondary signer checks and the account abstraction authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: fee_payer,
            replay_protector: ReplayProtector::SequenceNumber(txn_sequence_number),
            txn_authentication_key: txn_sender_public_key,
        };
        aborts_if len(secondary_signer_public_key_hashes) != len(secondary_signer_addresses);
        include FeePayerAuthKeyAbortsIf;
    }

    spec unified_epilogue(
        account: signer,
        gas_payer: signer,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf { gas_payer: signer::address_of(gas_payer) };
        include IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    spec unified_prologue_v2(
        sender: signer,
        txn_sender_public_key: Option<vector<u8>>,
        replay_protector: ReplayProtector,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
    ) {
        // Partial: the aborts of the nonce history update, the secondary signer checks and the account
        // abstraction authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf { gas_payer: sender, txn_authentication_key: txn_sender_public_key };
        aborts_if len(secondary_signer_public_key_hashes) != len(secondary_signer_addresses);
    }

    spec unified_prologue_fee_payer_v2(
        sender: signer,
        fee_payer: signer,
        txn_sender_public_key: Option<vector<u8>>,
        fee_payer_public_key_hash: Option<vector<u8>>,
        replay_protector: ReplayProtector,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<Option<vector<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
    ) {
        // Partial: the aborts of the nonce history update, the secondary signer checks and the account
        // abstraction authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf { gas_payer: fee_payer, txn_authentication_key: txn_sender_public_key };
        aborts_if len(secondary_signer_public_key_hashes) != len(secondary_signer_addresses);
        include FeePayerAuthKeyAbortsIf;
    }

    /// Unless skipped in simulation, the fee payer's key must match, or be absent with account abstraction enabled.
    spec schema FeePayerAuthKeyAbortsIf {
        fee_payer: signer;
        fee_payer_public_key_hash: Option<vector<u8>>;
        is_simulation: bool;
        let fee_payer_address = signer::address_of(fee_payer);
        let check_auth_key = !(is_simulation && (option::is_none(fee_payer_public_key_hash)
            || len(option::borrow(fee_payer_public_key_hash)) == 0));
        aborts_if check_auth_key && option::is_some(fee_payer_public_key_hash)
            && !account::spec_exists_at(fee_payer_address);
        aborts_if check_auth_key && option::is_some(fee_payer_public_key_hash)
            && option::borrow(fee_payer_public_key_hash) != account::spec_get_authentication_key(fee_payer_address);
        aborts_if check_auth_key && option::is_none(fee_payer_public_key_hash)
            && !features::spec_is_enabled(features::DERIVABLE_ACCOUNT_ABSTRACTION)
            && !features::spec_is_enabled(features::ACCOUNT_ABSTRACTION);
    }

    spec unified_epilogue_v2(
        account: signer,
        gas_payer: signer,
        storage_fee_refunded: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
        is_orderless_txn: bool,
    ) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf { gas_payer: signer::address_of(gas_payer) };
        include !is_orderless_txn ==> IncrementSequenceNumber { addr: signer::address_of(account) };
    }

    spec versioned_prologue(sender: signer, fee_payer: signer, args: PrologueArgs) {
        // Partial: the aborts of the nonce history update, the transaction limits check, the secondary signer
        // checks and the account abstraction authenticator lookup are not specified.
        pragma aborts_if_is_partial;
        include PrologueCommonAbortsIf {
            gas_payer: fee_payer,
            replay_protector: args.replay_protector,
            txn_authentication_key: args.txn_sender_public_key,
            txn_gas_price: args.txn_gas_price,
            txn_max_gas_units: args.txn_max_gas_units,
            txn_expiration_time: args.txn_expiration_time,
            chain_id: args.chain_id,
            is_simulation: args.is_simulation,
        };
        aborts_if len(args.secondary_signer_public_key_hashes) != len(args.secondary_signer_addresses);
        include args.needs_fee_payer_auth_check ==> FeePayerAuthKeyAbortsIf {
            fee_payer_public_key_hash: args.fee_payer_public_key_hash,
            is_simulation: args.is_simulation,
        };
    }

    spec versioned_epilogue(sender: signer, fee_payer: signer, args: EpilogueArgs) {
        // Partial: the aborts of minting the refund are not specified.
        pragma aborts_if_is_partial;
        include EpilogueGasChargeAbortsIf {
            gas_payer: signer::address_of(fee_payer),
            storage_fee_refunded: args.fee_statement.storage_fee_refund_octas,
            txn_gas_price: args.txn_gas_price,
            txn_max_gas_units: args.txn_max_gas_units,
            gas_units_remaining: args.gas_units_remaining,
            is_simulation: args.is_simulation,
        };
        include !args.is_orderless_txn ==> IncrementSequenceNumber { addr: signer::address_of(sender) };
    }

    spec schema EpilogueGasPayerAbortsIf {
        use std::option;
        use aptos_std::type_info;
        use aptos_framework::account::{Account};
        use aptos_framework::aptos_coin::{AptosCoin};
        use aptos_framework::coin;
        use aptos_framework::coin::{CoinStore, CoinInfo};
        use aptos_framework::optional_aggregator;
        use aptos_framework::transaction_fee::{AptosCoinCapabilities, AptosCoinMintCapability};

        account: signer;
        gas_payer: address;
        storage_fee_refunded: u64;
        txn_gas_price: u64;
        txn_max_gas_units: u64;
        gas_units_remaining: u64;

        // Check transaction invariants.
        aborts_if !(txn_max_gas_units >= gas_units_remaining);
        let gas_used = txn_max_gas_units - gas_units_remaining;
        aborts_if !(txn_gas_price * gas_used <= MAX_U64);
        let transaction_fee_amount = txn_gas_price * gas_used;

        // Check account invariants.
        let addr = signer::address_of(account);
        // TODO(fa_migration)
        // let pre_balance = global<coin::CoinStore<AptosCoin>>(gas_payer).coin.value;
        // let post balance = global<coin::CoinStore<AptosCoin>>(gas_payer).coin.value;
        let pre_account = global<account::Account>(addr);
        let post account = global<account::Account>(addr);

        aborts_if !exists<CoinStore<AptosCoin>>(gas_payer);
        aborts_if !exists<Account>(addr);
        aborts_if !(global<Account>(addr).sequence_number < MAX_U64);
        // aborts_if pre_balance < transaction_fee_amount;
        // ensures balance == pre_balance - transaction_fee_amount + storage_fee_refunded;
        ensures account.sequence_number == pre_account.sequence_number + 1;

        // Check burning.
        //   (Check the total supply aggregator when enabled.)
        let amount_to_burn = transaction_fee_amount - storage_fee_refunded;
        let apt_addr = type_info::type_of<AptosCoin>().account_address;
        let maybe_apt_supply = global<CoinInfo<AptosCoin>>(apt_addr).supply;
        let total_supply_enabled = option::is_some(maybe_apt_supply);
        let apt_supply = option::borrow(maybe_apt_supply);
        let apt_supply_value = optional_aggregator::optional_aggregator_value(apt_supply);
        let post post_maybe_apt_supply = global<CoinInfo<AptosCoin>>(apt_addr).supply;
        let post post_apt_supply = option::borrow(post_maybe_apt_supply);
        let post post_apt_supply_value = optional_aggregator::optional_aggregator_value(post_apt_supply);

        aborts_if amount_to_burn > 0 && !exists<AptosCoinCapabilities>(@aptos_framework);
        aborts_if amount_to_burn > 0 && !exists<CoinInfo<AptosCoin>>(apt_addr);
        aborts_if amount_to_burn > 0 && total_supply_enabled && apt_supply_value < amount_to_burn;
        ensures total_supply_enabled ==> apt_supply_value - amount_to_burn == post_apt_supply_value;

        // Check minting.
        let amount_to_mint = storage_fee_refunded - transaction_fee_amount;
        let total_supply = coin::supply<AptosCoin>;
        let post post_total_supply = coin::supply<AptosCoin>;

        aborts_if amount_to_mint > 0 && !exists<CoinStore<AptosCoin>>(addr);
        aborts_if amount_to_mint > 0 && !exists<AptosCoinMintCapability>(@aptos_framework);
        aborts_if amount_to_mint > 0 && total_supply + amount_to_mint > MAX_U128;
        ensures amount_to_mint > 0 ==> post_total_supply == total_supply + amount_to_mint;

        let aptos_addr = type_info::type_of<AptosCoin>().account_address;
        aborts_if (amount_to_mint != 0) && !exists<coin::CoinInfo<AptosCoin>>(aptos_addr);
        include coin::CoinAddAbortsIf<AptosCoin> { amount: amount_to_mint };
    }

    spec metered_prologue(sender: signer, _fee_payer: signer, args: MeteredPrologueArgs) {
        include CreateSenderAccountIfNeeded { sender, replay_protector: args.replay_protector };
    }

    spec create_sender_account_if_needed(sender: &signer, replay_protector: ReplayProtector) {
        include CreateSenderAccountIfNeeded;
    }

    /// A transaction with sequence number 0 creates the sender's `Account` resource if it is missing.
    spec schema CreateSenderAccountIfNeeded {
        use std::bcs;
        use aptos_framework::account::{Account};
        sender: signer;
        replay_protector: ReplayProtector;
        let addr = signer::address_of(sender);
        let creates = (replay_protector is ReplayProtector::SequenceNumber) && replay_protector.0 == 0;
        aborts_if creates && !exists<Account>(addr) && (
            addr == @vm_reserved || addr == @aptos_framework || addr == @aptos_token
                || len(bcs::to_bytes(addr)) != 32
        );
        ensures creates ==> exists<Account>(addr);
    }
}
