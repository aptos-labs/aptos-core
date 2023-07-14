/// This is an example module that shows a lockup-based voting system. The flow works as below:
///
/// 1. Users can lock up $VOTING to receive a voting certificate in exchange. The voting certificate is an object that
/// can be freely transferred. The longer the lockup duration, the higher the voting power a certificate will have.
/// Over time, as remaining lockup duration decreases, the certiticate's voting power will also decrease linearly.
/// Users get 100% of voting power (equal to amount of $VOTING locked) when locking for the max lockup duration. If they
/// lock for half the max lockup duration, they get 50% of voting power, and so on. Developers can tweak this formula
/// if needed to achieve a different multiplier. Note that voting power only decreases at the end of each epoch (duration
/// can be customized) instead of continuously over time.
/// 2. Users can vote by calling the rest of the system, which can check how much voting power the certificate currently
/// has remaining and what the current total voting power is across all certificates. This allows the system to calculate
/// the percentage of voting power a certificate has and use that to determine the weight of the vote.
/// 3. Users can also extend the lockup of a certificate or add more $VOTING to it. In this case, the voting power will
/// be recalculated based on the new lockup duration and amount.
/// 4. Once the lockup has expired, the users can withdraw their $VOTING from the certificate. This also destroys the
/// certificate.
///
/// Note that the voting power cannot be changed for the current epoch when locked amount or lockup duration changes.
/// If this is a desired behavior, developers can update the code to allow this.
///
/// Calculating the remaining voting power of a certificate is simple but calculating the total voting power across all
/// certificates is more complicated as they can all have different lockup durations and amounts. To solve this, this
/// module tracks and updates the total voting power of all certificates all the way until the current time + max lockup
/// duration. This operation can result in a max of 365 steps, which is why we use a SmartTable to optimize gas. This
/// approach is usually prohibitively expensive on other chains, but on Aptos, this is cheap and fast.
module vote_lockup::vote {
    use aptos_framework::fungible_asset::{Self, FungibleAsset};
    use aptos_framework::object::{Self, DeleteRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_std::smart_table::{Self, SmartTable};
    use std::signer;
    use vote_lockup::epoch;
    use vote_lockup::package_manager;
    use vote_lockup::voting_token;

    // 1 epoch is 1 day.
    const MIN_LOCKUP_EPOCHS: u64 = 24;
    const MAX_LOCKUP_EPOCHS: u64 = 104;

    /// Only $VOTING are accepted.
    const EONLY_VOTING_ACCEPTED: u64 = 1;
    /// The given lockup period is shorter than the minimum allowed.
    const ELOCKUP_TOO_SHORT: u64 = 2;
    /// The given lockup period is longer than the maximum allowed.
    const ELOCKUP_TOO_LONG: u64 = 3;
    /// The given certificate is not owned by the given signer.
    const ENOT_CERTIFICATE_OWNER: u64 = 4;
    /// The lockup period for the given certificate has not expired yet.
    const ELOCKUP_HAS_NOT_EXPIRED: u64 = 5;
    /// Either locked amount or lockup duration or both has to increase.
    const EINVALID_LOCKUP_CHANGE: u64 = 6;
    /// The new lockup period has to be strictly longer than the old one.
    const ELOCKUP_MUST_BE_EXTENDED: u64 = 7;
    /// The amount to lockup must be more than zero.
    const EINVALID_AMOUNT: u64 = 8;
    /// Voting power and total supply can only be looked up for the current epoch or in the future.
    const ECANNOT_LOOK_UP_PAST_VOTING_POWER: u64 = 9;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Core resource for a voting certificate that stores locked $VOTING and allows users to vote with a voting power
    /// that increases the higher the remaining lockup time (decreasing over time if lockup is not extended).
    struct VotingCertificate has key {
        locked_amount: u64,
        end_epoch: u64,
        // Required to destroy the certificate later during withdrawal.
        delete_ref: DeleteRef,
    }

    /// Important data structure that tracks the total voting power across all certificates. This is updated for every
    /// certificate creation or lockup/amount changes.
    struct VoteConfig has key {
        // This is the total voting power across all voting certificates, multiplied by max_lockup_epochs to minimize
        // rounding error.
        // Total voting power is computed for each epoch until the maximum number of epochs allowed from the last
        // user's lockup update. If there's no value, the supply is zero because all lockups have already expired.
        // We store this as a SmartTable to optimize gas.
        unscaled_total_voting_power_per_epoch: SmartTable<u64, u128>,
    }

    public entry fun initialize() {
        if (is_initialized()) {
            return
        };
        voting_token::initialize();
        move_to(&package_manager::get_signer(), VoteConfig {
            unscaled_total_voting_power_per_epoch: smart_table::new(),
        });
    }

    #[view]
    public fun is_initialized(): bool {
        exists<VoteConfig>(@vote_lockup)
    }

    #[view]
    /// Returns the current remaining voting power of the given certificate.
    public fun get_voting_power(certificate: Object<VotingCertificate>): u64 acquires VotingCertificate {
        get_voting_power_at_epoch(certificate, epoch::now())
    }

    #[view]
    /// Returns the remaining voting power of the given certificate at the given epoch.
    public fun get_voting_power_at_epoch(certificate: Object<VotingCertificate>, epoch: u64): u64 acquires VotingCertificate {
        assert!(epoch >= epoch::now(), ECANNOT_LOOK_UP_PAST_VOTING_POWER);
        let token_data = safe_certificate(&certificate);
        let lockup_end_epoch = token_data.end_epoch;
        if (lockup_end_epoch <= epoch) {
            0
        } else {
            token_data.locked_amount * (lockup_end_epoch - epoch) / MAX_LOCKUP_EPOCHS
        }
    }

    #[view]
    /// Returns the current total voting power across all certificates.
    public fun total_voting_power(): u128 acquires VoteConfig {
        total_voting_power_at(epoch::now())
    }

    #[view]
    /// Returns the total voting power across all certificates at the given epoch.
    public fun total_voting_power_at(epoch: u64): u128 acquires VoteConfig {
        assert!(epoch >= epoch::now(), ECANNOT_LOOK_UP_PAST_VOTING_POWER);
        let total_voting_power_per_epoch = &safe_vote_config().unscaled_total_voting_power_per_epoch;
        let unscaled_voting_power = *smart_table::borrow_with_default(total_voting_power_per_epoch, epoch, &0);
        unscaled_voting_power / (MAX_LOCKUP_EPOCHS as u128)
    }

    #[view]
    /// Returns the number of epochs until the given certificate's lockup expires.
    public fun remaining_lockup_epochs(certificate: Object<VotingCertificate>): u64 acquires VotingCertificate {
        let end_epoch = get_lockup_expiration_epoch(certificate);
        let current_epoch = epoch::now();
        if (end_epoch <= current_epoch) {
            0
        } else {
            end_epoch - current_epoch
        }
    }

    #[view]
    /// Returns the epoch number when the given certificate's lockup expires.
    public fun get_lockup_expiration_epoch(certificate: Object<VotingCertificate>): u64 acquires VotingCertificate {
        safe_certificate(&certificate).end_epoch
    }

    #[view]
    /// Return the timestamp (in seconds) when the given certificate's lockup expires.
    public fun get_lockup_expiration_time(certificate: Object<VotingCertificate>): u64 acquires VotingCertificate {
        epoch::to_seconds(get_lockup_expiration_epoch(certificate))
    }

    #[view]
    /// Return true if a given address is a certificate object.
    public fun certificate_exists(certificate: address): bool {
        exists<VotingCertificate>(certificate)
    }

    #[view]
    /// Return the maximum number of epochs a lockup can be created for.
    public fun max_lockup_epochs(): u64 {
        MAX_LOCKUP_EPOCHS
    }

    /// Mint a voting certificate and lock $VOTING from the owner's primary store.
    public entry fun create_lock_entry(owner: &signer, amount: u64, lockup_epochs: u64) acquires VoteConfig {
        create_lock(owner, amount, lockup_epochs);
    }

    public entry fun create_lock_for(
        owner: &signer,
        amount: u64,
        lockup_epochs: u64,
        recipient: address,
    ) acquires VoteConfig {
        let voting_tokens = primary_fungible_store::withdraw(owner, voting_token::token(), amount);
        create_lock_with(voting_tokens, lockup_epochs, recipient);
    }

    /// Non-entry version that also returns a reference to the certificate object.
    public fun create_lock(
        owner: &signer,
        amount: u64,
        lockup_epochs: u64,
    ): Object<VotingCertificate> acquires VoteConfig {
        let voting_tokens = primary_fungible_store::withdraw(owner, voting_token::token(), amount);
        create_lock_with(voting_tokens, lockup_epochs, signer::address_of(owner))
    }

    /// Create a lock with the given amount and lockup duration and send the certificate to the given recipient.
    public fun create_lock_with(
        tokens: FungibleAsset,
        lockup_epochs: u64,
        recipient: address,
    ): Object<VotingCertificate> acquires VoteConfig {
        let amount = fungible_asset::amount(&tokens);
        assert!(amount > 0, EINVALID_AMOUNT);

        validate_lockup_epochs(lockup_epochs);
        let voting_token_metadata = voting_token::token();
        assert!(
            fungible_asset::asset_metadata(&tokens) == object::convert(voting_token_metadata),
            EONLY_VOTING_ACCEPTED,
        );

        // Create a new certificate.
        let package_signer = &package_manager::get_signer();
        let certificate = &object::create_object_from_account(package_signer);
        let certificate_signer = &object::generate_signer(certificate);
        let lockup_end_epoch = epoch::now() + lockup_epochs;
        move_to(certificate_signer, VotingCertificate {
            locked_amount: amount,
            end_epoch: lockup_end_epoch,
            delete_ref: object::generate_delete_ref(certificate),
        });

        // Turn the certificate into a fungible store so we can store the locked up $VOTING there.
        let certificate_as_store = fungible_asset::create_store(certificate, voting_token_metadata);
        fungible_asset::deposit(certificate_as_store, tokens);
        // Disable owner transfers to lock the $VOTING inside the certificate so it cannot be moved until the lockup
        // has expired.
        // This also prevents anyone from adding more $VOTING into the certificate token without going through the flow
        // here.
        voting_token::disable_transfer(certificate_as_store);

        // Send the certificate to the recipient.
        object::transfer(package_signer, certificate_as_store, recipient);

        // Has to called for every function that modifies amount or lockup duration of any voting certificates.
        // Always at the end of a function so we don't forget.
        // Old amount is 0 because this is a new lockup.
        update_manifested_total_supply(0, 0, amount, lockup_end_epoch);

        object::object_from_constructor_ref(certificate)
    }

    /// Increase the lockup duration of a voting certificate by the given number of epochs. The new effective new lockup
    /// end epoch would be current epoch + lockup epochs from now.
    /// This can also be called for a voting certificate that has already expired to re-lock it.
    public entry fun extend_lockup(
        owner: &signer,
        certificate: Object<VotingCertificate>,
        lockup_epochs_from_now: u64,
    ) acquires VoteConfig, VotingCertificate {
        validate_lockup_epochs(lockup_epochs_from_now);
        let certificate_data = owner_only_mut_certificate(owner, certificate);
        let old_lockup_end_epoch = certificate_data.end_epoch;
        let new_lockup_end_epoch = epoch::now() + lockup_epochs_from_now;
        assert!(new_lockup_end_epoch > old_lockup_end_epoch, ELOCKUP_MUST_BE_EXTENDED);
        certificate_data.end_epoch = new_lockup_end_epoch;
        // Amount didn't change.
        let locked_amount = certificate_data.locked_amount;

        // Has to called for every function that modifies amount or lockup duration of any voting certificate.
        // Always at the end of a function so we don't forget.
        update_manifested_total_supply(locked_amount, old_lockup_end_epoch, locked_amount, new_lockup_end_epoch);
    }

    /// Add more $VOTING to a voting certificate.
    public entry fun increase_amount(
        owner: &signer,
        certificate: Object<VotingCertificate>,
        amount: u64,
    ) acquires VoteConfig, VotingCertificate {
        let voting_tokens = primary_fungible_store::withdraw(owner, voting_token::token(), amount);
        increase_amount_with(certificate, voting_tokens);
    }

    /// Add more $VOTING to a voting certificate.
    public fun increase_amount_with(
        certificate: Object<VotingCertificate>,
        tokens: FungibleAsset,
    ) acquires VoteConfig, VotingCertificate {
        let certificate_data = unchecked_mut_certificate(&certificate);
        let amount = fungible_asset::amount(&tokens);
        assert!(amount > 0, EINVALID_AMOUNT);
        let old_amount = certificate_data.locked_amount;
        let new_amount = old_amount + amount;
        certificate_data.locked_amount = new_amount;
        voting_token::deposit(certificate, tokens);

        // Has to called for every function that modifies amount or lockup duration of any voting certificate.
        // Always at the end of a function so we don't forget.
        let end_epoch = certificate_data.end_epoch;
        update_manifested_total_supply(old_amount, end_epoch, new_amount, end_epoch);
    }

    /// Withdraw the $VOTING from a voting certificate. The certificate must have expired.
    public entry fun withdraw_entry(
        owner: &signer,
        certificate: Object<VotingCertificate>,
    ) acquires VotingCertificate {
        let assets = withdraw(owner, certificate);
        primary_fungible_store::deposit(signer::address_of(owner), assets);
    }

    /// Withdraw the $VOTING from a voting certificate. The certificate must have expired.
    public fun withdraw(
        owner: &signer,
        certificate: Object<VotingCertificate>,
    ): FungibleAsset acquires VotingCertificate {
        // Extract the unlocked $VOTING and burn the ve token.
        let tokens = voting_token::withdraw(certificate, fungible_asset::balance(certificate));
        let end_epoch = owner_only_destroy_certificate(owner, certificate);
        // This would fail if the lockup has not expired yet.
        assert!(end_epoch <= epoch::now(), ELOCKUP_HAS_NOT_EXPIRED);
        tokens
        // Withdraw doesn't need to update total voting power because this lockup should not have any effect on any
        // epochs, including the current one, as it has already expired.
    }

    fun update_manifested_total_supply(
        old_amount: u64,
        old_lockup_end_epoch: u64,
        new_amount: u64,
        new_lockup_end_epoch: u64,
    ) acquires VoteConfig {
        assert!(
            new_amount > old_amount || new_lockup_end_epoch > old_lockup_end_epoch,
            EINVALID_LOCKUP_CHANGE,
        );

        // We only need to update the total supply starting from the current epoch since the total voting powers of
        // past epochs are already set in stone.
        let curr_epoch = epoch::now();
        let total_voting_power_per_epoch = &mut unchecked_vote_config().unscaled_total_voting_power_per_epoch;
        while (curr_epoch < new_lockup_end_epoch) {
            // Old epoch delta can be zero if there was no previous lockup (old_amount = 0) or lockup has expired.
            let old_epoch_delta = if (old_amount == 0 || old_lockup_end_epoch <= curr_epoch) {
                0
            } else {
                old_amount * (old_lockup_end_epoch - curr_epoch)
            };
            let new_epoch_delta = new_amount * (new_lockup_end_epoch - curr_epoch);
            // This cannot underflow due to the assertion that either the amount or the lockup duration or both must
            // increase.
            let voting_power_delta = ((new_epoch_delta - old_epoch_delta) as u128);
            let total_voting_power = smart_table::borrow_mut_with_default(total_voting_power_per_epoch, curr_epoch, 0);
            *total_voting_power = *total_voting_power + voting_power_delta;
            curr_epoch = curr_epoch + 1;
        }
    }

    inline fun owner_only_destroy_certificate(
        owner: &signer,
        certificate: Object<VotingCertificate>,
    ): u64 acquires VotingCertificate {
        assert!(object::is_owner(certificate, signer::address_of(owner)), ENOT_CERTIFICATE_OWNER);
        let certificate_addr = object::object_address(&certificate);
        let VotingCertificate { locked_amount: _, end_epoch, delete_ref } =
            move_from<VotingCertificate>(certificate_addr);
        object::delete(delete_ref);
        end_epoch
    }

    inline fun owner_only_mut_certificate(
        owner: &signer,
        certificate: Object<VotingCertificate>,
    ): &mut VotingCertificate acquires VotingCertificate {
        assert!(object::is_owner(certificate, signer::address_of(owner)), ENOT_CERTIFICATE_OWNER);
        unchecked_mut_certificate(&certificate)
    }

    inline fun safe_certificate(certificate: &Object<VotingCertificate>): &VotingCertificate acquires VotingCertificate {
        borrow_global<VotingCertificate>(object::object_address(certificate))
    }

    inline fun safe_vote_config(): &VoteConfig {
        borrow_global<VoteConfig>(@vote_lockup)
    }

    inline fun unchecked_mut_certificate(certificate: &Object<VotingCertificate>): &mut VotingCertificate acquires VotingCertificate {
        borrow_global_mut<VotingCertificate>(object::object_address(certificate))
    }

    inline fun unchecked_vote_config(): &mut VoteConfig {
        borrow_global_mut<VoteConfig>(@vote_lockup)
    }

    inline fun validate_lockup_epochs(lockup_epochs: u64) {
        assert!(lockup_epochs >= MIN_LOCKUP_EPOCHS, ELOCKUP_TOO_SHORT);
        assert!(lockup_epochs <= MAX_LOCKUP_EPOCHS, ELOCKUP_TOO_LONG);
    }
}
