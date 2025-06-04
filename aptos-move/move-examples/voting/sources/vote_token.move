module voting::vote_token {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata, FungibleStore};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use std::signer;
    use std::option;
    use std::string;

    /// Only fungible asset metadata owner can make changes.
    const ERR_NOT_OWNER: u64 = 1;

    const VOTE_TOKEN_SYMBOL: vector<u8> = b"VOTE";
    const VOTE_TOKEN_DECIMALS: u8 = 8;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Hold refs to control the minting, transfer and burning of fungible assets.
    struct ManagingRefs has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
        burn_ref: BurnRef,
    }

    /// Initialize metadata object and store the refs specified by `ref_flags`.
    /// Automatically called by the VM as part of deployment.
    fun init_module(voting_signer: &signer) {
        let constructor_ref = &object::create_named_object(voting_signer, VOTE_TOKEN_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            string::utf8(VOTE_TOKEN_SYMBOL),
            string::utf8(VOTE_TOKEN_SYMBOL),
            VOTE_TOKEN_DECIMALS,
            string::utf8(b""),
            string::utf8(b""),
        );

        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            ManagingRefs {
                mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
                transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
                burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
            }
        )
    }

    #[view]
    public fun token(): Object<Metadata> {
        object::address_to_object(object::create_object_address(&@voting, VOTE_TOKEN_SYMBOL))
    }

    /// Mint as the owner of metadata object to the primary fungible stores of the accounts with amounts of FAs.
    public entry fun mint_to(
        admin: &signer,
        asset: Object<Metadata>,
        to: vector<address>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let receiver_primary_stores = to.map(|addr| primary_fungible_store::ensure_primary_store_exists(addr, asset));
        mint(admin, asset, receiver_primary_stores, amounts);
    }

    /// Mint as the owner of metadata object to multiple fungible stores with amounts of FAs.
    public entry fun mint(
        admin: &signer,
        asset: Object<Metadata>,
        stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>,
    ) acquires ManagingRefs {
        let mint_ref = authorized_borrow_mint_ref(admin, asset);
        stores.zip(amounts, |store, amount| {
            fungible_asset::mint_to(mint_ref, store, amount);
        });
    }

    inline fun authorized_borrow_mint_ref(
        owner: &signer,
        asset: Object<Metadata>,
    ): &MintRef acquires ManagingRefs {
        assert!(object::is_owner(asset, signer::address_of(owner)), ERR_NOT_OWNER);
        &ManagingRefs[object::object_address(&asset)].mint_ref
    }

    #[test_only]
    public fun init_for_test(deployer: &signer) {
        init_module(deployer);
    }
}
