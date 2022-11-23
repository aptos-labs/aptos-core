/**
 * This provides an example for sending locked coins to recipients to be unlocked after a specific time.
 *
 * Locked coins flow:
 * 1. Deploy the lockup contract. Deployer can decide if the contract is upgradable or not.
 * 2. Sponsor accounts add locked APTs for custom expiration time + amount for recipients.
 * 3. Sponsor accounts can revoke a lock or change lockup (reduce or extend) anytime. This gives flexibility in case of
 * contract violation or special circumstances. If this is not desired, the deployer can remove these functionalities
 * before deploying.
 * 4. Once the lockup has expired, the recipient can call claim to get the unlocked tokens.
 **/
module defi::locked_coins {
    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use std::error;
    use std::signer;
    use std::vector;

    /// Represents a lock of coins until some specified unlock time. Afterward, the recipient can claim the coins.
    struct Lock<phantom CoinType> has store {
        coins: Coin<CoinType>,
        unlock_time_secs: u64,
    }

    /// Holder for a map from recipients => locks.
    /// There can be at most one lock per recipient.
    struct Locks<phantom CoinType> has key {
        locks: Table<address, Lock<CoinType>>,
        cancel_lockup_events: EventHandle<CancelLockupEvent>,
        claim_events: EventHandle<ClaimEvent>,
        update_lockup_events: EventHandle<UpdateLockupEvent>,
    }

    /// Event emitted when a lock is canceled.
    struct CancelLockupEvent has drop, store {
        recipient: address,
        amount: u64,
    }

    /// Event emitted when a recipient claims unlocked coins.
    struct ClaimEvent has drop, store {
        recipient: address,
        amount: u64,
        claimed_time_secs: u64,
    }

    /// Event emitted when lockup is updated for an existing lock.
    struct UpdateLockupEvent has drop, store {
        recipient: address,
        old_unlock_time_secs: u64,
        new_unlock_time_secs: u64,
    }

    /// No locked coins found to claim.
    const ELOCK_NOT_FOUND: u64 = 1;
    /// Lockup has not expired yet.
    const ELOCKUP_HAS_NOT_EXPIRED: u64 = 2;
    /// Can only create one active lock per recipient at once.
    const ELOCK_ALREADY_EXISTS: u64 = 3;
    /// The length of the recipients list doesn't match the amounts.
    const EINVALID_RECIPIENTS_LIST_LENGTH: u64 = 3;

    /// Batch version of add_locked_coins to process multiple recipients and corresponding amounts.
    public entry fun batch_add_locked_coins<CoinType>(
        sponsor: &signer, recipients: vector<address>, amounts: vector<u64>, unlock_time_secs: u64) acquires Locks {
        let len = vector::length(&recipients);
        assert!(len == vector::length(&amounts), error::invalid_argument(EINVALID_RECIPIENTS_LIST_LENGTH));
        let i = 0;
        while (i < len) {
            let recipient = *vector::borrow(&recipients, i);
            let amount = *vector::borrow(&amounts, i);
            add_locked_coins<CoinType>(sponsor, recipient, amount, unlock_time_secs);
            i = i + 1;
        }
    }

    /// `Sponsor` can add locked coins for `recipient` with given unlock timestamp (in seconds).
    /// There's no restriction on unlock timestamp so sponsors could technically add coins for an unlocked time in the
    /// past, which means the coins are immediately unlocked.
    public entry fun add_locked_coins<CoinType>(
        sponsor: &signer, recipient: address, amount: u64, unlock_time_secs: u64) acquires Locks {
        let sponsor_address = signer::address_of(sponsor);
        if (!exists<Locks<CoinType>>(sponsor_address)) {
            move_to(sponsor, Locks {
                locks: table::new<address, Lock<CoinType>>(),
                cancel_lockup_events: account::new_event_handle<CancelLockupEvent>(sponsor),
                claim_events: account::new_event_handle<ClaimEvent>(sponsor),
                update_lockup_events: account::new_event_handle<UpdateLockupEvent>(sponsor),
            })
        };

        let locks = borrow_global_mut<Locks<CoinType>>(sponsor_address);
        let coins = coin::withdraw<CoinType>(sponsor, amount);
        assert!(!table::contains(&locks.locks, recipient), error::already_exists(ELOCK_ALREADY_EXISTS));
        table::add(&mut locks.locks, recipient, Lock<CoinType> { coins, unlock_time_secs });
    }

    /// Recipient can claim coins that are fully unlocked (unlock time has passed).
    /// To claim, `recipient` would need the sponsor's address. In the case where each sponsor always deploys this
    /// module anew, it'd just be the module's hosted account address.
    public entry fun claim<CoinType>(recipient: &signer, sponsor: address) acquires Locks {
        assert!(exists<Locks<CoinType>>(sponsor), error::not_found(ELOCK_NOT_FOUND));
        let locks = borrow_global_mut<Locks<CoinType>>(sponsor);
        let recipient_address = signer::address_of(recipient);
        assert!(table::contains(&locks.locks, recipient_address), error::not_found(ELOCK_NOT_FOUND));

        // Delete the lock entry both to keep records clean and keep storage usage minimal.
        // This would be reverted if validations fail later (transaction atomicity).
        let Lock { coins, unlock_time_secs } = table::remove(&mut locks.locks, recipient_address);
        let now_secs = timestamp::now_seconds();
        assert!(now_secs >= unlock_time_secs, error::invalid_state(ELOCKUP_HAS_NOT_EXPIRED));

        let amount = coin::value(&coins);
        // This would fail if the recipient account is not registered to receive CoinType.
        coin::deposit(recipient_address, coins);

        event::emit_event(&mut locks.claim_events, ClaimEvent {
            recipient: recipient_address,
            amount,
            claimed_time_secs: now_secs,
        });
    }

    /// Batch version of update_lockup.
    public entry fun batch_update_lockup<CoinType>(
        sponsor: &signer, recipients: vector<address>, new_unlock_time_secs: u64) acquires Locks {
        let sponsor_address = signer::address_of(sponsor);
        assert!(exists<Locks<CoinType>>(sponsor_address), error::not_found(ELOCK_NOT_FOUND));

        let len = vector::length(&recipients);
        let i = 0;
        while (i < len) {
            let recipient = *vector::borrow(&recipients, i);
            update_lockup<CoinType>(sponsor, recipient, new_unlock_time_secs);
            i = i + 1;
        };
    }

    /// Sponsor can update the lockup of an existing lock.
    public entry fun update_lockup<CoinType>(
        sponsor: &signer, recipient: address, new_unlock_time_secs: u64) acquires Locks {
        let sponsor_address = signer::address_of(sponsor);
        assert!(exists<Locks<CoinType>>(sponsor_address), error::not_found(ELOCK_NOT_FOUND));
        let locks = borrow_global_mut<Locks<CoinType>>(sponsor_address);
        assert!(table::contains(&locks.locks, recipient), error::not_found(ELOCK_NOT_FOUND));

        let lock = table::borrow_mut(&mut locks.locks, recipient);
        let old_unlock_time_secs = lock.unlock_time_secs;
        lock.unlock_time_secs = new_unlock_time_secs;

        event::emit_event(&mut locks.update_lockup_events, UpdateLockupEvent {
            recipient,
            old_unlock_time_secs,
            new_unlock_time_secs,
        });
    }

    /// Batch version of cancel_lockup to cancel the lockup for multiple recipients.
    public entry fun batch_cancel_lockup<CoinType>(sponsor: &signer, recipients: vector<address>) acquires Locks {
        let sponsor_address = signer::address_of(sponsor);
        assert!(exists<Locks<CoinType>>(sponsor_address), error::not_found(ELOCK_NOT_FOUND));

        let len = vector::length(&recipients);
        let i = 0;
        while (i < len) {
            let recipient = *vector::borrow(&recipients, i);
            cancel_lockup<CoinType>(sponsor, recipient);
            i = i + 1;
        };
    }

    /// Sponsor can cancel an existing lock.
    public entry fun cancel_lockup<CoinType>(sponsor: &signer, recipient: address) acquires Locks {
        let sponsor_address = signer::address_of(sponsor);
        assert!(exists<Locks<CoinType>>(sponsor_address), error::not_found(ELOCK_NOT_FOUND));
        let locks = borrow_global_mut<Locks<CoinType>>(sponsor_address);
        assert!(table::contains(&locks.locks, recipient), error::not_found(ELOCK_NOT_FOUND));

        // Remove the lock and deposit coins backed into the sponsor account.
        let Lock { coins, unlock_time_secs: _ } = table::remove(&mut locks.locks, recipient);
        let amount = coin::value(&coins);
        coin::deposit(sponsor_address, coins);

        event::emit_event(&mut locks.cancel_lockup_events, CancelLockupEvent { recipient, amount });
    }

    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::coin::BurnCapability;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_framework::aptos_account;

    #[test_only]
    fun get_unlock_time(sponsor: address, recipient: address): u64 acquires Locks {
        let locks = borrow_global_mut<Locks<AptosCoin>>(sponsor);
        table::borrow(&locks.locks, recipient).unlock_time_secs
    }

    #[test_only]
    fun setup(aptos_framework: &signer, sponsor: &signer): BurnCapability<AptosCoin> {
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<AptosCoin>(
            aptos_framework,
            string::utf8(b"TC"),
            string::utf8(b"TC"),
            8,
            false,
        );
        account::create_account_for_test(signer::address_of(sponsor));
        coin::register<AptosCoin>(sponsor);
        let coins = coin::mint<AptosCoin>(2000, &mint_cap);
        coin::deposit(signer::address_of(sponsor), coins);
        coin::destroy_mint_cap(mint_cap);
        coin::destroy_freeze_cap(freeze_cap);
        burn_cap
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient = @0x234)]
    public entry fun test_recipient_can_claim_coins(
        aptos_framework: &signer, sponsor: &signer, recipient: &signer) acquires Locks {
        let recipient_addr = signer::address_of(recipient);
        aptos_account::create_account(recipient_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        add_locked_coins<AptosCoin>(sponsor, recipient_addr, 1000, 1000);
        timestamp::fast_forward_seconds(1000);
        claim<AptosCoin>(recipient, signer::address_of(sponsor));
        assert!(coin::balance<AptosCoin>(recipient_addr) == 1000, 0);
        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient = @0x234)]
    #[expected_failure(abort_code = 0x30002)]
    public entry fun test_recipient_cannot_claim_coins_if_lockup_has_not_expired(
        aptos_framework: &signer, sponsor: &signer, recipient: &signer) acquires Locks {
        let recipient_addr = signer::address_of(recipient);
        aptos_account::create_account(recipient_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        add_locked_coins<AptosCoin>(sponsor, recipient_addr, 1000, 1000);
        timestamp::fast_forward_seconds(500);
        claim<AptosCoin>(recipient, signer::address_of(sponsor));
        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient = @0x234)]
    #[expected_failure(abort_code = 0x60001)]
    public entry fun test_recipient_cannot_claim_twice(
        aptos_framework: &signer, sponsor: &signer, recipient: &signer) acquires Locks {
        let recipient_addr = signer::address_of(recipient);
        aptos_account::create_account(recipient_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        add_locked_coins<AptosCoin>(sponsor, recipient_addr, 1000, 1000);
        timestamp::fast_forward_seconds(1000);
        claim<AptosCoin>(recipient, signer::address_of(sponsor));
        claim<AptosCoin>(recipient, signer::address_of(sponsor));
        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient = @0x234)]
    public entry fun test_sponsor_can_update_lockup(
        aptos_framework: &signer, sponsor: &signer, recipient: &signer) acquires Locks {
        let sponsor_addr = signer::address_of(sponsor);
        let recipient_addr = signer::address_of(recipient);
        aptos_account::create_account(recipient_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        add_locked_coins<AptosCoin>(sponsor, recipient_addr, 1000, 1000);
        assert!(get_unlock_time(sponsor_addr, recipient_addr) == 1000, 0);
        // Extend lockup.
        update_lockup<AptosCoin>(sponsor, recipient_addr, 2000);
        assert!(get_unlock_time(sponsor_addr, recipient_addr) == 2000, 1);
        // Reduce lockup.
        update_lockup<AptosCoin>(sponsor, recipient_addr, 1500);
        assert!(get_unlock_time(sponsor_addr, recipient_addr) == 1500, 2);

        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient_1 = @0x234, recipient_2 = @0x345)]
    public entry fun test_sponsor_can_batch_update_lockup(
        aptos_framework: &signer, sponsor: &signer, recipient_1: &signer, recipient_2: &signer) acquires Locks {
        let sponsor_addr = signer::address_of(sponsor);
        let recipient_1_addr = signer::address_of(recipient_1);
        let recipient_2_addr = signer::address_of(recipient_2);
        aptos_account::create_account(recipient_1_addr);
        aptos_account::create_account(recipient_2_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        batch_add_locked_coins<AptosCoin>(sponsor, vector[recipient_1_addr, recipient_2_addr], vector[1000, 1000], 1000);
        assert!(get_unlock_time(sponsor_addr, recipient_1_addr) == 1000, 0);
        assert!(get_unlock_time(sponsor_addr, recipient_2_addr) == 1000, 0);
        // Extend lockup.
        batch_update_lockup<AptosCoin>(sponsor, vector[recipient_1_addr, recipient_2_addr], 2000);
        assert!(get_unlock_time(sponsor_addr, recipient_1_addr) == 2000, 1);
        assert!(get_unlock_time(sponsor_addr, recipient_2_addr) == 2000, 1);
        // Reduce lockup.
        batch_update_lockup<AptosCoin>(sponsor, vector[recipient_1_addr, recipient_2_addr], 1500);
        assert!(get_unlock_time(sponsor_addr, recipient_1_addr) == 1500, 2);
        assert!(get_unlock_time(sponsor_addr, recipient_2_addr) == 1500, 2);

        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient = @0x234)]
    public entry fun test_sponsor_can_cancel_lockup(
        aptos_framework: &signer, sponsor: &signer, recipient: &signer) acquires Locks {
        let recipient_addr = signer::address_of(recipient);
        aptos_account::create_account(recipient_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        add_locked_coins<AptosCoin>(sponsor, recipient_addr, 1000, 1000);
        cancel_lockup<AptosCoin>(sponsor, recipient_addr);
        let locks = borrow_global_mut<Locks<AptosCoin>>(signer::address_of(sponsor));
        assert!(!table::contains(&locks.locks, recipient_addr), 0);
        coin::destroy_burn_cap(burn_cap);
    }

    #[test(aptos_framework = @0x1, sponsor = @0x123, recipient_1 = @0x234, recipient_2 = @0x345)]
    public entry fun test_sponsor_can_batch_cancel_lockup(
        aptos_framework: &signer, sponsor: &signer, recipient_1: &signer, recipient_2: &signer) acquires Locks {
        let recipient_1_addr = signer::address_of(recipient_1);
        let recipient_2_addr = signer::address_of(recipient_2);
        aptos_account::create_account(recipient_1_addr);
        aptos_account::create_account(recipient_2_addr);
        let burn_cap = setup(aptos_framework, sponsor);
        batch_add_locked_coins<AptosCoin>(sponsor, vector[recipient_1_addr, recipient_2_addr], vector[1000, 1000], 1000);
        batch_cancel_lockup<AptosCoin>(sponsor, vector[recipient_1_addr, recipient_2_addr]);
        let locks = borrow_global_mut<Locks<AptosCoin>>(signer::address_of(sponsor));
        assert!(!table::contains(&locks.locks, recipient_1_addr), 0);
        assert!(!table::contains(&locks.locks, recipient_2_addr), 0);
        coin::destroy_burn_cap(burn_cap);
    }
}
