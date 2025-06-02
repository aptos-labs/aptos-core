/// This module implements the Confidential Asset (CA) Standard, a privacy-focused protocol for managing fungible assets (FA).
/// It enables private transfers by obfuscating token amounts while keeping sender and recipient addresses visible.
module aptos_experimental::confidential_asset {
    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::Self;
    use aptos_std::ristretto255_bulletproofs::Self as bulletproofs;
    use aptos_std::string_utils;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::event;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::{Metadata};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;

    use aptos_experimental::confidential_balance;
    use aptos_experimental::confidential_proof::{
        Self, NormalizationProof, RotationProof, TransferProof, WithdrawalProof
    };
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    #[test_only]
    use aptos_std::ristretto255::Scalar;

    //
    // Errors
    //

    /// The range proof system does not support sufficient range.
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE: u64 = 1;

    /// The confidential asset store has already been published for the given user-token pair.
    const ECA_STORE_ALREADY_PUBLISHED: u64 = 2;

    /// The confidential asset store has not been published for the given user-token pair.
    const ECA_STORE_NOT_PUBLISHED: u64 = 3;

    /// The deserialization of the auditor EK failed.
    const EAUDITOR_EK_DESERIALIZATION_FAILED: u64 = 4;

    /// The sender is not the registered auditor.
    const ENOT_AUDITOR: u64 = 5;

    /// The provided auditors or auditor proofs are invalid.
    const EINVALID_AUDITORS: u64 = 6;

    /// The confidential asset account is already frozen.
    const EALREADY_FROZEN: u64 = 7;

    /// The confidential asset account is not frozen.
    const ENOT_FROZEN: u64 = 8;

    /// The pending balance must be zero for this operation.
    const ENOT_ZERO_BALANCE: u64 = 9;

    /// The operation requires the actual balance to be normalized.
    const ENORMALIZATION_REQUIRED: u64 = 10;

    /// The balance is already normalized and cannot be normalized again.
    const EALREADY_NORMALIZED: u64 = 11;

    /// The token is already allowed for confidential transfers.
    const ETOKEN_ENABLED: u64 = 12;

    /// The token is not allowed for confidential transfers.
    const ETOKEN_DISABLED: u64 = 13;

    /// The allow list is already enabled.
    const EALLOW_LIST_ENABLED: u64 = 14;

    /// The allow list is already disabled.
    const EALLOW_LIST_DISABLED: u64 = 15;

    /// An internal error occurred, indicating unexpected behavior.
    const EINTERNAL_ERROR: u64 = 16;

    //
    // Constants
    //

    /// The maximum number of transactions can be aggregated on the pending balance before rollover is required.
    const MAX_TRANSFERS_BEFORE_ROLLOVER: u64 = 65534;

    /// The mainnet chain ID. If the chain ID is 1, the allow list is enabled.
    const MAINNET_CHAIN_ID: u8 = 1;

    //
    // Structs
    //

    /// The `confidential_asset` module stores a `ConfidentialAssetStore` object for each user-token pair.
    struct ConfidentialAssetStore has key {
        /// Indicates if the account is frozen. If `true`, transactions are temporarily disabled
        /// for this account. This is particularly useful during key rotations, which require
        /// two transactions: rolling over the pending balance to the actual balance and rotating
        /// the encryption key. Freezing prevents the user from accepting additional payments
        /// between these two transactions.
        frozen: bool,

        /// A flag indicating whether the actual balance is normalized. A normalized balance
        /// ensures that all chunks fit within the defined 16-bit bounds, preventing overflows.
        normalized: bool,

        /// Tracks the maximum number of transactions the user can accept before normalization
        /// is required. For example, if the user can accept up to 2^16 transactions and each
        /// chunk has a 16-bit limit, the maximum chunk value before normalization would be
        /// 2^16 * 2^16 = 2^32. Maintaining this counter is crucial because users must solve
        /// a discrete logarithm problem of this size to decrypt their balances.
        pending_counter: u64,

        /// Stores the user's pending balance, which is used for accepting incoming payments.
        /// Represented as four 16-bit chunks (p0 + 2^16 * p1 + 2^32 * p2 + 2^48 * p3), that can grow up to 32 bits.
        /// All payments are accepted into this pending balance, which users must roll over into the actual balance
        /// to perform transactions like withdrawals or transfers.
        /// This separation helps protect against front-running attacks, where small incoming transfers could force
        /// frequent regenerating of zk-proofs.
        pending_balance: confidential_balance::CompressedConfidentialBalance,

        /// Represents the actual user balance, which is available for sending payments.
        /// It consists of eight 16-bit chunks (p0 + 2^16 * p1 + ... + 2^112 * p8), supporting a 128-bit balance.
        /// Users can decrypt this balance with their decryption keys and by solving a discrete logarithm problem.
        actual_balance: confidential_balance::CompressedConfidentialBalance,

        /// The encryption key associated with the user's confidential asset account, different for each token.
        ek: twisted_elgamal::CompressedPubkey,
    }

    /// Represents the controller for the primary FA stores and `FAConfig` objects.
    struct FAController has key {
        /// Indicates whether the allow list is enabled. If `true`, only tokens from the allow list can be transferred.
        /// This flag is managed by the governance module.
        allow_list_enabled: bool,

        /// Used to derive a signer that owns all the FAs' primary stores and `FAConfig` objects.
        extend_ref: ExtendRef
    }

    /// Represents the configuration of a token.
    struct FAConfig has key {
        /// Indicates whether the token is allowed for confidential transfers.
        /// If allow list is disabled, all tokens are allowed.
        /// Can be toggled by the governance module. The withdrawals are always allowed.
        allowed: bool,

        /// The auditor's public key for the token. If the auditor is not set, this field is `None`.
        /// Otherwise, each confidential transfer must include the auditor as an additional party,
        /// alongside the recipient, who has access to the decrypted transferred amount.
        auditor_ek: Option<twisted_elgamal::CompressedPubkey>,
    }

    //
    // Events
    //

    #[event]
    /// Emitted when tokens are brought into the protocol.
    struct Deposited has drop, store {
        from: address,
        to: address,
        amount: u64
    }

    #[event]
    /// Emitted when tokens are brought out of the protocol.
    struct Withdrawn has drop, store {
        from: address,
        to: address,
        amount: u64
    }

    #[event]
    /// Emitted when tokens are transferred within the protocol between users' confidential balances.
    /// Note that a numeric amount is not included, as it is hidden.
    struct Transferred has drop, store {
        from: address,
        to: address
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    fun init_module(deployer: &signer) {
        assert!(
            bulletproofs::get_max_range_bits() >= confidential_proof::get_bulletproofs_num_bits(),
            error::internal(ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        let deployer_address = signer::address_of(deployer);

        let fa_controller_ctor_ref = &object::create_object(deployer_address);

        move_to(deployer, FAController {
            allow_list_enabled: chain_id::get() == MAINNET_CHAIN_ID,
            extend_ref: object::generate_extend_ref(fa_controller_ctor_ref),
        });
    }

    //
    // Entry functions
    //

    /// Registers an account for a specified token. Users must register an account for each token they
    /// intend to transact with.
    ///
    /// Users are also responsible for generating a Twisted ElGamal key pair on their side.
    public entry fun register(
        sender: &signer,
        token: Object<Metadata>,
        ek: vector<u8>) acquires FAController, FAConfig
    {
        let ek = twisted_elgamal::new_pubkey_from_bytes(ek).extract();

        register_internal(sender, token, ek);
    }

    /// Brings tokens into the protocol, transferring the passed amount from the sender's primary FA store
    /// to the pending balance of the recipient.
    /// The initial confidential balance is publicly visible, as entering the protocol requires a normal transfer.
    /// However, tokens within the protocol become obfuscated through confidential transfers, ensuring privacy in
    /// subsequent transactions.
    public entry fun deposit_to(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        amount: u64) acquires ConfidentialAssetStore, FAController, FAConfig
    {
        deposit_to_internal(sender, token, to, amount)
    }

    /// The same as `deposit_to`, but the recipient is the sender.
    public entry fun deposit(
        sender: &signer,
        token: Object<Metadata>,
        amount: u64) acquires ConfidentialAssetStore, FAController, FAConfig
    {
        deposit_to_internal(sender, token, signer::address_of(sender), amount)
    }

    /// The same as `deposit_to`, but converts coins to missing FA first.
    public entry fun deposit_coins_to<CoinType>(
        sender: &signer,
        to: address,
        amount: u64) acquires ConfidentialAssetStore, FAController, FAConfig
    {
        let token = ensure_sufficient_fa<CoinType>(sender, amount).extract();

        deposit_to_internal(sender, token, to, amount)
    }

    /// The same as `deposit`, but converts coins to missing FA first.
    public entry fun deposit_coins<CoinType>(
        sender: &signer,
        amount: u64) acquires ConfidentialAssetStore, FAController, FAConfig
    {
        let token = ensure_sufficient_fa<CoinType>(sender, amount).extract();

        deposit_to_internal(sender, token, signer::address_of(sender), amount)
    }

    /// Brings tokens out of the protocol by transferring the specified amount from the sender's actual balance to
    /// the recipient's primary FA store.
    /// The withdrawn amount is publicly visible, as this process requires a normal transfer.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    public entry fun withdraw_to(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        amount: u64,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>) acquires ConfidentialAssetStore, FAController
    {
        let new_balance = confidential_balance::new_actual_balance_from_bytes(new_balance).extract();
        let proof = confidential_proof::deserialize_withdrawal_proof(sigma_proof, zkrp_new_balance).extract();

        withdraw_to_internal(sender, token, to, amount, new_balance, proof);

        event::emit(Withdrawn { from: signer::address_of(sender), to, amount });
    }

    /// The same as `withdraw_to`, but the recipient is the sender.
    public entry fun withdraw(
        sender: &signer,
        token: Object<Metadata>,
        amount: u64,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>) acquires ConfidentialAssetStore, FAController
    {
        withdraw_to(
            sender,
            token,
            signer::address_of(sender),
            amount,
            new_balance,
            zkrp_new_balance,
            sigma_proof
        )
    }

    /// Transfers tokens from the sender's actual balance to the recipient's pending balance.
    /// The function hides the transferred amount while keeping the sender and recipient addresses visible.
    /// The sender encrypts the transferred amount with the recipient's encryption key and the function updates the
    /// recipient's confidential balance homomorphically.
    /// Additionally, the sender encrypts the transferred amount with the auditors' EKs, allowing auditors to decrypt
    /// the it on their side.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    /// Warning: If the auditor feature is enabled, the sender must include the auditor as the first element in the
    /// `auditor_eks` vector.
    public entry fun confidential_transfer(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        new_balance: vector<u8>,
        transfer_amount: vector<u8>,
        auditor_eks: vector<u8>,
        auditor_amounts: vector<u8>,
        zkrp_new_balance: vector<u8>,
        zkrp_transfer_amount: vector<u8>,
        sigma_proof: vector<u8>) acquires ConfidentialAssetStore, FAConfig, FAController
    {
        let new_balance = confidential_balance::new_actual_balance_from_bytes(new_balance).extract();
        let transfer_amount = confidential_balance::new_pending_balance_from_bytes(transfer_amount).extract();
        let auditor_eks = deserialize_auditor_eks(auditor_eks).extract();
        let auditor_amounts = deserialize_auditor_amounts(auditor_amounts).extract();
        let proof = confidential_proof::deserialize_transfer_proof(
            sigma_proof,
            zkrp_new_balance,
            zkrp_transfer_amount
        ).extract();

        confidential_transfer_internal(
            sender,
            token,
            to,
            new_balance,
            transfer_amount,
            auditor_eks,
            auditor_amounts,
            proof
        )
    }

    /// Rotates the encryption key for the user's confidential balance, updating it to a new encryption key.
    /// The function ensures that the pending balance is zero before the key rotation, requiring the sender to
    /// call `rollover_pending_balance_and_freeze` beforehand if necessary.
    /// The sender provides their new normalized confidential balance, encrypted with the new encryption key and fresh randomness
    /// to preserve privacy.
    public entry fun rotate_encryption_key(
        sender: &signer,
        token: Object<Metadata>,
        new_ek: vector<u8>,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>) acquires ConfidentialAssetStore
    {
        let new_ek = twisted_elgamal::new_pubkey_from_bytes(new_ek).extract();
        let new_balance = confidential_balance::new_actual_balance_from_bytes(new_balance).extract();
        let proof = confidential_proof::deserialize_rotation_proof(sigma_proof, zkrp_new_balance).extract();

        rotate_encryption_key_internal(sender, token, new_ek, new_balance, proof);
    }

    /// Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
    /// Most functions perform implicit normalization by accepting a new normalized confidential balance as a parameter.
    /// However, explicit normalization is required before rolling over the pending balance, as multiple rolls may cause
    /// chunk overflows.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    public entry fun normalize(
        sender: &signer,
        token: Object<Metadata>,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>) acquires ConfidentialAssetStore
    {
        let new_balance = confidential_balance::new_actual_balance_from_bytes(new_balance).extract();
        let proof = confidential_proof::deserialize_normalization_proof(sigma_proof, zkrp_new_balance).extract();

        normalize_internal(sender, token, new_balance, proof);
    }

    /// Freezes the confidential account for the specified token, disabling all incoming transactions.
    public entry fun freeze_token(sender: &signer, token: Object<Metadata>) acquires ConfidentialAssetStore {
        freeze_token_internal(sender, token);
    }

    /// Unfreezes the confidential account for the specified token, re-enabling incoming transactions.
    public entry fun unfreeze_token(sender: &signer, token: Object<Metadata>) acquires ConfidentialAssetStore {
        unfreeze_token_internal(sender, token);
    }

    /// Adds the pending balance to the actual balance for the specified token, resetting the pending balance to zero.
    /// This operation is necessary to use tokens from the pending balance for outgoing transactions.
    public entry fun rollover_pending_balance(
        sender: &signer,
        token: Object<Metadata>) acquires ConfidentialAssetStore
    {
        rollover_pending_balance_internal(sender, token);
    }

    /// Before calling `rotate_encryption_key`, we need to rollover the pending balance and freeze the token to prevent
    /// any new payments being come.
    public entry fun rollover_pending_balance_and_freeze(
        sender: &signer,
        token: Object<Metadata>) acquires ConfidentialAssetStore
    {
        rollover_pending_balance(sender, token);
        freeze_token(sender, token);
    }

    /// After rotating the encryption key, we may want to unfreeze the token to allow payments.
    /// This function facilitates making both calls in a single transaction.
    public entry fun rotate_encryption_key_and_unfreeze(
        sender: &signer,
        token: Object<Metadata>,
        new_ek: vector<u8>,
        new_confidential_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        rotate_proof: vector<u8>) acquires ConfidentialAssetStore
    {
        rotate_encryption_key(sender, token, new_ek, new_confidential_balance, zkrp_new_balance, rotate_proof);
        unfreeze_token(sender, token);
    }

    //
    // Public governance functions
    //

    /// Enables the allow list, restricting confidential transfers to tokens on the allow list.
    public fun enable_allow_list(aptos_framework: &signer) acquires FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_controller = borrow_global_mut<FAController>(@aptos_experimental);

        assert!(!fa_controller.allow_list_enabled, error::invalid_state(EALLOW_LIST_ENABLED));

        fa_controller.allow_list_enabled = true;
    }

    /// Disables the allow list, allowing confidential transfers for all tokens.
    public fun disable_allow_list(aptos_framework: &signer) acquires FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_controller = borrow_global_mut<FAController>(@aptos_experimental);

        assert!(fa_controller.allow_list_enabled, error::invalid_state(EALLOW_LIST_DISABLED));

        fa_controller.allow_list_enabled = false;
    }

    /// Enables confidential transfers for the specified token.
    public fun enable_token(aptos_framework: &signer, token: Object<Metadata>) acquires FAConfig, FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(ensure_fa_config_exists(token));

        assert!(!fa_config.allowed, error::invalid_state(ETOKEN_ENABLED));

        fa_config.allowed = true;
    }

    /// Disables confidential transfers for the specified token.
    public fun disable_token(aptos_framework: &signer, token: Object<Metadata>) acquires FAConfig, FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(ensure_fa_config_exists(token));

        assert!(fa_config.allowed, error::invalid_state(ETOKEN_DISABLED));

        fa_config.allowed = false;
    }

    /// Sets the auditor's public key for the specified token.
    public fun set_auditor(
        aptos_framework: &signer,
        token: Object<Metadata>,
        new_auditor_ek: vector<u8>) acquires FAConfig, FAController
    {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(ensure_fa_config_exists(token));

        fa_config.auditor_ek = if (new_auditor_ek.length() == 0) {
            std::option::none()
        } else {
            let new_auditor_ek = twisted_elgamal::new_pubkey_from_bytes(new_auditor_ek);
            assert!(new_auditor_ek.is_some(), error::invalid_argument(EAUDITOR_EK_DESERIALIZATION_FAILED));
            new_auditor_ek
        };
    }

    //
    // Public view functions
    //

    #[view]
    /// Checks if the user has a confidential asset store for the specified token.
    public fun has_confidential_asset_store(user: address, token: Object<Metadata>): bool {
        exists<ConfidentialAssetStore>(get_user_address(user, token))
    }

    #[view]
    /// Checks if the token is allowed for confidential transfers.
    public fun is_token_allowed(token: Object<Metadata>): bool acquires FAController, FAConfig {
        if (!is_allow_list_enabled()) {
            return true
        };

        let fa_config_address = get_fa_config_address(token);

        if (!exists<FAConfig>(fa_config_address)) {
            return false
        };

        borrow_global<FAConfig>(fa_config_address).allowed
    }

    #[view]
    /// Checks if the allow list is enabled.
    /// If the allow list is enabled, only tokens from the allow list can be transferred.
    /// Otherwise, all tokens are allowed.
    public fun is_allow_list_enabled(): bool acquires FAController {
        borrow_global<FAController>(@aptos_experimental).allow_list_enabled
    }

    #[view]
    /// Returns the pending balance of the user for the specified token.
    public fun pending_balance(
        owner: address,
        token: Object<Metadata>): confidential_balance::CompressedConfidentialBalance acquires ConfidentialAssetStore
    {
        assert!(has_confidential_asset_store(owner, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        let ca_store = borrow_global<ConfidentialAssetStore>(get_user_address(owner, token));

        ca_store.pending_balance
    }

    #[view]
    /// Returns the actual balance of the user for the specified token.
    public fun actual_balance(
        owner: address,
        token: Object<Metadata>): confidential_balance::CompressedConfidentialBalance acquires ConfidentialAssetStore
    {
        assert!(has_confidential_asset_store(owner, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        let ca_store = borrow_global<ConfidentialAssetStore>(get_user_address(owner, token));

        ca_store.actual_balance
    }

    #[view]
    /// Returns the encryption key (EK) of the user for the specified token.
    public fun encryption_key(
        user: address,
        token: Object<Metadata>): twisted_elgamal::CompressedPubkey acquires ConfidentialAssetStore
    {
        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token)).ek
    }

    #[view]
    /// Checks if the user's actual balance is normalized for the specified token.
    public fun is_normalized(user: address, token: Object<Metadata>): bool acquires ConfidentialAssetStore {
        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        borrow_global<ConfidentialAssetStore>(get_user_address(user, token)).normalized
    }

    #[view]
    /// Checks if the user's confidential asset store is frozen for the specified token.
    public fun is_frozen(user: address, token: Object<Metadata>): bool acquires ConfidentialAssetStore {
        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        borrow_global<ConfidentialAssetStore>(get_user_address(user, token)).frozen
    }

    #[view]
    /// Returns the asset-specific auditor's encryption key.
    /// If the auditing feature is disabled for the token, the encryption key is set to `None`.
    public fun get_auditor(
        token: Object<Metadata>): Option<twisted_elgamal::CompressedPubkey> acquires FAConfig, FAController
    {
        let fa_config_address = get_fa_config_address(token);

        if (!is_allow_list_enabled() && !exists<FAConfig>(fa_config_address)) {
            return std::option::none();
        };

        borrow_global<FAConfig>(fa_config_address).auditor_ek
    }

    #[view]
    /// Returns the circulating supply of the confidential asset.
    public fun confidential_asset_balance(token: Object<Metadata>): u64 acquires FAController {
        let fa_store_address = get_fa_store_address();
        assert!(primary_fungible_store::primary_store_exists(fa_store_address, token), EINTERNAL_ERROR);

        primary_fungible_store::balance(fa_store_address, token)
    }

    #[view]
    /// Checks if the user has a confidential asset store for the specified token.
    public fun get_pending_balance_transfer_count(user: address, token: Object<Metadata>): u64 acquires ConfidentialAssetStore {
        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        borrow_global<ConfidentialAssetStore>(get_user_address(user, token)).pending_counter
    }

    //
    // Public functions that correspond to the entry functions and don't require serializtion of the input data.
    // These function can be useful for external contracts that want to integrate with the Confidential Asset protocol.
    //

    /// Implementation of the `register` entry function.
    public fun register_internal(
        sender: &signer,
        token: Object<Metadata>,
        ek: twisted_elgamal::CompressedPubkey) acquires FAController, FAConfig
    {
        assert!(is_token_allowed(token), error::invalid_argument(ETOKEN_DISABLED));

        let user = signer::address_of(sender);

        assert!(!has_confidential_asset_store(user, token), error::already_exists(ECA_STORE_ALREADY_PUBLISHED));

        let ca_store = ConfidentialAssetStore {
            frozen: false,
            normalized: true,
            pending_counter: 0,
            pending_balance: confidential_balance::new_compressed_pending_balance_no_randomness(),
            actual_balance: confidential_balance::new_compressed_actual_balance_no_randomness(),
            ek,
        };

        move_to(&get_user_signer(sender, token), ca_store);
    }

    /// Implementation of the `deposit_to` entry function.
    public fun deposit_to_internal(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        amount: u64) acquires ConfidentialAssetStore, FAController, FAConfig
    {
        assert!(is_token_allowed(token), error::invalid_argument(ETOKEN_DISABLED));
        assert!(!is_frozen(to, token), error::invalid_state(EALREADY_FROZEN));

        let from = signer::address_of(sender);

        let sender_fa_store = primary_fungible_store::ensure_primary_store_exists(from, token);
        let ca_fa_store = primary_fungible_store::ensure_primary_store_exists(get_fa_store_address(), token);

        dispatchable_fungible_asset::transfer(sender, sender_fa_store, ca_fa_store, amount);

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(to, token));
        let pending_balance = confidential_balance::decompress_balance(&ca_store.pending_balance);

        confidential_balance::add_balances_mut(
            &mut pending_balance,
            &confidential_balance::new_pending_balance_u64_no_randonmess(amount)
        );

        ca_store.pending_balance = confidential_balance::compress_balance(&pending_balance);

        assert!(
            ca_store.pending_counter < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_argument(EINTERNAL_ERROR)
        );

        ca_store.pending_counter += 1;

        event::emit(Deposited { from, to, amount });
    }

    /// Implementation of the `withdraw_to` entry function.
    /// Withdrawals are always allowed, regardless of the token allow status.
    public fun withdraw_to_internal(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        amount: u64,
        new_balance: confidential_balance::ConfidentialBalance,
        proof: WithdrawalProof) acquires ConfidentialAssetStore, FAController
    {
        let from = signer::address_of(sender);

        let sender_ek = encryption_key(from, token);

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(from, token));
        let current_balance = confidential_balance::decompress_balance(&ca_store.actual_balance);

        confidential_proof::verify_withdrawal_proof(&sender_ek, amount, &current_balance, &new_balance, &proof);

        ca_store.normalized = true;
        ca_store.actual_balance = confidential_balance::compress_balance(&new_balance);

        primary_fungible_store::transfer(&get_fa_store_signer(), token, to, amount);
    }

    /// Implementation of the `confidential_transfer` entry function.
    public fun confidential_transfer_internal(
        sender: &signer,
        token: Object<Metadata>,
        to: address,
        new_balance: confidential_balance::ConfidentialBalance,
        transfer_amount: confidential_balance::ConfidentialBalance,
        auditor_eks: vector<twisted_elgamal::CompressedPubkey>,
        auditor_amounts: vector<confidential_balance::ConfidentialBalance>,
        proof: TransferProof) acquires ConfidentialAssetStore, FAConfig, FAController
    {
        assert!(is_token_allowed(token), error::invalid_argument(ETOKEN_DISABLED));
        assert!(!is_frozen(to, token), error::invalid_state(EALREADY_FROZEN));
        assert!(
            validate_auditors(token, &transfer_amount, &auditor_eks, &auditor_amounts, &proof),
            error::invalid_argument(EINVALID_AUDITORS)
        );

        let from = signer::address_of(sender);

        let sender_ek = encryption_key(from, token);
        let recipient_ek = encryption_key(to, token);

        let sender_ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(from, token));

        let sender_current_actual_balance = confidential_balance::decompress_balance(
            &sender_ca_store.actual_balance
        );

        confidential_proof::verify_transfer_proof(
            &sender_ek,
            &recipient_ek,
            &sender_current_actual_balance,
            &new_balance,
            &transfer_amount,
            &auditor_eks,
            &auditor_amounts,
            &proof);

        sender_ca_store.normalized = true;
        sender_ca_store.actual_balance = confidential_balance::compress_balance(&new_balance);

        // Cannot create multiple mutable references to the same type, so we need to drop it
        let ConfidentialAssetStore { .. } = sender_ca_store;

        let recipient_ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(to, token));

        assert!(
            recipient_ca_store.pending_counter < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_argument(EINTERNAL_ERROR)
        );

        let recipient_pending_balance = confidential_balance::decompress_balance(
            &recipient_ca_store.pending_balance
        );
        confidential_balance::add_balances_mut(&mut recipient_pending_balance, &transfer_amount);

        recipient_ca_store.pending_counter += 1;
        recipient_ca_store.pending_balance = confidential_balance::compress_balance(&recipient_pending_balance);

        event::emit(Transferred { from, to });
    }

    /// Implementation of the `rotate_encryption_key` entry function.
    public fun rotate_encryption_key_internal(
        sender: &signer,
        token: Object<Metadata>,
        new_ek: twisted_elgamal::CompressedPubkey,
        new_balance: confidential_balance::ConfidentialBalance,
        proof: RotationProof) acquires ConfidentialAssetStore
    {
        let user = signer::address_of(sender);
        let current_ek = encryption_key(user, token);

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token));

        let pending_balance = confidential_balance::decompress_balance(&ca_store.pending_balance);

        // We need to ensure that the pending balance is zero before rotating the key.
        // To guarantee this, the user must call `rollover_pending_balance_and_freeze` beforehand.
        assert!(confidential_balance::is_zero_balance(&pending_balance), error::invalid_state(ENOT_ZERO_BALANCE));

        let current_balance = confidential_balance::decompress_balance(&ca_store.actual_balance);

        confidential_proof::verify_rotation_proof(&current_ek, &new_ek, &current_balance, &new_balance, &proof);

        ca_store.ek = new_ek;
        // We don't need to update the pending balance here, as it has been asserted to be zero.
        ca_store.actual_balance = confidential_balance::compress_balance(&new_balance);
        ca_store.normalized = true;
    }

    /// Implementation of the `normalize` entry function.
    public fun normalize_internal(
        sender: &signer,
        token: Object<Metadata>,
        new_balance: confidential_balance::ConfidentialBalance,
        proof: NormalizationProof) acquires ConfidentialAssetStore
    {
        let user = signer::address_of(sender);
        let sender_ek = encryption_key(user, token);

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token));

        assert!(!ca_store.normalized, error::invalid_state(EALREADY_NORMALIZED));

        let current_balance = confidential_balance::decompress_balance(&ca_store.actual_balance);

        confidential_proof::verify_normalization_proof(&sender_ek, &current_balance, &new_balance, &proof);

        ca_store.actual_balance = confidential_balance::compress_balance(&new_balance);
        ca_store.normalized = true;
    }

    /// Implementation of the `rollover_pending_balance` entry function.
    public fun rollover_pending_balance_internal(
        sender: &signer,
        token: Object<Metadata>) acquires ConfidentialAssetStore
    {
        let user = signer::address_of(sender);

        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token));

        assert!(ca_store.normalized, error::invalid_state(ENORMALIZATION_REQUIRED));

        let actual_balance = confidential_balance::decompress_balance(&ca_store.actual_balance);
        let pending_balance = confidential_balance::decompress_balance(&ca_store.pending_balance);

        confidential_balance::add_balances_mut(&mut actual_balance, &pending_balance);

        ca_store.normalized = false;
        ca_store.pending_counter = 0;
        ca_store.actual_balance = confidential_balance::compress_balance(&actual_balance);
        ca_store.pending_balance = confidential_balance::new_compressed_pending_balance_no_randomness();
    }

    /// Implementation of the `freeze_token` entry function.
    public fun freeze_token_internal(
        sender: &signer,
        token: Object<Metadata>) acquires ConfidentialAssetStore
    {
        let user = signer::address_of(sender);

        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token));

        assert!(!ca_store.frozen, error::invalid_state(EALREADY_FROZEN));

        ca_store.frozen = true;
    }

    /// Implementation of the `unfreeze_token` entry function.
    public fun unfreeze_token_internal(
        sender: &signer,
        token: Object<Metadata>) acquires ConfidentialAssetStore
    {
        let user = signer::address_of(sender);

        assert!(has_confidential_asset_store(user, token), error::not_found(ECA_STORE_NOT_PUBLISHED));

        let ca_store = borrow_global_mut<ConfidentialAssetStore>(get_user_address(user, token));

        assert!(ca_store.frozen, error::invalid_state(ENOT_FROZEN));

        ca_store.frozen = false;
    }

    //
    // Private functions.
    //

    /// Ensures that the `FAConfig` object exists for the specified token.
    /// If the object does not exist, creates it.
    /// Used only for internal purposes.
    fun ensure_fa_config_exists(token: Object<Metadata>): address acquires FAController {
        let fa_config_address = get_fa_config_address(token);

        if (!exists<FAConfig>(fa_config_address)) {
            let fa_config_singer = get_fa_config_signer(token);

            move_to(&fa_config_singer, FAConfig {
                allowed: false,
                auditor_ek: std::option::none(),
            });
        };

        fa_config_address
    }

    /// Returns an object for handling all the FA primary stores, and returns a signer for it.
    fun get_fa_store_signer(): signer acquires FAController {
        object::generate_signer_for_extending(&borrow_global<FAController>(@aptos_experimental).extend_ref)
    }

    /// Returns the address that handles all the FA primary stores.
    fun get_fa_store_address(): address acquires FAController {
        object::address_from_extend_ref(&borrow_global<FAController>(@aptos_experimental).extend_ref)
    }

    /// Returns an object for handling the `ConfidentialAssetStore` and returns a signer for it.
    fun get_user_signer(user: &signer, token: Object<Metadata>): signer {
        let user_ctor = &object::create_named_object(user, construct_user_seed(token));

        object::generate_signer(user_ctor)
    }

    /// Returns the address that handles the user's `ConfidentialAssetStore` object for the specified user and token.
    fun get_user_address(user: address, token: Object<Metadata>): address {
        object::create_object_address(&user, construct_user_seed(token))
    }

    /// Returns an object for handling the `FAConfig`, and returns a signer for it.
    fun get_fa_config_signer(token: Object<Metadata>): signer acquires FAController {
        let fa_ext = &borrow_global<FAController>(@aptos_experimental).extend_ref;
        let fa_ext_signer = object::generate_signer_for_extending(fa_ext);

        let fa_ctor = &object::create_named_object(&fa_ext_signer, construct_fa_seed(token));

        object::generate_signer(fa_ctor)
    }

    /// Returns the address that handles primary FA store and `FAConfig` objects for the specified token.
    fun get_fa_config_address(token: Object<Metadata>): address acquires FAController {
        let fa_ext = &borrow_global<FAController>(@aptos_experimental).extend_ref;
        let fa_ext_address = object::address_from_extend_ref(fa_ext);

        object::create_object_address(&fa_ext_address, construct_fa_seed(token))
    }

    /// Constructs a unique seed for the user's `ConfidentialAssetStore` object.
    /// As all the `ConfidentialAssetStore`'s have the same type, we need to differentiate them by the seed.
    fun construct_user_seed(token: Object<Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::token::{}::user",
                @aptos_experimental,
                object::object_address(&token)
            )
        )
    }

    /// Constructs a unique seed for the FA's `FAConfig` object.
    /// As all the `FAConfig`'s have the same type, we need to differentiate them by the seed.
    fun construct_fa_seed(token: Object<Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::token::{}::fa",
                @aptos_experimental,
                object::object_address(&token)
            )
        )
    }

    /// Validates that the auditor-related fields in the confidential transfer are correct.
    /// Returns `false` if the transfer amount is not the same as the auditor amounts.
    /// Returns `false` if the number of auditors in the transfer proof and auditor lists do not match.
    /// Returns `false` if the first auditor in the list and the asset-specific auditor do not match.
    /// Note: If the asset-specific auditor is not set, the validation is successful for any list of auditors.
    /// Otherwise, returns `true`.
    fun validate_auditors(
        token: Object<Metadata>,
        transfer_amount: &confidential_balance::ConfidentialBalance,
        auditor_eks: &vector<twisted_elgamal::CompressedPubkey>,
        auditor_amounts: &vector<confidential_balance::ConfidentialBalance>,
        proof: &TransferProof): bool acquires FAConfig, FAController
    {
        if (
            !auditor_amounts.all(|auditor_amount| {
                confidential_balance::balance_c_equals(transfer_amount, auditor_amount)
            })
        ) {
            return false
        };

        if (
            auditor_eks.length() != auditor_amounts.length() ||
                auditor_eks.length() != confidential_proof::auditors_count_in_transfer_proof(proof)
        ) {
            return false
        };

        let asset_auditor_ek = get_auditor(token);
        if (asset_auditor_ek.is_none()) {
            return true
        };

        if (auditor_eks.length() == 0) {
            return false
        };

        let asset_auditor_ek = twisted_elgamal::pubkey_to_point(&asset_auditor_ek.extract());
        let first_auditor_ek = twisted_elgamal::pubkey_to_point(&auditor_eks[0]);

        ristretto255::point_equals(&asset_auditor_ek, &first_auditor_ek)
    }

    /// Deserializes the auditor EKs from a byte array.
    /// Returns `Some(vector<twisted_elgamal::CompressedPubkey>)` if the deserialization is successful, otherwise `None`.
    fun deserialize_auditor_eks(
        auditor_eks_bytes: vector<u8>): Option<vector<twisted_elgamal::CompressedPubkey>>
    {
        if (auditor_eks_bytes.length() % 32 != 0) {
            return std::option::none()
        };

        let auditors_count = auditor_eks_bytes.length() / 32;

        let auditor_eks = vector::range(0, auditors_count).map(|i| {
            twisted_elgamal::new_pubkey_from_bytes(auditor_eks_bytes.slice(i * 32, (i + 1) * 32))
        });

        if (auditor_eks.any(|ek| ek.is_none())) {
            return std::option::none()
        };

        std::option::some(auditor_eks.map(|ek| ek.extract()))
    }

    /// Deserializes the auditor amounts from a byte array.
    /// Returns `Some(vector<confidential_balance::ConfidentialBalance>)` if the deserialization is successful, otherwise `None`.
    fun deserialize_auditor_amounts(
        auditor_amounts_bytes: vector<u8>): Option<vector<confidential_balance::ConfidentialBalance>>
    {
        if (auditor_amounts_bytes.length() % 256 != 0) {
            return std::option::none()
        };

        let auditors_count = auditor_amounts_bytes.length() / 256;

        let auditor_amounts = vector::range(0, auditors_count).map(|i| {
            confidential_balance::new_pending_balance_from_bytes(auditor_amounts_bytes.slice(i * 256, (i + 1) * 256))
        });

        if (auditor_amounts.any(|ek| ek.is_none())) {
            return std::option::none()
        };

        std::option::some(auditor_amounts.map(|balance| balance.extract()))
    }

    /// Converts coins to missing FA.
    /// Returns `Some(Object<Metadata>)` if user has a suffucient amount of FA to proceed, otherwise `None`.
    fun ensure_sufficient_fa<CoinType>(sender: &signer, amount: u64): Option<Object<Metadata>> {
        let user = signer::address_of(sender);
        let fa = coin::paired_metadata<CoinType>();

        if (fa.is_none()) {
            return fa;
        };

        let fa_balance = primary_fungible_store::balance(user, *fa.borrow());

        if (fa_balance >= amount) {
            return fa;
        };

        if (coin::balance<CoinType>(user) < amount) {
            return std::option::none();
        };

        let coin_amount = coin::withdraw<CoinType>(sender, amount - fa_balance);
        let fa_amount = coin::coin_to_fungible_asset(coin_amount);

        primary_fungible_store::deposit(user, fa_amount);

        fa
    }

    //
    // Test-only functions
    //

    #[test_only]
    public fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    #[test_only]
    public fun verify_pending_balance(
        user: address,
        token: Object<Metadata>,
        user_dk: &Scalar,
        amount: u64): bool acquires ConfidentialAssetStore
    {
        let ca_store = borrow_global<ConfidentialAssetStore>(get_user_address(user, token));
        let pending_balance = confidential_balance::decompress_balance(&ca_store.pending_balance);

        confidential_balance::verify_pending_balance(&pending_balance, user_dk, amount)
    }

    #[test_only]
    public fun verify_actual_balance(
        user: address,
        token: Object<Metadata>,
        user_dk: &Scalar,
        amount: u128): bool acquires ConfidentialAssetStore
    {
        let ca_store = borrow_global<ConfidentialAssetStore>(get_user_address(user, token));
        let actual_balance = confidential_balance::decompress_balance(&ca_store.actual_balance);

        confidential_balance::verify_actual_balance(&actual_balance, user_dk, amount)
    }

    #[test_only]
    public fun serialize_auditor_eks(auditor_eks: &vector<twisted_elgamal::CompressedPubkey>): vector<u8> {
        let auditor_eks_bytes = vector[];

        auditor_eks.for_each_ref(|auditor| {
            auditor_eks_bytes.append(twisted_elgamal::pubkey_to_bytes(auditor));
        });

        auditor_eks_bytes
    }

    #[test_only]
    public fun serialize_auditor_amounts(
        auditor_amounts: &vector<confidential_balance::ConfidentialBalance>
    ): vector<u8> {
        let auditor_amounts_bytes = vector[];

        auditor_amounts.for_each_ref(|balance| {
            auditor_amounts_bytes.append(confidential_balance::balance_to_bytes(balance));
        });

        auditor_amounts_bytes
    }
}
