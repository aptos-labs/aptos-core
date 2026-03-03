/// Confidential Asset (CA) Standard: privacy-focused fungible asset transfers with obfuscated amounts.
module aptos_experimental::confidential_asset {
    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use aptos_std::math64;
    use aptos_std::ristretto255::{CompressedRistretto, new_compressed_point_from_bytes, new_scalar_from_u64};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};
    use aptos_std::string_utils;
    use aptos_framework::chain_id;
    use aptos_framework::event;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::Self;
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_experimental::confidential_balance::{Self, get_chunk_size_bits, get_chunk_upper_bound,
        new_pending_u64_no_randomness, add_assign_pending, new_compressed_available_from_bytes,
        add_assign_available_excluding_auditor, new_zero_pending_compressed, new_zero_available_compressed
    };
    use aptos_experimental::sigma_protocol_utils::deserialize_compressed_points;
    use aptos_experimental::confidential_amount::{Self, CompressedAmount};
    use aptos_experimental::confidential_balance::{Pending, Available, CompressedBalance, Balance};
    use aptos_experimental::sigma_protocol_key_rotation;
    use aptos_experimental::sigma_protocol_registration;
    use aptos_experimental::sigma_protocol_withdraw;
    use aptos_experimental::sigma_protocol_transfer;
    use aptos_experimental::sigma_protocol_proof;
    use aptos_experimental::confidential_range_proofs;


    #[test_only]
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use aptos_experimental::confidential_balance::{new_pending_from_p_and_r, generate_available_randomness,
        new_available_from_amount, split_available_into_chunks, split_pending_into_chunks, generate_pending_randomness
    };
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::{points_clone, decompress_points};


    // === Errors (2 out of 15) ===

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

    /// The receiver's pending balance has accumulated too many incoming transfers and must be rolled over into the available balance.
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

    /// The number of auditor R components in the proof does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 12;

    /// There are no pending transfers to roll over.
    const E_NOTHING_TO_ROLLOVER: u64 = 13;

    /// The auditor encryption key must not be the identity (zero) point.
    const E_AUDITOR_EK_IS_IDENTITY: u64 = 14;

    /// Self-transfers are not allowed: sender and recipient must be different.
    const E_SELF_TRANSFER: u64 = 15;

    /// An internal error occurred: there is either a bug or a misconfiguration in the contract.
    const E_INTERNAL_ERROR: u64 = 999;

    /// #[test_only] The confidential asset module initialization failed.
    const E_INIT_MODULE_FAILED_FOR_DEVNET: u64 = 1000;

    // === Constants (3 out of 14) ===

    /// The maximum number of transactions can be aggregated on the pending balance before rollover is required.
    /// i.e., `ConfidentialStore::transfers_received` will never exceed this value.
    const MAX_TRANSFERS_BEFORE_ROLLOVER: u64 = 65536;

    /// The mainnet chain ID. If the chain ID is 1, the allow list is enabled.
    const MAINNET_CHAIN_ID: u8 = 1;

    /// The testnet chain ID.
    const TESTNET_CHAIN_ID: u8 = 2;

    // === Structs (4 out of 14) ===

    /// Bundles an auditor's encryption key with its epoch counter (both always modified together).
    enum AuditorEK has store, drop, copy {
        V1 {
            ek: Option<CompressedRistretto>,

            /// Tracks how many times the auditor EK has been installed or changed (not removed).
            /// Starts at 0 and increments each time a new EK is set (None → Some(ek) or Some(old_ek) → Some(new_ek)).
            epoch: u64,
        }
    }

    /// Global configuration for the confidential asset protocol, installed during `init_module`.
    enum GlobalConfig has key {
        V1 {
            /// Indicates whether the allow list is enabled. If `true`, only asset types from the allow list can be transferred.
            /// This flag is managed by the governance module.
            allow_list_enabled: bool,

            /// The global auditor. Asset-specific auditors take precedence.
            global_auditor: AuditorEK,

            /// Used to derive a signer that owns all the FAs' primary stores and `AssetConfig` objects.
            extend_ref: ExtendRef
        }
    }

    /// Per-asset-type configuration (allow-listing, auditor).
    enum AssetConfig has key {
        V1 {
            /// Indicates whether the asset type is allowed for confidential transfers, can be toggled by the governance
            /// module. Withdrawals are always allowed, even when this is set to `false`.
            /// If `GlobalConfig::allow_list_enabled` is `false`, all asset types are allowed, even if this is `false`.
            allowed: bool,

            /// The asset-specific auditor. Takes precedence over the global auditor.
            auditor: AuditorEK,
        }
    }

    /// Per-(user, asset-type) encrypted balance store (confidential variant of `FungibleStore`).
    enum ConfidentialStore has key {
        V1 {
            /// Must be paused before key rotation to prevent mid-rotation pending balance changes.
            pause_incoming: bool,
            /// True if all available balance chunks are within 16-bit bounds (required before rollover).
            normalized: bool,
            /// Number of transfers received; upper-bounds pending balance chunk sizes.
            transfers_received: u64,
            /// Incoming transfers accumulate here; must be rolled over into `available_balance` to spend.
            pending_balance: CompressedBalance<Pending>,
            /// Spendable balance (8 chunks, 128-bit). R_aud components for auditor decryption (empty if no auditor).
            available_balance: CompressedBalance<Available>,
            /// User's encryption key for this asset type.
            ek: CompressedRistretto
        }
    }

    // === Events (5 out of 14) ===

    #[event]
    enum Deposited has drop, store {
        V1 {
            addr: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    #[event]
    enum Withdrawn has drop, store {
        V1 {
            from: address,
            to: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    #[event]
    enum Normalized has drop, store {
        V1 {
            addr: address,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    #[event]
    enum Transferred has drop, store {
        V1 {
            from: address,
            to: address,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    // === Module initialization (6 out of 14) ===

    /// Called once when this module is first published on-chain.
    fun init_module(deployer: &signer) {
        // This is me being overly cautious: I added it to double-check my understanding that the VM always passes
        // the publishing account as deployer. It does, so the assert is redundant (it can never fail).
        assert!(signer::address_of(deployer) == @aptos_experimental, error::internal(E_INTERNAL_ERROR));
        assert!(math64::pow(2, get_chunk_size_bits()) == get_chunk_upper_bound(), error::internal(E_INTERNAL_ERROR));

        assert!(
            bulletproofs::get_max_range_bits() >= confidential_range_proofs::get_bulletproofs_num_bits(),
            error::internal(E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        let deployer_address = signer::address_of(deployer);
        let is_mainnet = chain_id::get() == MAINNET_CHAIN_ID;

        move_to(
            deployer,
            GlobalConfig::V1 {
                allow_list_enabled: is_mainnet,
                global_auditor: AuditorEK::V1 { ek: std::option::none(), epoch: 0 },
                // DO NOT CHANGE: using long syntax until framework change is released to mainnet
                extend_ref: object::generate_extend_ref(&object::create_object(deployer_address))
            }
        );

        // On mainnet, allow APT by default
        if (is_mainnet) {
            let apt_metadata = object::address_to_object<fungible_asset::Metadata>(@aptos_fungible_asset);
            let config_signer = get_asset_config_signer(apt_metadata);
            move_to(&config_signer, AssetConfig::V1 { allowed: true, auditor: AuditorEK::V1 { ek: std::option::none(), epoch: 0 } });
        };
    }

    /// Initializes the module for devnet/tests. Asserts non-mainnet, non-testnet chain.
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

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                                    //
    // *** SECURITY-SENSITIVE functions (7 out of 14) *** //
    //         (bugs here could lead to stolen funds)     //
    //                                                    //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

    /// Deserializes cryptographic data and forwards to `register`.
    public entry fun register_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires GlobalConfig, AssetConfig {
        let ek = new_compressed_point_from_bytes(ek).extract();
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = RegistrationProof::V1 { sigma };

        register(sender, asset_type, ek, proof);
    }

    /// Registers a confidential store for a specified asset type, encrypted under the given EK.
    public fun register(
        sender: &signer, asset_type:
        Object<fungible_asset::Metadata>,
        ek: CompressedRistretto,
        proof: RegistrationProof
    ) acquires GlobalConfig, AssetConfig {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));

        assert!(
            !has_confidential_store(signer::address_of(sender), asset_type),
            error::already_exists(E_CONFIDENTIAL_STORE_ALREADY_REGISTERED)
        );

        // Makes sure the user knows their decryption key.
        assert_valid_registration_proof(sender, asset_type, &ek, proof);

        let ca_store = ConfidentialStore::V1 {
            pause_incoming: false,
            normalized: true,
            transfers_received: 0,
            pending_balance: new_zero_pending_compressed(),
            available_balance: new_zero_available_compressed(),
            ek
        };

        move_to(&get_confidential_store_signer(sender, asset_type), ca_store);
    }

    /// Deposits tokens from the sender's primary FA store into their pending balance.
    public entry fun deposit(
        depositor: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        amount: u64
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        let addr = signer::address_of(depositor);

        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(addr, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));

        // Note: This sets up the "confidential asset pool" for this asset type, if one is not already set up, such as
        // when someone first veils this asset type for the first time.
        let pool_fa_store = primary_fungible_store::ensure_primary_store_exists(
            get_global_config_address(), asset_type
        );

        // Step 1: Transfer the asset from the user's account into the confidential asset pool
        let depositor_fa_store = primary_fungible_store::primary_store(addr, asset_type);
        dispatchable_fungible_asset::transfer(depositor, depositor_fa_store, pool_fa_store, amount);

        // Step 2: "Mint" corresponding confidential assets for the depositor, and add them to their pending balance.
        let ca_store = borrow_confidential_store_mut(addr, asset_type);

        // Make sure the depositor has "room" in their pending balance for this deposit
        assert!(
            ca_store.transfers_received < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        add_assign_pending(&mut ca_store.pending_balance, &new_pending_u64_no_randomness(amount));
        ca_store.transfers_received += 1;

        event::emit(Deposited::V1 { addr, amount, asset_type });
    }

    /// Deserializes cryptographic data and forwards to `withdraw_to`.
    public entry fun withdraw_to_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        new_balance_P: vector<vector<u8>>,
        new_balance_R: vector<vector<u8>>,
        new_balance_R_aud: vector<vector<u8>>,  // effective auditor R component
        zkrp_new_balance: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        let compressed_new_balance = new_compressed_available_from_bytes(new_balance_P, new_balance_R, new_balance_R_aud);
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma };

        withdraw_to(sender, asset_type, to, amount, proof);
    }

    /// Withdraws tokens from the sender's available balance to recipient's primary FA store. Also used internally by `normalize` (amount = 0).
    public fun withdraw_to(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        proof: WithdrawalProof
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        let sender_addr = signer::address_of(sender);

        // Read values before mutable borrow to avoid conflicting borrows of ConfidentialStore
        let ek = get_encryption_key(sender_addr, asset_type);
        let old_balance = get_available_balance(sender_addr, asset_type);
        let auditor_ek = get_effective_auditor(asset_type);

        let compressed_new_balance = assert_valid_withdrawal_proof(
            sender,
            asset_type,
            &ek,
            amount,
            &old_balance,
            &auditor_ek,
            proof
        );

        let ca_store = borrow_confidential_store_mut(sender_addr, asset_type);
        ca_store.normalized = true;
        ca_store.available_balance = compressed_new_balance;

        if (amount > 0) {
            primary_fungible_store::transfer(&get_global_config_signer(), asset_type, to, amount);
            event::emit(Withdrawn::V1 { from: sender_addr, to, amount, asset_type });
        } else {
            event::emit(Normalized::V1 { addr: sender_addr, asset_type });
        };
    }

    /// Deserializes cryptographic data and forwards to `confidential_transfer`.
    public entry fun confidential_transfer_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        new_balance_P: vector<vector<u8>>,
        new_balance_R: vector<vector<u8>>,
        new_balance_R_eff_aud: vector<vector<u8>>, // new balance R component for the *effective* auditor only
        amount_P: vector<vector<u8>>,
        amount_R_sender: vector<vector<u8>>,
        amount_R_recip: vector<vector<u8>>,
        amount_R_eff_aud: vector<vector<u8>>, // amount R components for the *effective* auditor only
        ek_volun_auds: vector<vector<u8>>, // contains EKs for the *voluntary* auditors only
        amount_R_volun_auds: vector<vector<vector<u8>>>, // amount R components for the *voluntary* auditors only
        zkrp_new_balance: vector<u8>,
        zkrp_amount: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let compressed_new_balance = new_compressed_available_from_bytes(new_balance_P, new_balance_R, new_balance_R_eff_aud);

        let compressed_amount = confidential_amount::new_compressed_from_bytes(
            amount_P, amount_R_sender, amount_R_recip, amount_R_eff_aud, amount_R_volun_auds,
        );

        let compressed_ek_volun_auds = ek_volun_auds.map(|bytes| {
            new_compressed_point_from_bytes(bytes).extract()
        });

        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
        let zkrp_amount = bulletproofs::range_proof_from_bytes(zkrp_amount);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = TransferProof::V1 {
            compressed_new_balance,
            compressed_amount,
            compressed_ek_volun_auds,
            zkrp_new_balance, zkrp_amount, sigma
        };

        confidential_transfer(
            sender,
            asset_type,
            to,
            proof
        )
    }

    /// Transfers a secret amount of tokens from sender's available balance to recipient's pending balance.
    public fun confidential_transfer(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        proof: TransferProof
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(to, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));

        let from = signer::address_of(sender);
        assert!(from != to, error::invalid_argument(E_SELF_TRANSFER));
        let ek_effective_auditor = get_effective_auditor(asset_type);
        let ek_sender = get_encryption_key(from, asset_type);
        let ek_recip = get_encryption_key(to, asset_type);
        let old_balance = get_available_balance(from, asset_type);

        // Note: Sender's amount is not used;y only included for indexing to reliably pick it up for dapps that need it
        let (compressed_new_balance,recipient_amount) =
            assert_valid_transfer_proof(
                sender, to, asset_type,
                &ek_sender, &ek_recip,
                &old_balance, &ek_effective_auditor,
                proof
            );

        // Update sender's confidential store
        let sender_ca_store = borrow_confidential_store_mut(from, asset_type);
        sender_ca_store.normalized = true;
        sender_ca_store.available_balance = compressed_new_balance;

        // Update recipient's confidential store
        let recip_ca_store = borrow_confidential_store_mut(to, asset_type);
        add_assign_pending(&mut recip_ca_store.pending_balance, &recipient_amount);
        recip_ca_store.transfers_received += 1;

        assert!(
            recip_ca_store.transfers_received <= MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        event::emit(Transferred::V1 { from, to, asset_type });
    }

    /// Deserializes cryptographic data and forwards to `rotate_encryption_key`.
    public entry fun rotate_encryption_key_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_ek: vector<u8>,
        resume_incoming_transfers: bool,
        new_R: vector<vector<u8>>, // part of the proof
        sigma_proto_comm: vector<vector<u8>>, // part of the proof
        sigma_proto_resp: vector<vector<u8>>, // part of the proof
    ) acquires ConfidentialStore {
        // Just parse stuff and forward to the more type-safe function
        let compressed_new_ek = new_compressed_point_from_bytes(new_ek).extract();
        let compressed_new_R = deserialize_compressed_points(new_R);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(
            sigma_proto_comm, sigma_proto_resp
        );

        rotate_encryption_key(
            sender, asset_type,
            KeyRotationProof::V1 { compressed_new_ek, compressed_new_R, sigma },
            resume_incoming_transfers
        );
    }

    public fun rotate_encryption_key(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        proof: KeyRotationProof,
        resume_incoming_transfers: bool,
    ) {
        // Step 1: Assert (a) incoming transfers are paused & (b) pending balance is zero / has been rolled over
        let ca_store = borrow_confidential_store_mut(signer::address_of(owner), asset_type);
        // (a) Assert incoming transfers are paused & unpause them after, if flag is set.
        assert!(ca_store.pause_incoming, error::invalid_state(E_INCOMING_TRANSFERS_NOT_PAUSED));
        // (b) The user must have called `rollover_pending_balance` before rotating their key.
        assert!(
            ca_store.pending_balance.is_zero(),
            error::invalid_state(E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION)
        );
        // Over-asserting invariants, in an abundance of caution.
        assert!(
            ca_store.transfers_received == 0,
            error::invalid_state(E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION)
        );

        // Step 2: Verify the $\Sigma$-protocol proof of correct re-encryption
        let (compressed_new_ek, compressed_new_R) = assert_valid_key_rotation_proof(
            owner, asset_type, &ca_store.ek, &ca_store.available_balance, proof
        );

        // Step 3: Install the new EK and the new re-encrypted available balance
        ca_store.ek = compressed_new_ek;
        // We're just updating the available balance's EK-dependant R component & leaving the pending balance the same.
        confidential_balance::set_available_R(&mut ca_store.available_balance, compressed_new_R);
        if (resume_incoming_transfers) {
            ca_store.pause_incoming = false;
        }
    }

    /// Deserializes cryptographic data and ultimately forwards to `withdraw_to` with amount = 0.
    public entry fun normalize_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_balance_P: vector<vector<u8>>,
        new_balance_R: vector<vector<u8>>,
        new_balance_R_aud: vector<vector<u8>>,  // effective auditor's R component
        zkrp_new_balance: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let user = signer::address_of(sender);
        assert!(!is_normalized(user, asset_type), error::invalid_state(E_ALREADY_NORMALIZED));

        withdraw_to_raw(
            sender, asset_type, user, 0,
            new_balance_P, new_balance_R, new_balance_R_aud,
            zkrp_new_balance, sigma_proto_comm, sigma_proto_resp
        );
    }

    /// Re-encrypts the available balance to ensure all chunks are within 16-bit bounds (required before rollover).
    public fun normalize(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        proof: WithdrawalProof
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let user = signer::address_of(sender);
        assert!(!is_normalized(user, asset_type), error::invalid_state(E_ALREADY_NORMALIZED));

        // Normalization is withdrawal with v = 0
        withdraw_to(sender, asset_type, user, 0, proof);
    }

    /// Rolls over pending balance into available balance, resetting pending to zero.
    public entry fun rollover_pending_balance(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);
        let ca_store = borrow_confidential_store_mut(user, asset_type);

        assert!(ca_store.normalized, error::invalid_state(E_NORMALIZATION_REQUIRED));
        assert!(ca_store.transfers_received > 0, error::invalid_state(E_NOTHING_TO_ROLLOVER));

        add_assign_available_excluding_auditor(&mut ca_store.available_balance, &ca_store.pending_balance);
        // A components remain stale — will be refreshed on normalize/withdraw/transfer

        ca_store.normalized = false;
        ca_store.transfers_received = 0;
        ca_store.pending_balance = new_zero_pending_compressed();
    }

    /// Rollover + pause incoming transfers (required before key rotation).
    public entry fun rollover_pending_balance_and_pause(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        rollover_pending_balance(sender, asset_type);
        set_incoming_transfers_paused(sender, asset_type, true);
    }

    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //
    //                                             //
    // ^^^ End of SECURITY-SENSITIVE functions ^^^ //
    //                                             //
    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //

    // === Public, non-security-sensitive functions (8 out of 14) ===
    //
    // Note: These functions can be useful for external contracts that want to integrate with the Confidential Asset
    // protocol.

    /// Pauses or resumes incoming transfers. Pausing is required before key rotation.
    public entry fun set_incoming_transfers_paused(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        paused: bool
    ) acquires ConfidentialStore {
        borrow_confidential_store_mut(signer::address_of(owner), asset_type).pause_incoming = paused;
    }

    // ==================================================================== //
    //     SECURITY-SENSITIVE public governance functions (9 out of 14)     //
    //               (bugs here could lead to loss of privacy)              //
    // ==================================================================== //

    /// Enables or disables the allow list for confidential transfers.
    public fun set_allow_listing(aptos_framework: &signer, enabled: bool) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        borrow_global_mut<GlobalConfig>(@aptos_experimental).allow_list_enabled = enabled;
    }

    /// Enables or disables confidential transfers for a specific asset type.
    public fun set_confidentiality_for_asset_type(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        allowed: bool
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        let asset_config = borrow_global_mut<AssetConfig>(get_asset_config_address_or_create(asset_type));
        asset_config.allowed = allowed;
    }

    /// Sets or removes the auditor for a specific asset type. Epoch increments only on install/change.
    public fun set_auditor_for_asset_type(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        auditor_ek: Option<vector<u8>>
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let asset_config = borrow_global_mut<AssetConfig>(get_asset_config_address_or_create(asset_type));
        update_auditor(&mut asset_config.auditor, auditor_ek);
    }

    /// Sets or removes the global auditor (fallback when no asset-specific auditor). Epoch increments only on install/change.
    public fun set_global_auditor(aptos_framework: &signer, auditor_ek: Option<vector<u8>>) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let config = borrow_global_mut<GlobalConfig>(@aptos_experimental);
        update_auditor(&mut config.global_auditor, auditor_ek);
    }

    /// Shared logic for setting/removing an auditor EK. Validates non-identity, increments epoch on install/change.
    fun update_auditor(auditor: &mut AuditorEK, new_ek_bytes: Option<vector<u8>>) {
        let new_ek = new_ek_bytes.map(|ek| new_compressed_point_from_bytes(ek).extract());

        if (new_ek.is_some()) {
            assert!(!new_ek.borrow().is_identity(), error::invalid_argument(E_AUDITOR_EK_IS_IDENTITY));
        };

        // Increment epoch only when installing or changing the EK (not when removing)
        let should_increment = if (new_ek.is_some()) {
            if (auditor.ek.is_some()) {
                !new_ek.borrow().compressed_point_equals(auditor.ek.borrow())
            } else {
                true // None → Some: installing
            }
        } else {
            false // removing or no-op
        };

        if (should_increment) {
            auditor.epoch += 1;
        };

        auditor.ek = new_ek;
    }

    // ============================================================== //
    //     End of SECURITY-SENSITIVE public governance functions      //
    // ============================================================== //

    // === Public view functions (10 out of 14) ===

    #[view]
    public fun has_confidential_store(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): bool {
        exists<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    #[view]
    public fun is_confidentiality_enabled_for_asset_type(asset_type: Object<fungible_asset::Metadata>): bool acquires GlobalConfig, AssetConfig {
        if (!is_allow_listing_required()) {
            return true
        };

        let asset_config_address = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(asset_config_address)) {
            return false
        };

        borrow_global<AssetConfig>(asset_config_address).allowed
    }

    #[view]
    /// If the allow list is enabled, only asset types from the allow list can be transferred confidentially. Otherwise, all asset types are allowed.
    public fun is_allow_listing_required(): bool acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).allow_list_enabled
    }

    #[view]
    public fun get_pending_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedBalance<Pending> acquires ConfidentialStore {
        borrow_confidential_store(owner, asset_type).pending_balance
    }

    #[view]
    public fun get_available_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedBalance<Available> acquires ConfidentialStore {
        borrow_confidential_store(owner, asset_type).available_balance
    }

    #[view]
    public fun get_encryption_key(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedRistretto acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).ek
    }

    #[view]
    public fun is_normalized(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): bool acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).normalized
    }

    #[view]
    public fun incoming_transfers_paused(user: address, asset_type: Object<fungible_asset::Metadata>): bool acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).pause_incoming
    }

    #[view]
    /// This ignores the global auditor, if any, and only returns the asset-specific auditor EK. Also, it returns the EK
    /// even if the asset_type is no longer allow-listed.
    public fun get_auditor_for_asset_type(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires AssetConfig, GlobalConfig {
        let asset_config_address = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(asset_config_address)) {
            return std::option::none();
        };

        borrow_global<AssetConfig>(asset_config_address).auditor.ek
    }

    #[view]
    public fun get_global_auditor(): Option<CompressedRistretto> acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor.ek
    }

    #[view]
    /// Returns the effective auditor: asset-specific if set, else global.
    public fun get_effective_auditor(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires AssetConfig, GlobalConfig {
        // 1. Check asset-specific auditor
        let config_addr = get_asset_config_address(asset_type);
        if (exists<AssetConfig>(config_addr)) {
            let asset_auditor = borrow_global<AssetConfig>(config_addr).auditor.ek;
            if (asset_auditor.is_some()) {
                return asset_auditor
            };
        };
        // 2. Fall back to global auditor
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor.ek
    }

    #[view]
    public fun get_global_auditor_epoch(): u64 acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor.epoch
    }

    #[view]
    public fun get_auditor_epoch_for_asset_type(
        asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires AssetConfig, GlobalConfig {
        let asset_config_address = get_asset_config_address(asset_type);
        if (!exists<AssetConfig>(asset_config_address)) {
            return 0
        };
        borrow_global<AssetConfig>(asset_config_address).auditor.epoch
    }

    #[view]
    public fun get_effective_auditor_epoch(
        asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires AssetConfig, GlobalConfig {
        let config_addr = get_asset_config_address(asset_type);
        if (exists<AssetConfig>(config_addr)) {
            let ac = borrow_global<AssetConfig>(config_addr);
            if (ac.auditor.ek.is_some()) {
                return ac.auditor.epoch
            };
        };
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor.epoch
    }

    #[view]
    /// Returns the circulating supply of the confidential asset.
    public fun get_total_confidential_supply(asset_type: Object<fungible_asset::Metadata>): u64 acquires GlobalConfig {
        let fa_store_address = get_global_config_address();
        assert!(
            primary_fungible_store::primary_store_exists(fa_store_address, asset_type),
            error::not_found(E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE)
        );

        primary_fungible_store::balance(fa_store_address, asset_type)
    }

    #[view]
    public fun get_num_transfers_received(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).transfers_received
    }

    #[view]
    public fun get_max_transfers_before_rollover(): u64 {
        MAX_TRANSFERS_BEFORE_ROLLOVER
    }

    // === Private, internal functions (11 out of 14) ===

    fun get_asset_config_address(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let config_ext = &borrow_global<GlobalConfig>(@aptos_experimental).extend_ref;
        let config_ext_address = object::address_from_extend_ref(config_ext);
        object::create_object_address(&config_ext_address, construct_asset_config_seed(asset_type))
    }

    fun get_asset_config_address_or_create(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let addr = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(addr)) {
            let asset_config_signer = get_asset_config_signer(asset_type);

            move_to(
                &asset_config_signer,
                // We disallow the asset type from being made confidential since this function is
                // called in a lot of different contexts.
                AssetConfig::V1 { allowed: false, auditor: AuditorEK::V1 { ek: std::option::none(), epoch: 0 } }
            );
        };

        addr
    }

    fun get_global_config_signer(): signer acquires GlobalConfig {
        object::generate_signer_for_extending(&borrow_global<GlobalConfig>(@aptos_experimental).extend_ref)
    }

    fun get_global_config_address(): address acquires GlobalConfig {
        object::address_from_extend_ref(&borrow_global<GlobalConfig>(@aptos_experimental).extend_ref)
    }

    fun get_confidential_store_signer(user: &signer, asset_type: Object<fungible_asset::Metadata>): signer {
        object::generate_signer(&object::create_named_object(user, construct_confidential_store_seed(asset_type)))
    }

    fun get_confidential_store_address(user: address, asset_type: Object<fungible_asset::Metadata>): address {
        object::create_object_address(&user, construct_confidential_store_seed(asset_type))
    }

    inline fun borrow_confidential_store(user: address, asset_type: Object<fungible_asset::Metadata>): &ConfidentialStore {
        assert!(has_confidential_store(user, asset_type), error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED));
        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    inline fun borrow_confidential_store_mut(user: address, asset_type: Object<fungible_asset::Metadata>): &mut ConfidentialStore {
        assert!(has_confidential_store(user, asset_type), error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED));
        borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    fun get_asset_config_signer(asset_type: Object<fungible_asset::Metadata>): signer acquires GlobalConfig {
        let config_ext = &borrow_global<GlobalConfig>(@aptos_experimental).extend_ref;
        let config_ext_signer = object::generate_signer_for_extending(config_ext);

        let config_ctor =
            &object::create_named_object(&config_ext_signer, construct_asset_config_seed(asset_type));

        object::generate_signer(config_ctor)
    }

    /// Unique seed per (user, asset-type) for the ConfidentialStore object address.
    fun construct_confidential_store_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::ConfidentialStore",
                @aptos_experimental,
                object::object_address(&asset_type)
            )
        )
    }

    /// Unique seed per asset-type for the AssetConfig object address.
    fun construct_asset_config_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::AssetConfig",
                @aptos_experimental,
                object::object_address(&asset_type)
            )
        )
    }

    // === Test-only functions (12 out of 14) ===

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
            get_pending_balance(user, asset_type).decompress();

        pending_balance.check_decrypts_to(pending_balance.get_R(), user_dk, (amount as u128))
    }

    #[test_only]
    /// Checks that the available balance decrypts to `amount`.
    /// When `is_auditor_dk` is true, decrypts the auditor's ciphertext rather than the user's.
    public fun check_available_balance_decrypts_to(
        user: address,
        asset_type: Object<fungible_asset::Metadata>,
        dk: &Scalar,
        amount: u128,
        is_auditor_dk: bool,
    ): bool acquires ConfidentialStore {
        let available_balance =
            get_available_balance(user, asset_type).decompress();

        let decrypt_R = if (is_auditor_dk) { available_balance.get_R_aud() } else { available_balance.get_R() };
        available_balance.check_decrypts_to(decrypt_R, dk, amount)
    }

    #[test_only]
    public fun get_amount_ciphertext_for_effective_auditor(proof: &TransferProof): Balance<Pending> {
        let TransferProof::V1 { compressed_amount, .. } = proof;
        let p = decompress_points(compressed_amount.get_compressed_P());
        let r_eff_aud = decompress_points(compressed_amount.get_compressed_R_eff_aud());
        new_pending_from_p_and_r(p, r_eff_aud)
    }

    #[test_only]
    public fun get_amount_ciphertexts_for_volun_auditors(proof: &TransferProof): vector<Balance<Pending>> {
        let TransferProof::V1 { compressed_amount, .. } = proof;
        let p = decompress_points(compressed_amount.get_compressed_P());
        compressed_amount.get_compressed_R_volun_auds().map_ref(|r| {
            new_pending_from_p_and_r(points_clone(&p), decompress_points(r))
        })
    }

    // === Proof enums and verification functions (13 out of 14) ===

    /// Proof of knowledge of DK for registration: $\Sigma$-protocol proving $H = \mathsf{dk} \cdot \mathsf{ek}$.
    enum RegistrationProof has drop {
        V1 {
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Withdrawal proof: new normalized balance, range proof, and $\Sigma$-protocol for $\mathcal{R}^{-}_\mathsf{withdraw}$.
    enum WithdrawalProof has drop {
        V1 {
            compressed_new_balance: CompressedBalance<Available>,
            zkrp_new_balance: RangeProof,
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Transfer proof: new balance, encrypted amount, range proofs, and $\Sigma$-protocol for $\mathcal{R}^{-}_\mathsf{txfer}$.
    enum TransferProof has drop {
        V1 {
            compressed_new_balance: CompressedBalance<Available>,
            compressed_amount: CompressedAmount,
            compressed_ek_volun_auds: vector<CompressedRistretto>,
            zkrp_new_balance: RangeProof,
            zkrp_amount: RangeProof,
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Key rotation proof: new EK, re-encrypted R components, and $\Sigma$-protocol for correct re-encryption.
    enum KeyRotationProof has drop {
        V1 {
            compressed_new_ek: CompressedRistretto,
            compressed_new_R: vector<CompressedRistretto>,
            sigma: sigma_protocol_proof::Proof,
        }
    }

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                                         //
    // *** SECURITY-SENSITIVE proof verification functions *** //
    //         (bugs here could lead to stolen funds)          //
    //                                                         //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

    fun assert_valid_registration_proof(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: &CompressedRistretto,
        proof: RegistrationProof
    ) {
        let RegistrationProof::V1 { sigma } = proof;
        let stmt = sigma_protocol_registration::new_registration_statement(*ek);
        let session = sigma_protocol_registration::new_session(sender, asset_type);
        session.assert_verifies(&stmt, &sigma);
    }

    /// Verifies range proof + $\Sigma$-protocol for withdrawal. Returns compressed new balance.
    fun assert_valid_withdrawal_proof(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: &CompressedRistretto,
        amount: u64,
        old_balance: &CompressedBalance<Available>,
        compressed_ek_aud: &Option<CompressedRistretto>,
        proof: WithdrawalProof
    ): CompressedBalance<Available> {
        let WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma } = proof;

        let v = new_scalar_from_u64(amount);

        let (stmt, new_balance_P) = sigma_protocol_withdraw::new_withdrawal_statement(
            *ek, old_balance, &compressed_new_balance, compressed_ek_aud, v,
        );
        confidential_range_proofs::assert_valid_range_proof(&new_balance_P, &zkrp_new_balance);

        let session = sigma_protocol_withdraw::new_session(sender, asset_type, compressed_ek_aud.is_some());
        session.assert_verifies(&stmt, &sigma);
        compressed_new_balance
    }

    /// Verifies range proofs + $\Sigma$-protocol for transfer. Returns (new_balance, recipient_pending).
    fun assert_valid_transfer_proof(
        sender: &signer,
        recipient_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        compressed_ek_sender: &CompressedRistretto,
        compressed_ek_recip: &CompressedRistretto,
        compressed_old_balance: &CompressedBalance<Available>,
        compressed_ek_eff_aud: &Option<CompressedRistretto>,
        proof: TransferProof
    ): (
        CompressedBalance<Available>,
        Balance<Pending>,
    ) {

        let TransferProof::V1 {
            compressed_new_balance, compressed_amount,
            compressed_ek_volun_auds,
            zkrp_new_balance, zkrp_amount, sigma
        } = proof;

        let has_effective_auditor = compressed_ek_eff_aud.is_some();
        let num_volun_auditors = compressed_ek_volun_auds.length();

        // Auditor count checks are performed inside new_transfer_statement
        let (stmt, new_balance_P, recip_pending) = sigma_protocol_transfer::new_transfer_statement(
            *compressed_ek_sender, *compressed_ek_recip,
            compressed_old_balance, &compressed_new_balance,
            &compressed_amount,
            compressed_ek_eff_aud, &compressed_ek_volun_auds,
        );

        confidential_range_proofs::assert_valid_range_proof(recip_pending.get_P(), &zkrp_amount);
        confidential_range_proofs::assert_valid_range_proof(&new_balance_P, &zkrp_new_balance);

        let session = sigma_protocol_transfer::new_session(
            sender, recipient_addr, asset_type, has_effective_auditor, num_volun_auditors,
        );
        session.assert_verifies(&stmt, &sigma);

        (compressed_new_balance, recip_pending)
    }

    fun assert_valid_key_rotation_proof(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        old_ek: &CompressedRistretto,
        old_balance: &CompressedBalance<Available>,
        proof: KeyRotationProof
    ): (CompressedRistretto, vector<CompressedRistretto>) {
        let KeyRotationProof::V1 { compressed_new_ek, compressed_new_R, sigma } = proof;

        let stmt = sigma_protocol_key_rotation::new_key_rotation_statement(
            *old_ek,
            compressed_new_ek,
            old_balance.get_compressed_R(),
            &compressed_new_R,
        );

        let session = sigma_protocol_key_rotation::new_session(owner, asset_type);
        session.assert_verifies(&stmt, &sigma);

        (compressed_new_ek, compressed_new_R)
    }

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                                                //
    // *** End of SECURITY-SENSITIVE proof verification functions *** //
    //                                                                //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

    // === Test-only proof generation functions (14 out of 14) ===

    #[test_only]
    public fun prove_registration(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        dk: &Scalar,
    ): RegistrationProof {
        let (stmt, witn) = sigma_protocol_registration::compute_statement_and_witness(dk);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);
        let session = sigma_protocol_registration::new_session(&sender, asset_type);
        RegistrationProof::V1 { sigma: session.prove(&stmt, &witn) }
    }

    #[test_only]
    fun prove_withdrawal_internal(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        dk_sender: &Scalar,
        v: u64,
        new_amount: u128,
    ): (CompressedBalance<Available>, RangeProof, sigma_protocol_proof::Proof)
        acquires ConfidentialStore, AssetConfig, GlobalConfig
    {
        let ek = get_encryption_key(sender_addr, asset_type);
        let compressed_ek_aud = get_effective_auditor(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);

        let new_balance_randomness = generate_available_randomness();
        let new_balance = new_available_from_amount(
            new_amount, &new_balance_randomness, &ek, &compressed_ek_aud
        );

        let new_r = new_balance_randomness.scalars();
        let new_a = split_available_into_chunks(new_amount);
        let zkrp_new_balance = confidential_range_proofs::prove_range(&new_a, new_r);

        let v = new_scalar_from_u64(v);
        let compressed_old_balance = get_available_balance(sender_addr, asset_type);
        let compressed_new_balance = new_balance.compress();
        let (stmt, _) = sigma_protocol_withdraw::new_withdrawal_statement(
            ek, &compressed_old_balance, &compressed_new_balance, &compressed_ek_aud, v
        );
        let witn = sigma_protocol_withdraw::new_withdrawal_witness(*dk_sender, new_a, *new_r);
        let session = sigma_protocol_withdraw::new_session(&sender, asset_type, compressed_ek_aud.is_some());
        (compressed_new_balance, zkrp_new_balance, session.prove(&stmt, &witn))
    }

    #[test_only]
    public fun prove_withdrawal(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount: u64,
        new_amount: u128,
    ): WithdrawalProof acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let (compressed_new_balance, zkrp_new_balance, sigma) =
            prove_withdrawal_internal(sender_addr, asset_type, sender_dk, amount, new_amount);
        WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma }
    }

    #[test_only]
    public fun prove_normalization(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount: u128,
    ): WithdrawalProof acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let (compressed_new_balance, zkrp_new_balance, sigma) =
            prove_withdrawal_internal(sender_addr, asset_type, sender_dk, 0, amount);
        WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma }
    }

    #[test_only]
    public fun prove_transfer(
        sender_addr: address,
        recipient_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount_u64: u64,
        new_balance_u128: u128,
        compressed_ek_volun_auds: &vector<CompressedRistretto>,
    ): TransferProof acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let ek_sender = get_encryption_key(sender_addr, asset_type);
        let ek_recipient = get_encryption_key(recipient_addr, asset_type);
        let compressed_old_balance = get_available_balance(sender_addr, asset_type);
        let compressed_ek_eff_aud = get_effective_auditor(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);
        let has_effective_auditor = compressed_ek_eff_aud.is_some();
        let num_volun_auditors = compressed_ek_volun_auds.length();

        let new_balance_randomness = generate_available_randomness();
        let amount_randomness = generate_pending_randomness();

        let (stmt, witn, new_balance, amount) =
            sigma_protocol_transfer::build_transfer_statement_and_witness(
                sender_dk, &ek_sender, &ek_recipient, &compressed_old_balance,
                &compressed_ek_eff_aud, compressed_ek_volun_auds,
                amount_u64, new_balance_u128, &new_balance_randomness, &amount_randomness,
            );

        let new_a = split_available_into_chunks(new_balance_u128);
        let v = split_pending_into_chunks((amount_u64 as u128));
        let zkrp_new_balance = confidential_range_proofs::prove_range(&new_a, new_balance_randomness.scalars());
        let zkrp_amount = confidential_range_proofs::prove_range(&v, amount_randomness.scalars());

        let session = sigma_protocol_transfer::new_session(
            &sender, recipient_addr, asset_type, has_effective_auditor, num_volun_auditors,
        );

        TransferProof::V1 {
            compressed_new_balance: new_balance.compress(),
            compressed_amount: amount.compress(),
            compressed_ek_volun_auds: *compressed_ek_volun_auds,
            zkrp_new_balance, zkrp_amount, sigma: session.prove(&stmt, &witn)
        }
    }

    #[test_only]
    public fun prove_key_rotation(
        owner_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        new_dk: &Scalar,
    ): KeyRotationProof acquires ConfidentialStore {
        let owner = aptos_framework::account::create_signer_for_test(owner_addr);

        // Get old EK and available balance
        let compressed_old_ek = get_encryption_key(owner_addr, asset_type);
        let available_balance = get_available_balance(owner_addr, asset_type);

        // Build statement and witness using the helper
        let (stmt, witn, compressed_new_ek, compressed_new_R) =
            sigma_protocol_key_rotation::compute_statement_and_witness_from_keys_and_old_ctxt(
                sender_dk, new_dk,
                compressed_old_ek,
                available_balance.get_compressed_R(),
            );

        // Prove
        let session = sigma_protocol_key_rotation::new_session(&owner, asset_type);

        KeyRotationProof::V1 {
            compressed_new_ek,
            compressed_new_R,
            sigma: session.prove(&stmt, &witn),
        }
    }
}
