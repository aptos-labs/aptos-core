/// This is an example of a voting token and based on the managed_fungible_asset example from Aptos' move-examples.
/// The mint and burn functions are friend-only so they can be called as part of a protocol's functionalities and flows.
/// This also offers disable_transfer for the vote module to managed locked voting positions and transfer (via transfer
/// ref) to allow bypassing the disable_transfer flag if needed. Vote module uses this to allow merging voting positions
///
/// Note that after deployment, deployer should call initialize() to create the token.
///
/// In this specific example, mint and burn are not called by the vote module.
module vote_lockup::voting_token {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset, FungibleStore};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use std::signer;
    use std::string;
    use std::option;
    use vote_lockup::package_manager;

    friend vote_lockup::vote;

    const TOKEN_NAME: vector<u8> = b"VOTING";
    const TOKEN_SYMBOL: vector<u8> = b"VOTING";
    const TOKEN_DECIMALS: u8 = 8;
    const TOKEN_URI: vector<u8> = b"voting.apt";
    const PROJECT_URI: vector<u8> = b"voting.apt";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Fungible asset refs used to manage the $VOTING token.
    struct VotingToken has key {
        burn_ref: BurnRef,
        mint_ref: MintRef,
        transfer_ref: TransferRef,
    }

    /// Deploy the $VOTING token. This can only be called once.
    public entry fun initialize() {
        if (is_initialized()) {
            return
        };

        // Create the voting token as a fungible asset.
        let voting_token_metadata = &object::create_named_object(&package_manager::get_signer(), TOKEN_NAME);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            voting_token_metadata,
            option::none(),
            string::utf8(TOKEN_NAME),
            string::utf8(TOKEN_SYMBOL),
            TOKEN_DECIMALS,
            string::utf8(TOKEN_URI),
            string::utf8(PROJECT_URI),
        );
        let voting_token = &object::generate_signer(voting_token_metadata);
        // Vote module needs transfer ref for locking voting tokens up.
        // Mint/Burn are not directly used in this example but can be kept around for mint/burn functionalities.
        // If not needed, developers can delete them.
        move_to(voting_token, VotingToken {
            burn_ref: fungible_asset::generate_burn_ref(voting_token_metadata),
            mint_ref: fungible_asset::generate_mint_ref(voting_token_metadata),
            transfer_ref: fungible_asset::generate_transfer_ref(voting_token_metadata),
        });
        package_manager::add_address(string::utf8(TOKEN_NAME), signer::address_of(voting_token));

        // Developers can add any logic for preminting tokens at launch here.
    }

    #[view]
    public fun is_initialized(): bool {
        package_manager::address_exists(string::utf8(TOKEN_NAME))
    }

    #[view]
    /// Return the address of the metadata that's created when this module is deployed.
    public fun token_address(): address {
        package_manager::get_address(string::utf8(TOKEN_NAME))
    }

    #[view]
    /// Return the $VOTING token metadata object.
    public fun token(): Object<VotingToken> {
        object::address_to_object(token_address())
    }

    #[view]
    /// Return the total supply of $VOTING tokens.
    public fun total_supply(): u128 {
        option::get_with_default(&fungible_asset::supply(token()), 0)
    }

    #[view]
    /// Return the total supply of $VOTING tokens.
    public fun balance(user: address): u64 {
        primary_fungible_store::balance(user, token())
    }

    public(friend) fun mint(amount: u64): FungibleAsset acquires VotingToken {
        fungible_asset::mint(&unchecked_token_refs().mint_ref, amount)
    }

    public(friend) fun burn(voting_tokens: FungibleAsset) acquires VotingToken {
        fungible_asset::burn(&unchecked_token_refs().burn_ref, voting_tokens);
    }

    /// For depositing $VOTING into a fungible asset store. This can be the voting certificate, which cannot be
    /// deposited into normally as it's frozen (no owner transfers).
    public(friend) fun deposit<T: key>(store: Object<T>, voting_tokens: FungibleAsset) acquires VotingToken {
        fungible_asset::deposit_with_ref(&unchecked_token_refs().transfer_ref, store, voting_tokens);
    }

    /// For withdrawing $VOTING from a voting certificate.
    public(friend) fun withdraw<T: key>(store: Object<T>, amount: u64): FungibleAsset acquires VotingToken {
        fungible_asset::withdraw_with_ref(&unchecked_token_refs().transfer_ref, store, amount)
    }

    /// For extracting $VOTING from the voting certificate when owner withdraws after the lockup has expired.
    public(friend) fun transfer<T: key>(
        from: Object<T>,
        to: Object<FungibleStore>,
        amount: u64,
    ) acquires VotingToken {
        let from = object::convert(from);
        let transfer_ref = &unchecked_token_refs().transfer_ref;
        fungible_asset::transfer_with_ref(transfer_ref, from, to, amount);
    }

    /// Used to lock $VOTING in when creating voting certificates.
    public(friend) fun disable_transfer<T: key>(voting_store: Object<T>) acquires VotingToken {
        let transfer_ref = &unchecked_token_refs().transfer_ref;
        fungible_asset::set_frozen_flag(transfer_ref, voting_store, true);
    }

    inline fun unchecked_token_refs(): &VotingToken {
        borrow_global<VotingToken>(token_address())
    }

    #[test_only]
    friend vote_lockup::voting_token_tests;

    #[test_only]
    public fun test_mint(amount: u64): FungibleAsset acquires VotingToken {
        mint(amount)
    }

    #[test_only]
    public fun test_burn(tokens: FungibleAsset) acquires VotingToken {
        burn(tokens)
    }
}
