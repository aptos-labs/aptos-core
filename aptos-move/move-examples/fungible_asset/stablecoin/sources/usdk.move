/// Example of a managed stablecoin with minting allowances, confiscation, denylisting,
/// pausing and two-step role rotation.
///
/// See DESIGN.md in this package for the security model these functions implement.
module stablecoin::usdk {
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::event;
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata, FungibleAsset, FungibleStore};
    use aptos_framework::object::{Self, Object, ExtendRef, ObjectCore};
    use aptos_framework::primary_fungible_store;
    use aptos_std::table::{Self, Table};
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String, utf8};

    /// Caller is not authorized to make this call
    const EUNAUTHORIZED: u64 = 1;
    /// No value movement is allowed when the contract is paused
    const EPAUSED: u64 = 2;
    /// The account is already a minter
    const EALREADY_MINTER: u64 = 3;
    /// The account is not a minter
    const ENOT_MINTER: u64 = 4;
    /// The account is denylisted
    const EDENYLISTED: u64 = 5;
    /// Stablecoin is already initialized
    const EALREADY_INITIALIZED: u64 = 6;
    /// The account has no primary store for the stablecoin
    const ESTORE_NOT_FOUND: u64 = 7;
    /// The mint would exceed the minter's remaining allowance
    const EINSUFFICIENT_ALLOWANCE: u64 = 8;

    const ASSET_SYMBOL: vector<u8> = b"USDK";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Privileged addresses and the mint allowance of each authorized minter.
    ///
    /// Every role carries an optional pending successor so that rotation is a two-step
    /// handoff: a proposal alone confers no authority, so a mistyped address cannot
    /// strand a role.
    ///
    /// `mint_allowances` is keyed by address so that authorization and allowance lookup
    /// cost the same regardless of how many minters exist. Membership in this table *is*
    /// the minter role.
    struct Roles has key {
        master_minter: address,
        pending_master_minter: Option<address>,
        pauser: address,
        pending_pauser: Option<address>,
        denylister: address,
        pending_denylister: Option<address>,
        confiscator: address,
        pending_confiscator: Option<address>,
        mint_allowances: Table<address, u64>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Capabilities for managing the asset, held by the metadata object itself.
    struct Management has key {
        /// Unused today. Retained so a future upgrade can obtain the metadata object's
        /// signer (e.g. to add resources to the object) without recreating the asset.
        extend_ref: ExtendRef,
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Whether value movement is currently halted.
    struct State has key {
        paused: bool,
    }

    #[event]
    /// Emitted when new units are issued.
    struct Mint has drop, store {
        minter: address,
        to: address,
        amount: u64,
        remaining_allowance: u64,
    }

    #[event]
    /// Emitted when a minter destroys units from its own balance.
    struct Burn has drop, store {
        minter: address,
        amount: u64,
    }

    #[event]
    /// Emitted when the confiscator destroys units held by another account.
    struct Confiscate has drop, store {
        confiscator: address,
        from: address,
        store: Object<FungibleStore>,
        amount: u64,
    }

    #[event]
    /// Emitted when the pause state changes. Not emitted when the state is unchanged.
    struct Pause has drop, store {
        pauser: address,
        is_paused: bool,
    }

    #[event]
    /// Emitted when an account is denylisted or removed from the denylist.
    struct Denylist has drop, store {
        denylister: address,
        account: address,
        is_denylisted: bool,
    }

    #[event]
    /// Emitted when a minter is granted the role or has its allowance changed.
    struct MinterConfigured has drop, store {
        master_minter: address,
        minter: address,
        allowance: u64,
    }

    #[event]
    /// Emitted when a minter's authority is revoked.
    struct MinterRemoved has drop, store {
        master_minter: address,
        minter: address,
    }

    #[event]
    /// Emitted when the current holder of a role nominates a successor.
    struct RoleProposed has drop, store {
        role: String,
        current: address,
        proposed: address,
    }

    #[event]
    /// Emitted when a nominated successor takes over a role.
    struct RoleAccepted has drop, store {
        role: String,
        previous: address,
        current: address,
    }

    #[view]
    /// Return the deterministic address of the USDK metadata object.
    ///
    /// This is derived from the module address and is valid before initialization.
    public fun usdk_address(): address {
        object::create_object_address(&@stablecoin, ASSET_SYMBOL)
    }

    #[view]
    /// Return the USDK metadata object. Aborts if the stablecoin is not yet initialized.
    public fun metadata(): Object<Metadata> {
        object::address_to_object(usdk_address())
    }

    #[view]
    /// Return whether `initialize` has already run.
    ///
    /// Clients should check this before calling the other views, which read resources
    /// that only exist after initialization.
    public fun is_initialized(): bool {
        exists<Roles>(usdk_address())
    }

    #[view]
    /// Return whether value movement is currently halted.
    public fun is_paused(): bool acquires State {
        State[usdk_address()].paused
    }

    #[view]
    /// Return whether the account is authorized to mint.
    ///
    /// The master minter is not implicitly a minter: it must grant itself an allowance,
    /// which is an auditable on-chain event.
    public fun is_minter(account: address): bool acquires Roles {
        Roles[usdk_address()].mint_allowances.contains(account)
    }

    #[view]
    /// Return the account's remaining mint allowance, or zero if it is not a minter.
    public fun mint_allowance(account: address): u64 acquires Roles {
        let allowances = &Roles[usdk_address()].mint_allowances;
        if (allowances.contains(account)) *allowances.borrow(account) else 0
    }

    #[view]
    /// Return whether the account is denylisted.
    ///
    /// An account that has never held USDK has no primary store and is never denylisted.
    public fun is_denylisted(account: address): bool {
        primary_fungible_store::is_frozen(account, metadata())
    }

    #[view]
    /// Return the address that administers minters.
    public fun master_minter(): address acquires Roles {
        Roles[usdk_address()].master_minter
    }

    #[view]
    /// Return the address that may pause and unpause the stablecoin.
    public fun pauser(): address acquires Roles {
        Roles[usdk_address()].pauser
    }

    #[view]
    /// Return the address that may denylist accounts.
    public fun denylister(): address acquires Roles {
        Roles[usdk_address()].denylister
    }

    #[view]
    /// Return the address that may confiscate balances.
    public fun confiscator(): address acquires Roles {
        Roles[usdk_address()].confiscator
    }

    #[view]
    /// Return the pending master minter, if a rotation has been proposed.
    public fun pending_master_minter(): Option<address> acquires Roles {
        Roles[usdk_address()].pending_master_minter
    }

    #[view]
    /// Return the pending pauser, if a rotation has been proposed.
    public fun pending_pauser(): Option<address> acquires Roles {
        Roles[usdk_address()].pending_pauser
    }

    #[view]
    /// Return the pending denylister, if a rotation has been proposed.
    public fun pending_denylister(): Option<address> acquires Roles {
        Roles[usdk_address()].pending_denylister
    }

    #[view]
    /// Return the pending confiscator, if a rotation has been proposed.
    public fun pending_confiscator(): Option<address> acquires Roles {
        Roles[usdk_address()].pending_confiscator
    }

    /// Create the stablecoin. Must be called once, by the account that published the module.
    ///
    /// This is an explicit entry call rather than `init_module`, so a deployment is not
    /// usable until this transaction succeeds.
    ///
    /// Creates the fungible asset with primary store support, makes every store for the
    /// asset untransferable, stores the role/capability/state resources in the metadata
    /// object, and registers the module's `withdraw`/`deposit` hooks so that all transfers
    /// are subject to the pause and denylist checks.
    ///
    /// Aborts with `EALREADY_INITIALIZED` if it has already run, or `EUNAUTHORIZED` if the
    /// caller is not the module address.
    entry fun initialize(stablecoin_signer: &signer) {
        assert!(!object::object_exists<ObjectCore>(usdk_address()), EALREADY_INITIALIZED);
        authorize(signer::address_of(stablecoin_signer), @stablecoin);

        let constructor_ref = &object::create_named_object(stablecoin_signer, ASSET_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(), /* unlimited supply; issuance is bounded per minter instead */
            utf8(ASSET_SYMBOL), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            6, /* decimals, matching the convention used by USDC/USDT */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
        );

        // Prevent a store from being transferred out from under the denylist checks.
        fungible_asset::set_untransferable(constructor_ref);

        let metadata_object_signer = &constructor_ref.generate_signer();
        move_to(metadata_object_signer, Roles {
            master_minter: @master_minter,
            pending_master_minter: option::none(),
            pauser: @pauser,
            pending_pauser: option::none(),
            denylister: @denylister,
            pending_denylister: option::none(),
            confiscator: @confiscator,
            pending_confiscator: option::none(),
            mint_allowances: table::new(),
        });

        move_to(metadata_object_signer, Management {
            extend_ref: constructor_ref.generate_extend_ref(),
            mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
            burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
        });

        move_to(metadata_object_signer, State { paused: false });

        // Overriding deposit and withdraw overrides transfer, so every transfer runs the
        // checks in this module.
        let deposit = function_info::new_function_info(
            stablecoin_signer,
            string::utf8(b"usdk"),
            string::utf8(b"deposit"),
        );
        let withdraw = function_info::new_function_info(
            stablecoin_signer,
            string::utf8(b"usdk"),
            string::utf8(b"withdraw"),
        );
        dispatchable_fungible_asset::register_dispatch_functions(
            constructor_ref,
            option::some(withdraw),
            option::some(deposit),
            option::none(),
        );
    }

    /// Deposit hook. Rejects the transfer if the stablecoin is paused or the destination
    /// is denylisted.
    ///
    /// Registered via `register_dispatch_functions`; not called directly by this module.
    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
    ) acquires State {
        assert_not_paused();
        assert_store_not_denylisted(store);
        transfer_ref.deposit_with_ref(store, fa);
    }

    /// Withdraw hook. Rejects the transfer if the stablecoin is paused or the source is
    /// denylisted.
    ///
    /// Checking the root owner as well as the direct owner is what stops a denylisted
    /// account from moving funds through a store owned by an intermediate object it
    /// controls: the framework's own check accepts indirect ownership.
    ///
    /// Registered via `register_dispatch_functions`; not called directly by this module.
    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset acquires State {
        assert_not_paused();
        assert_store_not_denylisted(store);
        transfer_ref.withdraw_with_ref(store, amount)
    }

    /// Issue `amount` units to `to`, drawing on the caller's mint allowance.
    ///
    /// Aborts if paused, if the caller is not a minter, if `to` is denylisted, or if the
    /// amount exceeds the caller's remaining allowance. Minting zero is a no-op.
    public entry fun mint(minter: &signer, to: address, amount: u64) acquires Management, Roles, State {
        assert_not_paused();
        let minter_address = signer::address_of(minter);
        assert_is_minter(minter_address);
        assert_not_denylisted(to);
        if (amount == 0) { return };

        // Debit the allowance first so that a failure later cannot issue for free.
        let allowance = Roles[usdk_address()].mint_allowances.borrow_mut(minter_address);
        assert!(*allowance >= amount, EINSUFFICIENT_ALLOWANCE);
        *allowance -= amount;
        let remaining_allowance = *allowance;

        let management = &Management[usdk_address()];
        let tokens = management.mint_ref.mint(amount);
        // The pause and denylist checks above already cover this deposit, so deposit with
        // the ref directly rather than re-entering the dispatch hook.
        management.transfer_ref.deposit_with_ref(
            primary_fungible_store::ensure_primary_store_exists(to, metadata()),
            tokens,
        );

        event::emit(Mint { minter: minter_address, to, amount, remaining_allowance });
    }

    /// Destroy `amount` units from the caller's own balance.
    ///
    /// Minters can only burn what they hold; destroying another account's balance requires
    /// the confiscator role. Aborts if paused, if the caller is not a minter, or if the
    /// caller has no primary store. Burning zero is a no-op.
    public entry fun burn(minter: &signer, amount: u64) acquires Management, Roles, State {
        assert_not_paused();
        let minter_address = signer::address_of(minter);
        assert_is_minter(minter_address);
        if (amount == 0) { return };

        let metadata = metadata();
        assert!(primary_fungible_store::primary_store_exists(minter_address, metadata), ESTORE_NOT_FOUND);

        let management = &Management[usdk_address()];
        let store = primary_fungible_store::primary_store(minter_address, metadata);
        let tokens = management.transfer_ref.withdraw_with_ref(store, amount);
        management.burn_ref.burn(tokens);

        event::emit(Burn { minter: minter_address, amount });
    }

    /// Destroy `amount` units held in another account's primary store.
    ///
    /// This is the compliance seizure path and deliberately bypasses the frozen flag and
    /// the pause, so that funds can be recovered from a denylisted account during an
    /// incident. Restricted to the confiscator role. Aborts if the account has no primary
    /// store. Confiscating zero is a no-op.
    public entry fun confiscate(confiscator: &signer, from: address, amount: u64) acquires Management, Roles {
        let confiscator_address = signer::address_of(confiscator);
        authorize(confiscator_address, Roles[usdk_address()].confiscator);
        if (amount == 0) { return };

        let metadata = metadata();
        assert!(primary_fungible_store::primary_store_exists(from, metadata), ESTORE_NOT_FOUND);

        let management = &Management[usdk_address()];
        let store = primary_fungible_store::primary_store(from, metadata);
        let tokens = management.transfer_ref.withdraw_with_ref(store, amount);
        management.burn_ref.burn(tokens);

        event::emit(Confiscate { confiscator: confiscator_address, from, store, amount });
    }

    /// Halt or resume value movement. Restricted to the pauser.
    ///
    /// Setting the state to its current value is a no-op and emits no event.
    public entry fun set_pause(pauser: &signer, paused: bool) acquires Roles, State {
        let usdk_address = usdk_address();
        let pauser_address = signer::address_of(pauser);
        authorize(pauser_address, Roles[usdk_address].pauser);

        let state = &mut State[usdk_address];
        if (state.paused == paused) { return };
        state.paused = paused;

        event::emit(Pause { pauser: pauser_address, is_paused: paused });
    }

    /// Freeze an account, blocking it from sending or receiving USDK.
    ///
    /// Available while paused: denylisting is part of responding to an incident.
    public entry fun denylist(denylister: &signer, account: address) acquires Management, Roles {
        set_denylist(denylister, account, true);
    }

    /// Unfreeze an account. Available while paused.
    public entry fun undenylist(denylister: &signer, account: address) acquires Management, Roles {
        set_denylist(denylister, account, false);
    }

    /// Grant the minter role with a finite allowance. Restricted to the master minter.
    ///
    /// Blocked while paused: a compromised master-minter key must not be able to expand
    /// issuing authority during an incident. Aborts with `EALREADY_MINTER` if the account
    /// already holds the role; use `set_mint_allowance` to change an existing allowance.
    public entry fun add_minter(admin: &signer, minter: address, allowance: u64) acquires Roles, State {
        assert_not_paused();
        let admin_address = signer::address_of(admin);
        let roles = &mut Roles[usdk_address()];
        authorize(admin_address, roles.master_minter);
        assert!(!roles.mint_allowances.contains(minter), EALREADY_MINTER);
        roles.mint_allowances.add(minter, allowance);

        event::emit(MinterConfigured { master_minter: admin_address, minter, allowance });
    }

    /// Change an existing minter's remaining allowance. Restricted to the master minter.
    ///
    /// Increases are blocked while paused; decreases remain available so that issuing
    /// authority can be wound down during an incident. Aborts with `ENOT_MINTER` if the
    /// account does not hold the role.
    public entry fun set_mint_allowance(admin: &signer, minter: address, allowance: u64) acquires Roles, State {
        let admin_address = signer::address_of(admin);
        let roles = &mut Roles[usdk_address()];
        authorize(admin_address, roles.master_minter);
        assert!(roles.mint_allowances.contains(minter), ENOT_MINTER);

        let current = *roles.mint_allowances.borrow(minter);
        if (allowance > current) {
            assert_not_paused();
        };
        roles.mint_allowances.upsert(minter, allowance);

        event::emit(MinterConfigured { master_minter: admin_address, minter, allowance });
    }

    /// Revoke the minter role and its remaining allowance. Restricted to the master minter.
    ///
    /// Available while paused: revoking a compromised minter is part of responding to an
    /// incident. Revoking an account that is not a minter is a no-op.
    public entry fun remove_minter(admin: &signer, minter: address) acquires Roles {
        let admin_address = signer::address_of(admin);
        let roles = &mut Roles[usdk_address()];
        authorize(admin_address, roles.master_minter);
        if (!roles.mint_allowances.contains(minter)) { return };
        roles.mint_allowances.remove(minter);

        event::emit(MinterRemoved { master_minter: admin_address, minter });
    }

    /// Nominate a successor for the master minter role. Restricted to the current holder.
    ///
    /// The nomination confers no authority until `accept_master_minter` is called, and may
    /// be replaced by a later proposal.
    public entry fun propose_master_minter(admin: &signer, proposed: address) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let current = roles.master_minter;
        authorize(signer::address_of(admin), current);
        roles.pending_master_minter = option::some(proposed);

        event::emit(RoleProposed { role: utf8(b"master_minter"), current, proposed });
    }

    /// Take over the master minter role. Restricted to the nominated successor.
    public entry fun accept_master_minter(proposed: &signer) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let new_holder = accept_role(proposed, &mut roles.pending_master_minter);
        let previous = roles.master_minter;
        roles.master_minter = new_holder;

        event::emit(RoleAccepted { role: utf8(b"master_minter"), previous, current: new_holder });
    }

    /// Nominate a successor for the pauser role. Restricted to the current holder.
    public entry fun propose_pauser(pauser: &signer, proposed: address) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let current = roles.pauser;
        authorize(signer::address_of(pauser), current);
        roles.pending_pauser = option::some(proposed);

        event::emit(RoleProposed { role: utf8(b"pauser"), current, proposed });
    }

    /// Take over the pauser role. Restricted to the nominated successor.
    public entry fun accept_pauser(proposed: &signer) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let new_holder = accept_role(proposed, &mut roles.pending_pauser);
        let previous = roles.pauser;
        roles.pauser = new_holder;

        event::emit(RoleAccepted { role: utf8(b"pauser"), previous, current: new_holder });
    }

    /// Nominate a successor for the denylister role. Restricted to the current holder.
    public entry fun propose_denylister(denylister: &signer, proposed: address) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let current = roles.denylister;
        authorize(signer::address_of(denylister), current);
        roles.pending_denylister = option::some(proposed);

        event::emit(RoleProposed { role: utf8(b"denylister"), current, proposed });
    }

    /// Take over the denylister role. Restricted to the nominated successor.
    public entry fun accept_denylister(proposed: &signer) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let new_holder = accept_role(proposed, &mut roles.pending_denylister);
        let previous = roles.denylister;
        roles.denylister = new_holder;

        event::emit(RoleAccepted { role: utf8(b"denylister"), previous, current: new_holder });
    }

    /// Nominate a successor for the confiscator role. Restricted to the current holder.
    public entry fun propose_confiscator(confiscator: &signer, proposed: address) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let current = roles.confiscator;
        authorize(signer::address_of(confiscator), current);
        roles.pending_confiscator = option::some(proposed);

        event::emit(RoleProposed { role: utf8(b"confiscator"), current, proposed });
    }

    /// Take over the confiscator role. Restricted to the nominated successor.
    public entry fun accept_confiscator(proposed: &signer) acquires Roles {
        let roles = &mut Roles[usdk_address()];
        let new_holder = accept_role(proposed, &mut roles.pending_confiscator);
        let previous = roles.confiscator;
        roles.confiscator = new_holder;

        event::emit(RoleAccepted { role: utf8(b"confiscator"), previous, current: new_holder });
    }

    /// Set or clear an account's frozen flag and emit the corresponding event.
    ///
    /// Skips the write when the account is already in the requested state, and avoids
    /// creating a primary store just to unfreeze an account that never held USDK.
    fun set_denylist(denylister: &signer, account: address, denied: bool) acquires Management, Roles {
        let usdk_address = usdk_address();
        let denylister_address = signer::address_of(denylister);
        authorize(denylister_address, Roles[usdk_address].denylister);

        let metadata = metadata();
        let store_exists = primary_fungible_store::primary_store_exists(account, metadata);
        if (!denied && !store_exists) { return };
        if (store_exists && primary_fungible_store::is_frozen(account, metadata) == denied) { return };

        let transfer_ref = &Management[usdk_address].transfer_ref;
        primary_fungible_store::set_frozen_flag(transfer_ref, account, denied);

        event::emit(Denylist {
            denylister: denylister_address,
            account,
            is_denylisted: denied,
        });
    }

    /// Consume a pending role nomination, returning the accepting address.
    ///
    /// Aborts with `EUNAUTHORIZED` if there is no nomination or the caller is not the
    /// nominated address.
    inline fun accept_role(proposed: &signer, pending: &mut Option<address>): address {
        let proposed_address = signer::address_of(proposed);
        assert!(pending.contains(&proposed_address), EUNAUTHORIZED);
        *pending = option::none();
        proposed_address
    }

    /// Abort unless the account currently holds the minter role.
    inline fun assert_is_minter(account: address) {
        assert!(Roles[usdk_address()].mint_allowances.contains(account), EUNAUTHORIZED);
    }

    /// Abort if value movement is currently halted.
    inline fun assert_not_paused() {
        assert!(!State[usdk_address()].paused, EPAUSED);
    }

    /// Abort if either the direct owner or the root owner of the store is denylisted.
    ///
    /// The two are the same for an ordinary primary store, so the second lookup is skipped
    /// in the common case.
    inline fun assert_store_not_denylisted<T: key>(store: Object<T>) {
        let owner = store.owner();
        assert_not_denylisted(owner);
        let root_owner = store.root_owner();
        if (root_owner != owner) {
            assert_not_denylisted(root_owner);
        };
    }

    /// Abort if the account's primary store is frozen.
    ///
    /// An account with no primary store cannot be denylisted and is therefore allowed.
    inline fun assert_not_denylisted(account: address) {
        let metadata = metadata();
        // Must not call pfs::store_exists from the withdraw/deposit hooks: that would risk
        // a circular module dependency. Use the inlined variants instead.
        if (primary_fungible_store::primary_store_exists_inlined(account, metadata)) {
            assert!(
                !fungible_asset::is_frozen(primary_fungible_store::primary_store_inlined(account, metadata)),
                EDENYLISTED
            );
        }
    }

    /// Abort unless the caller is the expected address.
    inline fun authorize(caller: address, expected: address) {
        assert!(caller == expected, EUNAUTHORIZED);
    }

    #[test_only]
    /// Run `initialize` from a test, which cannot call an entry function directly.
    public fun init_for_test(usdk_signer: &signer) {
        initialize(usdk_signer);
    }
}
