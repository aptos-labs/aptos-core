/// Confidential Asset (CA) Standard: privacy-focused fungible asset transfers with obfuscated amounts.
module aptos_framework::confidential_asset {
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
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::system_addresses;
    use aptos_framework::confidential_balance::{Self, get_chunk_size_bits, get_chunk_upper_bound,
        new_pending_u64_no_randomness, add_assign_pending, new_compressed_available_from_bytes,
        new_zero_pending_compressed, new_zero_available_compressed};
    use aptos_framework::sigma_protocol_utils::deserialize_compressed_points;
    use aptos_framework::confidential_amount::{Self, CompressedAmount};
    use aptos_framework::confidential_balance::{Pending, Available, CompressedBalance, Balance};
    use aptos_framework::sigma_protocol_key_rotation;
    use aptos_framework::sigma_protocol_registration;
    use aptos_framework::sigma_protocol_withdraw;
    use aptos_framework::sigma_protocol_transfer;
    use aptos_framework::sigma_protocol_proof;
    use aptos_framework::confidential_range_proofs;

    #[test_only]
    friend aptos_framework::confidential_asset_tests;

    // === Errors (1 out of 13) ===

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

    /// The encryption key must not be the identity (zero) point.
    const E_EK_IS_IDENTITY: u64 = 14;

    /// Self-transfers are not allowed: sender and recipient must be different.
    const E_SELF_TRANSFER: u64 = 15;

    /// "Dispatchable" fungible asset types whose withdraw, deposit, balance or supply functions can be customized are not supported, for now.
    const E_UNSAFE_DISPATCHABLE_FA: u64 = 16;

    /// Allow listing is not enabled yet.
    const E_ALLOW_LISTING_IS_DISABLED: u64 = 17;

    /// Pointlessly depositing zero into one's confidential balance would unncessarily increment the `transfers_received` counter.
    const E_POINTLESSLY_DEPOSITING_ZERO: u64 = 18;

    /// Memo in confidential transfer must not exceed `MAX_MEMO_BYTES`.
    const E_MEMO_TOO_LONG: u64 = 19;

    /// All user operations are paused by governance via emergency pause.
    const E_EMERGENCY_PAUSED: u64 = 20;

    /// An internal error occurred: there is either a bug or a misconfiguration in the contract.
    const E_INTERNAL_ERROR: u64 = 999;

    /// #[test_only] The confidential asset module initialization failed.
    const E_INIT_MODULE_FAILED_FOR_DEVNET: u64 = 1000;

    // === Constants (2 out of 13) ===

    /// Any natural number that fits in this # of bits will be less than the Ristretto255 order $p$ and thus fit in its scalar field $\mathbb{Z}_p$ without "wrapping around."
    const MAX_NUM_BITS_IN_SCALAR_FIELD : u64 = 252;

    /// The maximum number of transactions can be aggregated on the pending balance before rollover is required.
    /// i.e., `ConfidentialStore::transfers_received` will never exceed this value.
    const MAX_TRANSFERS_BEFORE_ROLLOVER: u64 = 65536;

    /// Maximum number of bytes a confidential transfer's memo is allowed to be
    const MAX_MEMO_BYTES: u64 = 256;

    /// The mainnet chain ID. If the chain ID is 1, the allow list is enabled.
    const MAINNET_CHAIN_ID: u8 = 1;

    /// The testnet chain ID.
    const TESTNET_CHAIN_ID: u8 = 2;

    // === Structs (3 out of 13) ===

    /// Bundles an auditor's encryption key with its epoch counter (both always modified together).
    enum AuditorConfig has store, drop, copy {
        V1 {
            ek: Option<CompressedRistretto>,

            /// Tracks how many times the auditor EK has been installed or changed (not removed). Starts at 0, indicating
            /// no auditor was ever installed. Increments each time a new EK is set (None → Some(ek) or Some(old) → Some(new)).
            epoch: u64,
        }
    }

    /// When developers fetch the effective auditor config, we wrap it in this struct to indicate whether they've fetched the global or the asset-specific auditor config
    enum EffectiveAuditorConfig has store, drop, copy {
        V1 {
            is_global: bool,
            config: AuditorConfig,
        }
    }

    /// When auditors fetch the effective auditor epoch from a `ConfidentialStore`, they need both the `epoch` number and the `is_global` flag to tell if the auditor ciphertext is stale
    enum EffectiveAuditorHint has store, drop, copy {
        V1 {
            is_global: bool,
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
            global_auditor: AuditorConfig,

            /// Used to derive a signer that owns all the FAs' primary stores and `AssetConfig` objects.
            extend_ref: ExtendRef
        },
        V2 {
            allow_list_enabled: bool,
            global_auditor: AuditorConfig,
            extend_ref: ExtendRef,
            /// When true, all user operations are paused. Managed by governance via `set_emergency_paused`.
            emergency_paused: bool,
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
            auditor: AuditorConfig,
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
            ek: CompressedRistretto,
            /// Tracks which auditor the balance ciphertext is encrypted for: global/effective and epoch
            auditor_hint: Option<EffectiveAuditorHint>
        }
    }

    // === Events (4 out of 13) ===

    #[event]
    enum Registered has drop, store {
        V1 { addr: address, asset_type: Object<fungible_asset::Metadata>, ek: CompressedRistretto }
    }

    #[event]
    enum Deposited has drop, store {
        V1 {
            addr: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>,
            new_pending_balance: CompressedBalance<Pending>,
        }
    }

    #[event]
    enum Withdrawn has drop, store {
        V1 {
            from: address,
            to: address,
            amount: u64,
            asset_type: Object<fungible_asset::Metadata>,
            new_available_balance: CompressedBalance<Available>,
            auditor_hint: Option<EffectiveAuditorHint>,
        }
    }

    #[event]
    enum Transferred has drop, store {
        V1 {
            from: address,
            to: address,
            asset_type: Object<fungible_asset::Metadata>,
            amount: CompressedAmount,
            ek_volun_auds: vector<CompressedRistretto>,
            sender_auditor_hint: Option<EffectiveAuditorHint>,
            new_sender_available_balance: CompressedBalance<Available>,
            new_recip_pending_balance: CompressedBalance<Pending>,
            memo: vector<u8>,
        }
    }

    #[event]
    enum Normalized has drop, store {
        V1 {
            addr: address,
            asset_type: Object<fungible_asset::Metadata>,
            new_available_balance: CompressedBalance<Available>,
            auditor_hint: Option<EffectiveAuditorHint>,
        }
    }

    #[event]
    enum RolledOver has drop, store {
        V1 {
            addr: address,
            asset_type: Object<fungible_asset::Metadata>,
            new_available_balance: CompressedBalance<Available>,
        }
    }

    #[event]
    enum KeyRotated has drop, store {
        V1 {
            addr: address,
            asset_type: Object<fungible_asset::Metadata>,
            new_ek: CompressedRistretto,
            new_available_balance: CompressedBalance<Available>,
        }
    }

    #[event]
    enum IncomingTransfersPauseChanged has drop, store {
        V1 { addr: address, asset_type: Object<fungible_asset::Metadata>, paused: bool }
    }

    #[event]
    enum AllowListingChanged has drop, store {
        V1 { enabled: bool }
    }

    #[event]
    enum ConfidentialityForAssetTypeChanged has drop, store {
        V1 { asset_type: Object<fungible_asset::Metadata>, allowed: bool }
    }

    #[event]
    /// SDK note: when you see this event, call `get_effective_auditor` to determine the current effective EK
    /// for any asset that doesn't have an asset-specific auditor override.
    enum GlobalAuditorChanged has drop, store {
        V1 { new: AuditorConfig }
    }

    // SDK note: when you see this event, call `get_effective_auditor` to determine the current effective EK for this asset.
    #[event]
    enum AssetSpecificAuditorChanged has drop, store {
        V1 { asset_type: Object<fungible_asset::Metadata>, new: AuditorConfig }
    }

    #[event]
    enum EmergencyPauseChanged has drop, store {
        V1 { paused: bool }
    }

    // === Module initialization (5 out of 13) ===

    /// Called once when this module is first published on-chain.
    fun init_module(deployer: &signer) {
        // This is me being overly cautious: I added it to double-check my understanding that the VM always passes
        // the publishing account as deployer. It does, so the assert is redundant (it can never fail).
        assert!(signer::address_of(deployer) == @aptos_framework, error::internal(E_INTERNAL_ERROR));
        assert!(math64::pow(2, get_chunk_size_bits()) == get_chunk_upper_bound(), error::internal(E_INTERNAL_ERROR));
        assert!(
            bulletproofs::get_max_range_bits() >= confidential_balance::get_chunk_size_bits(),
            error::internal(E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        // Available must have more chunks than pending (rollover safety)
        let num_avail_chunks = confidential_balance::get_num_available_chunks();
        let num_pend_chunks = confidential_balance::get_num_pending_chunks();
        assert!(num_avail_chunks >= num_pend_chunks);

        // Available balance chunking must be done so that any balance is representable as a Scalar, w/o wrap-around
        let avail_balance_upper_bound = get_chunk_size_bits() * num_avail_chunks;
        // FA balances use u128 amounts
        assert!(avail_balance_upper_bound == 128);
        // no modular wraparound on available balances
        assert!(avail_balance_upper_bound <= MAX_NUM_BITS_IN_SCALAR_FIELD);

        // Pending balance chunking must be done so that any balance is representable as a Scalar, w/o wrap-around
        let pend_balance_upper_bound = get_chunk_size_bits() * num_pend_chunks;
        // FA deposit/withdraw use u64 amounts
        assert!(pend_balance_upper_bound == 64);
        // no modular wraparound on pending balances nor transferred amounts
        assert!(pend_balance_upper_bound <= MAX_NUM_BITS_IN_SCALAR_FIELD);

        let deployer_address = signer::address_of(deployer);
        let chain_id = chain_id::get();
        move_to(
            deployer,
            GlobalConfig::V2 {
                allow_list_enabled: chain_id == MAINNET_CHAIN_ID || chain_id == TESTNET_CHAIN_ID,
                global_auditor: AuditorConfig::V1 { ek: std::option::none(), epoch: 0 },
                // DO NOT CHANGE: using long syntax until framework change is released to mainnet
                extend_ref: object::create_object(deployer_address).generate_extend_ref(),
                emergency_paused: false,
            }
        );
    }

    /// Initializes the module for devnet/tests. Asserts non-mainnet, non-testnet chain.
    fun init_module_for_devnet(deployer: &signer) {
        assert!(
            signer::address_of(deployer) == @aptos_framework,
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

    // === Entry functions (6 out of 13) ===

    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //
    //                                      //
    // *** SECURITY-SENSITIVE functions *** //
    // (bugs here can lead to stolen funds) //
    //                                      //
    // $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ //

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

    /// Dispatchable fungible asset (DFA) types can, for example, dynamically change user balances upon a call to
    /// `fungible_asset::balance()`, based say, on a multiplier. We do not yet see how to *generically* handle such
    /// dynamic behavior in a confidential context, where balances are encrypted on-chain and cannot be modified in
    /// arbitrary ways. Similarly, we also forbid "total supply" dispatch functions, out of an abundance of caution.
    ///
    /// Furthermore, even for DFAs that only have custom "withdraw/deposit" dispatch functions, it is unclear how to
    /// *generically* support any such functionality. As a result, for now we only support non-dispatchable (vanilla)
    /// fungible asset (FA) types.
    ///
    /// For example, sender blocklists implemented via "withdraw" dispatching would only be enforced when users veil/
    /// unveil their tokens into/from the confidential asset pool. (This is because a `confidential_transfer` cannot,
    /// by definition, interact with any (D)FA functions, or it would be forced to leak amounts/balances). In the future,
    /// we could add support for dispatch functions that only look at the sender's address (and not at the amount/
    /// balances). This way, we could *generically* handle them here, given they are implemented in a type-safe way that
    /// allows us to check they are enabled.
    fun is_safe_for_confidentiality(asset_type: &Object<fungible_asset::Metadata>): bool {
        !fungible_asset::is_asset_type_dispatchable(*asset_type)
    }

    /// Registers a confidential store for a specified asset type, encrypted under the given EK.
    public(friend) fun register(
        sender: &signer, asset_type: Object<fungible_asset::Metadata>,
        ek: CompressedRistretto,
        proof: RegistrationProof
    ) acquires GlobalConfig, AssetConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));
        assert!(is_safe_for_confidentiality(&asset_type), error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));

        assert!(
            !has_confidential_store(signer::address_of(sender), asset_type),
            error::already_exists(E_CONFIDENTIAL_STORE_ALREADY_REGISTERED)
        );
        assert!(!ek.is_identity(), error::invalid_argument(E_EK_IS_IDENTITY));

        // Makes sure the user knows their decryption key.
        assert_valid_registration_proof(sender, asset_type, &ek, proof);

        let ca_store = ConfidentialStore::V1 {
            pause_incoming: false,
            normalized: true,
            transfers_received: 0,
            pending_balance: new_zero_pending_compressed(),
            available_balance: new_zero_available_compressed(),
            ek,
            auditor_hint: std::option::none() // balance == 0 is publicly-known ==> auditor ciphertext is left empty
        };

        move_to(&get_confidential_store_signer(sender, asset_type), ca_store);
        event::emit(Registered::V1 { addr: signer::address_of(sender), asset_type, ek });
    }

    /// Deposits tokens from the sender's primary FA store into their pending balance.
    public entry fun deposit(
        depositor: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        amount: u64
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        let addr = signer::address_of(depositor);

        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));
        assert!(is_safe_for_confidentiality(&asset_type), error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(addr, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));
        assert!(amount != 0, error::invalid_argument(E_POINTLESSLY_DEPOSITING_ZERO));

        // Note: Gets the "confidential asset pool" for this asset type, or sets it up if this asset type is veiled for the first time
        let pool_fa_store = ensure_pool_fa_store(asset_type);

        // Step 1: Transfer the asset from the user's account into the confidential asset pool.
        //
        // Note: Dispatchable transfers may deliver less than `amount` (e.g., due to fees for deflationary tokens), so
        // we measure the pool balance before & after to credit only what was actually received.
        let before = fungible_asset::balance(pool_fa_store);
        let depositor_fa_store = primary_fungible_store::primary_store(addr, asset_type);
        dispatchable_fungible_asset::transfer(depositor, depositor_fa_store, pool_fa_store, amount);

        // Step 2: "Mint" corresponding confidential assets for the depositor, and add them to their pending balance.
        let ca_store = borrow_confidential_store_mut(addr, asset_type);

        add_assign_pending(&mut ca_store.pending_balance, &new_pending_u64_no_randomness(amount));
        ca_store.transfers_received += 1;

        // Make sure the depositor has "room" in their pending balance for this deposit
        assert!(
            ca_store.transfers_received <= MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        event::emit(Deposited::V1 { addr, amount, asset_type, new_pending_balance: ca_store.pending_balance });

        // Abundantly-paranoid: Re-asserting dispatchable FA functionality that charges fees on withdraw/deposit was not invoked.
        assert!(amount == fungible_asset::balance(pool_fa_store) - before, error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));
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
    public(friend) fun withdraw_to(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        amount: u64,
        proof: WithdrawalProof
    ) acquires ConfidentialStore, GlobalConfig, AssetConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));
        assert!(is_safe_for_confidentiality(&asset_type), error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));

        let sender_addr = signer::address_of(sender);

        // Read values before mutable borrow to avoid conflicting borrows of ConfidentialStore
        let ek = get_encryption_key(sender_addr, asset_type);
        let old_balance = get_available_balance(sender_addr, asset_type);
        let effective_auditor = get_effective_auditor_config(asset_type);

        let compressed_new_balance = assert_valid_withdrawal_proof(
            sender, asset_type,
            &ek, amount, &old_balance, &effective_auditor.config.ek, proof
        );

        let ca_store = borrow_confidential_store_mut(sender_addr, asset_type);
        if(amount == 0 && ca_store.normalized) {
            abort(error::invalid_state(E_ALREADY_NORMALIZED));
        };
        ca_store.normalized = true;
        ca_store.available_balance = compressed_new_balance;
        ca_store.update_auditor_hint(&effective_auditor); // enables auditor to later tell whether their balance ciphertext is stale

        // Copy state for the event (before any further borrows)
        let new_available_balance = ca_store.available_balance;
        let auditor_hint = ca_store.auditor_hint;

        if (amount > 0) {
            let pool_fa_store = get_pool_fa_store(asset_type);  // must exist b.c. sender's CA store exists
            let before = fungible_asset::balance(pool_fa_store);

            dispatchable_fungible_asset::transfer(&get_global_config_signer(), pool_fa_store, primary_fungible_store::ensure_primary_store_exists(to, asset_type), amount);
            event::emit(Withdrawn::V1 { from: sender_addr, to, amount, asset_type, new_available_balance, auditor_hint });

            // Re-asserting dispatchable FA functionality that charges fees on withdraw/deposit was not invoked.
            assert!(amount == before - fungible_asset::balance(pool_fa_store), error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));
        } else {
            event::emit(Normalized::V1 { addr: sender_addr, asset_type, new_available_balance, auditor_hint });
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
        sigma_proto_resp: vector<vector<u8>>,
        memo: vector<u8>,
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
            proof,
            memo,
        )
    }

    /// Transfers a secret amount of tokens from sender's available balance to recipient's pending balance.
    public(friend) fun confidential_transfer(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        to: address,
        proof: TransferProof,
        memo: vector<u8>,
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));
        assert!(is_safe_for_confidentiality(&asset_type), error::invalid_argument(E_UNSAFE_DISPATCHABLE_FA));
        assert!(is_confidentiality_enabled_for_asset_type(asset_type), error::invalid_argument(E_ASSET_TYPE_DISALLOWED));
        assert!(!incoming_transfers_paused(to, asset_type), error::invalid_state(E_INCOMING_TRANSFERS_PAUSED));
        assert!(memo.length() <= MAX_MEMO_BYTES, error::invalid_argument(E_MEMO_TOO_LONG));

        let from = signer::address_of(sender);
        assert!(from != to, error::invalid_argument(E_SELF_TRANSFER));
        let effective_auditor = get_effective_auditor_config(asset_type);
        let ek_sender = get_encryption_key(from, asset_type);
        let ek_recip = get_encryption_key(to, asset_type);
        let old_balance = get_available_balance(from, asset_type);

        // Note: Sender's amount in `TransferProof::compressed_amount::compressed_R_sender` is not used here; only included so it can be indexed for dapps that need it
        let (compressed_new_balance, amount, compressed_amount, ek_volun_auds) =
            assert_valid_transfer_proof(
                sender, to, asset_type,
                &ek_sender, &ek_recip,
                &old_balance, &effective_auditor.config.ek,
                proof
            );

        // Update recipient's confidential store
        let recip_ca_store = borrow_confidential_store_mut(to, asset_type);
        let new_pending_balance = add_assign_pending(&mut recip_ca_store.pending_balance, &amount);
        recip_ca_store.transfers_received += 1;

        assert!(
            recip_ca_store.transfers_received <= MAX_TRANSFERS_BEFORE_ROLLOVER,
            error::invalid_state(E_PENDING_BALANCE_MUST_BE_ROLLED_OVER)
        );

        // Update sender's confidential store
        let sender_ca_store = borrow_confidential_store_mut(from, asset_type);
        sender_ca_store.normalized = true;
        sender_ca_store.available_balance = compressed_new_balance;
        sender_ca_store.update_auditor_hint(&effective_auditor); // enables auditor to later tell whether their balance ciphertext is stale

        event::emit(Transferred::V1 {
            from, to, asset_type, amount: compressed_amount, ek_volun_auds,
            sender_auditor_hint: sender_ca_store.auditor_hint,
            new_sender_available_balance: compressed_new_balance,
            new_recip_pending_balance: new_pending_balance,
            memo,
        });
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
    ) acquires ConfidentialStore, GlobalConfig {
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

    public(friend) fun rotate_encryption_key(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        proof: KeyRotationProof,
        resume_incoming_transfers: bool,
    ) acquires ConfidentialStore, GlobalConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));
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
        assert!(!compressed_new_ek.is_identity(), error::invalid_argument(E_EK_IS_IDENTITY));
        ca_store.ek = compressed_new_ek;
        // We're just updating the available balance's EK-dependant R component & leaving the pending balance the same.
        confidential_balance::set_available_R(&mut ca_store.available_balance, compressed_new_R);
        if (resume_incoming_transfers) {
            ca_store.pause_incoming = false;
            event::emit(IncomingTransfersPauseChanged::V1 { addr: signer::address_of(owner), asset_type, paused: false });
        };
        event::emit(KeyRotated::V1 {
            addr: signer::address_of(owner), asset_type, new_ek: compressed_new_ek, new_available_balance: ca_store.available_balance,
        });
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

        withdraw_to_raw(
            sender, asset_type, user, 0,
            new_balance_P, new_balance_R, new_balance_R_aud,
            zkrp_new_balance, sigma_proto_comm, sigma_proto_resp
        );
    }

    /// Re-encrypts the available balance to ensure all chunks are within 16-bit bounds (required before rollover).
    fun normalize(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        proof: WithdrawalProof
    ) acquires ConfidentialStore, AssetConfig, GlobalConfig {
        let user = signer::address_of(sender);

        // Normalization is withdrawal with v = 0
        withdraw_to(sender, asset_type, user, 0, proof);
    }

    /// Rolls over pending balance into available balance, resetting pending to zero.
    public entry fun rollover_pending_balance(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore, GlobalConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));

        let user = signer::address_of(sender);
        let ca_store = borrow_confidential_store_mut(user, asset_type);

        assert!(ca_store.normalized, error::invalid_state(E_NORMALIZATION_REQUIRED));
        assert!(ca_store.transfers_received > 0, error::invalid_state(E_NOTHING_TO_ROLLOVER));

        ca_store.available_balance.add_assign_available_excluding_auditor(&ca_store.pending_balance);
        // Note: R_aud components [must] remain stale, but will be refreshed on the next normalize/withdraw/transfer
        // Note: Since this function does not update the *auditor's* available balance, we do not update the auditor hint.

        ca_store.normalized = false;
        ca_store.transfers_received = 0;
        ca_store.pending_balance = new_zero_pending_compressed();

        event::emit(RolledOver::V1 { addr: user, asset_type, new_available_balance: ca_store.available_balance });
    }

    /// Rollover + pause incoming transfers (required before key rotation).
    public entry fun rollover_pending_balance_and_pause(
        sender: &signer,
        asset_type: Object<fungible_asset::Metadata>
    ) acquires ConfidentialStore, GlobalConfig {
        rollover_pending_balance(sender, asset_type);
        set_incoming_transfers_paused(sender, asset_type, true);
    }

    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //
    //                                             //
    // ^^^ End of SECURITY-SENSITIVE functions ^^^ //
    //                                             //
    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //

    // === Public, non-security-sensitive functions (7 out of 13) ===
    //
    // Note: These functions can be useful for external contracts that want to integrate with the Confidential Asset
    // protocol.

    /// Pauses or resumes incoming transfers. Pausing is required before key rotation.
    public entry fun set_incoming_transfers_paused(
        owner: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        paused: bool
    ) acquires ConfidentialStore, GlobalConfig {
        assert!(!is_emergency_paused(), error::invalid_state(E_EMERGENCY_PAUSED));

        let ca_store = borrow_confidential_store_mut(signer::address_of(owner), asset_type);
        let old_paused = ca_store.pause_incoming;
        if (old_paused != paused) {
            ca_store.pause_incoming = paused;
            event::emit(IncomingTransfersPauseChanged::V1 { addr: signer::address_of(owner), asset_type, paused });
        }
    }

    // === Public, governance functions (8 out of 13) ===

    // ======================================================= //
    //     SECURITY-SENSITIVE public governance functions      //
    //       (bugs here could lead to loss of privacy)         //
    // ======================================================= //

    /// Enables or disables the allow list for confidential transfers.
    public fun set_allow_listing(aptos_framework: &signer, enabled: bool) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let global_config = borrow_global_mut<GlobalConfig>(@aptos_framework);
        let old_allow_listing_enabled = global_config.allow_list_enabled;
        if (old_allow_listing_enabled != enabled) {
            global_config.allow_list_enabled = enabled;
            event::emit(AllowListingChanged::V1 { enabled });
        }
    }

    /// Enables or disables confidentiality for the APT token.
    public fun set_confidentiality_for_apt(aptos_framework: &signer, allowed: bool) {
        system_addresses::assert_aptos_framework(aptos_framework);
        let asset_type = object::address_to_object<fungible_asset::Metadata>(@aptos_fungible_asset);
        set_confidentiality_for_asset_type(aptos_framework, asset_type, allowed)
    }

    /// When allow listing is on, this enables or disables confidential transfers for a specific asset type. In contrast,
    /// if allow listing is disabled, this aborts. Note that, in this case, `is_confidentiality_enabled_for_asset_type`
    /// will correctly return `false` for any asset type.
    public fun set_confidentiality_for_asset_type(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        allowed: bool
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        // When allow listing is disabled, updates to `AssetConfig::V1::allowed` are not meaningful, so we forbid them.
        assert!(is_allow_listing_required(), error::invalid_state(E_ALLOW_LISTING_IS_DISABLED));

        let config = borrow_global_mut<AssetConfig>(get_asset_config_address_or_create(asset_type));
        if (config.allowed != allowed) {
            config.allowed = allowed;
            event::emit(ConfidentialityForAssetTypeChanged::V1 { asset_type, allowed });
        }
    }

    /// Sets or removes the auditor for a specific asset type. Epoch increments only on install/change.
    public fun set_asset_specific_auditor(
        aptos_framework: &signer,
        asset_type: Object<fungible_asset::Metadata>,
        auditor_ek: Option<vector<u8>>
    ) acquires AssetConfig, GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);

        let config_addr = get_asset_config_address_or_create(asset_type);
        if (update_auditor(&mut borrow_global_mut<AssetConfig>(config_addr).auditor, auditor_ek)) {
            let new = borrow_global<AssetConfig>(config_addr).auditor;
            event::emit(AssetSpecificAuditorChanged::V1 { asset_type, new });
        }
    }

    /// Sets or removes the global auditor (fallback when no asset-specific auditor). Epoch increments only on install/change.
    public fun set_global_auditor(aptos_framework: &signer, auditor_ek: Option<vector<u8>>) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let config = borrow_global_mut<GlobalConfig>(@aptos_framework);
        if (update_auditor(&mut config.global_auditor, auditor_ek)) {
            event::emit(GlobalAuditorChanged::V1 { new: config.global_auditor });
        }
    }

    /// Shared logic for installing/changing/removing an auditor EK. Validates non-identity, increments epoch on install/change.
    /// Returns `true` if the auditor config actually changed.
    fun update_auditor(auditor: &mut AuditorConfig, new_ek_bytes: Option<vector<u8>>): bool {
        let new_ek = new_ek_bytes.map(|ek| new_compressed_point_from_bytes(ek).extract());

        if (new_ek.is_some()) {
            assert!(!new_ek.borrow().is_identity(), error::invalid_argument(E_EK_IS_IDENTITY));
        };

        // Increment epoch only when installing or changing the EK (not when removing):
        // i.e.,  should_increment = [ Is None --> Some(ek) ? ] or [ Is Some(old) --> Some(new), with new != old ? ]
        let should_increment = if (new_ek.is_some()) {
            if (auditor.ek.is_some()) {
                !new_ek.borrow().compressed_point_equals(auditor.ek.borrow())   // i.e., new != old
            } else {
                true // None --> Some(ek): installing
            }
        } else {
            false // removing or no-op
        };

        // Changed if: epoch incremented (install/change), or EK removed (Some → None)
        let is_removal = auditor.ek.is_some() && new_ek.is_none();
        let changed = should_increment || is_removal;

        auditor.epoch += if (should_increment) { 1 } else { 0 };
        auditor.ek = new_ek;

        changed
    }

    /// Pauses or unpauses all user operations. Upgrades GlobalConfig from V1 to V2 on first call (for testnet compatibility).
    public fun set_emergency_paused(aptos_framework: &signer, paused: bool) acquires GlobalConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let config = borrow_global_mut<GlobalConfig>(@aptos_framework);
        if (config is GlobalConfig::V2) {
            let GlobalConfig::V2 { emergency_paused, .. } = config;
            if (*emergency_paused != paused) {
                *emergency_paused = paused;
                event::emit(EmergencyPauseChanged::V1 { paused });
            }
        } else {
            // Upgrade V1 → V2: move_from, destructure, reconstruct with new field
            let GlobalConfig::V1 { allow_list_enabled, global_auditor, extend_ref } =
                move_from<GlobalConfig>(@aptos_framework);
            move_to(aptos_framework, GlobalConfig::V2 {
                allow_list_enabled, global_auditor, extend_ref, emergency_paused: paused,
            });
            // V1 is implicitly unpaused, so only emit if actually changing to paused
            if (paused) {
                event::emit(EmergencyPauseChanged::V1 { paused });
            }
        }
    }

    // ============================================================== //
    //     End of SECURITY-SENSITIVE public governance functions      //
    // ============================================================== //

    // === Public view functions (9 out of 13) ===

    #[view]
    public fun is_emergency_paused(): bool acquires GlobalConfig {
        let config = borrow_global<GlobalConfig>(@aptos_framework);
        match (config) {
            GlobalConfig::V2 { emergency_paused, .. } => *emergency_paused,
            _ => false,
        }
    }

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
        borrow_global<GlobalConfig>(@aptos_framework).allow_list_enabled
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
    /// Returns the auditor hint for a user's confidential store, indicating which auditor the balance ciphertext is encrypted for.
    public fun get_effective_auditor_hint(
        user: address, asset_type: Object<fungible_asset::Metadata>
    ): Option<EffectiveAuditorHint> acquires ConfidentialStore {
        borrow_confidential_store(user, asset_type).auditor_hint
    }

    #[view]
    /// Note: Dapp developers should **not** need to call this function, which is why (for now) it is marked private.
    ///
    /// This ignores the global auditor, if any, and only returns the asset-specific auditor config. Furthermore, it returns
    /// the auditor config even if the asset_type is no longer allow-listed.
    fun get_asset_specific_auditor_config(
        asset_type: Object<fungible_asset::Metadata>
    ): AuditorConfig acquires AssetConfig, GlobalConfig {
        let asset_config_address = get_asset_config_address(asset_type);

        borrow_global<AssetConfig>(asset_config_address).auditor
    }

    #[view]
    /// Note: Dapp developers should **not** need to call this function, which is why (for now) it is marked private.
    ///
    /// This ignores asset-specific auditors, if any, and only returns the global auditor config. Furthermore, it returns
    /// the auditor config even if the asset_type is no longer allow-listed.
    fun get_global_auditor_config(): AuditorConfig acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_framework).global_auditor
    }

    #[view]
    /// Returns the effective auditor: asset-specific if its EK is set, else global.
    /// Used by dapp developers to fetch the right auditor EK to create withdraw, normalize or transfer transactions.
    public fun get_effective_auditor_config(
        asset_type: Object<fungible_asset::Metadata>
    ): EffectiveAuditorConfig acquires AssetConfig, GlobalConfig {
        let config_addr = get_asset_config_address(asset_type); // first, check asset-specific auditor
        if (exists<AssetConfig>(config_addr)) {
            let auditor = borrow_global<AssetConfig>(config_addr).auditor;
            // Only use asset-specific auditor if its EK is actually set; otherwise fall through to global.
            if (auditor.ek.is_some()) {
                return EffectiveAuditorConfig::V1 {
                    is_global: false,
                    config: auditor
                };
            };
        };

        EffectiveAuditorConfig::V1 {      // otherwise, fall back to global auditor
            is_global: true,
            config: borrow_global<GlobalConfig>(@aptos_framework).global_auditor
        }
    }

    #[view]
    /// Returns the circulating supply of the confidential asset.
    public fun get_total_confidential_supply(asset_type: Object<fungible_asset::Metadata>): u64 acquires GlobalConfig {
        fungible_asset::balance(get_pool_fa_store(asset_type))
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

    #[view]
    public fun get_max_memo_bytes(): u64 {
        MAX_MEMO_BYTES
    }

    // === Private, internal functions (10 out of 13) ===

    /// Updates the auditor hint stored in a confidential store to track the currently-set on-chain effective auditor
    fun update_auditor_hint(self: &mut ConfidentialStore, effective_auditor: &EffectiveAuditorConfig) {
        if (effective_auditor.config.ek.is_none()) {
            // If there is no effective auditor EK set, we unset the effective auditor hint
            self.auditor_hint = std::option::none()
        } else {
            // Otherwise, we update the effective auditor hint: type [global or asset-specific] and epoch
            self.auditor_hint = std::option::some(EffectiveAuditorHint::V1 {
                is_global: effective_auditor.is_global,
                epoch: effective_auditor.config.epoch,
            })
        };
    }

    fun get_asset_config_address(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let config_ext = &borrow_global<GlobalConfig>(@aptos_framework).extend_ref;
        let config_ext_address = config_ext.address_from_extend_ref();
        object::create_object_address(&config_ext_address, construct_asset_config_seed(asset_type))
    }

    fun get_asset_config_address_or_create(asset_type: Object<fungible_asset::Metadata>): address acquires GlobalConfig {
        let addr = get_asset_config_address(asset_type);

        if (!exists<AssetConfig>(addr)) {
            let asset_config_signer = get_asset_config_signer(asset_type);

            move_to(
                &asset_config_signer,
                // We disallow the asset type from being made confidential since this function is called in a lot of different contexts.
                AssetConfig::V1 { allowed: false, auditor: AuditorConfig::V1 { ek: std::option::none(), epoch: 0 } }
            );
        };

        addr
    }

    fun get_global_config_signer(): signer acquires GlobalConfig {
        borrow_global<GlobalConfig>(@aptos_framework).extend_ref.generate_signer_for_extending()
    }

    fun get_pool_fa_store(asset_type: Object<fungible_asset::Metadata>): Object<FungibleStore> {
        let global_config_addr = borrow_global<GlobalConfig>(@aptos_framework).extend_ref.address_from_extend_ref();
        assert!(primary_fungible_store::primary_store_exists(global_config_addr, asset_type), error::not_found(E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE));
        primary_fungible_store::primary_store(global_config_addr, asset_type)
    }

    fun ensure_pool_fa_store(asset_type: Object<fungible_asset::Metadata>): Object<FungibleStore> {
        let global_config_addr = borrow_global<GlobalConfig>(@aptos_framework).extend_ref.address_from_extend_ref();
        primary_fungible_store::ensure_primary_store_exists(global_config_addr, asset_type)
    }

    fun get_confidential_store_signer(user: &signer, asset_type: Object<fungible_asset::Metadata>): signer {
        object::create_named_object(user, construct_confidential_store_seed(asset_type)).generate_signer()
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
        let config_ext = &borrow_global<GlobalConfig>(@aptos_framework).extend_ref;
        let config_ext_signer = config_ext.generate_signer_for_extending();

        let config_ctor =
            &object::create_named_object(&config_ext_signer, construct_asset_config_seed(asset_type));

        config_ctor.generate_signer()
    }

    /// Unique seed per (user, asset-type) for the ConfidentialStore object address.
    fun construct_confidential_store_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::ConfidentialStore",
                @aptos_framework,
                asset_type.object_address()
            )
        )
    }

    /// Unique seed per asset-type for the AssetConfig object address.
    fun construct_asset_config_seed(asset_type: Object<fungible_asset::Metadata>): vector<u8> {
        bcs::to_bytes(
            &string_utils::format2(
                &b"confidential_asset::{}::asset_type::{}::AssetConfig",
                @aptos_framework,
                asset_type.object_address()
            )
        )
    }

    // === Test-only functions (11 out of 13) ===

    #[test_only]
    public(friend) fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    #[test_only]
    public(friend) fun get_effective_auditor_ek(
        asset_type: Object<fungible_asset::Metadata>
    ): Option<CompressedRistretto> acquires AssetConfig, GlobalConfig {
        get_effective_auditor_config(asset_type).config.ek
    }

    #[test_only]
    public(friend) fun new_registration_proof(sigma: sigma_protocol_proof::Proof): RegistrationProof {
        RegistrationProof::V1 { sigma }
    }

    #[test_only]
    public(friend) fun new_withdrawal_proof(
        compressed_new_balance: CompressedBalance<Available>, zkrp_new_balance: RangeProof, sigma: sigma_protocol_proof::Proof,
    ): WithdrawalProof {
        WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma }
    }

    #[test_only]
    public(friend) fun new_transfer_proof(
        compressed_new_balance: CompressedBalance<Available>, compressed_amount: CompressedAmount,
        compressed_ek_volun_auds: vector<CompressedRistretto>,
        zkrp_new_balance: RangeProof, zkrp_amount: RangeProof, sigma: sigma_protocol_proof::Proof,
    ): TransferProof {
        TransferProof::V1 { compressed_new_balance, compressed_amount, compressed_ek_volun_auds, zkrp_new_balance, zkrp_amount, sigma }
    }

    #[test_only]
    public(friend) fun new_key_rotation_proof(
        compressed_new_ek: CompressedRistretto, compressed_new_R: vector<CompressedRistretto>, sigma: sigma_protocol_proof::Proof,
    ): KeyRotationProof {
        KeyRotationProof::V1 { compressed_new_ek, compressed_new_R, sigma }
    }

    #[test_only]
    public(friend) fun get_transfer_proof_compressed_amount(proof: &TransferProof): &CompressedAmount {
        &proof.compressed_amount
    }

    // === Proof enums & verification functions (12 out of 13) ===

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

        let stmt = sigma_protocol_withdraw::new_withdrawal_statement(
            *ek, old_balance, &compressed_new_balance, compressed_ek_aud, v,
        );
        confidential_range_proofs::assert_valid_range_proof(compressed_new_balance.get_compressed_P(), &zkrp_new_balance);

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
        CompressedAmount,
        vector<CompressedRistretto>,
    ) {

        let TransferProof::V1 {
            compressed_new_balance, compressed_amount,
            compressed_ek_volun_auds,
            zkrp_new_balance, zkrp_amount, sigma
        } = proof;

        // Note: `update_auditor` already guarantees that `compressed_ek_eff_aud` is not the identity, but the voluntary
        // auditor EKs need to be manually checked.
        compressed_ek_volun_auds.for_each_ref(|ek| {
            assert!(!ek.is_identity(), error::invalid_argument(E_EK_IS_IDENTITY));
        });

        let has_effective_auditor = compressed_ek_eff_aud.is_some();
        let num_volun_auditors = compressed_ek_volun_auds.length();

        // Auditor count checks are performed inside new_transfer_statement
        let (stmt, amount) = sigma_protocol_transfer::new_transfer_statement(
            *compressed_ek_sender, *compressed_ek_recip,
            compressed_old_balance, &compressed_new_balance,
            &compressed_amount,
            compressed_ek_eff_aud, &compressed_ek_volun_auds,
        );

        confidential_range_proofs::assert_valid_range_proof(compressed_amount.get_compressed_P(), &zkrp_amount);
        confidential_range_proofs::assert_valid_range_proof(compressed_new_balance.get_compressed_P(), &zkrp_new_balance);

        let session = sigma_protocol_transfer::new_session(
            sender, recipient_addr, asset_type, has_effective_auditor, num_volun_auditors,
        );
        session.assert_verifies(&stmt, &sigma);

        (compressed_new_balance, amount, compressed_amount, compressed_ek_volun_auds)
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

    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //
    //                                                                //
    // ^^^ End of SECURITY-SENSITIVE proof verification functions ^^^ //
    //                                                                //
    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ //
}
