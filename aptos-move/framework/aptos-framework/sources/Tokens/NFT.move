/// This module provides the foundation for Tokens.
module AptosFramework::NFT {
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
    struct NFT has store {
        id: NFTId,
        // the edition of an NFT token. O indicates the master edition.
        edition: u64,
    }

    /// Represents a unique identity for the token
    struct NFTId has copy, drop, store {
        // The creator of this token
        creator: address,
        // The collection or set of related tokens within the creator's account
        collection: ASCII::String,
        // Unique name within a collection within the creator's account, e.g: token_name + edition
        name: ASCII::String,
    }

    /// Represents token resources owned by token owner
    struct NFTStore has key {
        tokens: Table<NFTId, vector<u64>>,
        deposit_events: EventHandle<DepositNFTEvent>,
        withdraw_events: EventHandle<WithdrawNFTEvent>,
    }

    /// Set of data sent to the event stream during a receive
    struct DepositNFTEvent has drop, store {
        id: NFTId,
    }

    /// Set of data sent to the event stream during a withdrawal
    struct WithdrawNFTEvent has drop, store {
        id: NFTId,
    }

    /// create collection event with creator address and collection name
    struct CreateNFTCollectionEvent has drop, store {
        creator: address,
        collection_name: ASCII::String,
        uri: ASCII::String,
        description: ASCII::String,
        maximum: Option<u64>,
    }

    /// token creation event id of token created
    struct CreateNFTEvent has drop, store {
        id: NFTId,
        token_data: NFTData,
    }

    /// mint token event. This event triggered when creator adds more supply to existing token
    struct MintNFTEvent has drop, store {
        id: NFTId,
    }

    //
    // Core data structures for creating and maintaining tokens
    //

    /// Represent collection and token metadata for a creator
    struct Collections has key {
        collections: Table<ASCII::String, CollectionData>,
        token_data: Table<NFTId, NFTData>,
        burn_capabilities: Table<NFTId, BurnCapability>,
        mint_capabilities: Table<NFTId, MintCapability>,
        create_collection_events: EventHandle<CreateNFTCollectionEvent>,
        create_token_events: EventHandle<CreateNFTEvent>,
        mint_token_events: EventHandle<MintNFTEvent>,
    }

    /// Represent the collection metadata
    struct CollectionData has store {
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
        // allow collection metadata to be mutable
        mutable: bool,
    }

    /// The data associated with the Tokens
    struct NFTData has copy, drop, store {
        // Describes this Token
        description: ASCII::String,
        // the maxium number of edition this token can print.
        max: u64,
        // Total number of editions supplied by this type of Token
        supply: u64,
        /// URL for additional information / media
        uri: ASCII::String,
        /// allow NFTData to be mutable with MutateCapability
        mutable: bool
    }

    /// Capability required to mint tokens.
    struct MintCapability has store {
        token_id: NFTId,
    }

    /// Capability required to burn tokens.
    struct BurnCapability has store {
        token_id: NFTId,
    }

    //
    // Creator Script functions
    //
}
