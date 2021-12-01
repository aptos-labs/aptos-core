module Sender::NFTStandard {
    use Std::GUID::{Self, GUID};
    use Std::Signer;
    use Std::Vector;
    use Std::Option::{Self, Option};
    use Std::Errors;

    /// Errors
    const EID_NOT_FOUND: u64 = 0;
    const EID_EXISTS: u64 = 1;
    const ENFT_COLLECTION_NOT_PUBLISHED: u64 = 2;

    /// A non-fungible token of a specific `NFTType`
    struct NFT<NFTType: store + drop> has key, store {
        /// A globally unique identifier, which includes the address of the NFT
        /// creator, as well as the globally unique ID.
        id: GUID,
        /// A struct to enable type-specific fields that will be different for each Token.
        /// For example, `NFT<Painting>` with
        /// `struct Painting { name: vector<u84, painter: vector<u8>, year: u64, ... }`,
        /// Or, `NFT<DigitalPirateInGameItem> { item_type: u8, item_power: u8, ... }`
        type: NFTType,
        /// pointer to where the content and metadata is stored. Could be a DiemID domain, IPFS, Dropbox url, etc
        content_uri: vector<u8>,
    }

    /// A collection of NFTs of a specific type NFTType
    struct NFTCollection<NFTType: store + drop> has key {
        nfts: vector<NFT<NFTType>>,
    }

    /// Return the globally unique identifier of `nft`
    public fun id<NFTType: store + drop>(nft: &NFT<NFTType>): &GUID {
        &nft.id
    }

    /// Return the creator of this NFT
    public fun creator<NFTType: store + drop>(nft: &NFT<NFTType>): address {
        GUID::creator_address(id<NFTType>(nft))
    }

    /// View the underlying token of a NFT
    public fun type<NFTType: store + drop>(nft: &NFT<NFTType>): &NFTType {
        &nft.type
    }

    /// Initialize NFTColletion of type NFTType
    public fun initialize<NFTType: store + drop>(account: &signer) {
        if (!exists<NFTCollection<NFTType>>(Signer::address_of(account))) {
            move_to(account, NFTCollection { nfts: Vector::empty<NFT<NFTType>>() });
        };
    }

    /// Script function wrapper of initialize()
    public(script) fun initialize_nft_collection<NFTType: store + drop>(account: signer) {
        initialize<NFTType>(&account);
    }

    /// Create a `NFT<Type>` that wraps id, type and content_uri
    public fun create<NFTType: store + drop>(
        account: &signer, type: NFTType, content_uri: vector<u8>
    ): NFT<NFTType> {
        let token_id = GUID::create(account);
        NFT { id: token_id, type, content_uri }
    }

    /// Publish the non-fungible token `nft` under `account`.
    public fun add<NFTType: store + drop>(account: address, nft: NFT<NFTType>) acquires NFTCollection {
        assert!(exists<NFTCollection<NFTType>>(account), Errors::not_published(ENFT_COLLECTION_NOT_PUBLISHED));
        assert!(!has_token<NFTType>(account, &GUID::id(&nft.id)), Errors::already_published(EID_EXISTS));
        let nft_collection = &mut borrow_global_mut<NFTCollection<NFTType>>(account).nfts;
        Vector::push_back(
            nft_collection,
            nft,
        );
    }

    /// Remove the `NFT<Type>` under `account`
    fun remove<NFTType: store + drop>(owner: address, id: &GUID::ID): NFT<NFTType> acquires NFTCollection {
        let nft_collection = &mut borrow_global_mut<NFTCollection<NFTType>>(owner).nfts;
        let nft_index = index_of_token<NFTType>(nft_collection, id);
        assert!(Option::is_some(&nft_index), Errors::limit_exceeded(EID_NOT_FOUND));
        Vector::remove(nft_collection, Option::extract(&mut nft_index))
    }

    /// Transfer the non-fungible token `nft` with GUID identifiable by `creator` and `creation_num`
    /// Transfer from `account` to `to`
    public(script) fun transfer<NFTType: store + drop>(
        account: signer,
        to: address,
        creator: address,
        creation_num: u64
    ) acquires NFTCollection {
        let owner_address = Signer::address_of(&account);

        // Remove NFT from `owner`'s collection
        let id = GUID::create_id(creator, creation_num);
        let nft = remove<NFTType>(owner_address, &id);

        // Add NFT to `to`'s collection
        add<NFTType>(to, nft);
    }

    /// Returns whether the owner has a token with given id.
    public fun has_token<NFTType: store + drop>(owner: address, token_id: &GUID::ID): bool acquires NFTCollection {
        Option::is_some(&index_of_token(&borrow_global<NFTCollection<NFTType>>(owner).nfts, token_id))
    }

    /// Finds the index of token with the given id in the nft_collection.
    fun index_of_token<NFTType: store + drop>(nft_collection: &vector<NFT<NFTType>>, id: &GUID::ID): Option<u64> {
        let i = 0;
        let len = Vector::length(nft_collection);
        while (i < len) {
            if (GUID::id(id<NFTType>(Vector::borrow(nft_collection, i))) == *id) {
                return Option::some(i)
            };
            i = i + 1;
        };
        Option::none()
    }
}
