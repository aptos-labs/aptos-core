/// This module creates and mints NFTs using the modern Token v2 (aptos_token_objects) API.
/// Updated to handle Token v2 constraint where only collection creator can mint tokens.
module open_marketplace::create_nft {
    use std::error;
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::debug;
    use aptos_framework::object::{Self, Object};
    use aptos_token_objects::collection::{Self, Collection};
    use aptos_token_objects::token;

    /// Default collection description
    const DEFAULT_COLLECTION_DESCRIPTION: vector<u8> = b"NFT Collection for Marketplace Testing";
    /// Default collection URI
    const DEFAULT_COLLECTION_URI: vector<u8> = b"https://marketplace.example.com/collection";

    /// Create a collection for NFTs with custom metadata
    public entry fun create_collection(
        creator: &signer,
        collection_name: String,
        collection_description: String,
        collection_uri: String,
    ) {
        debug::print(&string::utf8(b"Creating collection with name:"));
        debug::print(&collection_name);
        
        collection::create_unlimited_collection(
            creator,
            collection_description,
            collection_name,
            option::none(), // No royalty
            collection_uri,
        );
        
        debug::print(&string::utf8(b"Collection created successfully"));
    }

    /// Create a collection with default description and URI, custom name
    public entry fun create_collection_with_defaults(
        creator: &signer,
        collection_name: String,
    ) {
        create_collection(
            creator,
            collection_name,
            string::utf8(DEFAULT_COLLECTION_DESCRIPTION),
            string::utf8(DEFAULT_COLLECTION_URI),
        );
    }

    /// Mint an NFT to the receiver address using Token v2 API.
    /// NOTE: You must call create_collection() first before minting tokens.
    public entry fun mint_nft_to_address(
        creator: &signer, // The signer who owns the collection
        collection_name: String,
        receiver_address: address,
        token_name: String,
        token_description: String,
        token_uri: String,
    ) {
        let creator_address = signer::address_of(creator);
        debug::print(&string::utf8(b"Minting token with creator address:"));
        debug::print(&creator_address);
        
        debug::print(&string::utf8(b"Using collection name:"));
        debug::print(&collection_name);

        let constructor_ref = token::create_named_token(
            creator,
            collection_name,
            token_description,
            token_name,
            option::none(), // No royalty
            token_uri,
        );
        debug::print(&constructor_ref);
        
        // Generate transfer ref and transfer to receiver
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        debug::print(&transfer_ref);
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        debug::print(&linear_transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, receiver_address);
    }



    /// Create collection and mint a test NFT in one call with default description/URI
    public entry fun create_collection_and_mint_test_nft_with_defaults(
        creator: &signer,
        collection_name: String,
        receiver_address: address
    ) {
        // First create the collection with defaults
        create_collection_with_defaults(creator, collection_name);
        
        // Then mint the test NFT
        mint_nft_to_address(
            creator,
            collection_name,
            receiver_address,
            string::utf8(b"Test NFT"),
            string::utf8(b"A test NFT for marketplace"),
            string::utf8(b"https://marketplace.example.com/nft/test"),
        );
    }

    /// Create collection and mint a test NFT in one call
    public entry fun create_collection_and_mint_test_nft(
        creator: &signer,
        collection_name: String,
        collection_description: String,
        collection_uri: String,
        receiver_address: address
    ) {
        // First create the collection
        create_collection(creator, collection_name, collection_description, collection_uri);
        
        // Then mint the test NFT
        mint_nft_to_address(
            creator,
            collection_name,
            receiver_address,
            string::utf8(b"Test NFT"),
            string::utf8(b"A test NFT for marketplace"),
            string::utf8(b"https://marketplace.example.com/nft/test"),
        );
    }

    /// Simple mint function with default values for easy testing.
    /// NOTE: You must call create_collection() first before minting tokens.
    public entry fun mint_test_nft(
        creator: &signer, // The signer who owns the collection
        collection_name: String,
        receiver_address: address
    ) {
        mint_nft_to_address(
            creator,
            collection_name,
            receiver_address,
            string::utf8(b"Test NFT"),
            string::utf8(b"A test NFT for marketplace"),
            string::utf8(b"https://marketplace.example.com/nft/test"),
        );
    }

    /// Mint a test NFT to the creator's own address for convenience
    /// NOTE: You must call create_collection() first before minting tokens.
    public entry fun mint_test_nft_to_self(
        creator: &signer, // The signer who owns the collection
        collection_name: String,
    ) {
        let creator_address = signer::address_of(creator);
        mint_test_nft(creator, collection_name, creator_address);
    }

    /// Create collection with defaults and mint a test NFT to self in one call
    public entry fun create_collection_and_mint_test_nft_to_self_with_defaults(
        creator: &signer,
        collection_name: String,
    ) {
        let creator_address = signer::address_of(creator);
        create_collection_and_mint_test_nft_with_defaults(creator, collection_name, creator_address);
    }

    /// Create collection and mint a test NFT to self in one call with default collection metadata
    public entry fun create_default_collection_and_mint_test_nft_to_self(
        creator: &signer
    ) {
        create_collection_and_mint_test_nft_to_self_with_defaults(
            creator, 
            string::utf8(b"Marketplace Collection")
        );
    }
}
