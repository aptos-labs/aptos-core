/// This module is the part 3 of our NFT Move tutorial, building on top of part 2. In this module, we introduce an admin account and
/// admin functions to update the configuration of this module.
///
/// We need admin accounts, because we give up control over the smart contracts when we deploy a module using a resource account.
/// Sometimes that's okay, but there are times when we still want to retain some control over the module - for example,
/// when we want to update the configuration of the module. This is where an admin account comes in. We can add a few admin functions
/// to control the configurations of the module, as well as adding the admin account's address to the Move.toml file so the admin functions
/// can check if the caller to the admin functions is actually a valid admin.
///
/// In this part of the NFT tutorial, we are adding an admin account and two admin functions on top of the previous tutorial, so we can set
/// and update when/if we want to enable token minting for this contract.
///
/// How to interact with this module:
/// 1. Create and configure an admin account (in addition to the source account and nft-receiver account that we created in the earlier parts).
/// - 1.a run `velor init --profile admin` to create an admin account
/// - 1.b go to `Move.toml` and replace `admin_addr = "0xcafe"` with the actual admin address we just created
///
/// 2. Publish the module under a resource account.
/// - 2.a Make sure you're in the right directory.
/// Run the following command in directory `velor-core/velor-move/move-examples/mint_nft/3-Adding-Admin`.
/// - 2.b Run the following CLI command to publish the module under a resource account.
/// velor move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft --profile default --named-addresses source_addr=[default account's address]
///
/// example output:
    /*
    3-Adding-Admin % velor move create-resource-account-and-publish-package --seed 1239 --address-name mint_nft --profile default --named-addresses source_addr=a911e7374107ad434bbc5369289cf5855c3b1a2938a6bfce0776c1d296271cde

    Compiling, may take a little while to download git dependencies...
    INCLUDING DEPENDENCY VelorFramework
    INCLUDING DEPENDENCY VelorStdlib
    INCLUDING DEPENDENCY VelorToken
    INCLUDING DEPENDENCY MoveStdlib
    BUILDING Examples
    Do you want to publish this package under the resource account's address 34f5acaaef16988aa31bb56ad7b35e30d14ff9fc849d370be799617b61d3df04? [yes/no] >
    yes
    package size 5694 bytes
    Do you want to submit a transaction for a range of [1567300 - 2350900] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": "Success"
    }
    */
/// 3. Go over how we're using the admin account in the code below.
/// - 3.a In struct `ModuleData`, we added two additional fields: `expiration_timestamp` and `minting_enabled`. This will allow us to set and update
/// when this collection will expire, and also enable / disable minting ad-hoc.
/// - 3.b We added two admin functions `set_minting_enabled()` and `set_timestamp()` to update the `expiration_timestamp` and `minting_enabled` fields.
/// In the admin functions, we check if the caller is calling from the valid admin's address. If not, we abort because the caller does not have permission to
/// update the config of this module.
/// - 3.c In `mint_event_ticket()`, we added two assert statements to make sure that the user can only mint token from this collection if minting is enabled and
/// the collection is not expired.
///
/// 4. Mint an NFT to the nft-receiver account.
/// - 4.a Run the following command to mint an NFT (failure expected).
/// velor move run --function-id [resource account's address]::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
/// example output:
    /*
    3-Adding-Admin % velor move run --function-id 34f5acaaef16988aa31bb56ad7b35e30d14ff9fc849d370be799617b61d3df04::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
    {
      "Error": "Simulation failed with status: Move abort in 0x34f5acaaef16988aa31bb56ad7b35e30d14ff9fc849d370be799617b61d3df04::create_nft_with_resource_and_admin_accounts: EMINTING_DISABLED(0x50003): The collection minting is disabled"
    }
    */
/// Running this command fails because minting is disabled in `init_module()`. We will use the admin account to update the flag `minting_enabled` to true and try again.
/// - 4.b Running the following command from the admin account to update field `minting_enabled` to true.
/// velor move run --function-id [resource account's address]::create_nft_with_resource_and_admin_accounts::set_minting_enabled --args bool:true --profile admin
/// example output:
    /*
    3-Adding-Admin % velor move run --function-id 34f5acaaef16988aa31bb56ad7b35e30d14ff9fc849d370be799617b61d3df04::create_nft_with_resource_and_admin_accounts::set_minting_enabled --args bool:true --profile admin
    Do you want to submit a transaction for a range of [23100 - 34600] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": {
        "transaction_hash": "0x083a5af0e8cec07e3ffce8108db1d789f8a3954a8d8a468b9bb5cfb6815889d8",
        "gas_used": 231,
        "gas_unit_price": 100,
        "sender": "f42bcdc1fb9b8d4c0ac9e54568a53c8515d3d9afd7936484a923b0d7854e134f",
        "sequence_number": 0,
        "success": true,
        "timestamp_us": 1669666889848214,
        "version": 12874291,
        "vm_status": "Executed successfully"
      }
    }
    */
/// - 4.c Mint the NFT again (should be successful this time).
/// velor move run --function-id [resource account's address]::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
/// example output:
    /*
    3-Adding-Admin % velor move run --function-id 34f5acaaef16988aa31bb56ad7b35e30d14ff9fc849d370be799617b61d3df04::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
    Do you want to submit a transaction for a range of [504500 - 756700] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": {
        "transaction_hash": "0xffd0c7093bcd7ddf1ac023dc213253cff32382cf7d76060a1530ae003a61fc4e",
        "gas_used": 5075,
        "gas_unit_price": 100,
        "sender": "7d69283af198b1265d17a305ff0cca6da1bcee64d499ce5b35b659098b3a82dc",
        "sequence_number": 3,
        "success": true,
        "timestamp_us": 1669666995720264,
        "version": 12877056,
        "vm_status": "Executed successfully"
      }
    }
    */
/// - 4.d Check out the transactions on https://explorer.velorlabs.com/ by searching for the transaction hash.
module mint_nft::create_nft_with_resource_and_admin_accounts {
    use std::error;
    use std::string;
    use std::vector;

    use velor_token::token;
    use std::signer;
    use std::string::String;
    use velor_token::token::TokenDataId;
    use velor_framework::account::SignerCapability;
    use velor_framework::resource_account;
    use velor_framework::account;
    use velor_framework::timestamp;

    // This struct stores an NFT collection's relevant information
    struct ModuleData has key {
        // Storing the signer capability here, so the module can programmatically sign for transactions
        signer_cap: SignerCapability,
        token_data_id: TokenDataId,
        expiration_timestamp: u64,
        minting_enabled: bool,
    }

    /// Action not authorized because the signer is not the admin of this module
    const ENOT_AUTHORIZED: u64 = 1;
    /// The collection minting is expired
    const ECOLLECTION_EXPIRED: u64 = 2;
    /// The collection minting is disabled
    const EMINTING_DISABLED: u64 = 3;

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

        // store the token data id within the module, so we can refer to it later
        // when we're minting the NFT
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @source_addr);
        move_to(resource_signer, ModuleData {
            signer_cap: resource_signer_cap,
            token_data_id,
            minting_enabled: false,
            expiration_timestamp: 10000000000,
        });
    }

    /// Mint an NFT to the receiver. Note that different from the tutorial in 1-Create-NFT, here we only ask for the receiver's
    /// signer. This is because we used resource account to publish this module and stored the resource account's signer
    /// within the `ModuleData`, so we can programmatically sign for transactions instead of manually signing transactions.
    /// See https://velor.dev/concepts/accounts/#resource-accounts for more details.
    public entry fun mint_event_ticket(receiver: &signer) acquires ModuleData {
        // Mint token to the receiver.
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);

        // Check the config of this module to see if we enable minting tokens from this collection
        assert!(timestamp::now_seconds() < module_data.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

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

    /// Set if minting is enabled for this minting contract.
    public entry fun set_minting_enabled(caller: &signer, minting_enabled: bool) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        // Abort if the caller is not the admin of this module.
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        module_data.minting_enabled = minting_enabled;
    }

    /// Set the expiration timestamp of this minting contract.
    public entry fun set_timestamp(caller: &signer, expiration_timestamp: u64) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
        module_data.expiration_timestamp = expiration_timestamp;
    }
}
