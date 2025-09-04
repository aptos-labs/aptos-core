/// This module is the part 2 of our NFT Move tutorial, building on top of part 1. In this part of the tutorial, we introduce the concept of a resource
/// account and add a resource account on top of the previous tutorial.
///
/// Concept: resource account
/// A resource account is a developer feature used to manage resources independent of an account managed by a user, specifically publishing modules and automatically signing for transactions.
/// In this module, we are using a resource account to publish this module and programmatically sign for token minting and transferring transactions.
///
/// How to interact with this module:
/// 1. Create an nft-receiver account (in addition to the source account we created in the last part). We'll use this account to receive an NFT in this tutorial.
/// velor init --profile nft-receiver
///
/// 2. Publish the module under a resource account.
/// - 2.a Make sure you're in the right directory.
/// Run the following command in directory `velor-core/velor-move/move-examples/mint_nft/2-Using-Resource-Account`
/// - 2.b Run the following CLI command to publish the module under a resource account.
/// velor move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft --profile default --named-addresses source_addr=[default account's address]
///
/// example output:
    /*
    2-Using-Resource-Account % velor move create-resource-account-and-publish-package --seed 1235 --address-name mint_nft --profile default --named-addresses source_addr=a911e7374107ad434bbc5369289cf5855c3b1a2938a6bfce0776c1d296271cde
    Compiling, may take a little while to download git dependencies...
    INCLUDING DEPENDENCY VelorFramework
    INCLUDING DEPENDENCY VelorStdlib
    INCLUDING DEPENDENCY VelorToken
    INCLUDING DEPENDENCY MoveStdlib
    BUILDING Examples
    Do you want to publish this package under the resource account's address 3ad2cce668ed2186da580b95796ffe8534566583363cd3b03547bec9542662dc? [yes/no] >
    yes
    package size 2928 bytes
    Do you want to submit a transaction for a range of [1371100 - 2056600] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": "Success"
    }
    */
///
/// 3. Go over our newly added code on resource account.
/// - 3.a In 2.b, we published this module under the resource account's address using the CLI command `create-resource-account-and-publish-package`.
/// Publishing a module under a resource account means that we will not be able to update the module, and the module will be immutable and autonomous.
/// This introduces a challenge:
/// What if we want to update the configuration of this module? In the next part of this tutorial, we will go over how to add an admin account and admin functions
/// to update the configuration of this module without interfering with the automaticity and immunity that come with using a resource account.
/// - 3.b In `init_module`, we store the resource account's signer capability within `ModuleData` for later usage.
/// - 3.c In `mint_event_ticket`, we create a resource signer by calling `account::create_signer_with_capability(&module_data.signer_cap)` to programmatically sign for `token::mint_token()` and `token::direct_transfer()` functions.
/// If we didn't use a resource account for this module, we would need to manually sign for those transactions.
///
/// 4. Mint an NFT to the nft-receiver account
/// - 4.a Run the following command
/// velor move run --function-id [resource account's address]::create_nft_with_resource_account::mint_event_ticket --profile nft-receiver
///
/// example output:
    /*
    2-Using-Resource-Account % velor move run --function-id 55328567ff8aa7d242951af7fc1872746fbeeb89dfed0e1ee2ff71b9bf4469d6::create_nft_with_resource_account::mint_event_ticket --profile nft-receiver
    Do you want to submit a transaction for a range of [502900 - 754300] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": {
        "transaction_hash": "0x720c06eafe77ff385dffcf31c6217839aab3185b65972d6900adbcc3838a4425",
        "gas_used": 5029,
        "gas_unit_price": 100,
        "sender": "7d69283af198b1265d17a305ff0cca6da1bcee64d499ce5b35b659098b3a82dc",
        "sequence_number": 1,
        "success": true,
        "timestamp_us": 1669662022240704,
        "version": 12784585,
        "vm_status": "Executed successfully"
      }
    }
    */
/// - 4.b Check out the transaction on https://explorer.velorlabs.com/ by searching for the transaction hash.
module mint_nft::create_nft_with_resource_account {
    use std::string;
    use std::vector;

    use velor_token::token;
    use std::signer;
    use std::string::String;
    use velor_token::token::TokenDataId;
    use velor_framework::account::SignerCapability;
    use velor_framework::resource_account;
    use velor_framework::account;

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        // Storing the signer capability here, so the module can programmatically sign for transactions
        signer_cap: SignerCapability,
        token_data_id: TokenDataId,
    }

    /// `init_module` is automatically called when publishing the module.
    /// In this function, we create an example NFT collection and an example token.
    fun init_module(resource_signer: &signer) {
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
        token::create_collection(resource_signer, collection_name, description, collection_uri, maximum_supply, mutate_setting);

        // Create a token data id to specify the token to be minted.
        let token_data_id = token::create_tokendata(
            resource_signer,
            collection_name,
            token_name,
            string::utf8(b""),
            0,
            token_uri,
            signer::address_of(resource_signer),
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
            // when a user successfully mints a token in the `mint_event_ticket()` function.
            vector<String>[string::utf8(b"given_to")],
            vector<vector<u8>>[b""],
            vector<String>[ string::utf8(b"address") ],
        );

        // Retrieve the resource signer's signer capability and store it within the `ModuleData`.
        // Note that by calling `resource_account::retrieve_resource_account_cap` to retrieve the resource account's signer capability,
        // we rotate th resource account's authentication key to 0 and give up our control over the resource account. Before calling this function,
        // the resource account has the same authentication key as the source account so we had control over the resource account.
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @source_addr);

        // Store the token data id and the resource account's signer capability within the module, so we can programmatically
        // sign for transactions in the `mint_event_ticket()` function.
        move_to(resource_signer, ModuleData {
            signer_cap: resource_signer_cap,
            token_data_id,
        });
    }

    /// Mint an NFT to the receiver. Note that different from the tutorial in part 1, here we only ask for the receiver's
    /// signer. This is because we used resource account to publish this module and stored the resource account's signer
    /// within the `ModuleData`, so we can programmatically sign for transactions instead of manually signing transactions.
    /// See https://velor.dev/concepts/accounts/#resource-accounts for more information about resource account.
    public entry fun mint_event_ticket(receiver: &signer) acquires ModuleData {
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);

        // Create a signer of the resource account from the signer capability stored in this module.
        // Using a resource account and storing its signer capability within the module allows the module to programmatically
        // sign transactions on behalf of the module.
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
        let token_id = token::mint_token(&resource_signer, module_data.token_data_id, 1);
        token::direct_transfer(&resource_signer, receiver, token_id, 1);

        // Mutate the token properties to update the property version of this token.
        // Note that here we are re-using the same token data id and only updating the property version.
        // This is because we are simply printing edition of the same token, instead of creating unique
        // tokens. The tokens created this way will have the same token data id, but different property versions.
        let (creator_address, collection, name) = token::get_token_data_id_fields(&module_data.token_data_id);
        token::mutate_token_properties(
            &resource_signer,
            signer::address_of(receiver),
            creator_address,
            collection,
            name,
            0,
            1,
            vector::empty<String>(),
            vector::empty<vector<u8>>(),
            vector::empty<String>(),
        );
    }
}
