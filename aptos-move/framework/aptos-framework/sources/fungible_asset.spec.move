spec aptos_framework::fungible_asset {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The metadata associated with the fungible asset is subject to precise size constraints.
    /// Criticality: Medium
    /// Implementation: The add_fungibility function has size limitations for the name, symbol, number of decimals,
    /// icon_uri, and project_uri field of the Metadata resource.
    /// Enforcement: Formally verified via [high-level-req-1](add_fungibility).
    ///
    /// No.: 2
    /// Requirement: Adding fungibility to an existing object should initialize the metadata and supply resources and store
    /// them under the metadata object address.
    /// Criticality: Low
    /// Implementation: The add_fungibility function initializes the Metadata and Supply resources and moves them under
    /// the metadata object.
    /// Enforcement: Audited that the Metadata and Supply resources are initialized properly.
    ///
    /// No.: 3
    /// Requirement: Generating mint, burn and transfer references can only be done at object creation time and if the
    /// object was added fungibility.
    /// Criticality: Low
    /// Implementation: The following functions generate the related references of the Metadata object: 1.
    /// generate_mint_ref 2. generate_burn_ref 3. generate_transfer_ref
    /// Enforcement: Audited that the Metadata object exists within the constructor ref.
    ///
    /// No.: 4
    /// Requirement: Only the owner of a store should be allowed to withdraw fungible assets from it.
    /// Criticality: High
    /// Implementation: The fungible_asset::withdraw function ensures that the signer owns the store by asserting that
    /// the object address matches the address of the signer.
    /// Enforcement: Audited that the address of the signer owns the object.
    ///
    /// No.: 5
    /// Requirement: The transfer, withdrawal and deposit operation should never change the current supply of the fungible
    /// asset.
    /// Criticality: High
    /// Implementation: The transfer function withdraws the fungible assets from the store and deposits them to the
    /// receiver. The withdraw function extracts the fungible asset from the fungible asset store. The deposit function
    /// adds the balance to the fungible asset store.
    /// Enforcement: Audited that the supply before and after the operation remains constant.
    ///
    /// No.: 6
    /// Requirement: The owner of the store should only be able to withdraw a certain amount if its store has sufficient
    /// balance and is not frozen, unless the withdrawal is performed with a reference, and afterwards the store balance
    /// should be decreased.
    /// Criticality: High
    /// Implementation: The withdraw function ensures that the store is not frozen before calling withdraw_internal
    /// which ensures that the withdrawing amount is greater than 0 and less than the total balance from the store.
    /// The withdraw_with_ref ensures that the reference's metadata matches the store metadata.
    /// Enforcement: Audited that it aborts if the withdrawing store is frozen. Audited that it aborts if the store doesn't have sufficient balance. Audited that the balance of the withdrawing store is reduced by amount.
    ///
    /// No.: 7
    /// Requirement: Only the same type of fungible assets should be deposited in a fungible asset store, if the store is
    /// not frozen, unless the deposit is performed with a reference, and afterwards the store balance should be
    /// increased.
    /// Criticality: High
    /// Implementation: The deposit function ensures that store is not frozen and proceeds to call the deposit_internal
    /// function which validates the store's metadata and the depositing asset's metadata followed by increasing the
    /// store balance by the given amount. The deposit_with_ref ensures that the reference's metadata matches the
    /// depositing asset's metadata.
    /// Enforcement: Audited that it aborts if the store is frozen. Audited that it aborts if the asset and asset store are different. Audited that the store's balance is increased by the deposited amount.
    ///
    /// No.: 8
    /// Requirement: An object should only be allowed to hold one store for fungible assets.
    /// Criticality: Medium
    /// Implementation: The create_store function initializes a new FungibleStore resource and moves it under the
    /// object address.
    /// Enforcement: Formally verified via [high-level-req-8](create_store).
    ///
    /// No.: 9
    /// Requirement: When a new store is created, the balance should be set by default to the value zero.
    /// Criticality: High
    /// Implementation: The create_store function initializes a new fungible asset store with zero balance and stores it
    /// under the given construtorRef object.
    /// Enforcement: Formally verified via [high-level-req-9](create_store).
    ///
    /// No.: 10
    /// Requirement: A store should only be deleted if its balance is zero.
    /// Criticality: Medium
    /// Implementation: The remove_store function validates the store's balance and removes the store under the object
    /// address.
    /// Enforcement: Formally verified via [high-level-req-10](remove_store).
    ///
    /// No.: 11
    /// Requirement: Minting and burning should alter the total supply value, and the store balances.
    /// Criticality: High
    /// Implementation: The mint process increases the total supply by the amount minted using the increase_supply
    /// function. The burn process withdraws the burn amount from the given store and decreases the total supply by the
    /// amount burned using the decrease_supply function.
    /// Enforcement: Audited the mint and burn functions that the supply was adjusted accordingly.
    ///
    /// No.: 12
    /// Requirement: It must not be possible to burn an amount of fungible assets larger than their current supply.
    /// Criticality: High
    /// Implementation: The burn process ensures that the store has enough balance to burn, by asserting that the
    /// supply.current >= amount inside the decrease_supply function.
    /// Enforcement: Audited that it aborts if the provided store doesn't have sufficient balance.
    ///
    /// No.: 13
    /// Requirement: Enabling or disabling store's frozen status should only be done with a valid transfer reference.
    /// Criticality: High
    /// Implementation: The set_frozen_flag function ensures that the TransferRef is provided via function argument and
    /// that the store's metadata matches the metadata from the reference. It then proceeds to update the frozen flag of
    /// the store.
    /// Enforcement: Audited that it aborts if the metadata doesn't match. Audited that the frozen flag is updated properly.
    ///
    /// No.: 14
    /// Requirement: Extracting a specific amount from the fungible asset should be possible only if the total amount that
    /// it holds is greater or equal to the provided amount.
    /// Criticality: High
    /// Implementation: The extract function validates that the fungible asset has enough balance to extract and then
    /// updates it by subtracting the extracted amount.
    /// Enforcement: Formally verified via [high-level-req-14](extract).
    ///
    /// No.: 15
    /// Requirement: Merging two fungible assets should only be possible if both share the same metadata.
    /// Criticality: Medium
    /// Implementation: The merge function validates the metadata of the src and dst asset.
    /// Enforcement: Formally verified via [high-level-req-15](merge).
    ///
    /// No.: 16
    /// Requirement: Post merging two fungible assets, the source asset should have the amount value equal to the sum of
    /// the two.
    /// Criticality: High
    /// Implementation: The merge function increases dst_fungible_asset.amount by src_fungible_asset.amount.
    /// Enforcement: Formally verified via [high-level-req-16](merge).
    ///
    /// No.: 17
    /// Requirement: Fungible assets with zero balance should be destroyed when the amount reaches value 0.
    /// Criticality: Medium
    /// Implementation: The destroy_zero ensures that the balance of the asset has the value 0 and destroy the asset.
    /// Enforcement: Formally verified via [high-level-req-17](destroy_zero).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        /// Once a store has a concurrent balance, its plain balance stays zero.
        invariant forall addr: address {exists<ConcurrentFungibleBalance>(addr)}
            where exists<ConcurrentFungibleBalance>(addr) && exists<FungibleStore>(addr):
            global<FungibleStore>(addr).balance == 0;
        /// A concurrent balance only exists alongside its store.
        invariant [suspendable] forall addr: address {exists<ConcurrentFungibleBalance>(addr)}
            where exists<ConcurrentFungibleBalance>(addr): exists<FungibleStore>(addr);
        /// APT has no dispatch functions.
        invariant !exists<DispatchFunctionStore>(@aptos_fungible_asset);
    }

    spec store_exists(store: address): bool {
        pragma opaque;
        aborts_if false;
        ensures result == exists<FungibleStore>(store);
    }

    spec name<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).name;
    }

    spec symbol<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).symbol;
    }

    spec decimals<T: key>(metadata: Object<T>): u8 {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).decimals;
    }

    spec icon_uri<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).icon_uri;
    }

    spec project_uri<T: key>(metadata: Object<T>): String {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner).project_uri;
    }

    spec metadata<T: key>(metadata: Object<T>): Metadata {
        pragma opaque;
        aborts_if !exists<Metadata>(metadata.inner);
        ensures result == global<Metadata>(metadata.inner);
    }

    spec store_metadata<T: key>(store: Object<T>): Object<Metadata> {
        pragma opaque;
        aborts_if !exists<FungibleStore>(store.inner);
        ensures result == global<FungibleStore>(store.inner).metadata;
    }

    spec is_untransferable<T: key>(metadata: Object<T>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == exists<Untransferable>(metadata.inner);
    }

    spec is_frozen<T: key>(store: Object<T>): bool {
        pragma opaque;
        aborts_if false;
        ensures result == (exists<FungibleStore>(store.inner) && global<FungibleStore>(store.inner).frozen);
    }

    spec create_store<T: key>(constructor_ref: &ConstructorRef, metadata: Object<T>): Object<FungibleStore> {
        let addr = constructor_ref.self;
        /// [high-level-req-8]
        aborts_if exists<FungibleStore>(addr);
        aborts_if !exists<object::ObjectCore>(addr);
        aborts_if !exists<object::ObjectCore>(metadata.inner) || !exists<Metadata>(metadata.inner);
        aborts_if exists<Untransferable>(metadata.inner) && exists<object::Untransferable>(addr);
        aborts_if features::spec_is_enabled(features::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE)
            && exists<ConcurrentFungibleBalance>(addr);
        ensures exists<FungibleStore>(addr);
        ensures global<FungibleStore>(addr).metadata.inner == metadata.inner;
        /// [high-level-req-9]
        ensures global<FungibleStore>(addr).balance == 0;
        ensures global<FungibleStore>(addr).frozen == false;
        ensures result.inner == addr;
    }

    spec remove_store(delete_ref: &DeleteRef) {
        // The store is removed before its concurrent balance.
        pragma disable_invariants_in_body;
        let addr = delete_ref.self;
        aborts_if !exists<object::ObjectCore>(addr);
        aborts_if !exists<FungibleStore>(addr);
        /// [high-level-req-10]
        aborts_if global<FungibleStore>(addr).balance != 0;
        /// [high-level-req-10]
        aborts_if exists<ConcurrentFungibleBalance>(addr)
            && aggregator_v2::spec_get_value(global<ConcurrentFungibleBalance>(addr).balance) != 0;
        ensures !exists<FungibleStore>(addr);
        ensures !exists<ConcurrentFungibleBalance>(addr);
    }

    spec extract(self: &mut FungibleAsset, amount: u64): FungibleAsset {
        /// [high-level-req-14]
        pragma opaque;
        aborts_if self.amount < amount;
        ensures self.amount == old(self).amount - amount;
        ensures self.metadata == old(self).metadata;
        ensures result.amount == amount;
        ensures result.metadata == old(self).metadata;
    }

    spec merge(self: &mut FungibleAsset, src_fungible_asset: FungibleAsset) {
        /// [high-level-req-15]
        pragma opaque;
        aborts_if src_fungible_asset.metadata != self.metadata;
        aborts_if self.amount + src_fungible_asset.amount > MAX_U64;
        /// [high-level-req-16]
        ensures self.amount == old(self).amount + src_fungible_asset.amount;
        ensures self.metadata == old(self).metadata;
    }

    spec destroy_zero(self: FungibleAsset) {
        /// [high-level-req-17]
        aborts_if self.amount != 0;
    }

    spec add_fungibility(
        constructor_ref: &ConstructorRef,
        maximum_supply: Option<u128>,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
    ): Object<Metadata> {
        let addr = constructor_ref.self;
        let concurrent = features::spec_is_enabled(features::CONCURRENT_FUNGIBLE_ASSETS);
        let post supply = global<ConcurrentSupply>(addr).current;
        include AddFungibilityAbortsIf;
        ensures global<Metadata>(addr) == Metadata { name, symbol, decimals, icon_uri, project_uri };
        ensures concurrent ==> aggregator_v2::spec_get_value(supply) == 0;
        ensures concurrent ==> aggregator_v2::spec_get_max_value(supply)
            == if (option::is_some(maximum_supply)) { option::borrow(maximum_supply) } else { MAX_U128 };
        ensures !concurrent ==> global<Supply>(addr) == Supply { current: 0, maximum: maximum_supply };
        ensures result.inner == addr;
    }

    spec schema AddFungibilityAbortsIf {
        constructor_ref: ConstructorRef;
        name: String;
        symbol: String;
        decimals: u8;
        icon_uri: String;
        project_uri: String;
        let addr = constructor_ref.self;
        let concurrent = features::spec_is_enabled(features::CONCURRENT_FUNGIBLE_ASSETS);
        aborts_if constructor_ref.can_delete;
        /// [high-level-req-1]
        aborts_if std::string::length(name) > MAX_NAME_LENGTH;
        aborts_if std::string::length(symbol) > MAX_SYMBOL_LENGTH;
        aborts_if decimals > MAX_DECIMALS;
        aborts_if std::string::length(icon_uri) > MAX_URI_LENGTH;
        aborts_if std::string::length(project_uri) > MAX_URI_LENGTH;
        aborts_if exists<Metadata>(addr);
        aborts_if concurrent && exists<ConcurrentSupply>(addr);
        aborts_if !concurrent && exists<Supply>(addr);
        aborts_if !exists<object::ObjectCore>(addr);
    }

    spec amount(self: &FungibleAsset): u64 {
        pragma opaque;
        aborts_if false;
        ensures result == self.amount;
    }

    spec asset_metadata(self: &FungibleAsset): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec metadata_from_asset(self: &FungibleAsset): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec mint_ref_metadata(self: &MintRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec transfer_ref_metadata(self: &TransferRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec burn_ref_metadata(self: &BurnRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    spec object_from_metadata_ref(self: &MutateMetadataRef): Object<Metadata> {
        pragma opaque;
        aborts_if false;
        ensures result == self.metadata;
    }

    /// The balance of the store at `addr`: kept in `FungibleStore`, or in `ConcurrentFungibleBalance` once the
    /// store has been upgraded.
    spec fun spec_balance(addr: address): u64 {
        if (spec_is_concurrent_balance(addr)) {
            aggregator_v2::spec_get_value(global<ConcurrentFungibleBalance>(addr).balance)
        } else {
            global<FungibleStore>(addr).balance
        }
    }

    spec fun spec_is_concurrent_balance(addr: address): bool {
        global<FungibleStore>(addr).balance == 0 && exists<ConcurrentFungibleBalance>(addr)
    }

    /// The current supply of the asset at `addr`: kept in `ConcurrentSupply` if present, else in `Supply`.
    spec fun spec_supply(addr: address): u128 {
        if (exists<ConcurrentSupply>(addr)) {
            aggregator_v2::spec_get_value(global<ConcurrentSupply>(addr).current)
        } else {
            global<Supply>(addr).current
        }
    }

    /// A store keeps its metadata and frozen flag, and whether its balance is concurrent.
    spec schema StoreBalanceFrame {
        store_addr: address;
        modifies global<FungibleStore>(store_addr);
        modifies global<ConcurrentFungibleBalance>(store_addr);
        ensures exists<FungibleStore>(store_addr);
        ensures global<FungibleStore>(store_addr).metadata == old(global<FungibleStore>(store_addr).metadata);
        ensures global<FungibleStore>(store_addr).frozen == old(global<FungibleStore>(store_addr).frozen);
        ensures exists<ConcurrentFungibleBalance>(store_addr) == old(exists<ConcurrentFungibleBalance>(store_addr));
        ensures spec_is_concurrent_balance(store_addr) == old(spec_is_concurrent_balance(store_addr));
        ensures exists<ConcurrentFungibleBalance>(store_addr) ==>
            aggregator_v2::spec_get_max_value(global<ConcurrentFungibleBalance>(store_addr).balance)
                == aggregator_v2::spec_get_max_value(old(global<ConcurrentFungibleBalance>(store_addr).balance));
    }

    spec schema UncheckedDepositAbortsIf {
        store_addr: address;
        fa: FungibleAsset;
        let store = global<FungibleStore>(store_addr);
        let concurrent = global<ConcurrentFungibleBalance>(store_addr).balance;
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if fa.metadata != store.metadata;
        aborts_if fa.amount != 0 && spec_is_concurrent_balance(store_addr)
            && aggregator_v2::spec_get_value(concurrent) + fa.amount > aggregator_v2::spec_get_max_value(concurrent);
        aborts_if fa.amount != 0 && !spec_is_concurrent_balance(store_addr) && store.balance + fa.amount > MAX_U64;
    }

    spec schema UncheckedDeposit {
        store_addr: address;
        fa: FungibleAsset;
        include UncheckedDepositAbortsIf;
        include StoreBalanceFrame;
        ensures spec_balance(store_addr) == old(spec_balance(store_addr)) + fa.amount;
    }

    spec schema UncheckedWithdrawAbortsIf {
        store_addr: address;
        amount: u64;
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if spec_balance(store_addr) < amount;
    }

    spec schema UncheckedWithdraw {
        store_addr: address;
        amount: u64;
        result: FungibleAsset;
        include UncheckedWithdrawAbortsIf;
        include StoreBalanceFrame;
        ensures spec_balance(store_addr) == old(spec_balance(store_addr)) - amount;
        ensures result == FungibleAsset { metadata: old(global<FungibleStore>(store_addr).metadata), amount };
    }

    spec unchecked_deposit(store_addr: address, fa: FungibleAsset) {
        pragma opaque;
        include UncheckedDeposit;
    }

    spec unchecked_withdraw(store_addr: address, amount: u64): FungibleAsset {
        pragma opaque;
        include UncheckedWithdraw;
    }

    spec increase_supply<T: key>(metadata: &Object<T>, amount: u64) {
        pragma opaque;
        let metadata_addr = metadata.inner;
        modifies global<Supply>(metadata_addr);
        modifies global<ConcurrentSupply>(metadata_addr);
        include IncreaseSupplyAbortsIf { metadata_addr };
        include SupplyFrame { metadata_addr };
        ensures amount != 0 ==> spec_supply(metadata_addr) == old(spec_supply(metadata_addr)) + amount;
    }

    spec schema IncreaseSupplyAbortsIf {
        metadata_addr: address;
        amount: u64;
        let supply = global<Supply>(metadata_addr);
        let concurrent = global<ConcurrentSupply>(metadata_addr).current;
        aborts_if amount != 0 && !exists<ConcurrentSupply>(metadata_addr) && !exists<Supply>(metadata_addr);
        aborts_if amount != 0 && exists<ConcurrentSupply>(metadata_addr)
            && aggregator_v2::spec_get_value(concurrent) + amount > aggregator_v2::spec_get_max_value(concurrent);
        aborts_if amount != 0 && !exists<ConcurrentSupply>(metadata_addr) && option::is_some(supply.maximum)
            && supply.current + amount > option::borrow(supply.maximum);
        aborts_if amount != 0 && !exists<ConcurrentSupply>(metadata_addr) && supply.current + amount > MAX_U128;
    }

    spec decrease_supply<T: key>(metadata: &Object<T>, amount: u64) {
        pragma opaque;
        let metadata_addr = metadata.inner;
        modifies global<Supply>(metadata_addr);
        modifies global<ConcurrentSupply>(metadata_addr);
        include DecreaseSupplyAbortsIf { metadata_addr };
        include SupplyFrame { metadata_addr };
        ensures amount != 0 ==> spec_supply(metadata_addr) == old(spec_supply(metadata_addr)) - amount;
    }

    spec schema DecreaseSupplyAbortsIf {
        metadata_addr: address;
        amount: u64;
        aborts_if amount != 0 && !exists<ConcurrentSupply>(metadata_addr) && !exists<Supply>(metadata_addr);
        aborts_if amount != 0 && spec_supply(metadata_addr) < amount;
    }

    /// Minting and burning change only the current supply.
    spec schema SupplyFrame {
        metadata_addr: address;
        amount: u64;
        ensures exists<Supply>(metadata_addr) == old(exists<Supply>(metadata_addr));
        ensures exists<ConcurrentSupply>(metadata_addr) == old(exists<ConcurrentSupply>(metadata_addr));
        ensures exists<Supply>(metadata_addr) ==>
            global<Supply>(metadata_addr).maximum == old(global<Supply>(metadata_addr).maximum);
        ensures amount == 0 ==> global<Supply>(metadata_addr) == old(global<Supply>(metadata_addr));
        ensures amount == 0 ==> global<ConcurrentSupply>(metadata_addr) == old(global<ConcurrentSupply>(metadata_addr));
        ensures exists<ConcurrentSupply>(metadata_addr) ==>
            aggregator_v2::spec_get_max_value(global<ConcurrentSupply>(metadata_addr).current)
                == aggregator_v2::spec_get_max_value(old(global<ConcurrentSupply>(metadata_addr).current));
        ensures exists<ConcurrentSupply>(metadata_addr) ==>
            global<Supply>(metadata_addr) == old(global<Supply>(metadata_addr));
    }

    spec supply<T: key>(metadata: Object<T>): Option<u128> {
        pragma opaque;
        let addr = metadata.inner;
        aborts_if addr != @aptos_fungible_asset && exists<DeriveSupply>(addr);
        ensures result == if (exists<ConcurrentSupply>(addr) || exists<Supply>(addr)) {
            option::spec_some(spec_supply(addr))
        } else {
            option::spec_none()
        };
    }

    /// Replaces the `Supply` with a `ConcurrentSupply` of the same current value and bound.
    spec upgrade_to_concurrent(ref: &ExtendRef) {
        pragma opaque;
        let addr = ref.self;
        let supply = global<Supply>(addr);
        modifies global<Supply>(addr);
        modifies global<ConcurrentSupply>(addr);
        aborts_if !features::spec_is_enabled(features::CONCURRENT_FUNGIBLE_ASSETS);
        aborts_if !exists<Supply>(addr);
        aborts_if exists<ConcurrentSupply>(addr);
        aborts_if option::is_some(supply.maximum) && supply.current > option::borrow(supply.maximum);
        ensures !exists<Supply>(addr) && exists<ConcurrentSupply>(addr);
        ensures spec_supply(addr) == supply.current;
        ensures aggregator_v2::spec_get_max_value(global<ConcurrentSupply>(addr).current)
            == if (option::is_some(supply.maximum)) { option::borrow(supply.maximum) } else { MAX_U128 };
    }

    /// Moves the store balance into a `ConcurrentFungibleBalance`, preserving it.
    spec upgrade_store_to_concurrent<T: key>(owner: &signer, store: Object<T>) {
        // Partial: the abort conditions of the ownership check are not specified.
        pragma aborts_if_is_partial;
        let addr = store.inner;
        let upgrading = !exists<ConcurrentFungibleBalance>(addr);
        aborts_if upgrading && !exists<FungibleStore>(addr);
        aborts_if upgrading && global<FungibleStore>(addr).frozen;
        aborts_if upgrading && !features::spec_is_enabled(features::CONCURRENT_FUNGIBLE_BALANCE);
        ensures exists<ConcurrentFungibleBalance>(addr);
        ensures spec_balance(addr) == old(spec_balance(addr));
    }

    spec mutate_metadata(
        self: &MutateMetadataRef,
        name: Option<String>,
        symbol: Option<String>,
        decimals: Option<u8>,
        icon_uri: Option<String>,
        project_uri: Option<String>
    ) {
        pragma opaque;
        let addr = self.metadata.inner;
        let pre = global<Metadata>(addr);
        let post post_metadata = global<Metadata>(addr);
        modifies global<Metadata>(addr);
        aborts_if !exists<Metadata>(addr);
        aborts_if option::is_some(name) && string::length(option::borrow(name)) > MAX_NAME_LENGTH;
        aborts_if option::is_some(symbol) && string::length(option::borrow(symbol)) > MAX_SYMBOL_LENGTH;
        aborts_if option::is_some(decimals) && option::borrow(decimals) > MAX_DECIMALS;
        aborts_if option::is_some(icon_uri) && string::length(option::borrow(icon_uri)) > MAX_URI_LENGTH;
        aborts_if option::is_some(project_uri) && string::length(option::borrow(project_uri)) > MAX_URI_LENGTH;
        ensures post_metadata == Metadata {
            name: option::get_with_default(name, pre.name),
            symbol: option::get_with_default(symbol, pre.symbol),
            decimals: option::get_with_default(decimals, pre.decimals),
            icon_uri: option::get_with_default(icon_uri, pre.icon_uri),
            project_uri: option::get_with_default(project_uri, pre.project_uri),
        };
    }

    /// Mirrors `has_balance_dispatch_function`, which is false for APT without looking up its function store.
    spec fun spec_has_balance_dispatch_function(metadata: Object<Metadata>): bool {
        metadata.inner != @aptos_fungible_asset && exists<DispatchFunctionStore>(metadata.inner)
            && option::is_some(global<DispatchFunctionStore>(metadata.inner).derived_balance_function)
    }

    /// Mirrors `has_deposit_dispatch_function`, which is false for APT without looking up its function store.
    spec fun spec_has_deposit_dispatch_function(metadata: Object<Metadata>): bool {
        metadata.inner != @aptos_fungible_asset && exists<DispatchFunctionStore>(metadata.inner)
            && option::is_some(global<DispatchFunctionStore>(metadata.inner).deposit_function)
    }

    /// Mirrors `has_withdraw_dispatch_function`, which is false for APT without looking up its function store.
    spec fun spec_has_withdraw_dispatch_function(metadata: Object<Metadata>): bool {
        metadata.inner != @aptos_fungible_asset && exists<DispatchFunctionStore>(metadata.inner)
            && option::is_some(global<DispatchFunctionStore>(metadata.inner).withdraw_function)
    }

    /// The asset of the store at `store_addr` has a derived-balance hook (dispatchable path, no APT short-circuit).
    spec fun spec_store_has_balance_hook(store_addr: address): bool {
        let m = global<FungibleStore>(store_addr).metadata.inner;
        exists<DispatchFunctionStore>(m) && option::is_some(global<DispatchFunctionStore>(m).derived_balance_function)
    }

    /// The asset of the store at `store_addr` has no deposit hook (dispatchable path, no APT short-circuit).
    spec fun spec_store_no_deposit_hook(store_addr: address): bool {
        let m = global<FungibleStore>(store_addr).metadata.inner;
        !exists<DispatchFunctionStore>(m) || option::is_none(global<DispatchFunctionStore>(m).deposit_function)
    }

    /// The asset of the store at `store_addr` has no withdraw hook (dispatchable path, no APT short-circuit).
    spec fun spec_store_no_withdraw_hook(store_addr: address): bool {
        let m = global<FungibleStore>(store_addr).metadata.inner;
        !exists<DispatchFunctionStore>(m) || option::is_none(global<DispatchFunctionStore>(m).withdraw_function)
    }

    spec balance<T: key>(store: Object<T>): u64 {
        pragma opaque;
        let addr = store.inner;
        aborts_if exists<FungibleStore>(addr)
            && spec_has_balance_dispatch_function(global<FungibleStore>(addr).metadata);
        ensures result == if (exists<FungibleStore>(addr)) { spec_balance(addr) } else { 0 };
    }

    /// The store at `store_addr` holds at least `amount`; a missing store counts as an empty one.
    spec fun spec_is_address_balance_at_least(store_addr: address, amount: u64): bool {
        if (exists<FungibleStore>(store_addr)) { spec_balance(store_addr) >= amount } else { amount == 0 }
    }

    spec is_address_balance_at_least(store_addr: address, amount: u64): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_is_address_balance_at_least(store_addr, amount);
    }

    /// Anyone can deposit an asset of the store's type into an unfrozen store.
    spec deposit<T: key>(store: Object<T>, fa: FungibleAsset) {
        pragma opaque;
        let fa_store = global<FungibleStore>(store.inner);
        aborts_if spec_has_deposit_dispatch_function(fa_store.metadata);
        aborts_if fa_store.frozen;
        include UncheckedDeposit { store_addr: store.inner };
    }

    /// The owner of an unfrozen store can withdraw up to its balance.
    spec withdraw<T: key>(owner: &signer, store: Object<T>, amount: u64): FungibleAsset {
        // Partial: the abort conditions of the ownership check are not specified.
        pragma aborts_if_is_partial;
        let fa_store = global<FungibleStore>(store.inner);
        aborts_if spec_has_withdraw_dispatch_function(fa_store.metadata);
        aborts_if fa_store.frozen;
        include UncheckedWithdraw { store_addr: store.inner };
    }

    /// Registration is rejected for APT, deletable objects, non-assets and assets that already have hooks.
    spec register_dispatch_functions(
        constructor_ref: &ConstructorRef,
        withdraw_function: Option<FunctionInfo>,
        deposit_function: Option<FunctionInfo>,
        derived_balance_function: Option<FunctionInfo>
    ) {
        // Partial: the aborts of the dispatch type compatibility checks are not specified.
        pragma aborts_if_is_partial;
        let addr = constructor_ref.self;
        aborts_if addr == @aptos_fungible_asset;
        aborts_if constructor_ref.can_delete;
        aborts_if !exists<Metadata>(addr);
        aborts_if exists<DispatchFunctionStore>(addr);
        ensures global<DispatchFunctionStore>(addr)
            == DispatchFunctionStore { withdraw_function, deposit_function, derived_balance_function };
    }

    spec register_derive_supply_dispatch_function(
        constructor_ref: &ConstructorRef, dispatch_function: Option<FunctionInfo>
    ) {
        // Partial: the aborts of the dispatch type compatibility check are not specified.
        pragma aborts_if_is_partial;
        let addr = constructor_ref.self;
        aborts_if addr == @aptos_fungible_asset;
        aborts_if constructor_ref.can_delete;
        aborts_if !exists<Metadata>(addr);
        aborts_if exists<DeriveSupply>(addr);
        ensures global<DeriveSupply>(addr) == DeriveSupply { dispatch_function };
    }

    spec supply_impl<T: key>(metadata: Object<T>): Option<u128> {
        let addr = metadata.inner;
        aborts_if false;
        ensures result == if (exists<ConcurrentSupply>(addr) || exists<Supply>(addr)) {
            option::spec_some(spec_supply(addr))
        } else {
            option::spec_none()
        };
    }

    /// The supply bound; an unbounded concurrent supply has none.
    spec maximum<T: key>(metadata: Object<T>): Option<u128> {
        let addr = metadata.inner;
        let max_value = aggregator_v2::spec_get_max_value(global<ConcurrentSupply>(addr).current);
        aborts_if false;
        ensures result == if (exists<ConcurrentSupply>(addr)) {
            if (max_value == MAX_U128) { option::spec_none() } else { option::spec_some(max_value) }
        } else if (exists<Supply>(addr)) {
            global<Supply>(addr).maximum
        } else {
            option::spec_none()
        };
    }

    spec balance_snapshot<T: key>(store: Object<T>): AggregatorSnapshot<u64> {
        let addr = store.inner;
        aborts_if exists<FungibleStore>(addr)
            && spec_has_balance_dispatch_function(global<FungibleStore>(addr).metadata);
        ensures aggregator_v2::spec_read_snapshot(result)
            == if (exists<FungibleStore>(addr)) { spec_balance(addr) } else { 0 };
    }

    spec withdraw_with_ref<T: key>(self: &TransferRef, store: Object<T>, amount: u64): FungibleAsset {
        aborts_if !exists<FungibleStore>(store.inner);
        aborts_if self.metadata != global<FungibleStore>(store.inner).metadata;
        include UncheckedWithdraw { store_addr: store.inner };
    }

    spec deposit_with_ref<T: key>(self: &TransferRef, store: Object<T>, fa: FungibleAsset) {
        aborts_if self.metadata != fa.metadata;
        include UncheckedDeposit { store_addr: store.inner };
    }

    /// Moves `amount` between two stores of the ref's asset, ignoring their frozen flags.
    spec transfer_with_ref<T: key>(self: &TransferRef, from: Object<T>, to: Object<T>, amount: u64) {
        let from_addr = from.inner;
        let to_addr = to.inner;
        let to_concurrent = global<ConcurrentFungibleBalance>(to_addr).balance;
        include UncheckedWithdrawAbortsIf { store_addr: from_addr };
        aborts_if self.metadata != global<FungibleStore>(from_addr).metadata;
        include from_addr != to_addr ==> UncheckedDepositAbortsIf {
            store_addr: to_addr, fa: FungibleAsset { metadata: self.metadata, amount }
        };
        // For a self-transfer the deposit runs on the reduced balance.
        aborts_if amount != 0 && from_addr == to_addr && spec_is_concurrent_balance(to_addr)
            && aggregator_v2::spec_get_value(to_concurrent) > aggregator_v2::spec_get_max_value(to_concurrent);
        ensures from_addr != to_addr ==> spec_balance(from_addr) == old(spec_balance(from_addr)) - amount
            && spec_balance(to_addr) == old(spec_balance(to_addr)) + amount;
        ensures from_addr == to_addr ==> spec_balance(from_addr) == old(spec_balance(from_addr));
    }

    spec burn(self: &BurnRef, fa: FungibleAsset) {
        let addr = fa.metadata.inner;
        aborts_if self.metadata != fa.metadata;
        include DecreaseSupplyAbortsIf { metadata_addr: addr, amount: fa.amount };
        ensures fa.amount != 0 ==> spec_supply(addr) == old(spec_supply(addr)) - fa.amount;
    }

    /// Burns `amount` from the store, ignoring its frozen flag.
    spec burn_from<T: key>(self: &BurnRef, store: Object<T>, amount: u64) {
        let store_addr = store.inner;
        let addr = self.metadata.inner;
        include UncheckedWithdrawAbortsIf;
        aborts_if self.metadata != global<FungibleStore>(store_addr).metadata;
        include DecreaseSupplyAbortsIf { metadata_addr: addr };
        ensures spec_balance(store_addr) == old(spec_balance(store_addr)) - amount;
        ensures amount != 0 ==> spec_supply(addr) == old(spec_supply(addr)) - amount;
    }

    /// Mints `amount` into an unfrozen store of the ref's asset.
    spec mint_to<T: key>(self: &MintRef, store: Object<T>, amount: u64) {
        let store_addr = store.inner;
        let addr = self.metadata.inner;
        aborts_if global<FungibleStore>(store_addr).frozen;
        include IncreaseSupplyAbortsIf { metadata_addr: addr };
        include UncheckedDeposit { fa: FungibleAsset { metadata: self.metadata, amount } };
        ensures amount != 0 ==> spec_supply(addr) == old(spec_supply(addr)) + amount;
    }

    spec set_frozen_flag<T: key>(self: &TransferRef, store: Object<T>, frozen: bool) {
        let addr = store.inner;
        aborts_if !exists<FungibleStore>(addr);
        aborts_if self.metadata != global<FungibleStore>(addr).metadata;
        ensures global<FungibleStore>(addr) == update_field(old(global<FungibleStore>(addr)), frozen, frozen);
    }
}
