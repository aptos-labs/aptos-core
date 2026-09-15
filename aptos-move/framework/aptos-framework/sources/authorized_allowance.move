/// Authorized allowances for fungible assets.
///
/// This module lets an account authorize a transfer without submitting the transaction itself. The
/// account signs a `TransferAllowance` payload off chain; anyone holding that signature can then
/// submit it, and the signature is verified on chain during the transaction before the funds move.
///
/// The payload names the chain, the sender, the recipient, the fungible asset, the maximum amount and
/// an expiration time, so a signature can only ever move the asset it describes, to the party it
/// describes, on the network it was signed for, and only until it expires.
///
/// The flow is:
/// 1. The sender picks an unused `nonce` and signs the payload off chain. `allowance_signing_message`
///    returns the exact bytes to sign.
/// 2. Any account submits `transfer_with_allowance` (or `redeem_with_allowance` for a partial pull)
///    together with the sender's public key and the signature.
/// 3. The signature is checked against the sender's current authentication key, the payload is checked
///    against the chain id and the current time, and the amount is checked against what has already
///    been pulled under this nonce. Only then is the transfer performed against the sender's and
///    recipient's primary stores.
///
/// An allowance may be redeemed in several smaller pulls as long as the total stays within `amount`.
/// What `amount` caps is the drop in the sender's balance, not just the sum of the amounts handed to
/// the recipient, so an asset with a dispatchable withdraw hook that charges a fee cannot be used to
/// take more out of the sender's store than they signed for. The sender can invalidate an outstanding
/// allowance early with `revoke`.
///
/// Assets that register a `derived_balance` hook are not supported, because the module cannot then
/// read the store balance it needs to enforce that cap; redemption aborts for them.
module aptos_framework::authorized_allowance {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::signer;
    use aptos_std::ed25519;
    use aptos_std::table::{Self, Table};
    use aptos_framework::account;
    use aptos_framework::chain_id;
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::event;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::timestamp;

    /// The allowance expired before it was redeemed.
    const EALLOWANCE_EXPIRED: u64 = 1;
    /// The sender revoked this allowance.
    const EALLOWANCE_REVOKED: u64 = 2;
    /// The requested amount exceeds what is left of the allowance.
    const EALLOWANCE_EXHAUSTED: u64 = 3;
    /// The sender already used this nonce for a different allowance.
    const ENONCE_ALREADY_USED: u64 = 4;
    /// An allowance cannot be redeemed for zero.
    const EZERO_AMOUNT: u64 = 5;
    /// No allowance is recorded under this sender and nonce.
    const EALLOWANCE_NOT_FOUND: u64 = 6;
    /// The allowance has not expired yet, so its record cannot be reclaimed.
    const EALLOWANCE_NOT_EXPIRED: u64 = 7;

    /// The payload a sender signs off chain to let `recipient` pull funds from its primary store.
    ///
    /// The signature covers the BCS encoding of `aptos_std::ed25519::SignedMessage<TransferAllowance>`,
    /// which embeds this struct's type. A signature produced for any other challenge in the framework
    /// therefore cannot be replayed here, and vice versa.
    struct TransferAllowance has copy, drop {
        /// The chain the allowance is valid on. Filled in on chain from `chain_id::get()`, so an
        /// allowance signed for one network cannot be redeemed on another.
        chain_id: u8,
        /// The account the funds are pulled from. Must be the account that produced the signature.
        sender: address,
        /// The only account that may receive the funds.
        recipient: address,
        /// Address of the fungible asset `Metadata` object the allowance applies to.
        metadata: address,
        /// The most the sender's balance may fall by across all redemptions of this allowance. A
        /// dispatchable asset that charges a withdrawal fee counts that fee against this cap too, so
        /// no sequence of redemptions can take more than `amount` out of the sender's store.
        amount: u64,
        /// Unix timestamp in seconds at which the allowance stops being redeemable.
        expiration_secs: u64,
        /// Sender-chosen identifier, unique among the allowances that sender has outstanding. It lets
        /// the sender revoke the allowance and keeps a fully redeemed payload from being reused.
        nonce: u64,
    }

    /// How much of a given allowance has been pulled, and whether it was revoked. Kept under the
    /// issuing account and created the first time one of that account's allowances is used.
    struct AllowanceRegistry has key {
        allowances: Table<u64, AllowanceState>,
    }

    struct AllowanceState has store, drop {
        /// SHA3-256 of the BCS-encoded `TransferAllowance` bound to this nonce. Empty for records
        /// created by `revoke`, which blocks a nonce without knowing the payload it was signed into.
        payload_hash: vector<u8>,
        /// The total charged so far, never more than the payload's `amount`. Each redemption is
        /// charged the larger of the amount it transferred and the amount it debited from the sender.
        redeemed: u64,
        /// Copy of the payload's expiration, used to decide when this record can be reclaimed.
        expiration_secs: u64,
        revoked: bool,
    }

    #[event]
    struct AllowanceRedeemed has drop, store {
        sender: address,
        recipient: address,
        metadata: address,
        nonce: u64,
        /// The amount transferred to the recipient by this redemption.
        amount: u64,
        /// The amount the sender's store actually lost, which a withdrawal fee can push above `amount`.
        debited: u64,
        /// The total charged against this allowance so far, including this redemption.
        total_redeemed: u64,
        /// The maximum the allowance permits in total.
        allowance_amount: u64,
    }

    #[event]
    struct AllowanceRevoked has drop, store {
        sender: address,
        nonce: u64,
    }

    /// Redeem a pre-signed allowance in full, transferring `amount` from `sender` to `recipient`.
    ///
    /// The arguments up to `nonce` reconstruct the payload that `sender` signed; `signature` must be
    /// `sender`'s signature over it under `account_scheme` (`0` for Ed25519, `1` for MultiEd25519) and
    /// `account_public_key`. Anyone may submit this transaction and pay for its gas.
    public entry fun transfer_with_allowance(
        sender: address,
        recipient: address,
        metadata: Object<Metadata>,
        amount: u64,
        expiration_secs: u64,
        nonce: u64,
        account_scheme: u8,
        account_public_key: vector<u8>,
        signature: vector<u8>,
    ) acquires AllowanceRegistry {
        redeem_with_allowance(
            sender,
            recipient,
            metadata,
            amount,
            expiration_secs,
            nonce,
            account_scheme,
            account_public_key,
            signature,
            amount,
        );
    }

    /// Redeem part of a pre-signed allowance, transferring `redeem_amount` of the `amount` the payload
    /// permits. The rest stays available under the same signature until the allowance expires or is
    /// revoked.
    ///
    /// If the asset charges a withdrawal fee, that fee is charged against the allowance alongside
    /// `redeem_amount`, so redeeming the payload in full requires leaving room for it.
    public entry fun redeem_with_allowance(
        sender: address,
        recipient: address,
        metadata: Object<Metadata>,
        amount: u64,
        expiration_secs: u64,
        nonce: u64,
        account_scheme: u8,
        account_public_key: vector<u8>,
        signature: vector<u8>,
        redeem_amount: u64,
    ) acquires AllowanceRegistry {
        assert!(redeem_amount > 0, error::invalid_argument(EZERO_AMOUNT));
        assert!(redeem_amount <= amount, error::invalid_argument(EALLOWANCE_EXHAUSTED));
        assert!(
            timestamp::now_seconds() < expiration_secs,
            error::invalid_state(EALLOWANCE_EXPIRED),
        );

        let metadata_address = object::object_address(&metadata);
        let allowance = TransferAllowance {
            chain_id: chain_id::get(),
            sender,
            recipient,
            metadata: metadata_address,
            amount,
            expiration_secs,
            nonce,
        };
        // Aborts unless `signature` verifies against the current authentication key of `sender`.
        account::verify_signed_message(
            sender,
            account_scheme,
            account_public_key,
            signature,
            allowance,
        );

        let sender_signer = create_signer(sender);
        let sender_store = primary_fungible_store::ensure_primary_store_exists(sender, metadata);

        // Charge the nominal amount before the funds move, so a dispatchable store hook that calls
        // back into this module cannot spend the same budget twice.
        let total_redeemed = consume_allowance(&sender_signer, &allowance, redeem_amount);

        let balance_before = fungible_asset::balance(sender_store);
        primary_fungible_store::transfer(&sender_signer, metadata, recipient, redeem_amount);
        let balance_after = fungible_asset::balance(sender_store);

        // A dispatchable withdraw hook may take more out of the store than it hands over, to charge a
        // fee for instance. That excess is charged to the allowance as well, so the sender's balance
        // can never fall by more than the `amount` they signed for, however many redemptions it takes.
        // If the excess does not fit, `consume_allowance` aborts and the transfer is rolled back.
        let debited = if (balance_before > balance_after) {
            balance_before - balance_after
        } else { 0 };
        if (debited > redeem_amount) {
            total_redeemed =
                consume_allowance(&sender_signer, &allowance, debited - redeem_amount);
        };

        event::emit(AllowanceRedeemed {
            sender,
            recipient,
            metadata: metadata_address,
            nonce,
            amount: redeem_amount,
            debited,
            total_redeemed,
            allowance_amount: amount,
        });
    }

    /// Invalidate the allowance issued under `nonce`, whether or not it has been redeemed. This holds
    /// even if the payload has not been submitted yet, so a sender who loses track of a signature can
    /// still take it out of circulation before it expires.
    public entry fun revoke(sender: &signer, nonce: u64) acquires AllowanceRegistry {
        let sender_address = signer::address_of(sender);
        ensure_registry_exists(sender);
        let allowances = &mut AllowanceRegistry[sender_address].allowances;
        if (allowances.contains(nonce)) {
            allowances.borrow_mut(nonce).revoked = true;
        } else {
            allowances.add(nonce, AllowanceState {
                payload_hash: vector[],
                redeemed: 0,
                expiration_secs: 0,
                revoked: true,
            });
        };
        event::emit(AllowanceRevoked { sender: sender_address, nonce });
    }

    /// Reclaim the record of an expired allowance. Anyone may call this: an expired allowance can never
    /// be redeemed again, so dropping what is tracked about it changes nothing. Revoked records are kept
    /// instead of reclaimed, because the payload they block carries an expiration this module never saw.
    public entry fun remove_expired(sender: address, nonce: u64) acquires AllowanceRegistry {
        assert!(exists<AllowanceRegistry>(sender), error::not_found(EALLOWANCE_NOT_FOUND));
        let allowances = &mut AllowanceRegistry[sender].allowances;
        assert!(allowances.contains(nonce), error::not_found(EALLOWANCE_NOT_FOUND));
        let state = allowances.borrow(nonce);
        assert!(!state.revoked, error::invalid_state(EALLOWANCE_REVOKED));
        assert!(
            timestamp::now_seconds() >= state.expiration_secs,
            error::invalid_state(EALLOWANCE_NOT_EXPIRED),
        );
        allowances.remove(nonce);
    }

    #[view]
    /// The exact bytes a sender must sign to authorize the described transfer. Wallets and SDKs can
    /// call this instead of reproducing the payload layout themselves.
    public fun allowance_signing_message(
        sender: address,
        recipient: address,
        metadata: Object<Metadata>,
        amount: u64,
        expiration_secs: u64,
        nonce: u64,
    ): vector<u8> {
        let allowance = TransferAllowance {
            chain_id: chain_id::get(),
            sender,
            recipient,
            metadata: object::object_address(&metadata),
            amount,
            expiration_secs,
            nonce,
        };
        bcs::to_bytes(&ed25519::new_signed_message(allowance))
    }

    #[view]
    /// How much of the allowance `sender` issued under `nonce` has been used up. Zero if the allowance
    /// has never been redeemed. A redemption uses up the larger of the amount it transferred and the
    /// amount it debited from the sender, so this can exceed the total the recipient received.
    public fun redeemed_amount(sender: address, nonce: u64): u64 acquires AllowanceRegistry {
        if (!exists<AllowanceRegistry>(sender)) {
            return 0
        };
        let allowances = &AllowanceRegistry[sender].allowances;
        if (allowances.contains(nonce)) {
            allowances.borrow(nonce).redeemed
        } else {
            0
        }
    }

    #[view]
    /// Whether `sender` has revoked the allowance issued under `nonce`.
    public fun is_revoked(sender: address, nonce: u64): bool acquires AllowanceRegistry {
        if (!exists<AllowanceRegistry>(sender)) {
            return false
        };
        let allowances = &AllowanceRegistry[sender].allowances;
        allowances.contains(nonce) && allowances.borrow(nonce).revoked
    }

    /// Charge `redeem_amount` against the allowance's nonce and return the new total charged. Aborts
    /// if the nonce was revoked, was bound to a different payload, or has too little left.
    fun consume_allowance(
        sender_signer: &signer,
        allowance: &TransferAllowance,
        redeem_amount: u64,
    ): u64 acquires AllowanceRegistry {
        let payload_hash = hash::sha3_256(bcs::to_bytes(allowance));
        let nonce = allowance.nonce;
        ensure_registry_exists(sender_signer);
        let allowances = &mut AllowanceRegistry[allowance.sender].allowances;
        if (!allowances.contains(nonce)) {
            allowances.add(nonce, AllowanceState {
                payload_hash,
                redeemed: 0,
                expiration_secs: allowance.expiration_secs,
                revoked: false,
            });
        };
        let state = allowances.borrow_mut(nonce);
        assert!(!state.revoked, error::permission_denied(EALLOWANCE_REVOKED));
        assert!(
            state.payload_hash == payload_hash,
            error::invalid_argument(ENONCE_ALREADY_USED),
        );
        assert!(
            redeem_amount <= allowance.amount - state.redeemed,
            error::invalid_argument(EALLOWANCE_EXHAUSTED),
        );
        state.redeemed += redeem_amount;
        state.redeemed
    }

    /// Create the sender's registry if this is the first allowance of theirs to be used or revoked.
    fun ensure_registry_exists(sender: &signer) {
        if (!exists<AllowanceRegistry>(signer::address_of(sender))) {
            move_to(sender, AllowanceRegistry { allowances: table::new() });
        };
    }

    #[test_only]
    const TEST_CHAIN_ID: u8 = 4;
    #[test_only]
    const ED25519_SCHEME: u8 = 0;
    #[test_only]
    const RECIPIENT: address = @0xface;

    #[test_only]
    /// Starts the clock, sets a chain id, mints 100 units of a fresh asset to an account whose
    /// authentication key is the returned key pair, and returns everything a test needs to sign an
    /// allowance for that account.
    fun setup(
        aptos_framework: &signer,
        creator: &signer,
    ): (ed25519::SecretKey, vector<u8>, address, Object<Metadata>) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        chain_id::initialize_for_test(aptos_framework, TEST_CHAIN_ID);

        let (secret_key, public_key) = ed25519::generate_keys();
        let public_key_bytes = ed25519::validated_public_key_to_bytes(&public_key);
        let sender = account::create_account_from_ed25519_public_key(public_key_bytes);
        let sender_address = signer::address_of(&sender);

        let (constructor_ref, token) = fungible_asset::create_test_token(creator);
        let (mint_ref, _transfer_ref, _burn_ref) =
            primary_fungible_store::init_test_metadata_with_primary_store_enabled(&constructor_ref);
        let metadata = object::convert<fungible_asset::TestToken, Metadata>(token);
        primary_fungible_store::mint(&mint_ref, sender_address, 100);

        (secret_key, public_key_bytes, sender_address, metadata)
    }

    #[test_only]
    fun new_allowance(
        sender: address,
        recipient: address,
        metadata: Object<Metadata>,
        amount: u64,
        expiration_secs: u64,
        nonce: u64,
    ): TransferAllowance {
        TransferAllowance {
            chain_id: TEST_CHAIN_ID,
            sender,
            recipient,
            metadata: object::object_address(&metadata),
            amount,
            expiration_secs,
            nonce,
        }
    }

    #[test_only]
    fun sign(secret_key: &ed25519::SecretKey, allowance: TransferAllowance): vector<u8> {
        ed25519::signature_to_bytes(&ed25519::sign_struct(secret_key, allowance))
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    fun test_transfer_with_allowance(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 60, 1000, 7);
        let signature = sign(&secret_key, allowance);

        transfer_with_allowance(
            sender, RECIPIENT, metadata, 60, 1000, 7, ED25519_SCHEME, public_key, signature,
        );

        assert!(primary_fungible_store::balance(sender, metadata) == 40, 1);
        assert!(primary_fungible_store::balance(RECIPIENT, metadata) == 60, 2);
        assert!(redeemed_amount(sender, 7) == 60, 3);
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    fun test_allowance_can_be_redeemed_in_parts(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 50, 1000, 1);
        let signature = sign(&secret_key, allowance);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 50, 1000, 1, ED25519_SCHEME, public_key, signature, 20,
        );
        assert!(redeemed_amount(sender, 1) == 20, 1);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 50, 1000, 1, ED25519_SCHEME, public_key, signature, 30,
        );
        assert!(redeemed_amount(sender, 1) == 50, 2);
        assert!(primary_fungible_store::balance(RECIPIENT, metadata) == 50, 3);
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_cannot_redeem_more_than_allowance(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 50, 1000, 1);
        let signature = sign(&secret_key, allowance);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 50, 1000, 1, ED25519_SCHEME, public_key, signature, 50,
        );
        redeem_with_allowance(
            sender, RECIPIENT, metadata, 50, 1000, 1, ED25519_SCHEME, public_key, signature, 1,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x30001, location = Self)]
    fun test_expired_allowance_is_rejected(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 10, 1000, 1);
        let signature = sign(&secret_key, allowance);

        timestamp::update_global_time_for_test_secs(1000);
        transfer_with_allowance(
            sender, RECIPIENT, metadata, 10, 1000, 1, ED25519_SCHEME, public_key, signature,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = aptos_framework::account)]
    fun test_allowance_signed_for_another_chain_is_rejected(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = TransferAllowance {
            chain_id: TEST_CHAIN_ID + 1,
            sender,
            recipient: RECIPIENT,
            metadata: object::object_address(&metadata),
            amount: 10,
            expiration_secs: 1000,
            nonce: 1,
        };
        let signature = sign(&secret_key, allowance);

        transfer_with_allowance(
            sender, RECIPIENT, metadata, 10, 1000, 1, ED25519_SCHEME, public_key, signature,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = aptos_framework::account)]
    fun test_redirecting_allowance_to_another_recipient_is_rejected(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 10, 1000, 1);
        let signature = sign(&secret_key, allowance);

        transfer_with_allowance(
            sender, @0xdead, metadata, 10, 1000, 1, ED25519_SCHEME, public_key, signature,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = aptos_framework::account)]
    fun test_raising_the_amount_is_rejected(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 10, 1000, 1);
        let signature = sign(&secret_key, allowance);

        transfer_with_allowance(
            sender, RECIPIENT, metadata, 90, 1000, 1, ED25519_SCHEME, public_key, signature,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_nonce_cannot_be_reused_for_a_second_allowance(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let first = new_allowance(sender, RECIPIENT, metadata, 10, 1000, 1);
        transfer_with_allowance(
            sender, RECIPIENT, metadata, 10, 1000, 1, ED25519_SCHEME, public_key,
            sign(&secret_key, first),
        );

        // Same nonce, longer expiration: a properly signed but distinct payload.
        let second = new_allowance(sender, RECIPIENT, metadata, 10, 2000, 1);
        transfer_with_allowance(
            sender, RECIPIENT, metadata, 10, 2000, 1, ED25519_SCHEME, public_key,
            sign(&secret_key, second),
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_revoked_allowance_cannot_be_redeemed(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        let signature = sign(&secret_key, allowance);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature, 10,
        );

        revoke(&create_signer(sender), 3);
        assert!(is_revoked(sender, 3), 1);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature, 10,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_allowance_can_be_revoked_before_first_use(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        let signature = sign(&secret_key, allowance);

        revoke(&create_signer(sender), 3);
        transfer_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    fun test_expired_record_can_be_reclaimed(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        let signature = sign(&secret_key, allowance);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature, 10,
        );
        assert!(redeemed_amount(sender, 3) == 10, 1);

        timestamp::update_global_time_for_test_secs(1000);
        remove_expired(sender, 3);
        assert!(redeemed_amount(sender, 3) == 0, 2);
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x30007, location = Self)]
    fun test_unexpired_record_cannot_be_reclaimed(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        let signature = sign(&secret_key, allowance);

        transfer_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature,
        );
        remove_expired(sender, 3);
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10005, location = Self)]
    fun test_zero_redemption_is_rejected(
        aptos_framework: &signer,
        creator: &signer,
    ) acquires AllowanceRegistry {
        let (secret_key, public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        let signature = sign(&secret_key, allowance);

        redeem_with_allowance(
            sender, RECIPIENT, metadata, 40, 1000, 3, ED25519_SCHEME, public_key, signature, 0,
        );
    }

    #[test(aptos_framework = @aptos_framework, creator = @0xcafe)]
    fun test_signing_message_matches_what_the_chain_verifies(
        aptos_framework: &signer,
        creator: &signer,
    ) {
        let (_secret_key, _public_key, sender, metadata) = setup(aptos_framework, creator);
        let allowance = new_allowance(sender, RECIPIENT, metadata, 40, 1000, 3);
        assert!(
            allowance_signing_message(sender, RECIPIENT, metadata, 40, 1000, 3)
                == bcs::to_bytes(&ed25519::new_signed_message(allowance)),
            1,
        );
    }
}
