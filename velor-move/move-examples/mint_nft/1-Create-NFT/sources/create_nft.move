/// This module is the part 1 of our NFT Move tutorial. In this module, we go over how to create a collection and token,
/// and then mint a token to a receiver.
///
/// Generally, there are two types of NFT:
/// 1. Event ticket / certificate: this kind of NFT has a base token, and every new NFT generated from this base token has the same
/// token data id and picture.
/// They are generally used as certificate. Each NFT created from the base token is considered a printing edition of the
/// base token.
/// An example is using this kind of NFT as event ticket: each NFT is a ticket and has properties like expiration_sec:u64, and is_ticket_used:bool.
/// When we mint the NFT, we can set an expiration time for the event ticket and set is_ticket_used to false. When the ticket is used, we can update
/// is_ticket_used to true.
/// 2. Pfp NFT: this kind of NFT has a unique token data id and picture for each token. There are generally no printing editions of this NFT.
/// Most NFT collections on NFT marketplaces are of this kind. They are generally proofs of ownership of an art piece.
///
/// In this tutorial, we are going to go over how to create and mint event ticket NFTs.
///
/// How to interact with this module:
/// 1. Create an account.
/// velor init (this will create a default account)
///
/// 2. Publish the module.
/// - 2.a Make sure you're in the right directory.
/// Run the following command in directory `velor-core/velor-move/move-examples/mint_nft/1-Create-NFT`.
/// - 2.b Run the following CLI command to publish the module.
/// velor move publish --named-addresses mint_nft=[default account's address]
/// (If you don't know the default account's address, run `nano ~/.velor/config.yaml` to see all addresses.)
///
/// example output:
    /*
    1-Create-NFT % velor move publish --named-addresses mint_nft=a911e7374107ad434bbc5369289cf5855c3b1a2938a6bfce0776c1d296271cde
    Compiling, may take a little while to download git dependencies...
    INCLUDING DEPENDENCY VelorFramework
    INCLUDING DEPENDENCY VelorStdlib
    INCLUDING DEPENDENCY VelorToken
    INCLUDING DEPENDENCY MoveStdlib
    BUILDING Examples
    package size 2770 bytes
    Do you want to submit a transaction for a range of [1164400 - 1746600] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": {
        "transaction_hash": "0x576a2e9481e71b629335b98ea75c87d124e1b435e843e7a2ef8938ae21bebfa3",
        "gas_used": 11679,
        "gas_unit_price": 100,
        "sender": "a911e7374107ad434bbc5369289cf5855c3b1a2938a6bfce0776c1d296271cde",
        "sequence_number": 0,
        "success": true,
        "timestamp_us": 1669659103283876,
        "version": 12735152,
        "vm_status": "Executed successfully"
      }
    }
    */
/// - 2.c Check the module we just published on the Velor Explorer.
/// Go to https://explorer.velorlabs.com/. At the top right of the screen, select the network you used (devnet, testnet, etc.).
/// Search for this transaction by putting the `transaction_hash` in the search box. (You'd need to run throught the above steps
/// yourself, and search for the transaction using your own unique transaction hash.)
/// We can see the changes we made by publishing this module under the `Changes` tab.
///
/// 3. Check out the `delayed_mint_event_ticket()` function below - we are not going to run a command to mint the NFT in this part, because
/// this function right now asks for two signers and that's inpractical to do using CLI commands.
/// In the next part of this tutorial, we will introduce a way to programmatically sign for transactions, so the module publisher
/// doesn't need to manually sign transactions, and only needs one signer (the nft receiver's signer) for the `delayed_mint_event_ticket()` function.
module mint_nft::create_nft {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    use velor_token::token;
    use velor_token::token::TokenDataId;

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        token_data_id: TokenDataId,
    }

    /// Action not authorized because the signer is not the admin of this module
    const ENOT_AUTHORIZED: u64 = 1;

    /// `init_module` is automatically called when publishing the module.
    /// In this function, we create an example NFT collection and an example token.
    fun init_module(source_account: &signer) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        // This means that the supply of the token will not be tracked.
        let maximum_supply = 0;
        // This variable sets if we want to allow mutation for collection description, uri, and maximum.
        // Here, we are setting all of them to false, which means that we don't allow mutations to any CollectionData fields.
        let mutate_setting = vector<bool>[ false, false, false ];

        // Create the nft collection.
        token::create_collection(source_account, collection_name, description, collection_uri, maximum_supply, mutate_setting);

        // Create a token data id to specify the token to be minted.
        let token_data_id = token::create_tokendata(
            source_account,
            collection_name,
            token_name,
            string::utf8(b""),
            0,
            token_uri,
            signer::address_of(source_account),
            1,
            0,
            // This variable sets if we want to allow mutation for token maximum, uri, royalty, description, and properties.
            // Here we enable mutation for properties by setting the last boolean in the vector to true.
            token::create_token_mutability_config(
                &vector<bool>[ false, false, false, false, true ]
            ),
            // We can use property maps to record attributes related to the token.
            // In this example, we are using it to record the receiver's address.
            // We will mutate this field to record the user's address
            // when a user successfully mints a token in the `mint_nft()` function.
            vector<String>[string::utf8(b"given_to")],
            vector<vector<u8>>[b""],
            vector<String>[ string::utf8(b"address") ],
        );

        // Store the token data id within the module, so we can refer to it later
        // when we're minting the NFT and updating its property version.
        move_to(source_account, ModuleData {
            token_data_id,
        });
    }

    /// Mint an NFT to the receiver. Note that here we ask two accounts to sign: the module owner and the receiver.
    /// This is not ideal in production, because we don't want to manually sign each transaction. It is also
    /// impractical/inefficient in general, because we either need to implement delayed execution on our own, or have
    /// two keys to sign at the same time.
    /// In part 2 of this tutorial, we will introduce the concept of "resource account" - it is
    /// an account controlled by smart contracts to automatically sign for transactions. Resource account is also known
    /// as PDA or smart contract account in general blockchain terms.
    public entry fun delayed_mint_event_ticket(module_owner: &signer, receiver: &signer) acquires ModuleData {
        // Assert that the module owner signer is the owner of this module.
        assert!(signer::address_of(module_owner) == @mint_nft, error::permission_denied(ENOT_AUTHORIZED));

        // Mint token to the receiver.
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        let token_id = token::mint_token(module_owner, module_data.token_data_id, 1);
        token::direct_transfer(module_owner, receiver, token_id, 1);

        // Mutate the token properties to update the property version of this token.
        // Note that here we are re-using the same token data id and only updating the property version.
        // This is because we are simply printing edition of the same token, instead of creating
        // tokens with unique names and token uris. The tokens created this way will have the same token data id,
        // but different property versions.
        let (creator_address, collection, name) = token::get_token_data_id_fields(&module_data.token_data_id);
        token::mutate_token_properties(
            module_owner,
            signer::address_of(receiver),
            creator_address,
            collection,
            name,
            0,
            1,
            // Mutate the properties to record the receiveer's address.
            vector<String>[string::utf8(b"given_to")],
            vector<vector<u8>>[bcs::to_bytes(&signer::address_of(receiver))],
            vector<String>[ string::utf8(b"address") ],
        );
    }
}
