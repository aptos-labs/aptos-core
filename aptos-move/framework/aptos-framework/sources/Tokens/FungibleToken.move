/// This module provides the foundation for FungibleTokens.
module AptosFramework::FungibleToken {
    use Std::ASCII;
    use Std::Errors;
    use Std::Event::{Self, EventHandle};
    use Std::Option::{Self, Option};
    use Std::Signer;

    use AptosFramework::Table::{Self, Table};

    const EALREADY_HAS_BALANCE: u64 = 0;
    const EBALANCE_NOT_PUBLISHED: u64 = 1;
    const ECOLLECTIONS_NOT_PUBLISHED: u64 = 2;
    const ECOLLECTION_NOT_PUBLISHED: u64 = 3;
    const ECOLLECTION_ALREADY_EXISTS: u64 = 4;
    const ECREATE_WOULD_EXCEED_MAXIMUM: u64 = 5;
    const EINSUFFICIENT_BALANCE: u64 = 6;
    const EINVALID_COLLECTION_NAME: u64 = 7;
    const EINVALID_TOKEN_MERGE: u64 = 8;
    const EMINT_WOULD_EXCEED_MAXIMUM: u64 = 9;
    const ENO_BURN_CAPABILITY: u64 = 10;
    const ENO_MINT_CAPABILITY: u64 = 11;
    const ETOKEN_ALREADY_EXISTS: u64 = 12;
    const ETOKEN_NOT_PUBLISHED: u64 = 13;
    const ETOKEN_STORE_NOT_PUBLISHED: u64 = 14;

    //
    // Core data structures for holding tokens
    //

    /// Represents ownership of a the data associated with this Token
    struct FungibleToken has store {
        id: FungibleTokenId,
        value: u64,
    }

    /// Represents a unique identity for the token
    struct FungibleTokenId has copy, drop, store {
        // The creator of this token
        creator: address,
        // The collection or set of related tokens within the creator's account
        collection: ASCII::String,
        // Unique name within a collection within the creator's account
        name: ASCII::String,
    }

    /// Represents token resources owned by token owner
    struct FungibleTokenStore has key {
        tokens: Table<FungibleTokenId, FungibleToken>,
        deposit_events: EventHandle<DepositFungibleTokenEvent>,
        withdraw_events: EventHandle<WithdrawFungibleTokenEvent>,
    }

    /// Set of data sent to the event stream during a receive
    struct DepositFungibleTokenEvent has drop, store {
        id: FungibleTokenId,
        amount: u64,
    }

    /// Set of data sent to the event stream during a withdrawal
    struct WithdrawFungibleTokenEvent has drop, store {
        id: FungibleTokenId,
        amount: u64,
    }

    /// create collection event with creator address and collection name
    struct CreateFungibleTokenCollectionEvent has drop, store {
        creator: address,
        collection_name: ASCII::String,
        uri: ASCII::String,
        description: ASCII::String,
        maximum: Option<u64>,
    }

    /// token creation event id of token created
    struct CreateFungibleTokenEvent has drop, store {
        id: FungibleTokenId,
        token_data: FungibleTokenData,
        initial_balance: u64,
    }

    /// mint token event. This event triggered when creator adds more supply to existing token
    struct MintFungibleTokenEvent has drop, store {
        id: FungibleTokenId,
        amount: u64,
    }

    //
    // Core data structures for creating and maintaining tokens
    //

    /// Represent collection and token metadata for a creator
    struct FungibleTokenCollections has key {
        collections: Table<ASCII::String, FungibleTokenCollection>,
        token_data: Table<FungibleTokenId, FungibleTokenData>,
        burn_capabilities: Table<FungibleTokenId, BurnCapability>,
        mint_capabilities: Table<FungibleTokenId, MintCapability>,
        create_collection_events: EventHandle<CreateFungibleTokenCollectionEvent>,
        create_token_events: EventHandle<CreateFungibleTokenEvent>,
        mint_token_events: EventHandle<MintFungibleTokenEvent>,
    }

    /// Represent the collection metadata
    struct FungibleTokenCollection has store {
        // Describes the collection
        description: ASCII::String,
        // Unique name within this creators account for this collection
        name: ASCII::String,
        // URL for additional information /media
        uri: ASCII::String,
        // Total number of distinct Tokens tracked by the collection
        count: u64,
        // Optional maximum number of tokens allowed within this collections
        maximum: Option<u64>,
    }

    /// The data associated with the Tokens
    struct FungibleTokenData has copy, drop, store {
        // Unique name within this creators account for this Token's collection
        collection: ASCII::String,
        // Describes this Token
        description: ASCII::String,
        // The name of this Token
        name: ASCII::String,
        // Optional maximum number of this type of Token.
        maximum: Option<u64>,
        // Total number of this type of Token
        supply: Option<u64>,
        /// URL for additional information / media
        uri: ASCII::String,
    }

    /// Capability required to mint tokens.
    struct MintCapability has store {
        token_id: FungibleTokenId,
    }

    /// Capability required to burn tokens.
    struct BurnCapability has store {
        token_id: FungibleTokenId,
    }
}