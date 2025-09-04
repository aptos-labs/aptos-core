/// This module can be included in a project to enable internal wrapping and unwrapping of coins into fungible assets.
/// This allows the project to only have to store and process fungible assets in core data structures, while still be
/// able to support both native fungible assets and coins. Note that the wrapper fungible assets are INTERNAL ONLY and
/// are not meant to be released to user's accounts outside of the project. Othwerwise, this would create multiple
/// conflicting fungible asset versions of a specific coin in the ecosystem.
///
/// The flow works as follows:
/// 1. Add the coin_wrapper module to the project.
/// 2. Add a friend declaration for any core modules that needs to call wrap/unwrap. Wrap/Unwrap are both friend-only
/// functions so external modules cannot call them and leak the internal fungible assets outside of the project.
/// 3. Add entry functions in the core modules that take coins. Those functions will be calling wrap to create the
/// internal fungible assets and store them.
/// 4. Add entry functions in the core modules that return coins. Those functions will be extract internal fungible
/// assets from the core data structures, unwrap them into and return the coins to the end users.
///
/// The fungible asset wrapper for a coin has the same name, symbol and decimals as the original coin. This allows for
/// easier accounting and tracking of the deposited/withdrawn coins.
module swap::coin_wrapper {
    use velor_framework::account::{Self, SignerCapability};
    use velor_framework::velor_account;
    use velor_framework::coin::{Self, Coin};
    use velor_framework::fungible_asset::{Self, BurnRef, FungibleAsset, Metadata, MintRef};
    use velor_framework::object::{Self, Object};
    use velor_framework::primary_fungible_store;
    use velor_std::smart_table::{Self, SmartTable};
    use velor_std::string_utils;
    use velor_std::type_info;
    use std::string::{Self, String};
    use std::option;
    use std::signer;
    use swap::package_manager;

    // Modules in the same package that need to wrap/unwrap coins need to be added as friends here.
    friend swap::router;

    const COIN_WRAPPER_NAME: vector<u8> = b"COIN_WRAPPER";

    /// Stores the refs for a specific fungible asset wrapper for wrapping and unwrapping.
    struct FungibleAssetData has store {
        // Used during unwrapping to burn the internal fungible assets.
        burn_ref: BurnRef,
        // Reference to the metadata object.
        metadata: Object<Metadata>,
        // Used during wrapping to mint the internal fungible assets.
        mint_ref: MintRef,
    }

    /// The resource stored in the main resource account to track all the fungible asset wrappers.
    /// This main resource account will also be the one holding all the deposited coins, each of which in a separate
    /// CoinStore<CoinType> resource. See coin.move in the Velor Framework for more details.
    struct WrapperAccount has key {
        // The signer cap used to withdraw deposited coins from the main resource account during unwrapping so the
        // coins can be returned to the end users.
        signer_cap: SignerCapability,
        // Map from an original coin type (represented as strings such as "0x1::velor_coin::VelorCoin") to the
        // corresponding fungible asset wrapper.
        coin_to_fungible_asset: SmartTable<String, FungibleAssetData>,
        // Map from a fungible asset wrapper to the original coin type.
        fungible_asset_to_coin: SmartTable<Object<Metadata>, String>,
    }

    /// Create the coin wrapper account to host all the deposited coins.
    public entry fun initialize() {
        if (is_initialized()) {
            return
        };

        let swap_signer = &package_manager::get_signer();
        let (coin_wrapper_signer, signer_cap) = account::create_resource_account(swap_signer, COIN_WRAPPER_NAME);
        package_manager::add_address(string::utf8(COIN_WRAPPER_NAME), signer::address_of(&coin_wrapper_signer));
        move_to(&coin_wrapper_signer, WrapperAccount {
            signer_cap,
            coin_to_fungible_asset: smart_table::new(),
            fungible_asset_to_coin: smart_table::new(),
        });
    }

    #[view]
    public fun is_initialized(): bool {
        package_manager::address_exists(string::utf8(COIN_WRAPPER_NAME))
    }

    #[view]
    /// Return the address of the resource account that stores all deposited coins.
    public fun wrapper_address(): address {
        package_manager::get_address(string::utf8(COIN_WRAPPER_NAME))
    }

    #[view]
    /// Return whether a specific CoinType has a wrapper fungible asset. This is only the case if at least one wrap()
    /// call has been made for that CoinType.
    public fun is_supported<CoinType>(): bool acquires WrapperAccount {
        let coin_type = type_info::type_name<CoinType>();
        smart_table::contains(&wrapper_account().coin_to_fungible_asset, coin_type)
    }

    #[view]
    /// Return true if the given fungible asset is a wrapper fungible asset.
    public fun is_wrapper(metadata: Object<Metadata>): bool acquires WrapperAccount {
        smart_table::contains(&wrapper_account().fungible_asset_to_coin, metadata)
    }

    #[view]
    /// Return the original CoinType for a specific wrapper fungible asset. This errors out if there's no such wrapper.
    public fun get_coin_type(metadata: Object<Metadata>): String acquires WrapperAccount {
        *smart_table::borrow(&wrapper_account().fungible_asset_to_coin, metadata)
    }

    #[view]
    /// Return the wrapper fungible asset for a specific CoinType. This errors out if there's no such wrapper.
    public fun get_wrapper<CoinType>(): Object<Metadata> acquires WrapperAccount {
        fungible_asset_data<CoinType>().metadata
    }

    #[view]
    /// Return the original CoinType if the given fungible asset is a wrapper fungible asset. Otherwise, return the
    /// given fungible asset itself, which means it's a native fungible asset (not wrapped).
    /// The return value is a String such as "0x1::velor_coin::VelorCoin" for an original coin or "0x12345" for a native
    /// fungible asset.
    public fun get_original(fungible_asset: Object<Metadata>): String acquires WrapperAccount {
        if (is_wrapper(fungible_asset)) {
            get_coin_type(fungible_asset)
        } else {
            format_fungible_asset(fungible_asset)
        }
    }

    #[view]
    /// Return the address string of a fungible asset (e.g. "0x1234").
    public fun format_fungible_asset(fungible_asset: Object<Metadata>): String {
        let fa_address = object::object_address(&fungible_asset);
        // This will create "@0x123"
        let fa_address_str = string_utils::to_string(&fa_address);
        // We want to strip the prefix "@"
        string::sub_string(&fa_address_str, 1, string::length(&fa_address_str))
    }

    /// Wrap the given coins into fungible asset. This will also create the fungible asset wrapper if it doesn't exist
    /// yet. The coins will be deposited into the main resource account.
    public(friend) fun wrap<CoinType>(coins: Coin<CoinType>): FungibleAsset acquires WrapperAccount {
        // Ensure the corresponding fungible asset has already been created.
        create_fungible_asset<CoinType>();

        // Deposit coins into the main resource account and mint&return the wrapper fungible assets.
        let amount = coin::value(&coins);
        velor_account::deposit_coins(wrapper_address(), coins);
        let mint_ref = &fungible_asset_data<CoinType>().mint_ref;
        fungible_asset::mint(mint_ref, amount)
    }

    /// Unwrap the given fungible asset into coins. This will burn the fungible asset and withdraw&return the coins from
    /// the main resource account.
    /// This errors out if the given fungible asset is not a wrapper fungible asset.
    public(friend) fun unwrap<CoinType>(fa: FungibleAsset): Coin<CoinType> acquires WrapperAccount {
        let amount = fungible_asset::amount(&fa);
        let burn_ref = &fungible_asset_data<CoinType>().burn_ref;
        fungible_asset::burn(burn_ref, fa);
        let wrapper_signer = &account::create_signer_with_capability(&wrapper_account().signer_cap);
        coin::withdraw(wrapper_signer, amount)
    }

    /// Create the fungible asset wrapper for the given CoinType if it doesn't exist yet.
    public(friend) fun create_fungible_asset<CoinType>(): Object<Metadata> acquires WrapperAccount {
        let coin_type = type_info::type_name<CoinType>();
        let wrapper_account = mut_wrapper_account();
        let coin_to_fungible_asset = &mut wrapper_account.coin_to_fungible_asset;
        let wrapper_signer = &account::create_signer_with_capability(&wrapper_account.signer_cap);
        if (!smart_table::contains(coin_to_fungible_asset, coin_type)) {
            let metadata_constructor_ref = &object::create_named_object(wrapper_signer, *string::bytes(&coin_type));
            primary_fungible_store::create_primary_store_enabled_fungible_asset(
                metadata_constructor_ref,
                option::none(),
                coin::name<CoinType>(),
                coin::symbol<CoinType>(),
                coin::decimals<CoinType>(),
                string::utf8(b""),
                string::utf8(b""),
            );

            let mint_ref = fungible_asset::generate_mint_ref(metadata_constructor_ref);
            let burn_ref = fungible_asset::generate_burn_ref(metadata_constructor_ref);
            let metadata = object::object_from_constructor_ref<Metadata>(metadata_constructor_ref);
            smart_table::add(coin_to_fungible_asset, coin_type, FungibleAssetData {
                metadata,
                mint_ref,
                burn_ref,
            });
            smart_table::add(&mut wrapper_account.fungible_asset_to_coin, metadata, coin_type);
        };
        smart_table::borrow(coin_to_fungible_asset, coin_type).metadata
    }

    inline fun fungible_asset_data<CoinType>(): &FungibleAssetData acquires WrapperAccount {
        let coin_type = type_info::type_name<CoinType>();
        smart_table::borrow(&wrapper_account().coin_to_fungible_asset, coin_type)
    }

    inline fun wrapper_account(): &WrapperAccount acquires WrapperAccount {
        borrow_global<WrapperAccount>(wrapper_address())
    }

    inline fun mut_wrapper_account(): &mut WrapperAccount acquires WrapperAccount {
        borrow_global_mut<WrapperAccount>(wrapper_address())
    }

    #[test_only]
    friend swap::coin_wrapper_tests;
}
