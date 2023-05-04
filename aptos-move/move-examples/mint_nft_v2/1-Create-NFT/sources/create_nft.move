/// This is a tutorial intended to mirror the `mint_nft` example in move-examples as closely as possible, with the only difference
/// being that we're using the no code token from aptos_token to demonstrate how to do the tutorial with token v2 instead of token v1.
///
/// 1.  Initialize a profile and create an account associated with it on-chain by running the following command:
///         `aptos init --profile default`
///     When prompted for the network and address, use `devnet` and just hit enter for the address.
///     This will create a default profile for you to use.
///
/// 2.a Ensure your terminal is in the correct directory:
///         `aptos-core/aptos-move/move-examples/mint_nft_v2/1-Create-Nft`
///
/// 2.b Publish the module by running the following command:
///         `aptos move publish --named-addresses mint_nft_v2=default --profile default`
///     Note that removing `--profile default` at the end results in the same thing, because it is the default profile.
///
/// 2.c Check the transaction output on the Aptos Explorer with the `transaction_hash` output after running the command.
///     Sample output is below:
///         aptos move publish --named-addresses mint_nft_v2=default --profile default
///
///         Compiling, may take a little while to download git dependencies...
///         INCLUDING DEPENDENCY AptosFramework
///         INCLUDING DEPENDENCY AptosStdlib
///         INCLUDING DEPENDENCY AptosTokenObjects
///         INCLUDING DEPENDENCY MoveStdlib
///         BUILDING Examples
///         package size 3370 bytes
///         Do you want to submit a transaction for a range of [373100 - 559600] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0x5b9a0410a054bc63889759d2069096c31a7a941597d4a177cd7de5dee15790d8",
///             "gas_used": 3731,
///             "gas_unit_price": 100,
///             "sender": "c9f5ffb4fa164729e28733d5878ee4e90604478640dfb5ccb477452c0fe1cd79",
///             "sequence_number": 16,
///             "success": true,
///             "timestamp_us": 1683074933526983,
///             "version": 3633993,
///             "vm_status": "Executed successfully"
///           }
///         }
///     To view the transaction details, replace the hash in the URL below with the `transaction_hash` from your output:
///         https://explorer.aptoslabs.com/txn/0x5b9a0410a054bc63889759d2069096c31a7a941597d4a177cd7de5dee15790d8?network=devnet
///
/// 3.a Ensure that you have an account to send an NFT to by running the following command:
///         `aptos init --profile nft-receiver`
///     To easily view the account address you just created:
///         `aptos account lookup-address --profile nft-receiver`
///     And look at the value in the `Result` field.
///     Sample output is below:
///         aptos account lookup-address --profile nft-receiver
///         {
///           "Result": "e9907369a82cc0d5b93c77e867e36b7e412912ed4825e17b3ca49541888cae67"
///         }
///
/// 3.b Run the `delayed_mint_event_ticket` function below. Note this is referred to as delayed because it requires
///     the asynchronous (more specifically, an off-chain) approval of the module_owner in order to mint.
///
///     Run the following command:
///         `aptos move run --function-id default::create_nft::delayed_mint_event_ticket --args address:nft-receiver --profile default`
///
///     Sample output is below:
///         aptos move run --function-id default::create_nft::delayed_mint_event_ticket --args address:nft-receiver --profile default
///
///         Do you want to submit a transaction for a range of [52300 - 78400] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0xd3f7bab04e18f4e631f40e669bf1e3224d8c08318da509aeedf1806cc0cd6cdd",
///             "gas_used": 523,
///             "gas_unit_price": 100,
///             "sender": "c9f5ffb4fa164729e28733d5878ee4e90604478640dfb5ccb477452c0fe1cd79",
///             "sequence_number": 18,
///             "success": true,
///             "timestamp_us": 1683077797249592,
///             "version": 3652742,
///             "vm_status": "Executed successfully"
///           }
///         }
///
///     View the transaction on the explorer here:
///     https://explorer.aptoslabs.com/txn/0xd3f7bab04e18f4e631f40e669bf1e3224d8c08318da509aeedf1806cc0cd6cdd?network=devnet
module mint_nft_v2::create_nft {
    use std::bcs;
    use std::error;
    use std::signer;
    use std::object;
    use std::string::{Self, String};
    use aptos_framework::account;

    use aptos_token_objects::aptos_token::{Self, AptosToken};
    use aptos_token_objects::collection;

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        collection_name: String,
        creator: address,
        token_description: String,
        token_name: String,
        token_uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        collection_object_address: address,
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
        let maximum_supply = 1000;

        aptos_token::create_collection(
            source_account,
            description,
            maximum_supply,
            collection_name,
            collection_uri,
            false, // mutable_description
            false, // mutable_royalty
            false, // mutable_uri
            false, // mutable_token_description
            false, // mutable_token_name
            true, // mutable_token_properties
            false, // mutable_token_uri
            false, // tokens_burnable_by_creator
            false, // tokens_freezable_by_creator
            5, // royalty_numerator
            100, // royalty_denominator
        );

        let source_address = signer::address_of(source_account);
        let collection_object_address = collection::create_collection_address(&source_address, &collection_name);

        move_to(source_account, ModuleData {
            collection_name,
            creator: source_address,
            token_description: string::utf8(b""),
            token_name,
            token_uri,
            property_keys: vector<String>[string::utf8(b"given_to")],
            property_types: vector<String>[ string::utf8(b"address") ],
            property_values: vector<vector<u8>>[bcs::to_bytes(&source_address)],
            collection_object_address,
        });

    }

    /// Mint an NFT to the receiver. Note that we don't need the receiver to sign to receive a token/object
    /// with token v2- you only need to pass the `receiver_address` to the entry function.
    public entry fun delayed_mint_event_ticket(module_owner: &signer, receiver_address: address) acquires ModuleData {
        let module_owner_address = signer::address_of(module_owner);
        assert!(module_owner_address == @mint_nft_v2, error::permission_denied(ENOT_AUTHORIZED));

        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);

        // mint token to the receiver
        let token_creation_num = account::get_guid_next_creation_num(module_owner_address);
        aptos_token::mint(
            module_owner,
            module_data.collection_name,
            module_data.token_description,
            module_data.token_name,
            module_data.token_uri,
            module_data.property_keys,
            module_data.property_types,
            module_data.property_values,
        );
        let token_object = object::address_to_object<AptosToken>(object::create_guid_object_address(module_owner_address, token_creation_num));
        object::transfer(module_owner, token_object, receiver_address);

        // update "given_to" to the value of the new receiver.
        aptos_token::update_property(
            module_owner,
            token_object,
            string::utf8(b"given_to"),
            string::utf8(b"address"),
            bcs::to_bytes(&receiver_address),
        );
    }
}
