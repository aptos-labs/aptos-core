/// This module implements the Confidential Asset (CA) Standard, a privacy-focused protocol for managing fungible assets (FA).
/// It enables private transfers by obfuscating transaction amounts while keeping sender and recipient addresses visible.
module aptos_experimental::confidential_asset {
    use std::bcs;
    use std::error;
    use std::option::Option;
    use std::signer;
    use aptos_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};
    use aptos_std::string_utils;
    use aptos_framework::chain_id;
    use aptos_framework::event;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::Self;
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_experimental::sigma_protocol_utils::{deserialize_points, decompress_points, points_clone};
    use aptos_experimental::confidential_pending_balance::{Self, PendingBalance, CompressedPendingBalance};
    use aptos_experimental::confidential_available_balance::{Self, AvailableBalance, CompressedAvailableBalance};
    use aptos_experimental::sigma_protocol_key_rotation;
    use aptos_experimental::sigma_protocol_registration;
    use aptos_experimental::sigma_protocol_withdraw;
    use aptos_experimental::sigma_protocol_transfer;
    use aptos_experimental::sigma_protocol_proof;
    use aptos_experimental::confidential_proof;
    use aptos_experimental::ristretto255_twisted_elgamal;

    #[test_only]
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::compress_points;

    // === Errors (2 out of 14) ===

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

    /// The number of auditor D-components in the proof does not match the expected auditor count.
    const E_AUDITOR_COUNT_MISMATCH: u64 = 12;

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

    /// A resource that represents the global configuration for the confidential asset protocol, "installed" during
    /// `init_module` at @aptos_experimental.
    enum GlobalConfig has key {
        V1 {
            /// Indicates whether the allow list is enabled. If `true`, only asset types from the allow list can be transferred.
            /// This flag is managed by the governance module.
            allow_list_enabled: bool,

            /// The global auditor's encryption key. If set, all confidential transfers must include the auditor
            /// as an additional party who can decrypt the transferred amount. Asset-specific auditors take
            /// precedence over this global auditor. If neither is set, no auditor is required.
            global_auditor_ek: Option<CompressedRistretto>,

            /// Tracks how many times the global auditor EK has been installed or changed (not removed).
            /// Starts at 0 and increments each time a new EK is set (None→Some or Some(old)→Some(new)).
            global_auditor_epoch: u64,

            /// Used to derive a signer that owns all the FAs' primary stores and `AssetConfig` objects.
            extend_ref: ExtendRef
        }
    }

    /// An object that represents the per-asset-type configuration.
    enum AssetConfig has key {
        V1 {
            /// Indicates whether the asset type is allowed for confidential transfers, can be toggled by the governance
            /// module. Withdrawals are always allowed, even when this is set to `false`.
            /// If `GlobalConfig::allow_list_enabled` is `false`, all asset types are allowed, even if this is `false`.
            allowed: bool,

            /// The auditor's public key for the asset type. If the auditor is not set, this field is `None`.
            /// Otherwise, each confidential transfer must include the auditor as an additional party,
            /// alongside the recipient, who has access to the decrypted transferred amount.
            ///
            /// TODO(Feature): add support for multiple auditors here
            auditor_ek: Option<CompressedRistretto>,

            /// Tracks how many times the asset-specific auditor EK has been installed or changed (not removed).
            /// Starts at 0 and increments each time a new EK is set (None→Some or Some(old)→Some(new)).
            auditor_epoch: u64,
        }
    }

    /// An object that stores the encrypted balances for a specific confidential asset type and owning user.
    /// This should be thought of as a confidential variant of `aptos_framework::fungible_asset::FungibleStore`.
    ///
    /// e.g., for Alice's confidential APT, such an object will be created and stored at an Alice-specific and APT-specific
    ///   address. It will track Alice's confidential APT balance.
    enum ConfidentialStore has key {
        V1 {
            /// Indicates if incoming transfers are paused for this asset type, which is necessary to ensure the pending
            /// balance does not change during a key rotation, which would invalidate that key rotation and leave the account
            /// in an inconsistent state.
            pause_incoming: bool,

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
            pending_balance: CompressedPendingBalance,

            /// Represents the user's balance that is available for sending payments.
            /// It consists of eight 16-bit chunks $(a_0 + 2^{16} \cdot a_1 + ... + (2^{16})^15 \cdot a_15)$, supporting a
            /// 128-bit balance. Includes A components for auditor decryption (empty if no auditor).
            available_balance: CompressedAvailableBalance,

            /// The encryption key associated with the user's confidential asset account, different for each asset type.
            ek: CompressedRistretto
        }
    }

    // === Events (5 out of 14) ===

    #[event]
    /// Emitted when someone brings confidential assets into the protocol via `deposit`: i.e., by depositing a fungible
    /// asset into the "confidential pool" and minting a confidential asset as "proof" of this.
    enum Deposited has drop, store {
        V1 {
            addr: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    #[event]
    /// Emitted when someone brings confidential assets out of the protocol via `withdraw_to`: i.e., by burning a confidential
    /// asset as "proof" of being allowed to withdraw a fungible asset from the "confidential pool."
    enum Withdrawn has drop, store {
        V1 {
            from: address,
            to: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    #[event]
    /// Emitted when confidential assets are transferred within the protocol between users' confidential balances.
    /// Note that a numeric amount is not included, as the whole point of the protocol is to avoid leaking it.
    enum Transferred has drop, store {
        V1 {
            from: address,
            to: address,
            asset_type: Object<fungible_asset::Metadata>
        }
    }

    // === Module initialization (6 out of 14) ===

    /// Called only once, when this module is first published on the blockchain.
    fun init_module(deployer: &signer) {
        // This is me being overly cautious: I added it to double-check my understanding that the VM always passes
        // the publishing account as deployer. It does, so the assert is redundant (it can never fail).
        assert!(signer::address_of(deployer) == @aptos_experimental, error::internal(E_INTERNAL_ERROR));

        assert!(
            bulletproofs::get_max_range_bits() >= confidential_proof::get_bulletproofs_num_bits(),
            error::internal(E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        let deployer_address = signer::address_of(deployer);
        let is_mainnet = chain_id::get() == MAINNET_CHAIN_ID;

        move_to(
            deployer,
            GlobalConfig::V1 {
                allow_list_enabled: is_mainnet,
                global_auditor_ek: std::option::none(),
                global_auditor_epoch: 0,
                // DO NOT CHANGE: using long syntax until framework change is released to mainnet
                extend_ref: object::generate_extend_ref(&object::create_object(deployer_address))
            }
        );

        // On mainnet, allow APT by default
        if (is_mainnet) {
            let apt_metadata = object::address_to_object<fungible_asset::Metadata>(@aptos_fungible_asset);
            let config_signer = get_asset_config_signer(apt_metadata);
            move_to(&config_signer, AssetConfig::V1 { allowed: true, auditor_ek: std::option::none(), auditor_epoch: 0 });
        };
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

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                                    //
    // *** SECURITY-SENSITIVE functions (7 out of 14) *** //
    //         (bugs here could lead to stolen funds)     //
    //                                                    //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

    /// Registers an account for a specified asset type.
    /// Parses arguments and forwards to `register`; see that function for details.
    public entry fun register_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires GlobalConfig, AssetConfig {
        let ek = ristretto255::new_compressed_point_from_bytes(ek).extract();
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = RegistrationProof::V1 { sigma };

        register(sender, asset_type, ek, proof);
    }

    /// Registers an a confidential store for a specified asset type, encrypted under the given EK.
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
            pending_balance: confidential_pending_balance::new_zero_compressed(),
            available_balance: confidential_available_balance::new_zero_compressed(),
            ek
        };

        move_to(&get_confidential_store_signer(sender, asset_type), ca_store);
    }

    /// Brings tokens into the protocol, transferring the passed amount from the sender's primary FA store
    /// to the sender's own pending balance.
    /// The initial confidential balance is publicly visible, as entering the protocol requires a normal transfer.
    /// However, tokens within the protocol become obfuscated through confidential transfers, ensuring privacy in
    /// subsequent transactions.
    ///
    /// For convenience, we sometimes refer to this operation as "veiling."
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

        ca_store.pending_balance.add_assign(&confidential_pending_balance::new_u64_no_randomness(amount));
        ca_store.transfers_received += 1;

        event::emit(Deposited::V1 { addr, amount, asset_type });
    }

    /// The same as `withdraw_to_raw`, but the recipient is the sender.
    public entry fun withdraw_raw(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>, amount: u64,
        new_balance_C: vector<vector<u8>>,
        new_balance_D: vector<vector<u8>>,
        new_balance_A: vector<vector<u8>>,
        zkrp_new_balance: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        withdraw_to_raw(
            sender,
            asset_type,
            signer::address_of(sender),
            amount,
            new_balance_C,
            new_balance_D,
            new_balance_A,
            zkrp_new_balance,
            sigma_proto_comm,
            sigma_proto_resp
        )
    }

    /// Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
    /// the recipient's primary FA store.
    /// Parses arguments and forwards to `withdraw_to`; see that function for details.
    public entry fun withdraw_to_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        new_balance_C: vector<vector<u8>>,
        new_balance_D: vector<vector<u8>>,
        new_balance_A: vector<vector<u8>>,
        zkrp_new_balance: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        let (new_P, compressed_P) = deserialize_points(new_balance_C);
        let (new_R, compressed_R) = deserialize_points(new_balance_D);
        let (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
        let new_balance = confidential_available_balance::new_from_p_r_r_aud(new_P, new_R, new_R_aud);
        let compressed_new_balance = confidential_available_balance::new_compressed_from_p_r_r_aud(
            compressed_P, compressed_R, compressed_R_aud
        );
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = WithdrawalProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma };

        withdraw_to(sender, asset_type, to, amount, proof);
    }

    /// Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
    /// the recipient's primary FA store.
    /// The withdrawn amount is publicly visible, as this process requires a normal transfer.
    /// The proof contains the sender's new normalized confidential balance, encrypted with fresh randomness.
    /// Withdrawals are always allowed, regardless of whether the asset type is allow-listed.
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
        let current_balance = get_available_balance(sender_addr, asset_type);
        let auditor_ek = get_effective_auditor(asset_type);

        let compressed_new_balance = assert_valid_withdrawal_proof(
            sender,
            asset_type,
            &ek,
            amount,
            &current_balance,
            &auditor_ek,
            proof
        );

        let ca_store = borrow_confidential_store_mut(sender_addr, asset_type);
        ca_store.normalized = true;
        ca_store.available_balance = compressed_new_balance;

        primary_fungible_store::transfer(&get_global_config_signer(), asset_type, to, amount);

        event::emit(Withdrawn::V1 { from: sender_addr, to, amount, asset_type });
    }

    /// Transfers tokens from the sender's available balance to the recipient's pending balance.
    /// Parses arguments and forwards to `confidential_transfer`; see that function for details.
    ///
    /// The `extra_auditor_eks` should contain only additional auditor EKs (not the global auditor,
    /// which is fetched automatically by the contract).
    ///
    /// Only the D components are sent for the recipient and auditors, since they share the same
    /// C components as the sender's amount (C_i = amount_i * G + r_i * H). This saves 128 bytes
    /// per party (recipient + each auditor).
    public entry fun confidential_transfer_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        new_balance_C: vector<vector<u8>>,
        new_balance_D: vector<vector<u8>>,
        new_balance_A: vector<vector<u8>>,
        sender_amount_C: vector<vector<u8>>,
        sender_amount_D: vector<vector<u8>>,
        recipient_amount_R: vector<vector<u8>>,
        extra_auditor_eks: vector<vector<u8>>,
        auditor_amount_Ds: vector<vector<vector<u8>>>,
        zkrp_new_balance: vector<u8>,
        zkrp_amount: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        // Deserialize all point components, obtaining both decompressed and compressed forms in one pass
        let (new_P, compressed_P) = deserialize_points(new_balance_C);
        let (new_R, compressed_R) = deserialize_points(new_balance_D);
        let (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
        let new_balance = confidential_available_balance::new_from_p_r_r_aud(new_P, new_R, new_R_aud);
        let compressed_new_balance = confidential_available_balance::new_compressed_from_p_r_r_aud(
            compressed_P, compressed_R, compressed_R_aud
        );

        let (sender_P, compressed_sender_P) = deserialize_points(sender_amount_C);
        let (sender_R, compressed_sender_R) = deserialize_points(sender_amount_D);
        let sender_amount = confidential_pending_balance::new_from_p_and_r(sender_P, sender_R);
        let compressed_sender_amount = confidential_pending_balance::new_compressed_from_p_and_r(
            compressed_sender_P, compressed_sender_R
        );

        let (recipient_amount_R, compressed_recipient_amount_R) = deserialize_points(recipient_amount_R);

        let extra_auditor_eks = extra_auditor_eks.map(|bytes| {
            ristretto255::new_compressed_point_from_bytes(bytes).extract()
        });

        let decompressed_auditor_amount_Ds = vector[];
        let compressed_auditor_Rs = vector[];
        auditor_amount_Ds.for_each(|auditor_d| {
            let (d, cd) = deserialize_points(auditor_d);
            decompressed_auditor_amount_Ds.push_back(d);
            compressed_auditor_Rs.push_back(cd);
        });

        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
        let zkrp_transfer_amount = bulletproofs::range_proof_from_bytes(zkrp_amount);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = TransferProof::V1 {
            new_balance, compressed_new_balance,
            sender_amount, compressed_sender_amount,
            recipient_amount_R, compressed_recip_amount_R: compressed_recipient_amount_R,
            auditor_amount_Ds: decompressed_auditor_amount_Ds, compressed_auditor_Rs,
            zkrp_new_balance, zkrp_amount: zkrp_transfer_amount, sigma
        };

        confidential_transfer(
            sender,
            asset_type,
            to,
            extra_auditor_eks,
            proof
        )
    }

    /// Transfers tokens from the sender's available balance to the recipient's pending balance.
    /// The function hides the transferred amount while keeping the sender and recipient addresses visible.
    /// The proof contains: the sender's new balance, the transfer amount encrypted for sender/recipient/auditors,
    /// and range proofs for the new balance and transfer amount.
    /// The `extra_auditor_eks` should contain any additional auditor EKs beyond the global auditor
    /// (which is fetched automatically by the contract).
    public fun confidential_transfer(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        extra_auditor_eks: vector<CompressedRistretto>,
        proof: TransferProof
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(to, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));

        let from = signer::address_of(sender);

        // Compute extra auditor count before appending effective auditor
        let num_extra_auditors = extra_auditor_eks.length();

        // Append effective auditor EK (asset-specific first, then global fallback) to extra_auditor_eks
        let effective_auditor_ek = get_effective_auditor(asset_type);
        let has_effective_auditor = effective_auditor_ek.is_some();
        if (has_effective_auditor) {
            extra_auditor_eks.push_back(effective_auditor_ek.extract());
        };

        // Read values before mutable borrow to avoid conflicting borrows of ConfidentialStore
        let sender_ek = get_encryption_key(from, asset_type);
        let recipient_ek = get_encryption_key(to, asset_type);
        let sender_available_balance = get_available_balance(from, asset_type);

        // Note: Sender's amount is not used: we pass it as an argument just for visibility, so that indexing can reliably
        // pick it up for dapps that need to decrypt it quickly.
        let (compressed_new_balance, _sender_amount, recipient_amount, _auditor_amounts) =
            assert_valid_transfer_proof(
                sender,
                to,
                asset_type,
                &sender_ek,
                &recipient_ek,
                &sender_available_balance,
                &extra_auditor_eks,
                has_effective_auditor,
                num_extra_auditors,
                proof
            );

        // Update sender's confidential store
        let sender_ca_store = borrow_confidential_store_mut(from, asset_type);
        sender_ca_store.normalized = true;
        sender_ca_store.available_balance = compressed_new_balance;

        // Update recipient's confidential store
        let recip_ca_store = borrow_confidential_store_mut(to, asset_type);
        // Make sure the receiver has "room" in their pending balance for this transfer
        assert!(
            recip_ca_store.transfers_received < MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );
        recip_ca_store.pending_balance.add_assign(&recipient_amount);
        recip_ca_store.transfers_received += 1;

        event::emit(Transferred::V1 { from, to, asset_type });
    }

    /// Rotates the encryption key for the user's confidential balance, updating it to a new encryption key.
    /// Parses arguments and forwards to `rotate_encryption_key`; see that function for details.
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
        let (new_ek, compressed_new_ek) = ristretto255::new_point_and_compressed_from_bytes(new_ek);
        let (new_R, compressed_new_R) = deserialize_points(new_R);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(
            sigma_proto_comm, sigma_proto_resp
        );

        rotate_encryption_key(
            sender, asset_type, new_ek,
            KeyRotationProof::V1 { compressed_new_ek, new_R, compressed_new_R, sigma },
            resume_incoming_transfers
        );
    }

    public fun rotate_encryption_key(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_ek: RistrettoPoint,
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
            owner, asset_type, new_ek, &ca_store.ek, &ca_store.available_balance, proof
        );

        // Step 3: Install the new EK and the new re-encrypted available balance
        ca_store.ek = compressed_new_ek;
        // We're just updating the available balance's EK-dependant D-component & leaving the pending balance the same.
        ca_store.available_balance.set_compressed_R(compressed_new_R);
        if (resume_incoming_transfers) {
            ca_store.pause_incoming = false;
        }
    }

    /// Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
    /// Parses arguments and forwards to `normalize`; see that function for details.
    public entry fun normalize_raw(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_balance_C: vector<vector<u8>>,
        new_balance_D: vector<vector<u8>>,
        new_balance_A: vector<vector<u8>>,
        zkrp_new_balance: vector<u8>,
        sigma_proto_comm: vector<vector<u8>>,
        sigma_proto_resp: vector<vector<u8>>
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let (new_P, compressed_P) = deserialize_points(new_balance_C);
        let (new_R, compressed_R) = deserialize_points(new_balance_D);
        let (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
        let new_balance = confidential_available_balance::new_from_p_r_r_aud(new_P, new_R, new_R_aud);
        let compressed_new_balance = confidential_available_balance::new_compressed_from_p_r_r_aud(
            compressed_P, compressed_R, compressed_R_aud
        );
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
        let sigma = sigma_protocol_proof::new_proof_from_bytes(sigma_proto_comm, sigma_proto_resp);
        let proof = NormalizationProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma };

        normalize(sender, asset_type, proof);
    }

    /// Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
    /// Most functions perform implicit normalization by accepting a new normalized confidential balance as a parameter.
    /// However, explicit normalization is required before rolling over the pending balance, as multiple rolls may cause
    /// chunk overflows.
    /// The proof contains the sender's new normalized confidential balance, encrypted with fresh randomness.
    public fun normalize(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        proof: NormalizationProof
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let user = signer::address_of(sender);

        // Check normalized flag and read values before mutable borrow
        assert!(!is_normalized(user, asset_type), error::invalid_state(E_ALREADY_NORMALIZED));
        let ek = get_encryption_key(user, asset_type);
        let current_balance = get_available_balance(user, asset_type);
        let auditor_ek = get_effective_auditor(asset_type);

        let compressed_new_balance = assert_valid_normalization_proof(
            sender,
            asset_type,
            &ek,
            &current_balance,
            &auditor_ek,
            proof
        );

        let ca_store = borrow_confidential_store_mut(user, asset_type);
        ca_store.available_balance = compressed_new_balance;
        ca_store.normalized = true;
    }

    /// Adds the pending balance to the available balance for the specified asset type, resetting the pending balance to zero.
    /// This operation is needed when the owner wants to be able to send out tokens from their pending balance: the only
    /// way of doing so is to roll over these tokens into the available balance.
    public entry fun rollover_pending_balance(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        rollover_pending_balance_internal(sender, asset_type);
    }

    /// Before calling `rotate_encryption_key_raw`, we need to rollover the pending balance and pause incoming transfers
    /// for this asset type to prevent any new transfers from coming in.
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

    /// Pauses or resumes incoming transfers for the specified account and asset type.
    /// Pausing is needed before rotating the encryption key: the owner must pause incoming transfers so as to be able
    /// to roll over their pending balance fully. Then, to rotate their encryption key, the owner needs to only re-encrypt
    /// their available balance ciphertext. Once done, the owner can unpause incoming transfers.
    public entry fun set_incoming_transfers_paused(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        paused: bool
    ) acquires ConfidentialStore {
        borrow_confidential_store_mut(signer::address_of(owner), asset_type).pause_incoming = paused;
    }

    /// Implementation of the `rollover_pending_balance` entry function.
    public fun rollover_pending_balance_internal(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore {
        let user = signer::address_of(sender);
        let ca_store = borrow_confidential_store_mut(user, asset_type);

        assert!(ca_store.normalized, error::invalid_state(E_NORMALIZATION_REQUIRED));

        ca_store.available_balance.add_assign(&ca_store.pending_balance);
        // A components remain stale — will be refreshed on normalize/withdraw/transfer

        ca_store.normalized = false;
        ca_store.transfers_received = 0;
        ca_store.pending_balance = confidential_pending_balance::new_zero_compressed();
    }

    // ==================================================================== //
    //     SECURITY-SENSITIVE public governance functions (9 out of 14)     //
    //               (bugs here could lead to loss of privacy)              //
    // ==================================================================== //

    /// Enables or disables the allow list. When enabled, only asset types from the allow list can be transferred.
    public fun set_allow_listing(aptos_framework: &signer, enabled: bool) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        borrow_global_mut<GlobalConfig>(@aptos_experimental).allow_list_enabled = enabled;
    }

    /// Enables or disables confidential transfers for the specified asset type.
    public fun set_confidentiality_for_asset_type(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        allowed: bool
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        let asset_config = borrow_global_mut<AssetConfig>(get_asset_config_address_or_create(asset_type));
        asset_config.allowed = allowed;
    }

    /// Sets or removes the auditor for the specified asset type.
    ///
    /// Notes:
    /// - Ensures that new_auditor_ek is a valid Ristretto255 point
    /// - Ideally, this should require a ZKPoK of DK too. But, instead, we assume competent auditors.
    ///
    /// The `auditor_epoch` is incremented only when installing or changing the EK (not when removing):
    /// - None → Some(ek): epoch increments (installing)
    /// - Some(old) → Some(new) where old != new: epoch increments (changing)
    /// - Some(old) → Some(old): epoch stays (no change)
    /// - Some(_) → None: epoch stays (removing)
    /// - None → None: epoch stays (no-op)
    public fun set_auditor_for_asset_type(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        auditor_ek: Option<vector<u8>>
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        let asset_config = borrow_global_mut<AssetConfig>(get_asset_config_address_or_create(asset_type));

        let new_ek = auditor_ek.map(|ek|
            ristretto255::new_compressed_point_from_bytes(ek).extract()
        );

        // Increment epoch only when installing or changing the EK (not when removing)
        let should_increment = if (new_ek.is_some()) {
            if (asset_config.auditor_ek.is_some()) {
                !new_ek.borrow().compressed_point_equals(asset_config.auditor_ek.borrow())
            } else {
                true // None → Some: installing
            }
        } else {
            false // removing or no-op
        };

        if (should_increment) {
            asset_config.auditor_epoch = asset_config.auditor_epoch + 1;
        };

        asset_config.auditor_ek = new_ek;
    }

    /// Sets or removes the global auditor for all asset types. The global auditor is used as a fallback when no
    /// asset-specific auditor is set. (Ideally, this should require a ZKPoK of DK but we assume competent auditors.)
    ///
    /// The `global_auditor_epoch` is incremented only when installing or changing the EK (not when removing):
    /// - None → Some(ek): epoch increments (installing)
    /// - Some(old) → Some(new) where old != new: epoch increments (changing)
    /// - Some(old) → Some(old): epoch stays (no change)
    /// - Some(_) → None: epoch stays (removing)
    /// - None → None: epoch stays (no-op)
    public fun set_global_auditor(aptos_framework: &signer, auditor_ek: Option<vector<u8>>) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        let config = borrow_global_mut<GlobalConfig>(@aptos_experimental);

        let new_ek = auditor_ek.map(|ek|
            ristretto255::new_compressed_point_from_bytes(ek).extract()
        );

        // Increment epoch only when installing or changing the EK (not when removing)
        let should_increment = if (new_ek.is_some()) {
            if (config.global_auditor_ek.is_some()) {
                !new_ek.borrow().compressed_point_equals(config.global_auditor_ek.borrow())
            } else {
                true // None → Some: installing
            }
        } else {
            false // removing or no-op
        };

        if (should_increment) {
            config.global_auditor_epoch = config.global_auditor_epoch + 1;
        };

        config.global_auditor_ek = new_ek;
    }

    // ============================================================== //
    //     End of SECURITY-SENSITIVE public governance functions      //
    // ============================================================== //

    // === Public view functions (10 out of 14) ===

    #[view]
    /// Helper to get the number of available balance chunks.
    public fun get_num_available_chunks(): u64 {
        confidential_available_balance::get_num_chunks()
    }

    #[view]
    /// Helper to get the number of pending balance chunks.
    public fun get_num_pending_chunks(): u64 {
        confidential_pending_balance::get_num_chunks()
    }

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
    public fun is_confidentiality_enabled_for_asset_type(asset_type: Object<fungible_asset::Metadata>): bool acquires GlobalConfig, AssetConfig {
        if (!is_allow_listing_enabled()) {
            return true
        };

        let asset_config_address = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(asset_config_address)) {
            return false
        };

        borrow_global<AssetConfig>(asset_config_address).allowed
    }

    #[view]
    /// Checks if allow listing is enabled.
    /// If the allow list is enabled, only asset types from the allow list can be transferred confidentially.
    /// Otherwise, all asset types are allowed.
    public fun is_allow_listing_enabled(): bool acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).allow_list_enabled
    }

    #[view]
    /// Returns the pending balance of the user for the specified asset type.
    public fun get_pending_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedPendingBalance acquires ConfidentialStore {
        borrow_confidential_store(owner, asset_type).pending_balance
    }

    #[view]
    /// Returns the available balance of the user for the specified asset type.
    public fun get_available_balance(
        owner: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedAvailableBalance acquires ConfidentialStore {
        borrow_confidential_store(owner, asset_type).available_balance
    }

    #[view]
    /// Returns the encryption key (EK) of the user for the specified asset type.
    public fun get_encryption_key(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): CompressedRistretto acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).ek
    }

    #[view]
    /// Checks if the user's available balance is normalized for the specified asset type.
    public fun is_normalized(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): bool acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).normalized
    }

    #[view]
    /// Checks if the user's incoming transfers are paused for the specified asset type.
    public fun incoming_transfers_paused(user: address, asset_type: Object<fungible_asset::Metadata>): bool acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).pause_incoming
    }

    #[view]
    /// Returns the asset-specific auditor's encryption key.
    /// If the auditing feature is disabled for the asset type, the encryption key is set to `None`.
    public fun get_auditor_for_asset_type(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires AssetConfig, GlobalConfig {
        let asset_config_address = get_asset_config_address(asset_type);

        if (!is_allow_listing_enabled() && !exists<AssetConfig>(asset_config_address)) {
            return std::option::none();
        };

        borrow_global<AssetConfig>(asset_config_address).auditor_ek
    }

    #[view]
    /// Returns the global auditor's encryption key, or `None` if no global auditor is set.
    public fun get_global_auditor(): Option<CompressedRistretto> acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor_ek
    }

    #[view]
    /// Returns the effective auditor for a given asset type, checking the asset-specific auditor first
    /// and falling back to the global auditor.
    public fun get_effective_auditor(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires AssetConfig, GlobalConfig {
        // 1. Check asset-specific auditor
        let config_addr = get_asset_config_address(asset_type);
        if (exists<AssetConfig>(config_addr)) {
            let asset_auditor = borrow_global<AssetConfig>(config_addr).auditor_ek;
            if (asset_auditor.is_some()) {
                return asset_auditor
            };
        };
        // 2. Fall back to global auditor
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor_ek
    }

    #[view]
    /// Returns the global auditor epoch counter.
    public fun get_global_auditor_epoch(): u64 acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor_epoch
    }

    #[view]
    /// Returns the auditor epoch counter for a specific asset type. Returns 0 if no `AssetConfig`
    /// exists for this asset type (and allow-listing is disabled).
    public fun get_auditor_epoch_for_asset_type(
        asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires AssetConfig, GlobalConfig {
        let asset_config_address = get_asset_config_address(asset_type);
        if (!is_allow_listing_enabled() && !exists<AssetConfig>(asset_config_address)) {
            return 0
        };
        borrow_global<AssetConfig>(asset_config_address).auditor_epoch
    }

    #[view]
    /// Returns the effective auditor epoch: asset-specific epoch if the asset has an auditor,
    /// otherwise global auditor epoch.
    public fun get_effective_auditor_epoch(
        asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires AssetConfig, GlobalConfig {
        let config_addr = get_asset_config_address(asset_type);
        if (exists<AssetConfig>(config_addr)) {
            let ac = borrow_global<AssetConfig>(config_addr);
            if (ac.auditor_ek.is_some()) {
                return ac.auditor_epoch
            };
        };
        borrow_global<GlobalConfig>(@aptos_experimental).global_auditor_epoch
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
    /// Returns the number of transfers received into the pending balance for the specified asset type.
    public fun get_num_transfers_received(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): u64 acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).transfers_received
    }

    #[view]
    /// Returns the maximum number of transfers that can be accumulated in the pending balance before rollover is required.
    public fun get_max_transfers_before_rollover(): u64 {
        MAX_TRANSFERS_BEFORE_ROLLOVER
    }

    // === Private, internal functions (11 out of 14) ===

    /// Returns the address that handles primary FA store and `AssetConfig` objects for the specified asset type.
    fun get_asset_config_address(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let config_ext = &borrow_global<GlobalConfig>(@aptos_experimental).extend_ref;
        let config_ext_address = object::address_from_extend_ref(config_ext);
        object::create_object_address(&config_ext_address, construct_asset_config_seed(asset_type))
    }

    /// Ensures that the `AssetConfig` object exists for the specified asset type and returns its address.
    /// If the object does not exist, creates it. Used only for internal purposes.
    fun get_asset_config_address_or_create(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let addr = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(addr)) {
            let asset_config_signer = get_asset_config_signer(asset_type);

            move_to(
                &asset_config_signer,
                // We disallow the asset type from being made confidential since this function is
                // called in a lot of different contexts.
                AssetConfig::V1 { allowed: false, auditor_ek: std::option::none(), auditor_epoch: 0 }
            );
        };

        addr
    }

    /// Returns an object for handling all the FA primary stores, and returns a signer for it.
    fun get_global_config_signer(): signer acquires GlobalConfig {
        object::generate_signer_for_extending(&borrow_global<GlobalConfig>(@aptos_experimental).extend_ref)
    }

    /// Returns the address that handles all the FA primary stores.
    fun get_global_config_address(): address acquires GlobalConfig {
        object::address_from_extend_ref(&borrow_global<GlobalConfig>(@aptos_experimental).extend_ref)
    }

    /// Returns an object for handling the `ConfidentialStore` and returns a signer for it.
    fun get_confidential_store_signer(user: &signer, asset_type: Object<fungible_asset::Metadata>): signer {
        object::generate_signer(&object::create_named_object(user, construct_confidential_store_seed(asset_type)))
    }

    /// Returns the address that handles the user's `ConfidentialStore` object for the specified user and asset type.
    fun get_confidential_store_address(user: address, asset_type: Object<fungible_asset::Metadata>): address {
        object::create_object_address(&user, construct_confidential_store_seed(asset_type))
    }

    /// Borrows the `ConfidentialStore` for the given user and asset type, asserting it exists.
    inline fun borrow_confidential_store(user: address, asset_type: Object<fungible_asset::Metadata>): &ConfidentialStore {
        assert!(has_confidential_store(user, asset_type), error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED));
        borrow_global<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    /// Mutably borrows the `ConfidentialStore` for the given user and asset type, asserting it exists.
    inline fun borrow_confidential_store_mut(user: address, asset_type: Object<fungible_asset::Metadata>): &mut ConfidentialStore {
        assert!(has_confidential_store(user, asset_type), error::not_found(E_CONFIDENTIAL_STORE_NOT_REGISTERED));
        borrow_global_mut<ConfidentialStore>(get_confidential_store_address(user, asset_type))
    }

    /// Returns an object for handling the `AssetConfig`, and returns a signer for it.
    fun get_asset_config_signer(asset_type: Object<fungible_asset::Metadata>): signer acquires GlobalConfig {
        let config_ext = &borrow_global<GlobalConfig>(@aptos_experimental).extend_ref;
        let config_ext_signer = object::generate_signer_for_extending(config_ext);

        let config_ctor =
            &object::create_named_object(&config_ext_signer, construct_asset_config_seed(asset_type));

        object::generate_signer(config_ctor)
    }

    /// Constructs a unique seed for the user's `ConfidentialStore` object.
    /// As all the `ConfidentialStore`'s have the same type, we need to differentiate them by the seed.
    fun construct_confidential_store_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::ConfidentialStore",
                @aptos_experimental,
                object::object_address(&asset_type)
            )
        )
    }

    /// Constructs a unique seed for the `AssetConfig` object.
    /// As all the `AssetConfig`'s have the same type, we need to differentiate them by the seed.
    /// NOTE: The seed string is unchanged from the original to maintain address stability.
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

        pending_balance.check_decrypts_to(user_dk, (amount as u128))
    }

    #[test_only]
    public fun check_available_balance_decrypts_to(
        user: address,
        asset_type: Object<fungible_asset::Metadata>,
        user_dk: &Scalar,
        amount: u128
    ): bool acquires ConfidentialStore {
        let available_balance =
            get_available_balance(user, asset_type).decompress();

        available_balance.check_decrypts_to(user_dk, amount)
    }

    // === Proof enums and verification functions (13 out of 14) ===

    /// Proof of knowledge of the decryption key for registration.
    /// Contains a $\Sigma$-protocol proof that $H = \mathsf{dk} \cdot \mathsf{ek}$.
    enum RegistrationProof has drop {
        V1 {
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Represents the proof structure for validating a withdrawal operation.
    /// Contains the sender's new normalized available balance, a range proof, and a
    /// $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{withdraw}$ relation.
    enum WithdrawalProof has drop {
        V1 {
            /// The sender's new normalized available balance, encrypted with fresh randomness.
            new_balance: AvailableBalance,
            /// The compressed form of `new_balance`, obtained at parse time to avoid recompression.
            compressed_new_balance: CompressedAvailableBalance,
            /// Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
            zkrp_new_balance: RangeProof,
            /// $\Sigma$-protocol proof for the withdrawal relation.
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Represents the proof structure for validating a transfer operation.
    /// Contains the sender's new balance, the transfer amount encrypted for the sender,
    /// D-only components for the recipient and auditors, range proofs, and a
    /// $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{txfer}$ relation.
    enum TransferProof has drop {
        V1 {
            /// The sender's new normalized available balance, encrypted with fresh randomness.
            new_balance: AvailableBalance,
            /// The compressed form of `new_balance`, obtained at parse time to avoid recompression.
            compressed_new_balance: CompressedAvailableBalance,
            /// The transfer amount encrypted with the sender's encryption key.
            sender_amount: PendingBalance,
            /// The compressed form of `sender_amount`, obtained at parse time to avoid recompression.
            compressed_sender_amount: CompressedPendingBalance,
            /// The D components of the transfer amount encrypted with the recipient's encryption key.
            /// The C components are the same as `sender_amount`'s C components (structurally guaranteed).
            recipient_amount_R: vector<RistrettoPoint>,
            /// The compressed form of `recipient_R`, obtained at parse time to avoid recompression.
            compressed_recip_amount_R: vector<CompressedRistretto>,
            /// The D components of the transfer amount encrypted with each auditor's encryption key.
            /// The C components are the same as `sender_amount`'s C components (structurally guaranteed).
            auditor_amount_Ds: vector<vector<RistrettoPoint>>,
            /// The compressed form of each auditor's D components, obtained at parse time to avoid recompression.
            compressed_auditor_Rs: vector<vector<CompressedRistretto>>,
            /// Range proof ensuring that the resulting balance chunks for the sender are normalized.
            zkrp_new_balance: RangeProof,
            /// Range proof ensuring that the transferred amount chunks are normalized.
            zkrp_amount: RangeProof,
            /// $\Sigma$-protocol proof for the transfer relation.
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Represents the proof structure for validating a normalization operation.
    /// Contains the user's new normalized available balance, a range proof, and a
    /// $\Sigma$-protocol proof (reusing the withdrawal relation with $v = 0$).
    enum NormalizationProof has drop {
        V1 {
            /// The user's new normalized available balance, encrypted with fresh randomness.
            new_balance: AvailableBalance,
            /// The compressed form of `new_balance`, obtained at parse time to avoid recompression.
            compressed_new_balance: CompressedAvailableBalance,
            /// Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
            zkrp_new_balance: RangeProof,
            /// $\Sigma$-protocol proof for the normalization relation (withdrawal with $v = 0$).
            sigma: sigma_protocol_proof::Proof,
        }
    }

    /// Represents the proof structure for validating a key rotation operation.
    /// Contains the new encryption key, the re-encrypted D components, and a $\Sigma$-protocol proof
    /// that the re-encryption is correct.
    enum KeyRotationProof has drop {
        V1 {
            compressed_new_ek: CompressedRistretto,
            new_R: vector<RistrettoPoint>,
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

    /// Asserts the registration proof of knowledge is valid via $\Sigma$-protocol verification.
    public fun assert_valid_registration_proof(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: &CompressedRistretto,
        proof: RegistrationProof
    ) {
        let RegistrationProof::V1 { sigma } = proof;
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let stmt = sigma_protocol_registration::new_registration_statement(
            compressed_H, compressed_H.point_decompress(),
            *ek, ek.point_decompress(),
        );
        let session = sigma_protocol_registration::new_session(sender, asset_type);
        sigma_protocol_registration::assert_verifies(&session, &stmt, &sigma);
    }

    /// Asserts the validity of the `withdraw` operation.
    ///
    /// Checks that the new balance chunks are each in [0, 2^16) via a range proof.
    /// Verifies the $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{withdraw}$ relation.
    /// Consumes the proof and returns the compressed new balance on success.
    public fun assert_valid_withdrawal_proof(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: &CompressedRistretto,
        amount: u64,
        current_balance: &CompressedAvailableBalance,
        compressed_auditor_ek: &Option<CompressedRistretto>,
        proof: WithdrawalProof
    ): CompressedAvailableBalance {
        let WithdrawalProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma } = proof;
        confidential_proof::assert_valid_range_proof(new_balance.get_P(), &zkrp_new_balance);

        // Build base points
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        let v = ristretto255::new_scalar_from_u64(amount);

        let (aud_ek_compressed, compressed_new_R_aud, new_R_aud) = if (compressed_auditor_ek.is_some()) {
            let aud_ek = *compressed_auditor_ek.borrow();
            (std::option::some(aud_ek), *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()))
        } else {
            (std::option::none(), vector[], vector[])
        };

        let stmt = sigma_protocol_withdraw::new_withdrawal_statement(
            compressed_G, _G,
            compressed_H, _H,
            *ek, ek.point_decompress(),
            *current_balance.get_compressed_P(), decompress_points(current_balance.get_compressed_P()),
            *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
            *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
            *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
            aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
            compressed_new_R_aud, new_R_aud,
            v,
        );

        let session = sigma_protocol_withdraw::new_session(sender, asset_type);
        sigma_protocol_withdraw::assert_verifies_withdrawal(&session, &stmt, &sigma);
        compressed_new_balance
    }

    /// Asserts the validity of the `confidential_transfer` operation.
    ///
    /// Checks that the new balance and transfer amount chunks are each in [0, 2^16) via range proofs.
    /// Reconstructs full recipient and auditor balances from the sender_amount's C components and
    /// the provided D-only components. This structurally guarantees that C components match across
    /// all parties.
    /// Verifies the $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{txfer}$ relation.
    /// Consumes the proof and returns (new_balance, sender_amount, recipient_amount, auditor_amounts).
    public fun assert_valid_transfer_proof(
        sender: &signer,
        recipient_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_ek: &CompressedRistretto,
        recip_ek: &CompressedRistretto,
        old_balance: &CompressedAvailableBalance,
        auditor_eks: &vector<CompressedRistretto>,
        has_effective_auditor: bool,
        num_extra_auditors: u64,
        proof: TransferProof
    ): (
        CompressedAvailableBalance,
        PendingBalance,
        PendingBalance,
        vector<PendingBalance>
    ) {
        let TransferProof::V1 {
            new_balance, compressed_new_balance,
            sender_amount, compressed_sender_amount,
            recipient_amount_R, compressed_recip_amount_R,
            auditor_amount_Ds, compressed_auditor_Rs,
            zkrp_new_balance, zkrp_amount, sigma
        } = proof;

        // Verify the number of auditor D-components in the proof matches the expected auditor count
        // (cheap check before expensive sigma verification)
        assert!(
            auditor_amount_Ds.length() == auditor_eks.length(),
            error::invalid_argument(E_AUDITOR_COUNT_MISMATCH)
        );

        // Reconstruct full balances from sender_amount's C components and the D-only components.
        // This structurally guarantees C component equality (no explicit check needed).
        let sender_P = sender_amount.get_P();
        let recipient_amount = confidential_pending_balance::new_from_p_and_r(
            sender_P.map_ref(|c| c.point_clone()),
            recipient_amount_R
        );
        let auditor_amounts = auditor_amount_Ds.map(|d| {
            confidential_pending_balance::new_from_p_and_r(
                sender_P.map_ref(|c| c.point_clone()), d
            )
        });

        confidential_proof::assert_valid_range_proof(recipient_amount.get_P(), &zkrp_amount);
        confidential_proof::assert_valid_range_proof(new_balance.get_P(), &zkrp_new_balance);

        // Sigma protocol verification
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        // Effective auditor components (if any)
        let (ek_eff_aud_opt, compressed_new_R_eff_aud, new_R_eff_aud,
             compressed_amount_R_eff_aud, amount_R_eff_aud) =
            if (has_effective_auditor) {
                let eff_aud_ek = auditor_eks[auditor_eks.length() - 1];
                (
                    std::option::some(eff_aud_ek),
                    *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()),
                    compressed_auditor_Rs[compressed_auditor_Rs.length() - 1],
                    points_clone(auditor_amounts[auditor_amounts.length() - 1].get_R()),
                )
            } else {
                (std::option::none(), vector[], vector[], vector[], vector[])
            };

        // Extra auditor components (extras are at indices [0..num_extra) in the auditor vectors)
        let compressed_ek_extra_auds = vector[];
        let ek_extra_auds = vector[];
        let compressed_amount_R_extra_auds = vector[];
        let amount_R_extra_auds = vector[];
        let i = 0;
        while (i < num_extra_auditors) {
            compressed_ek_extra_auds.push_back(auditor_eks[i]);
            ek_extra_auds.push_back(auditor_eks[i].point_decompress());
            compressed_amount_R_extra_auds.push_back(compressed_auditor_Rs[i]);
            amount_R_extra_auds.push_back(points_clone(auditor_amounts[i].get_R()));
            i = i + 1;
        };

        let stmt = sigma_protocol_transfer::new_transfer_statement(
            compressed_G, _G,
            compressed_H, _H,
            *sender_ek, sender_ek.point_decompress(),
            *recip_ek, recip_ek.point_decompress(),
            *old_balance.get_compressed_P(), decompress_points(old_balance.get_compressed_P()),
            *old_balance.get_compressed_R(), decompress_points(old_balance.get_compressed_R()),
            *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
            *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
            *compressed_sender_amount.get_compressed_P(), points_clone(sender_amount.get_P()),
            *compressed_sender_amount.get_compressed_R(), points_clone(sender_amount.get_R()),
            compressed_recip_amount_R, points_clone(recipient_amount.get_R()),
            ek_eff_aud_opt, ek_eff_aud_opt.map(|p| p.point_decompress()),
            compressed_new_R_eff_aud, new_R_eff_aud,
            compressed_amount_R_eff_aud, amount_R_eff_aud,
            compressed_ek_extra_auds, ek_extra_auds,
            compressed_amount_R_extra_auds, amount_R_extra_auds,
        );

        let session = sigma_protocol_transfer::new_session(
            sender, recipient_addr, asset_type, has_effective_auditor, num_extra_auditors,
        );
        sigma_protocol_transfer::assert_verifies(&session, &stmt, &sigma);

        (compressed_new_balance, sender_amount, recipient_amount, auditor_amounts)
    }

    /// Asserts the validity of the `normalize` operation.
    ///
    /// Checks that the new balance chunks are each in [0, 2^16) via a range proof.
    /// Verifies the $\Sigma$-protocol proof for the normalization relation (withdrawal with $v = 0$).
    /// Consumes the proof and returns the compressed new balance on success.
    public fun assert_valid_normalization_proof(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        ek: &CompressedRistretto,
        current_balance: &CompressedAvailableBalance,
        auditor_ek: &Option<CompressedRistretto>,
        proof: NormalizationProof
    ): CompressedAvailableBalance {
        let NormalizationProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma } = proof;

        confidential_proof::assert_valid_range_proof(new_balance.get_P(), &zkrp_new_balance);

        // Build base points
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        // Normalization is withdrawal with v = 0
        let v = ristretto255::new_scalar_from_u64(0);

        let (aud_ek_compressed, compressed_new_R_aud, new_R_aud) = if (auditor_ek.is_some()) {
            let aud_ek = *auditor_ek.borrow();
            (std::option::some(aud_ek), *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()))
        } else {
            (std::option::none(), vector[], vector[])
        };

        let stmt = sigma_protocol_withdraw::new_withdrawal_statement(
            compressed_G, _G,
            compressed_H, _H,
            *ek, ek.point_decompress(),
            *current_balance.get_compressed_P(), decompress_points(current_balance.get_compressed_P()),
            *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
            *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
            *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
            aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
            compressed_new_R_aud, new_R_aud,
            v,
        );

        let session = sigma_protocol_withdraw::new_session(sender, asset_type);
        sigma_protocol_withdraw::assert_verifies_normalization(&session, &stmt, &sigma);

        compressed_new_balance
    }

    /// Asserts the validity of the key rotation proof via $\Sigma$-protocol verification.
    /// Returns (compressed_new_ek, compressed_new_R) on success.
    public fun assert_valid_key_rotation_proof(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        new_ek: RistrettoPoint,
        old_ek: &CompressedRistretto,
        current_balance: &CompressedAvailableBalance,
        proof: KeyRotationProof
    ): (CompressedRistretto, vector<CompressedRistretto>) {
        let KeyRotationProof::V1 { compressed_new_ek, new_R, compressed_new_R, sigma } = proof;

        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();

        let stmt = sigma_protocol_key_rotation::new_key_rotation_statement(
            compressed_H, compressed_H.point_decompress(),
            *old_ek, old_ek.point_decompress(),
            compressed_new_ek, new_ek,
            *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
            compressed_new_R, new_R,
        );

        let session = sigma_protocol_key_rotation::new_session(owner, asset_type);
        sigma_protocol_key_rotation::assert_verifies(&session, &stmt, &sigma);

        (compressed_new_ek, compressed_new_R)
    }

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                                                //
    // *** End of SECURITY-SENSITIVE proof verification functions *** //
    //                                                                //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

    // === Test-only proof generation functions (14 out of 14) ===

    #[test_only]
    /// Generates a $\Sigma$-protocol proof for the R_dl relation.
    public fun prove_registration(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        dk: &Scalar,
    ): RegistrationProof {
        let _H = ristretto255_twisted_elgamal::get_encryption_key_basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let ek = _H.point_mul(&dk.scalar_invert().extract());
        let compressed_ek = ek.point_compress();
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);
        let stmt = sigma_protocol_registration::new_registration_statement(
            compressed_H, _H,
            compressed_ek, ek,
        );
        let witn = sigma_protocol_registration::new_registration_witness(*dk);
        let session = sigma_protocol_registration::new_session(&sender, asset_type);
        let sigma = sigma_protocol_registration::prove(&session, &stmt, &witn);
        RegistrationProof::V1 { sigma }
    }

    #[test_only]
    /// Generates a $\Sigma$-protocol proof for the R_withdraw relation.
    public fun prove_withdrawal(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount: u64,
        new_amount: u128,
    ): WithdrawalProof acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        let ek = get_encryption_key(sender_addr, asset_type);
        let current_balance = get_available_balance(sender_addr, asset_type).decompress();
        let aud_ek_compressed = get_effective_auditor(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);

        let new_balance_r = confidential_available_balance::generate_balance_randomness();
        let new_balance = confidential_available_balance::new_from_amount(
            new_amount, &new_balance_r, &ek, &aud_ek_compressed
        );

        let new_balance_r_scalars = new_balance_r.scalars();
        let new_a = confidential_available_balance::split_into_chunks(new_amount);
        let zkrp_new_balance = confidential_proof::prove_range(&new_a, new_balance_r_scalars);

        let v = ristretto255::new_scalar_from_u64(amount);

        let (aud_ek_compressed, new_R_aud, compressed_new_R_aud) = if (aud_ek_compressed.is_some()) {
            let aud_ek_compressed = *aud_ek_compressed.borrow();
            (std::option::some(aud_ek_compressed), points_clone(new_balance.get_R_aud()), compress_points(new_balance.get_R_aud()))
        } else {
            (std::option::none(), vector[], vector[])
        };

        let stmt = sigma_protocol_withdraw::new_withdrawal_statement(
            compressed_G, _G,
            compressed_H, _H,
            ek, ek.point_decompress(),
            compress_points(current_balance.get_P()), points_clone(current_balance.get_P()),
            compress_points(current_balance.get_R()), points_clone(current_balance.get_R()),
            compress_points(new_balance.get_P()), points_clone(new_balance.get_P()),
            compress_points(new_balance.get_R()), points_clone(new_balance.get_R()),
            aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
            compressed_new_R_aud, new_R_aud,
            v,
        );

        let witn = sigma_protocol_withdraw::new_withdrawal_witness(
            *sender_dk, new_a, *new_balance_r_scalars
        );
        let session = sigma_protocol_withdraw::new_session(&sender, asset_type);
        let sigma = sigma_protocol_withdraw::prove_withdrawal(&session, &stmt, &witn);

        let compressed_new_balance = new_balance.compress();
        WithdrawalProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma }
    }

    #[test_only]
    /// Returns (TransferProof, test_auditor_amounts).
    /// The second element is a separate copy of auditor amounts for test assertions
    /// (since the original auditor amounts are consumed inside the proof).
    /// `extra_auditor_eks` should not include the effective auditor (asset-specific or global),
    /// which is read from chain and appended at the end (matching the on-chain order [...extra, effective]).
    /// Generates a $\Sigma$-protocol proof for the R_txfer relation.
    public fun prove_transfer(
        sender_addr: address,
        recipient_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount: u64,
        new_amount: u128,
        extra_auditor_eks: &vector<CompressedRistretto>,
    ): (TransferProof, vector<PendingBalance>) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        let sender_ek = get_encryption_key(sender_addr, asset_type);
        let recipient_ek = get_encryption_key(recipient_addr, asset_type);
        let current_balance = get_available_balance(sender_addr, asset_type).decompress();
        let effective_auditor_ek = get_effective_auditor(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);

        // Build combined auditor list: [...extra, effective (if set)]
        let has_effective_auditor = effective_auditor_ek.is_some();
        let num_extra_auditors = extra_auditor_eks.length();
        let all_auditor_eks = *extra_auditor_eks;
        if (has_effective_auditor) {
            all_auditor_eks.push_back(effective_auditor_ek.extract());
        };

        let amount_r = confidential_pending_balance::generate_balance_randomness();
        let new_balance_r = confidential_available_balance::generate_balance_randomness();

        // Compute the auditor EK for the new_balance A component.
        // We use the last auditor EK from the combined list, if any.
        // In practice, this should be the effective auditor (asset-specific or global).
        let new_balance_auditor_ek = if (all_auditor_eks.length() > 0) {
            std::option::some(*all_auditor_eks.borrow(all_auditor_eks.length() - 1))
        } else {
            std::option::none()
        };

        let new_balance = confidential_available_balance::new_from_amount(
            new_amount, &new_balance_r, &sender_ek, &new_balance_auditor_ek
        );
        let sender_amount = confidential_pending_balance::new_from_amount(
            (amount as u128), &amount_r, &sender_ek
        );
        let recipient_amount = confidential_pending_balance::new_from_amount(
            (amount as u128), &amount_r, &recipient_ek
        );
        let auditor_amounts = all_auditor_eks.map_ref(|ek| {
            confidential_pending_balance::new_from_amount((amount as u128), &amount_r, ek)
        });
        // Create a separate set of auditor amounts for test assertions (same randomness, same amount).
        let test_auditor_amounts = all_auditor_eks.map_ref(|ek| {
            confidential_pending_balance::new_from_amount((amount as u128), &amount_r, ek)
        });

        // Extract D-only components for recipient and auditors
        let (_, recipient_R) = recipient_amount.into_p_and_r();
        let auditor_Rs = auditor_amounts.map(|a| { let (_, d) = a.into_p_and_r(); d });

        let amount_r_scalars = amount_r.scalars();
        let new_balance_r_scalars = new_balance_r.scalars();

        let new_a = confidential_available_balance::split_into_chunks(new_amount);
        let v_chunks = confidential_pending_balance::split_into_chunks((amount as u128));
        let zkrp_new_balance = confidential_proof::prove_range(&new_a, new_balance_r_scalars);
        let zkrp_transfer_amount = confidential_proof::prove_range(&v_chunks, amount_r_scalars);

        // Effective auditor components (if any)
        let (ek_eff_aud_opt, compressed_new_R_eff_aud, new_R_eff_aud,
             compressed_amount_R_eff_aud, amount_R_eff_aud) =
            if (has_effective_auditor) {
                let eff_aud_ek = *all_auditor_eks.borrow(all_auditor_eks.length() - 1);
                let last_D = &auditor_Rs[auditor_Rs.length() - 1];
                (
                    std::option::some(eff_aud_ek),
                    compress_points(new_balance.get_R_aud()), points_clone(new_balance.get_R_aud()),
                    compress_points(last_D), last_D.map_ref(|d| d.point_clone()),
                )
            } else {
                (std::option::none(), vector[], vector[], vector[], vector[])
            };

        // Extra auditor components (extras are at indices [0..num_extra) in the auditor vectors)
        let compressed_ek_extra_auds = vector[];
        let ek_extra_auds = vector[];
        let compressed_amount_R_extra_auds = vector[];
        let amount_R_extra_auds = vector[];
        let i = 0;
        while (i < num_extra_auditors) {
            compressed_ek_extra_auds.push_back(all_auditor_eks[i]);
            ek_extra_auds.push_back(all_auditor_eks[i].point_decompress());
            compressed_amount_R_extra_auds.push_back(compress_points(&auditor_Rs[i]));
            amount_R_extra_auds.push_back(auditor_Rs[i].map_ref(|d| d.point_clone()));
            i = i + 1;
        };

        let stmt = sigma_protocol_transfer::new_transfer_statement(
            compressed_G, _G,
            compressed_H, _H,
            sender_ek, sender_ek.point_decompress(),
            recipient_ek, recipient_ek.point_decompress(),
            compress_points(current_balance.get_P()), points_clone(current_balance.get_P()),
            compress_points(current_balance.get_R()), points_clone(current_balance.get_R()),
            compress_points(new_balance.get_P()), points_clone(new_balance.get_P()),
            compress_points(new_balance.get_R()), points_clone(new_balance.get_R()),
            compress_points(sender_amount.get_P()), points_clone(sender_amount.get_P()),
            compress_points(sender_amount.get_R()), points_clone(sender_amount.get_R()),
            compress_points(&recipient_R), recipient_R.map_ref(|d| d.point_clone()),
            ek_eff_aud_opt, ek_eff_aud_opt.map(|p| p.point_decompress()),
            compressed_new_R_eff_aud, new_R_eff_aud,
            compressed_amount_R_eff_aud, amount_R_eff_aud,
            compressed_ek_extra_auds, ek_extra_auds,
            compressed_amount_R_extra_auds, amount_R_extra_auds,
        );

        let witn = sigma_protocol_transfer::new_transfer_witness(
            *sender_dk, new_a, *new_balance_r_scalars, v_chunks, *amount_r_scalars,
        );
        let session = sigma_protocol_transfer::new_session(
            &sender, recipient_addr, asset_type, has_effective_auditor, num_extra_auditors,
        );
        let sigma = sigma_protocol_transfer::prove(&session, &stmt, &witn);

        // Compress all components for the proof struct (test-only, extra compressions are fine)
        let compressed_new_balance = new_balance.compress();
        let compressed_sender_amount = sender_amount.compress();
        let compressed_recipient_R = recipient_R.map_ref(|d| d.point_compress());
        let compressed_auditor_Rs = auditor_Rs.map_ref(|ds| compress_points(ds));

        (
            TransferProof::V1 {
                new_balance, compressed_new_balance,
                sender_amount, compressed_sender_amount,
                recipient_amount_R: recipient_R, compressed_recip_amount_R: compressed_recipient_R,
                auditor_amount_Ds: auditor_Rs, compressed_auditor_Rs,
                zkrp_new_balance, zkrp_amount: zkrp_transfer_amount, sigma
            },
            test_auditor_amounts
        )
    }

    #[test_only]
    /// Generates a $\Sigma$-protocol proof for the normalization relation (withdrawal with $v = 0$).
    public fun prove_normalization(
        sender_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        amount: u128,
    ): NormalizationProof acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let compressed_G = ristretto255::basepoint_compressed();
        let _G = ristretto255::basepoint();
        let compressed_H = ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed();
        let _H = compressed_H.point_decompress();

        let ek = get_encryption_key(sender_addr, asset_type);
        let current_balance = get_available_balance(sender_addr, asset_type);
        let aud_ek_compressed = get_effective_auditor(asset_type);
        let sender = aptos_framework::account::create_signer_for_test(sender_addr);

        let new_balance_r = confidential_available_balance::generate_balance_randomness();
        let new_balance = confidential_available_balance::new_from_amount(
            amount, &new_balance_r, &ek, &aud_ek_compressed
        );

        let new_balance_r_scalars = new_balance_r.scalars();
        let new_a = confidential_available_balance::split_into_chunks(amount);
        let zkrp_new_balance = confidential_proof::prove_range(&new_a, new_balance_r_scalars);

        let v = ristretto255::new_scalar_from_u64(0);

        let (aud_ek_compressed, new_R_aud, compressed_new_R_aud) = if (aud_ek_compressed.is_some()) {
            let aud_ek_compressed = *aud_ek_compressed.borrow();
            (std::option::some(aud_ek_compressed), points_clone(new_balance.get_R_aud()), compress_points(new_balance.get_R_aud()))
        } else {
            (std::option::none(), vector[], vector[])
        };

        let stmt = sigma_protocol_withdraw::new_withdrawal_statement(
            compressed_G, _G,
            compressed_H, _H,
            ek, ek.point_decompress(),
            *current_balance.get_compressed_P(), decompress_points(current_balance.get_compressed_P()),
            *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
            compress_points(new_balance.get_P()), points_clone(new_balance.get_P()),
            compress_points(new_balance.get_R()), points_clone(new_balance.get_R()),
            aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
            compressed_new_R_aud, new_R_aud,
            v,
        );

        let witn = sigma_protocol_withdraw::new_withdrawal_witness(
            *sender_dk, new_a, *new_balance_r_scalars
        );
        let session = sigma_protocol_withdraw::new_session(&sender, asset_type);
        let sigma = sigma_protocol_withdraw::prove_normalization(&session, &stmt, &witn);

        let compressed_new_balance = new_balance.compress();
        NormalizationProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma }
    }

    #[test_only]
    /// Generates the `KeyRotationProof` needed to call the `rotate_encryption_key` function.
    public fun prove_key_rotation(
        owner_addr: address,
        asset_type: Object<fungible_asset::Metadata>,
        sender_dk: &Scalar,
        new_dk: &Scalar,
    ): KeyRotationProof acquires ConfidentialStore {
        let owner = aptos_framework::account::create_signer_for_test(owner_addr);

        // Get old EK and available balance
        let compressed_old_ek = get_encryption_key(owner_addr, asset_type);
        let old_ek = compressed_old_ek.point_decompress();
        let available_balance = get_available_balance(owner_addr, asset_type);

        // Build statement and witness using the helper
        let (stmt, witn, compressed_new_ek, new_R, compressed_new_R) =
            sigma_protocol_key_rotation::compute_statement_and_witness_from_keys_and_old_ctxt(
                sender_dk, new_dk,
                compressed_old_ek, old_ek,
                *available_balance.get_compressed_R(), decompress_points(available_balance.get_compressed_R()),
            );

        // Prove
        let ss = sigma_protocol_key_rotation::new_session(&owner, asset_type);

        KeyRotationProof::V1 {
            compressed_new_ek,
            new_R,
            compressed_new_R,
            sigma: sigma_protocol_key_rotation::prove(&ss, &stmt, &witn),
        }
    }
}
