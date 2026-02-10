/// This module implements the Confidential Asset (CA) Standard, a privacy-focused protocol for managing fungible assets (FA).
/// It enables private transfers by obfuscating transaction amounts while keeping sender and recipient addresses visible.
module aptos_experimental::confidential_asset {
    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;
    use aptos_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint};
    use aptos_std::ristretto255_bulletproofs::Self as bulletproofs;
    use aptos_std::string_utils;
    use aptos_framework::chain_id;
    use aptos_framework::event;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::Self;
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_experimental::confidential_balance::{get_num_pending_chunks, get_num_available_chunks,
        ConfidentialBalance
    };
    use aptos_experimental::sigma_protocol_utils;
    use aptos_experimental::sigma_protocol_key_rotation;
    use aptos_experimental::sigma_protocol_proof;

    use aptos_experimental::confidential_balance;
    use aptos_experimental::confidential_proof::{
        Self,
        NormalizationProof,
        TransferProof,
        WithdrawalProof
    };
    use aptos_experimental::ristretto255_twisted_elgamal;

    #[test_only]
    use aptos_std::ristretto255::Scalar;

    // ======
    // Errors
    // ======

    /// The range proof system does not support sufficient range.
    const E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE: u64 = 1;

    /// The confidential store has already been published for the given user and asset-type pair: user need not call `register` again.
    const E_CONFIDENTIAL_STORE_ALREADY_REGISTERED: u64 = 2;

    /// The confidential store has not been published for the given user and asset-type pair: user should call `register`.
    const E_CONFIDENTIAL_STORE_NOT_REGISTERED: u64 = 3;

    /// Incoming transfers must NOT be paused before depositing or receiving a transfer.
    const E_INCOMING_TRANSFERS_PAUSED: u64 = 4;

    /// The pending balance must be zero before rotating the encryption key.
    const E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION: u64 = 5;

    /// The receiver's pending balance has accumulated too many incoming transferes and must be rolled over into the available balance.
    const E_PENDING_BALANCE_MUST_BE_ROLLED_OVER: u64 = 6;

    /// The available balance must be normalized before roll-over to ensure available balance chunks remain 32-bit after.
    const E_NORMALIZATION_REQUIRED: u64 = 7;

    /// The balance is already normalized and cannot be normalized again.
    const E_ALREADY_NORMALIZED: u64 = 8;

    /// The asset type is currently not allowed for confidential transfers.
    const E_ASSET_TYPE_DISALLOWED: u64 = 9;

    /// Incoming transfers must be paused before key rotation.
    const E_INCOMING_TRANSFERS_NOT_PAUSED: u64 = 10;

    /// No user has deposited this asset type yet into their confidential store.
    const E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE: u64 = 11;

    /// An internal error occurred: there is either a bug or a misconfiguration in the contract.
    const E_INTERNAL_ERROR: u64 = 999;

    /// #[test_only] The confidential asset module initialization failed.
    const E_INIT_MODULE_FAILED_FOR_DEVNET: u64 = 1000;

    // =========
    // Constants
    // =========

    /// The maximum number of transactions can be aggregated on the pending balance before rollover is required.
    /// i.e., `ConfidentialStore::transfers_received` will never exceed this value.
    const MAX_TRANSFERS_BEFORE_ROLLOVER: u64 = 65536;

    /// The mainnet chain ID. If the chain ID is 1, the allow list is enabled.
    const MAINNET_CHAIN_ID: u8 = 1;

    /// The testnet chain ID.
    const TESTNET_CHAIN_ID: u8 = 2;

    // =======
    // Structs
    // =======

    /// An object that stores the encrypted balances for a specific confidential asset type and owning user.
    /// This should be thought of as a confidential variant of `aptos_framework::fungible_asset::FungibleStore`.
    ///
    /// e.g., for Alice's confidential APT, such an object will be created and stored at an Alice-specific and APT-specific
    ///   address. It will track Alice's confidential APT balance.
    struct ConfidentialStore has key {
        /// Indicates if incoming transfers are paused for this asset type, which is necessary to ensure the pending
        /// balance does not change during a key rotation, which would invalidate that key rotation and leave the account
        /// in an inconsistent state.
        pause_incoming_transfers: bool,

        /// A flag indicating whether the available balance is normalized. A normalized balance
        /// ensures that all chunks fit within the defined 16-bit bounds. This ensures that, after, roll-over all chunks
        /// remain 32-bit.
        normalized: bool,

        /// The number of payments received so far, which gives an upper bound on the size of the pending balance chunks
        /// and thus on the size of the available balance chunks, post roll-over.
        transfers_received: u64,

        /// Stores the user's pending balance, which is used for accepting incoming transfers.
        /// Represented as four 16-bit chunks $(p_0 + 2^{16} \cdot p_1 + ... + (2^{16})^15 \cdot p_15)$ that can grow
        /// up to 32 bits. All payments are accepted into this pending balance, which users should roll over into their
        /// periodically as they run out of available balance (see `available_balance` field below).
        ///
        /// This separation helps protect against front-running attacks, where small incoming transfers could force
        /// frequent regeneration of ZK proofs.
        pending_balance: confidential_balance::CompressedConfidentialBalance,

        /// Represents the user's balance that is available for sending payments.
        /// It consists of eight 16-bit chunks $(a_0 + 2^{16} \cdot a_1 + ... + (2^{16})^15 \cdot a_15)$, supporting a
        /// 128-bit balance.
        available_balance: confidential_balance::CompressedConfidentialBalance,

        /// The encryption key associated with the user's confidential asset account, different for each asset type.
        ek: CompressedRistretto
    }

    /// A resource that represents the controller for the primary FA stores and `FAConfig` objects, "installed" during
    /// `init_module` at @aptos_experimental.
    /// TODO(upgradeability): Should we make this into an enum to make it easier to upgrade it?
    struct FAController has key {
        /// Indicates whether the allow list is enabled. If `true`, only asset types from the allow list can be transferred.
        /// This flag is managed by the governance module.
        allow_list_enabled: bool,

        // TODO: I think this is where we can add a global auditor?

        /// Used to derive a signer that owns all the FAs' primary stores and `FAConfig` objects.
        extend_ref: ExtendRef
    }

    /// An object that represents the configuration of an asset type.
    ///
    /// TODO(upgradeability): Should we make this into an enum to make it easier to upgrade it?
    struct FAConfig has key {
        /// Indicates whether the asset type is allowed for confidential transfers, can be toggled by the governance
        /// module. Withdrawals are always allowed, even when this is set to `false`.
        /// If `FAController::allow_list_enabled` is `false`, all asset types are allowed, even if this is `false`.
        allowed: bool,

        /// The auditor's public key for the asset type. If the auditor is not set, this field is `None`.
        /// Otherwise, each confidential transfer must include the auditor as an additional party,
        /// alongside the recipient, who has access to the decrypted transferred amount.
        ///
        /// TODO(feature): add global auditor EK too
        /// TODO(feature): add support for multiple auditors here
        auditor_ek: Option<CompressedRistretto>
    }

    // ======
    // Events
    // ======

    #[event]
    /// Emitted when someone brings confidential assets into the protocol via `deposit_to`: i.e., by depositing a fungible
    /// asset into the "confidential pool" and minting a confidential asset as "proof" of this.
    struct Deposited has drop, store {
        from: address,
        to: address,
        amount: u64,
        asset_type: Object<fungible_asset::Metadata>
    }

    #[event]
    /// Emitted when someone brings confidential assets out of the protocol via `withdraw_to`: i.e., by burning a confidential
    /// asset as "proof" of being allowed to withdraw a fungible asset from the "confidential pool."
    struct Withdrawn has drop, store {
        from: address,
        to: address,
        amount: u64,
        asset_type: Object<fungible_asset::Metadata>
    }

    #[event]
    /// Emitted when confidential assets are transferred within the protocol between users' confidential balances.
    /// Note that a numeric amount is not included, as the whole point of the protocol is to avoid leaking it.
    struct Transferred has drop, store {
        from: address,
        to: address,
        asset_type: Object<fungible_asset::Metadata>
    }

    // =====================
    // Module initialization
    // =====================

    /// Called only once, when this module is first published on the blockchain.
    fun init_module(deployer: &signer) {
        // TODO: Just asserting if my understanding is correct that `deployer == @aptos_experimental`
        assert!(signer::address_of(deployer) == @aptos_experimental, error::internal(E_INTERNAL_ERROR));

        assert!(
            bulletproofs::get_max_range_bits()
                >= confidential_proof::get_bulletproofs_num_bits(),
            error::internal(E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        let deployer_address = signer::address_of(deployer);

        move_to(
            deployer,
            FAController {
                allow_list_enabled: chain_id::get() == MAINNET_CHAIN_ID,
                // DO NOT CHANGE: using long syntax until framework change is released to mainnet
                extend_ref: object::generate_extend_ref(&object::create_object(deployer_address))
            }
        );
    }

    /// Used to initialize the module for devnet and for tests in aptos-move/e2e-move-tests/
    entry fun init_module_for_devnet(deployer: &signer) {
        assert!(
            signer::address_of(deployer) == @aptos_experimental,
            error::invalid_argument(E_INIT_MODULE_FAILED_FOR_DEVNET)
        );
        assert!(
            chain_id::get() != MAINNET_CHAIN_ID,
            error::invalid_state(E_INIT_MODULE_FAILED_FOR_DEVNET)
        );
        assert!(
            chain_id::get() != TESTNET_CHAIN_ID,
            error::invalid_state(E_INIT_MODULE_FAILED_FOR_DEVNET)
        );

        init_module(deployer)
    }

    // =======================
    // Public, entry functions
    // =======================

    /// Registers an account for a specified asset type.
    /// TODO: make it independent of the asset type. the "confidential store", if non existant, can be created at receiving time
    /// TODO(Security): ZKPoK of DK
    ///
    /// Users are also responsible for generating a Twisted ElGamal key pair on their side.
    public entry fun register(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>, ek: vector<u8>
    ) acquires FAController, FAConfig {
        let ek = ristretto255::new_compressed_point_from_bytes(ek).extract();

        register_internal(sender, asset_type, ek);
    }

    /// Brings tokens into the protocol, transferring the passed amount from the sender's primary FA store
    /// to the pending balance of the recipient.
    /// The initial confidential balance is publicly visible, as entering the protocol requires a normal transfer.
    /// However, tokens within the protocol become obfuscated through confidential transfers, ensuring privacy in
    /// subsequent transactions.
    /// TODO: grieving attack so remove
    public entry fun deposit_to(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64
    ) acquires ConfidentialStore, FAController, FAConfig {
        deposit_to_internal(sender, asset_type, to, amount)
    }

    /// The same as `deposit_to`, but the recipient is the sender.
    public entry fun deposit(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>, amount: u64
    ) acquires ConfidentialStore, FAController, FAConfig {
        deposit_to_internal(
            sender,
            asset_type,
            signer::address_of(sender),
            amount
        )
    }

    /// Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
    /// the recipient's primary FA store.
    /// The withdrawn amount is publicly visible, as this process requires a normal transfer.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    public entry fun withdraw_to(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>
    ) acquires ConfidentialStore, FAController {
        let new_balance =
            confidential_balance::new_balance_from_bytes(new_balance, get_num_available_chunks()).extract();
        let proof =
            confidential_proof::deserialize_withdrawal_proof(sigma_proof, zkrp_new_balance)
                .extract();

        withdraw_to_internal(sender, asset_type, to, amount, new_balance, proof);
    }

    /// The same as `withdraw_to`, but the recipient is the sender.
    public entry fun withdraw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        amount: u64,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>
    ) acquires ConfidentialStore, FAController {
        withdraw_to(
            sender,
            asset_type,
            signer::address_of(sender),
            amount,
            new_balance,
            zkrp_new_balance,
            sigma_proof
        )
    }

    /// Transfers tokens from the sender's available balance to the recipient's pending balance.
    /// The function hides the transferred amount while keeping the sender and recipient addresses visible.
    /// The sender encrypts the transferred amount with the recipient's encryption key and the function updates the
    /// recipient's confidential balance homomorphically.
    /// Additionally, the sender encrypts the transferred amount with the auditors' EKs, allowing auditors to decrypt
    /// it on their side.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    /// Warning: If the auditor feature is enabled, the sender must include the auditor as the first element in the
    /// `auditor_eks` vector.
    public entry fun confidential_transfer(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        new_balance: vector<u8>,
        sender_amount: vector<u8>,
        recipient_amount: vector<u8>,
        auditor_eks: vector<u8>,
        auditor_amounts: vector<u8>,
        zkrp_new_balance: vector<u8>,
        zkrp_transfer_amount: vector<u8>,
        sigma_proof: vector<u8>
    ) acquires ConfidentialStore, FAConfig, FAController {
        let new_balance =
            confidential_balance::new_balance_from_bytes(new_balance, get_num_available_chunks()).extract();
        let sender_amount =
            confidential_balance::new_balance_from_bytes(sender_amount, get_num_pending_chunks()).extract();
        let recipient_amount =
            confidential_balance::new_balance_from_bytes(recipient_amount, get_num_pending_chunks()).extract();
        let auditor_eks = deserialize_auditor_eks(auditor_eks).extract();
        let auditor_amounts = deserialize_auditor_amounts(auditor_amounts).extract();
        let proof =
            confidential_proof::deserialize_transfer_proof(
                sigma_proof, zkrp_new_balance, zkrp_transfer_amount
            ).extract();

        confidential_transfer_internal(
            sender,
            asset_type,
            to,
            new_balance,
            sender_amount,
            recipient_amount,
            auditor_eks,
            auditor_amounts,
            proof
        )
    }

    /// Rotates the encryption key for the user's confidential balance, updating it to a new encryption key.
    /// Parses arguments and forwards to `rotate_encryption_key_internal`; see that function for details.
    public entry fun rotate_encryption_key(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_ek: vector<u8>,
        resume_incoming_transfers: bool,
        new_D: vector<vector<u8>>, // part of the proof
        sigma_proto_comm: vector<vector<u8>>, // part of the proof
        sigma_proto_resp: vector<vector<u8>>, // part of the proof
    ) acquires ConfidentialStore {
        // Just parse stuff and forward to the more type-safe function
        let (new_ek, compressed_new_ek) = ristretto255::new_point_and_compressed_from_bytes(new_ek);
        let (new_D, compressed_new_D) = sigma_protocol_utils::deserialize_points(new_D);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(
            sigma_proto_comm, sigma_proto_resp
        );

        rotate_encryption_key_internal(
            sender, asset_type, new_ek,
            KeyRotationProof::V1 { compressed_new_ek, new_D, compressed_new_D, sigma },
            resume_incoming_transfers
        );
    }

    /// TODO: Move this up at some point
    enum KeyRotationProof has drop {
        V1 {
            compressed_new_ek: CompressedRistretto,
            new_D: vector<RistrettoPoint>,
            compressed_new_D: vector<CompressedRistretto>,
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// TODO(Comment): add comments explaining the parameters
    public fun rotate_encryption_key_internal(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_ek: RistrettoPoint,
        proof: KeyRotationProof,
        resume_incoming_transfers: bool,
    ) {
        //
        // Step 1: Safety-checks that (1) incoming transfers are paused and (2) pending balance is zero because it has
        //         been rolled over
        //

        let ca_store = borrow_global_mut<ConfidentialStore>(
            get_confidential_store_address(signer::address_of(owner), asset_type)
        );

        // (1) Assert incoming transfers are paused & unpause them after if flag is set maybe
        assert!(ca_store.pause_incoming_transfers, error::invalid_state(E_INCOMING_TRANSFERS_NOT_PAUSED));

        // (2) Assert that the pending balance is zero before rotating the key. The user must call `rollover_pending_balance`
        // before rotating their key with `pause` set to `true`.
        assert!(
            confidential_balance::is_zero_balance(&ca_store.pending_balance),
            error::invalid_state(E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION)
        );
        // Over-asserting invariants, in an abundance of caution.
        assert!(
            ca_store.transfers_received == 0,
            error::invalid_state(E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION)
        );

        //
        // Step 2: Fetch old available balance and the old EK from on-chain
        //

        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let compressed_old_ek = ca_store.ek;
        let old_ek = ristretto255::point_decompress(&compressed_old_ek);
        let compressed_old_D = *ca_store.available_balance.get_compressed_D();
        let old_D = sigma_protocol_utils::decompress_points(&compressed_old_D);

        //
        // Step 3: Verify the Sigma protocol proof of correct re-encryption
        //
        let ss = sigma_protocol_key_rotation::new_session(owner, asset_type, get_num_available_chunks());
        let KeyRotationProof::V1 { compressed_new_ek, new_D, compressed_new_D, sigma } =  proof;
        // Note: Will check that compressed_old_D.length() == compressed_new_D.length() == num_chunks > 0
        let stmt = sigma_protocol_key_rotation::new_key_rotation_statement(
            // TODO(Perf): Can we avoid the expensive decompression of `H`? (May need a native function.)
            compressed_H, ristretto255::point_decompress(&compressed_H),
            compressed_old_ek, old_ek,
            compressed_new_ek, new_ek,
            compressed_old_D, old_D,
            compressed_new_D, new_D,
            get_num_available_chunks(),
        );
        sigma_protocol_key_rotation::assert_verifies(&ss, &stmt, &sigma, get_num_available_chunks());

        //
        // Step 4: Install the new EK and the new re-encrypted available balance
        //
        ca_store.ek = compressed_new_ek;
        // Note: The pending balance has been asserted to be zero. We're just updating the available balance.
        // The C components stay the same (they don't depend on the EK); only D = r * EK changes.
        ca_store.available_balance.set_compressed_D(compressed_new_D);

        // Note: ca_store.pause_incoming_transfers is already set to `true`
        if (resume_incoming_transfers) {
            ca_store.pause_incoming_transfers = false;
        }
    }

    /// Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
    /// Most functions perform implicit normalization by accepting a new normalized confidential balance as a parameter.
    /// However, explicit normalization is required before rolling over the pending balance, as multiple rolls may cause
    /// chunk overflows.
    /// The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
    public entry fun normalize(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        sigma_proof: vector<u8>
    ) acquires ConfidentialStore {
        let new_balance =
            confidential_balance::new_balance_from_bytes(new_balance, get_num_available_chunks()).extract();
        let proof =
            confidential_proof::deserialize_normalization_proof(
                sigma_proof, zkrp_new_balance
            ).extract();

        normalize_internal(sender, asset_type, new_balance, proof);
    }

    /// Pauses receiving incoming transfers for the specified account and asset type.
    /// Needed for one scenario:
    ///  1. Before rotating their encryption key, the owner must pause incoming transfers so as to be able to roll over
    ///     their pending balance fully. Then, to rotate their encryption key, the owner needs to only re-encrypt their
    ///     available balance ciphertext. Once done, the owner can unpause incoming transfers.
    public entry fun pause_incoming_transactions(
        owner: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        pause_incoming_transactions_internal(owner, asset_type);
    }

    /// Allows receiving incoming transfers for the specified account and asset type.
    public entry fun resume_incoming_transactions(
        owner: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        resume_incoming_transactions_internal(owner, asset_type);
    }

    /// Adds the pending balance to the available balance for the specified asset type, resetting the pending balance to zero.
    /// This operation is needed when the owner wants to be able to send out tokens from their pending balance: the only
    /// way of doing so is to roll over these tokens into the available balance.
    public entry fun rollover_pending_balance(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        rollover_pending_balance_internal(sender, asset_type);
    }

    /// Before calling `rotate_encryption_key`, we need to rollover the pending balance and freeze the asset type to
    /// prevent any new transfers from coming in.
    public entry fun rollover_pending_balance_and_freeze(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        rollover_pending_balance(sender, asset_type);
        pause_incoming_transactions(sender, asset_type);
    }

    // ===========================
    // Public governance functions
    // ===========================

    /// Enables the allow list, restricting confidential transfers to asset types on the allow list.
    public fun enable_allow_listing(aptos_framework: &signer) acquires FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_controller = borrow_global_mut<FAController>(@aptos_experimental);
        fa_controller.allow_list_enabled = true;
    }

    /// Disables the allow list, allowing confidential transfers for all asset types.
    public fun disable_allow_listing(aptos_framework: &signer) acquires FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_controller = borrow_global_mut<FAController>(@aptos_experimental);
        fa_controller.allow_list_enabled = false;
    }

    /// Enables confidential transfers for the specified asset type.
    public fun enable_confidentiality_for_asset_type(
        aptos_framework: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires FAConfig, FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(get_fa_config_address_or_create(asset_type));
        fa_config.allowed = true;
    }

    /// Disables confidential transfers for the specified asset type.
    public fun disable_confidentiality_for_asset_type(
        aptos_framework: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires FAConfig, FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(get_fa_config_address_or_create(asset_type));
        fa_config.allowed = false;
    }

    /// Sets the auditor for the specified asset type.
    ///
    /// NOTE: Ensures that new_auditor_ek is a valid Ristretto255 point
    /// TODO(Security): ZKPoK of DK?
    public fun set_auditor_for_asset_type(
        aptos_framework: &signer, asset_type: Object<fungible_asset::Metadata>, auditor_ek: vector<u8>
    ) acquires FAConfig, FAController {
        system_addresses::assert_aptos_framework(aptos_framework);

        let fa_config = borrow_global_mut<FAConfig>(get_fa_config_address_or_create(asset_type));
        fa_config.auditor_ek = std::option::some(ristretto255::new_compressed_point_from_bytes(auditor_ek).extract());
    }

    /// Sets the global auditor for all asset types.
    public fun set_auditor_globally(_aptos_framework: &signer, _auditor_ek: vector<u8>) {
        // TODO: Implement
    }

    // =====================
    // Public view functions
    // =====================

    #[view]
    /// Checks if the user has a confidential store for the specified asset type.
    public fun has_confidential_store(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): bool {
        exists<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    #[view]
    /// Returns true if confidentiality is enabled for all assets or if the asset type is allowed for confidential
    /// transfers. Returns false otherwise.
    public fun is_confidentiality_enabled_for_asset_type(asset_type: Object<fungible_asset::Metadata>): bool acquires FAController, FAConfig {
        if (!is_allow_listing_enabled()) {
            return true
        };

        let fa_config_address = get_fa_config_address(asset_type);

        if (!exists<FAConfig>(fa_config_address)) {
            return false
        };

        borrow_global<FAConfig>(fa_config_address).allowed
    }

    #[view]
    /// Checks if allow listing is enabled.
    /// If the allow list is enabled, only asset types from the allow list can be transferred confidentially.
    /// Otherwise, all asset types are allowed.
    public fun is_allow_listing_enabled(): bool acquires FAController {
        borrow_global<FAController>(@aptos_experimental).allow_list_enabled
    }

    #[view]
    /// Returns the pending balance of the user for the specified asset type.
    public fun get_pending_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): confidential_balance::CompressedConfidentialBalance acquires ConfidentialStore {
        assert!(
            has_confidential_store(owner, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        let ca_store =
            borrow_global<ConfidentialStore>(get_confidential_store_address(owner, asset_type));

        ca_store.pending_balance
    }

    #[view]
    /// Returns the available balance of the user for the specified asset type.
    public fun get_available_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): confidential_balance::CompressedConfidentialBalance acquires ConfidentialStore {
        assert!(
            has_confidential_store(owner, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        let ca_store =
            borrow_global<ConfidentialStore>(get_confidential_store_address(owner, asset_type));

        ca_store.available_balance
    }

    #[view]
    /// Returns the encryption key (EK) of the user for the specified asset type.
    public fun get_encryption_key(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedRistretto acquires ConfidentialStore {
        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type)).ek
    }

    #[view]
    /// Checks if the user's available balance is normalized for the specified asset type.
    public fun is_normalized(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): bool acquires ConfidentialStore {
        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type)).normalized
    }

    #[view]
    /// Checks if the user's incoming transfers are paused for the specified asset type.
    public fun incoming_transfers_paused(user: address, asset_type: Object<fungible_asset::Metadata>): bool acquires ConfidentialStore {
        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type)).pause_incoming_transfers
    }

    #[view]
    /// Returns the asset-specific auditor's encryption key.
    /// If the auditing feature is disabled for the asset type, the encryption key is set to `None`.
    public fun get_auditor_for_asset_type(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires FAConfig, FAController {
        let fa_config_address = get_fa_config_address(asset_type);

        if (!is_allow_listing_enabled() && !exists<FAConfig>(fa_config_address)) {
            return std::option::none();
        };

        borrow_global<FAConfig>(fa_config_address).auditor_ek
    }

    #[view]
    /// Returns the circulating supply of the confidential asset.
    /// TODO: rename to get_total_confidential_supply
    public fun get_total_supply(asset_type: Object<fungible_asset::Metadata>): u64 acquires FAController {
        let fa_store_address = get_fa_controller_address();
        assert!(
            primary_fungible_store::primary_store_exists(fa_store_address, asset_type),
            error::not_found(E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE)
        );

        primary_fungible_store::balance(fa_store_address, asset_type)
    }

    #[view]
    /// Returns the number of transfers received into the pending balance for the specified asset type.
    public fun get_num_transfers_received(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires ConfidentialStore {
        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type)).transfers_received
    }

    // ===========================
    // Public, non-entry functions
    // ===========================
    //
    // Note: These function can be useful for external contracts that want to integrate with the Confidential Asset
    // protocol.
    //
    // TODO(rename): The `_internal` suffix is somewhat of a misnomer. Should use `register` for this and `register_raw`
    //   for the function that takes raw bytes
    //

    /// Implementation of the `register` entry function.
    public fun register_internal(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>, ek: CompressedRistretto
    ) acquires FAController, FAConfig {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));

        let user = signer::address_of(sender);

        assert!(
            !has_confidential_store(user, asset_type),
            error::already_exists(E_CONFIDENTIAL_STORE_ALREADY_REGISTERED)
        );

        let ca_store = ConfidentialStore {
            pause_incoming_transfers: false,
            normalized: true,
            transfers_received: 0,
            pending_balance: confidential_balance::new_compressed_zero_balance(get_num_pending_chunks()),
            available_balance: confidential_balance::new_compressed_zero_balance(get_num_available_chunks()),
            ek
        };

        move_to(&get_confidential_store_signer(sender, asset_type), ca_store);
    }

    /// Implementation of the `deposit_to` entry function.
    /// For convenience, we often refer to this operation as "veiling."
    /// TODO: remove ability to deposit to another's account
    public fun deposit_to_internal(
        depositor: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64
    ) acquires ConfidentialStore, FAController, FAConfig {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(to, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));

        let depositor_addr = signer::address_of(depositor);

        let depositor_fa_store = primary_fungible_store::primary_store(depositor_addr, asset_type);

        // Note: This sets up the "confidential asset pool" for this asset type, if one is not already set up, such as
        // when someone first veils this asset type for the first time.
        let pool_fa_store =
            primary_fungible_store::ensure_primary_store_exists(
                get_fa_controller_address(), asset_type
            );

        //
        // Step 1: Transfer the asset from the user's account into the confidential asset pool
        //
        dispatchable_fungible_asset::transfer(
            depositor, depositor_fa_store, pool_fa_store, amount
        );

        //
        // Step 2: "Mint" correspodning confidential assets for the depositor, and add them to their pending balance.
        //
        let depositor_ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(to, asset_type));

        // Make sure the receiver has "room" in their pending balance for this deposit
        assert!(
            depositor_ca_store.transfers_received < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        let pending_balance =
            confidential_balance::decompress(&depositor_ca_store.pending_balance);

        confidential_balance::add_balances_mut(
            &mut pending_balance,
            &confidential_balance::new_pending_balance_u64_no_randomness(amount)
        );

        // Update the pending balance and increment the incoming transfers counter
        depositor_ca_store.pending_balance = confidential_balance::compress(&pending_balance);
        depositor_ca_store.transfers_received += 1;

        event::emit(Deposited { from: depositor_addr, to, amount, asset_type });
    }

    /// Implementation of the `withdraw_to` entry function.
    /// Withdrawals are always allowed, regardless of whether the asset type is allow-listed.
    public fun withdraw_to_internal(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        new_balance: ConfidentialBalance,
        proof: WithdrawalProof
    ) acquires ConfidentialStore, FAController {
        let from = signer::address_of(sender);

        let sender_ek = get_encryption_key(from, asset_type);

        let ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(from, asset_type));
        let current_balance =
            confidential_balance::decompress(&ca_store.available_balance);

        confidential_proof::verify_withdrawal_proof(
            &sender_ek,
            amount,
            &current_balance,
            &new_balance,
            &proof
        );

        ca_store.normalized = true;
        ca_store.available_balance = confidential_balance::compress(&new_balance);

        primary_fungible_store::transfer(&get_fa_controller_signer(), asset_type, to, amount);

        event::emit(Withdrawn { from: signer::address_of(sender), to, amount, asset_type });
    }

    /// Implementation of the `confidential_transfer` entry function.
    public fun confidential_transfer_internal(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        new_balance: ConfidentialBalance,
        sender_amount: ConfidentialBalance,
        recipient_amount: ConfidentialBalance,
        auditor_eks: vector<CompressedRistretto>,
        auditor_amounts: vector<ConfidentialBalance>,
        proof: TransferProof
    ) acquires ConfidentialStore, FAConfig, FAController {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(to, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));
        assert!(
            validate_auditors(
                asset_type,
                &recipient_amount,
                &auditor_eks,
                &auditor_amounts,
                &proof
            ),
            error::invalid_argument(E_INTERNAL_ERROR) // TODO: i removed the old code because validate_auditors() will be turned into fetch_auditor_eks()
        );

        // TODO: This will be removed when we build the more efficient $\Sigma$-protocol
        assert!(
            confidential_balance::balance_c_equals(&sender_amount, &recipient_amount),
            error::internal(E_INTERNAL_ERROR)   // note: i removed the old error code
        );

        let from = signer::address_of(sender);

        let sender_ek = get_encryption_key(from, asset_type);
        let recipient_ek = get_encryption_key(to, asset_type);

        let sender_ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(from, asset_type));

        let sender_current_available_balance =
            confidential_balance::decompress(&sender_ca_store.available_balance);

        confidential_proof::verify_transfer_proof(
            &sender_ek,
            &recipient_ek,
            &sender_current_available_balance,
            &new_balance,
            &sender_amount,
            &recipient_amount,
            &auditor_eks,
            &auditor_amounts,
            &proof
        );

        sender_ca_store.normalized = true;
        sender_ca_store.available_balance = confidential_balance::compress(
            &new_balance
        );

        // Cannot create multiple mutable references to the same type, so we need to drop it
        let ConfidentialStore { .. } = sender_ca_store;

        let recipient_ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(to, asset_type));

        // Make sure the receiver has "room" in their pending balance for this transfer
        assert!(
            recipient_ca_store.transfers_received < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        let recipient_pending_balance =
            confidential_balance::decompress(
                &recipient_ca_store.pending_balance
            );
        confidential_balance::add_balances_mut(
            &mut recipient_pending_balance, &recipient_amount
        );

        recipient_ca_store.transfers_received += 1;
        recipient_ca_store.pending_balance = confidential_balance::compress(
            &recipient_pending_balance
        );

        event::emit(Transferred { from, to, asset_type });
    }

    /// Implementation of the `normalize` entry function.
    public fun normalize_internal(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_balance: ConfidentialBalance,
        proof: NormalizationProof
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);
        let sender_ek = get_encryption_key(user, asset_type);

        let ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type));

        assert!(!ca_store.normalized, error::invalid_state(E_ALREADY_NORMALIZED));

        let current_balance =
            confidential_balance::decompress(&ca_store.available_balance);

        confidential_proof::verify_normalization_proof(
            &sender_ek,
            &current_balance,
            &new_balance,
            &proof
        );

        ca_store.available_balance = confidential_balance::compress(&new_balance);
        ca_store.normalized = true;
    }

    /// Implementation of the `rollover_pending_balance` entry function.
    public fun rollover_pending_balance_internal(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);

        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        let ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type));

        assert!(ca_store.normalized, error::invalid_state(E_NORMALIZATION_REQUIRED));

        let available_balance =
            confidential_balance::decompress(&ca_store.available_balance);
        let pending_balance =
            confidential_balance::decompress(&ca_store.pending_balance);

        confidential_balance::add_balances_mut(&mut available_balance, &pending_balance);

        ca_store.normalized = false;
        ca_store.transfers_received = 0;
        ca_store.available_balance = confidential_balance::compress(&available_balance);
        ca_store.pending_balance = confidential_balance::new_compressed_zero_balance(get_num_pending_chunks());
    }

    /// Implementation of the `pause_incoming_transactions` entry function.
    public fun pause_incoming_transactions_internal(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);

        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        let ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type));
        ca_store.pause_incoming_transfers = true;
    }

    /// Implementation of the `resume_incoming_transactions` entry function.
    public fun resume_incoming_transactions_internal(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);

        assert!(
            has_confidential_store(user, asset_type),
            error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED)
        );

        let ca_store =
            borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type));
        ca_store.pause_incoming_transfers = false;
    }

    // =================
    // Private functions
    // =================

    /// Returns the address that handles primary FA store and `FAConfig` objects for the specified asset type.
    fun get_fa_config_address(asset_type: Object<fungible_asset::Metadata>): address acquires FAController {
        let fa_ext = &borrow_global<FAController>(@aptos_experimental).extend_ref;
        let fa_ext_address = object::address_from_extend_ref(fa_ext);
        object::create_object_address(&fa_ext_address, construct_fa_config_seed(asset_type))
    }

    /// Ensures that the `FAConfig` object exists for the specified asset type and returns its address.
    /// If the object does not exist, creates it. Used only for internal purposes.
    fun get_fa_config_address_or_create(asset_type: Object<fungible_asset::Metadata>): address acquires FAController {
        let addr = get_fa_config_address(asset_type);

        if (!exists<FAConfig>(addr)) {
            let fa_config_signer = get_fa_config_signer(asset_type);

            move_to(
                &fa_config_signer,
                // We disallow the asset type from being made confidential since this function is
                // called in a lot of different contexts.
                FAConfig { allowed: false, auditor_ek: std::option::none() }
            );
        };

        addr
    }

    /// Returns an object for handling all the FA primary stores, and returns a signer for it.
    fun get_fa_controller_signer(): signer acquires FAController {
        object::generate_signer_for_extending(&borrow_global<FAController>(@aptos_experimental).extend_ref)
    }

    /// Returns the address that handles all the FA primary stores.
    fun get_fa_controller_address(): address acquires FAController {
        object::address_from_extend_ref(&borrow_global<FAController>(@aptos_experimental).extend_ref)
    }

    /// Returns an object for handling the `ConfidentialStore` and returns a signer for it.
    fun get_confidential_store_signer(user: &signer, asset_type: Object<fungible_asset::Metadata>): signer {
        object::generate_signer(&object::create_named_object(user, construct_confidential_store_seed(asset_type)))
    }

    /// Returns the address that handles the user's `ConfidentialStore` object for the specified user and asset type.
    fun get_confidential_store_address(user: address, asset_type: Object<fungible_asset::Metadata>): address {
        object::create_object_address(&user, construct_confidential_store_seed(asset_type))
    }

    /// Returns an object for handling the `FAConfig`, and returns a signer for it.
    fun get_fa_config_signer(asset_type: Object<fungible_asset::Metadata>): signer acquires FAController {
        let fa_ext = &borrow_global<FAController>(@aptos_experimental).extend_ref;
        let fa_ext_signer = object::generate_signer_for_extending(fa_ext);

        let fa_ctor =
            &object::create_named_object(&fa_ext_signer, construct_fa_config_seed(asset_type));

        object::generate_signer(fa_ctor)
    }

    /// Constructs a unique seed for the user's `ConfidentialStore` object.
    /// As all the `ConfidentialStore`'s have the same type, we need to differentiate them by the seed.
    fun construct_confidential_store_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::user",
                @aptos_experimental,
                object::object_address(&asset_type)
            )
        )
    }

    /// Constructs a unique seed for the FA's `FAConfig` object.
    /// As all the `FAConfig`'s have the same type, we need to differentiate them by the seed.
    fun construct_fa_config_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::fa",
                @aptos_experimental,
                object::object_address(&asset_type)
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
        asset_type: Object<fungible_asset::Metadata>,
        transfer_amount: &ConfidentialBalance,
        auditor_eks: &vector<CompressedRistretto>,
        auditor_amounts: &vector<ConfidentialBalance>,
        proof: &TransferProof
    ): bool acquires FAConfig, FAController {
        if (!auditor_amounts.all(|auditor_amount| {
            confidential_balance::balance_c_equals(transfer_amount, auditor_amount)
        })) {
            return false
        };

        if (auditor_eks.length() != auditor_amounts.length()
            || auditor_eks.length()
                != confidential_proof::auditors_count_in_transfer_proof(proof)) {
            return false
        };

        let asset_auditor_ek = get_auditor_for_asset_type(asset_type);
        if (asset_auditor_ek.is_none()) {
            return true
        };

        if (auditor_eks.length() == 0) {
            return false
        };

        let asset_auditor_ek = ristretto255::point_decompress(&asset_auditor_ek.extract());
        let first_auditor_ek = ristretto255::point_decompress(&auditor_eks[0]);

        ristretto255::point_equals(&asset_auditor_ek, &first_auditor_ek)
    }

    /// Deserializes the auditor EKs from a byte array.
    /// Returns `Some(vector<CompressedRistretto>)` if the deserialization is successful, otherwise `None`.
    fun deserialize_auditor_eks(
        auditor_eks_bytes: vector<u8>
    ): Option<vector<CompressedRistretto>> {
        if (auditor_eks_bytes.length() % 32 != 0) {
            return std::option::none()
        };

        let auditors_count = auditor_eks_bytes.length() / 32;

        let auditor_eks = vector::range(0, auditors_count).map(|i| {
            ristretto255::new_compressed_point_from_bytes(
                auditor_eks_bytes.slice(i * 32, (i + 1) * 32)
            )
        });

        if (auditor_eks.any(|ek| ek.is_none())) {
            return std::option::none()
        };

        std::option::some(auditor_eks.map(|ek| ek.extract()))
    }

    /// Deserializes the auditor amounts from a byte array.
    /// Returns `Some(vector<ConfidentialBalance>)` if the deserialization is successful, otherwise `None`.
    fun deserialize_auditor_amounts(
        auditor_amounts_bytes: vector<u8>
    ): Option<vector<ConfidentialBalance>> {
        if (auditor_amounts_bytes.length() % 256 != 0) {
            return std::option::none()
        };

        let auditors_count = auditor_amounts_bytes.length() / 256;

        let auditor_amounts = vector::range(0, auditors_count).map(|i| {
            confidential_balance::new_balance_from_bytes(
                auditor_amounts_bytes.slice(i * 256, (i + 1) * 256),
                get_num_pending_chunks()
            )
        });

        if (auditor_amounts.any(|ek| ek.is_none())) {
            return std::option::none()
        };

        std::option::some(
            auditor_amounts.map(|balance| balance.extract())
        )
    }

    // ===================
    // Test-only functions
    // ===================

    #[test_only]
    public fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    #[test_only]
    public fun check_pending_balance_decrypts_to(
        user: address,
        asset_type: Object<fungible_asset::Metadata>,
        user_dk: &Scalar,
        amount: u64
    ): bool acquires ConfidentialStore {
        let pending_balance =
            confidential_balance::decompress(&get_pending_balance(user, asset_type));

        confidential_balance::check_decrypts_to(&pending_balance, user_dk, (amount as u128))
    }

    #[test_only]
    public fun check_available_balance_decrypts_to(
        user: address,
        asset_type: Object<fungible_asset::Metadata>,
        user_dk: &Scalar,
        amount: u128
    ): bool acquires ConfidentialStore {
        let available_balance =
            confidential_balance::decompress(&get_available_balance(user, asset_type));

        confidential_balance::check_decrypts_to(&available_balance, user_dk, amount)
    }

    #[test_only]
    /// TODO: why not just do bcs::to_bytes?
    public fun serialize_auditor_eks(
        auditor_eks: &vector<CompressedRistretto>
    ): vector<u8> {
        let auditor_eks_bytes = vector[];

        auditor_eks.for_each_ref(|auditor| {
            auditor_eks_bytes.append(ristretto255::compressed_point_to_bytes(*auditor));
        });

        auditor_eks_bytes
    }

    #[test_only]
    /// TODO: why not just do bcs::to_bytes? SDK would have to replicate it.
    public fun serialize_auditor_amounts(
        auditor_amounts: &vector<ConfidentialBalance>
    ): vector<u8> {
        let auditor_amounts_bytes = vector[];

        auditor_amounts.for_each_ref(|balance| {
            auditor_amounts_bytes.append(confidential_balance::balance_to_bytes(balance));
        });

        auditor_amounts_bytes
    }

    // =========================================
    // Test-only proof generation wrapper functions
    // =========================================
    // These functions compute proofs and return serialized bytes that can be used
    // to call the corresponding entry functions from Rust e2e tests.

    #[test_only]
    /// Generates the proof bytes needed to call `withdraw_to`.
    /// Returns (new_balance_bytes, zkrp_new_balance_bytes, sigma_proof_bytes).
    public fun generate_withdrawal_proof_bytes(
        sender: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk_bytes: vector<u8>,
        withdraw_amount: u64,
        new_balance_amount: u128
    ): (vector<u8>, vector<u8>, vector<u8>) acquires ConfidentialStore {
        let sender_dk = ristretto255::new_scalar_from_bytes(sender_dk_bytes).extract();
        let sender_ek = get_encryption_key(sender, asset_type);
        let current_balance = confidential_balance::decompress(
            &get_available_balance(sender, asset_type)
        );

        let (proof, new_balance) = confidential_proof::prove_withdrawal(
            &sender_dk,
            &sender_ek,
            withdraw_amount,
            new_balance_amount,
            &current_balance
        );

        let new_balance_bytes = confidential_balance::balance_to_bytes(&new_balance);
        let (sigma_proof_bytes, zkrp_new_balance_bytes) =
            confidential_proof::serialize_withdrawal_proof(&proof);

        (new_balance_bytes, zkrp_new_balance_bytes, sigma_proof_bytes)
    }

    #[test_only]
    /// Generates the proof bytes needed to call `confidential_transfer`.
    /// Returns (new_balance_bytes, sender_amount_bytes, recipient_amount_bytes,
    ///          zkrp_new_balance_bytes, zkrp_transfer_amount_bytes, sigma_proof_bytes).
    public fun generate_transfer_proof_bytes(
        sender: address,
        recipient: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk_bytes: vector<u8>,
        transfer_amount: u64,
        new_balance_amount: u128
    ): (vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>) acquires ConfidentialStore {
        let sender_dk = ristretto255::new_scalar_from_bytes(sender_dk_bytes).extract();
        let sender_ek = get_encryption_key(sender, asset_type);
        let recipient_ek = get_encryption_key(recipient, asset_type);
        let current_balance = confidential_balance::decompress(
            &get_available_balance(sender, asset_type)
        );

        let (proof, new_balance, sender_amount, recipient_amount, _auditor_amounts) =
            confidential_proof::prove_transfer(
                &sender_dk,
                &sender_ek,
                &recipient_ek,
                transfer_amount,
                new_balance_amount,
                &current_balance,
                &vector[] // no auditors for simplicity
            );

        let (sigma_proof_bytes, zkrp_new_balance_bytes, zkrp_transfer_amount_bytes) =
            confidential_proof::serialize_transfer_proof(&proof);

        (
            confidential_balance::balance_to_bytes(&new_balance),
            confidential_balance::balance_to_bytes(&sender_amount),
            confidential_balance::balance_to_bytes(&recipient_amount),
            zkrp_new_balance_bytes,
            zkrp_transfer_amount_bytes,
            sigma_proof_bytes
        )
    }

    #[test_only]
    /// Generates the `KeyRotationProof` needed to call the new `rotate_encryption_key_internal` function.
    public fun generate_key_rotation_proof(
        owner_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        new_dk: &Scalar,
    ): KeyRotationProof acquires ConfidentialStore {
        let owner = aptos_framework::account::create_signer_for_test(owner_addr);
        let num_chunks = get_num_available_chunks();

        // Get old EK and available balance D components
        let compressed_old_ek = get_encryption_key(owner_addr, asset_type);
        let old_ek = ristretto255::point_decompress(&compressed_old_ek);

        let compressed_old_D = *get_available_balance(owner_addr, asset_type).get_compressed_D();
        let old_D = sigma_protocol_utils::decompress_points(&compressed_old_D);

        // Build statement and witness using the helper
        let (stmt, witn, compressed_new_ek, new_D, compressed_new_D) =
            sigma_protocol_key_rotation::compute_statement_and_witness_from_keys_and_old_ctxt(
                sender_dk, new_dk,
                compressed_old_ek, old_ek,
                compressed_old_D, old_D,
                num_chunks,
            );

        // Prove
        let ss = sigma_protocol_key_rotation::new_session(&owner, asset_type, num_chunks);

        KeyRotationProof::V1 {
            compressed_new_ek,
            new_D,
            compressed_new_D,
            sigma: sigma_protocol_key_rotation::prove(&ss, &stmt, &witn),
        }
    }

    #[test_only]
    /// Generates the proof bytes needed to call `normalize`.
    /// Returns (new_balance_bytes, zkrp_new_balance_bytes, sigma_proof_bytes).
    public fun generate_normalization_proof_bytes(
        sender: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk_bytes: vector<u8>,
        balance_amount: u128
    ): (vector<u8>, vector<u8>, vector<u8>) acquires ConfidentialStore {
        let sender_dk = ristretto255::new_scalar_from_bytes(sender_dk_bytes).extract();
        let sender_ek = get_encryption_key(sender, asset_type);
        let current_balance = confidential_balance::decompress(
            &get_available_balance(sender, asset_type)
        );

        let (proof, new_balance) = confidential_proof::prove_normalization(
            &sender_dk,
            &sender_ek,
            balance_amount,
            &current_balance
        );

        let new_balance_bytes = confidential_balance::balance_to_bytes(&new_balance);
        let (sigma_proof_bytes, zkrp_new_balance_bytes) =
            confidential_proof::serialize_normalization_proof(&proof);

        (new_balance_bytes, zkrp_new_balance_bytes, sigma_proof_bytes)
    }
}
