module aptos_framework::fungible_caps {
    use aptos_framework::fungible_asset::{Self, FungibleAsset};
    use aptos_framework::fungible_source::{Self, FungibleSource, init_fungible_source};
    use aptos_framework::fungible_store;
    use aptos_framework::object::{Object, ConstructorRef};
    use std::string::String;
    use std::signer;
    use std::error;

    /// The transfer cap and the the fungible asset do not match.
    const ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH: u64 = 1;

    /// Capability to mint fungible assets of the asset at `asset_addr`.
    struct MintCap has store {
        asset: Object<FungibleSource>
    }

    /// Capability to transfer fungible assets of `asset` in any account.
    struct TransferCap has store {
        asset: Object<FungibleSource>
    }

    /// Capability to burn fungible assets of the asset at `asset_addr`.
    struct BurnCap has store {
        asset: Object<FungibleSource>
    }

    /// The initialization of an object with `FungibleSource`.
    public fun init_fungible_source_with_caps(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8,
    ): (MintCap, TransferCap, BurnCap) {
        let asset = init_fungible_source(constructor_ref, maximum_supply, name, symbol, decimals);
        (MintCap { asset }, TransferCap { asset }, BurnCap { asset })
    }

    /// Mint the `amount` of coin with MintCap.
    public fun mint(cap: &MintCap, amount: u64, to: address) {
        let fa = fungible_asset::mint(&cap.asset, amount);
        fungible_store::deposit(fa, to);
    }

    /// Transfer the fungible asset account of `fungible_asset_owner` with TransferCap.
    public fun set_ungated_transfer(
        cap: &TransferCap,
        fungible_asset_owner: address,
        allow: bool,
    ) {
        fungible_store::set_ungated_transfer(fungible_asset_owner, &cap.asset, allow);
    }

    /// Burn the `amount` of coin with MintCap.
    public fun burn(cap: &BurnCap, amount: u64, from_account: address) {
        let fa = fungible_store::withdraw(from_account, &cap.asset, amount);
        fungible_asset::burn(fa);
    }

    /// Withdarw `amount` of fungible assets of `asset`.
    public fun withdraw<T: key>(fungible_asset_owner: &signer, asset: &Object<T>, amount: u64): FungibleAsset {
        let account_address = signer::address_of(fungible_asset_owner);
        let asset = fungible_source::verify(asset);
        fungible_store::withdraw(account_address, &asset, amount)
    }

    public fun withdraw_with_cap(transfer_cap: &TransferCap, account: address, amount: u64): FungibleAsset {
        let ungated_transfer_allowed = fungible_store::ungated_transfer_allowed(account, &transfer_cap.asset);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(transfer_cap, account, true);
        };
        let fa = fungible_store::withdraw(account, &transfer_cap.asset, amount);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(transfer_cap, account, false);
        };
        fa
    }

    public fun deposit_with_cap(transfer_cap: &TransferCap, fa: FungibleAsset, to: address) {
        assert!(
            &transfer_cap.asset == &fungible_asset::fungible_asset_source(&fa),
            error::invalid_argument(ETRANSFER_CAP_AND_FUNGIBLE_ASSET_MISMATCH)
        );
        let ungated_transfer_allowed = fungible_store::ungated_transfer_allowed(to, &transfer_cap.asset);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(transfer_cap, to, true);
        };
        fungible_store::deposit(fa, to);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(transfer_cap, to, false);
        };
    }

    /// Transfer `amount` of fungible assets of `asset` to `receiver`.
    /// Note: it does not move the underlying object.
    public fun transfer<T: key>(
        fungible_asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        receiver: address
    ) {
        let asset = fungible_source::verify(asset);
        let fa = withdraw(fungible_asset_owner, &asset, amount);
        fungible_store::deposit(fa, receiver);
    }

    public fun transfer_with_cap(
        transfer_cap: &TransferCap,
        amount: u64,
        from: address,
        to: address,
    ) {
        let fa = withdraw_with_cap(transfer_cap, from, amount);
        deposit_with_cap(transfer_cap, fa, to);
    }

    /// Self-explanatory.
    public fun destroy_mint_cap(cap: MintCap) {
        let MintCap { asset: _ } = cap;
    }

    /// Self-explanatory.
    public fun destroy_transfer_cap(cap: TransferCap) {
        let TransferCap { asset: _ } = cap;
    }

    /// Self-explanatory.
    public fun destroy_burn_cap(cap: BurnCap) {
        let BurnCap { asset: _ } = cap;
    }

    /// Self-explanatory.
    public fun asset_of_mint_cap(cap: &MintCap): Object<FungibleSource> {
        cap.asset
    }

    /// Self-explanatory.
    public fun asset_of_transfer_cap(cap: &TransferCap): Object<FungibleSource> {
        cap.asset
    }

    /// Self-explanatory.
    public fun asset_of_burn_cap(cap: &BurnCap): Object<FungibleSource> {
        cap.asset
    }

    #[test_only]
    use std::string;

    #[test_only]
    public fun destroy_caps(mint_cap: MintCap, transfer_cap: TransferCap, burn_cap: BurnCap) {
        destroy_mint_cap(mint_cap);
        destroy_transfer_cap(transfer_cap);
        destroy_burn_cap(burn_cap);
    }

    #[test_only]
    public fun init_default_fungible_source_with_caps(creator_ref: &ConstructorRef): (MintCap, TransferCap, BurnCap) {
        init_fungible_source_with_caps(
            creator_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        )
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flows(creator: &signer) {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        let (mint_cap, transfer_cap, burn_cap) = init_default_fungible_source_with_caps(&creator_ref);
        assert!(asset_of_mint_cap(&mint_cap) == asset_of_transfer_cap(&transfer_cap), 1);
        assert!(asset_of_burn_cap(&burn_cap) == asset_of_transfer_cap(&transfer_cap), 2);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(&mint_cap, 100, creator_address);
        assert!(fungible_source::get_current_supply(&asset) == 100, 3);
        transfer(creator, &asset, 80, aaron_address);
        burn(&burn_cap, 40, aaron_address);
        assert!(fungible_source::get_current_supply(&asset) == 60, 4);

        assert!(fungible_store::ungated_transfer_allowed(creator_address, &asset), 5);
        set_ungated_transfer(&transfer_cap, creator_address, false);
        assert!(!fungible_store::ungated_transfer_allowed(creator_address, &asset), 6);

        destroy_caps(mint_cap, transfer_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::fungible_asset)]
    fun test_ungated_transfer(creator: &signer) {
        let (creator_ref, _asset) = fungible_source::create_test_token(creator);
        let (mint_cap, transfer_cap, burn_cap) = init_default_fungible_source_with_caps(&creator_ref);
        let creator_address = signer::address_of(creator);

        set_ungated_transfer(&transfer_cap, creator_address, false);
        mint(&mint_cap, 100, creator_address);

        destroy_caps(mint_cap, transfer_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    fun test_transfer_with_ref(creator: &signer) {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        let (mint_cap, transfer_cap, burn_cap) = init_default_fungible_source_with_caps(&creator_ref);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(&mint_cap, 100, creator_address);
        set_ungated_transfer(&transfer_cap, creator_address, false);
        set_ungated_transfer(&transfer_cap, aaron_address, false);
        transfer_with_cap(&transfer_cap, 80, creator_address, aaron_address);
        assert!(fungible_store::balance(creator_address, &asset) == 20, 1);
        assert!(fungible_store::balance(aaron_address, &asset) == 80, 2);
        assert!(!fungible_store::ungated_transfer_allowed(creator_address, &asset), 3);
        assert!(!fungible_store::ungated_transfer_allowed(aaron_address, &asset), 4);

        destroy_caps(mint_cap, transfer_cap, burn_cap);
    }
}
