/// This module is the part 2 of our NFT Move tutorial, building on top of part 1. In this part of the tutorial, we introduce the concept of a resource
/// account and add a resource account on top of the previous tutorial.
///
/// Concept: resource account
/// A resource account is a developer feature used to manage resources independent of an account managed by a user, specifically publishing modules and automatically signing for transactions.
/// In this module, we are using a resource account to publish this module and programmatically sign for token minting and transferring transactions.
///
/// 1.  If you haven't already, initialize an `nft-receiver` profile.
///         `aptos init --profile nft-receiver`
///     To easily view the account address you just created:
///         `aptos account lookup-address --profile nft-receiver`
///     And look at the value in the `Result` field.
///     Sample output is below:
///         aptos account lookup-address --profile nft-receiver
///         {
///           "Result": "e9907369a82cc0d5b93c77e867e36b7e412912ed4825e17b3ca49541888cae67"
///         }
/// 2.a Ensure your terminal is in the correct directory:
///         `aptos-core/aptos-move/move-examples/mint_nft_v2/2-Using-Resource-Account`
///
/// 2.b Publish the module under a resource account with the following command:
///         `aptos move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft_v2 --profile default --named-addresses source_addr=default`
///     Sample output is below:
///         aptos move create-resource-account-and-publish-package --seed 1 --address-name mint_nft_v2 --profile default --named-addresses source_addr=default
///         Compiling, may take a little while to download git dependencies...
///         INCLUDING DEPENDENCY AptosFramework
///         INCLUDING DEPENDENCY AptosStdlib
///         INCLUDING DEPENDENCY AptosTokenObjects
///         INCLUDING DEPENDENCY MoveStdlib
///         BUILDING Examples
///         Do you want to publish this package under the resource account's address 3aac2425799642807080ad9344bc405a90bdcc57258346f158a1ce2008c585be? [yes/no] >
///         yes
///         package size 5501 bytes
///         Do you want to submit a transaction for a range of [540000 - 810000] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": "Success"
///         }
///
///     Note the resource account address in the above output: 3aac2425799642807080ad9344bc405a90bdcc57258346f158a1ce2008c585be
///
/// 3.a Go over our newly added code on resource account. In 2.b, we published this module under the resource account's address using the CLI command `create-resource-account-and-publish-package`.
///     Publishing a module under a resource account means that we will not be able to update the module, and the module will be immutable and autonomous.
///     This introduces a challenge:
///         What if we want to update the configuration of this module? In the next part of this tutorial, we will go over how to add an admin account and admin functions
///         to update the configuration of this module without interfering with the automaticity and immutability that come with using a resource account.
/// 3.b In `init_module`, we store the resource account's signer capability within `ModuleData` for later usage.
/// 3.c In `mint_event_ticket`, we create a resource signer by calling `account::create_signer_with_capability(&module_data.signer_cap)` to programmatically sign for `aptos_token::mint()` and `object::transfer()` functions.
///     If we didn't use a resource account for this module, we would need to manually sign for those transactions.
///
/// 4.a Mint an NFT to the nft-receiver account with the following command:
///     `aptos move run --function-id [resource account's address]::create_nft_with_resource_account::mint_event_ticket --profile nft-receiver`
///
///     Sample output below:
///         aptos move run --function-id 3aac2425799642807080ad9344bc405a90bdcc57258346f158a1ce2008c585be::create_nft_with_resource_account::mint_event_ticket --profile nft-receiver
///         Do you want to submit a transaction for a range of [52400 - 78600] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0xd5efb305bc22dd1b92f5d56c1e062a188e000a515f9c5c35cbba55045006003a",
///             "gas_used": 524,
///             "gas_unit_price": 100,
///             "sender": "e9907369a82cc0d5b93c77e867e36b7e412912ed4825e17b3ca49541888cae67",
///             "sequence_number": 1,
///             "success": true,
///             "timestamp_us": 1683082675358417,
///             "version": 3683299,
///             "vm_status": "Executed successfully"
///           }
///         }
///
///     View the transaction on the explorer here:
///     https://explorer.aptoslabs.com/txn/0xd5efb305bc22dd1b92f5d56c1e062a188e000a515f9c5c35cbba55045006003a?network=devnet
module mint_nft_v2::create_nft_with_resource_account {
    use std::string;
    use std::bcs;
    use std::object;

    use std::signer;
    use std::string::String;
    use aptos_framework::account::SignerCapability;
    use aptos_framework::resource_account;
    use aptos_framework::account;

    use aptos_token_objects::collection::{Self};
    use aptos_token_objects::aptos_token::{Self, AptosToken};

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        // Storing the signer capability here, so the module can programmatically sign for transactions
        signer_cap: SignerCapability,
        collection_name: String,
        token_description: String,
        token_name: String,
        token_uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        collection_object_address: address,
    }

    /// `init_module` is automatically called when publishing the module.
    /// In this function, we create an example NFT collection and an example token.
    fun init_module(resource_signer: &signer) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let token_name = string::utf8(b"Token name");
        let token_uri = string::utf8(b"Token uri");
        let maximum_supply = 1000;

        aptos_token::create_collection(
            resource_signer,
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

        // Retrieve the resource signer's signer capability and store it within the `ModuleData`.
        // Note that by calling `resource_account::retrieve_resource_account_cap` to retrieve the resource account's signer capability,
        // we rotate the resource account's authentication key to 0 and give up our control over the resource account. Before calling this function,
        // the resource account has the same authentication key as the source account so we had control over the resource account.
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @source_addr);
        let resource_address = signer::address_of(resource_signer);
        let collection_object_address = collection::create_collection_address(&resource_address, &collection_name);

        // Store the token data id and the resource account's signer capability within the module, so we can programmatically
        // sign for transactions in the `mint_event_ticket()` function.
        move_to(resource_signer, ModuleData {
            signer_cap: resource_signer_cap,
            collection_name,
            token_description: string::utf8(b""),
            token_name,
            token_uri,
            property_keys: vector<String>[string::utf8(b"given_to")],
            property_types: vector<String>[ string::utf8(b"address") ],
            property_values: vector<vector<u8>>[bcs::to_bytes(&@source_addr)],
            collection_object_address,
        });
    }

    /// Mint an NFT to the receiver. Note the difference from the tutorial in part 1, here we only ask for the receiver's
    /// address. This is because we used resource account to publish this module and stored the resource account's signer
    /// within the `ModuleData`, so we can programmatically sign for transactions instead of manually signing transactions.
    /// See https://aptos.dev/concepts/accounts/#resource-accounts for more information about resource account.
    /// Note also the difference with Token v2: we no longer need the receiver to sign the transaction. You can send objects
    /// to accounts without the prior approval of the account you're sending objects to.
    public entry fun mint_event_ticket(receiver: &signer) acquires ModuleData {
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);

        // Create a signer of the resource account from the signer capabiity stored in this module.
        // Using a resource account and storing its signer capability within the module allows the module to programmatically
        // sign transactions on behalf of the module.
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let resource_address = signer::address_of(&resource_signer);
        let token_creation_num = account::get_guid_next_creation_num(resource_address);

        aptos_token::mint(
            &resource_signer,
            module_data.collection_name,
            module_data.token_description,
            module_data.token_name,
            module_data.token_uri,
            module_data.property_keys,
            module_data.property_types,
            module_data.property_values,
        );

        let token_object = object::address_to_object<AptosToken>(object::create_guid_object_address(resource_address, token_creation_num));
        let receiver_address = signer::address_of(receiver);
        object::transfer(&resource_signer, token_object, receiver_address);

        // remove the property key "given_to" entirely.
        aptos_token::remove_property(
            &resource_signer,
            token_object,
            string::utf8(b"given_to"),
        );

        // add "given_to" and change its value to the new receiver.
        aptos_token::add_property(
            &resource_signer,
            token_object,
            string::utf8(b"given_to"),
            string::utf8(b"address"),
            bcs::to_bytes(&receiver_address),
        );
    }
}
