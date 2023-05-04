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
/// 1.  Create and configure an admin account (in addition to the source account and nft-receiver account that we created in the earlier parts).
///     run `aptos init --profile admin` to create an admin account
///     go to Move.toml and replace `admin_addr = 0xcafe` with the actual admin address we just created
///
/// 2.a Ensure your terminal is in the correct directory:
///         `aptos-core/aptos-move/move-examples/mint_nft_v2/3-Adding-Admin`
/// 2.b Publish the module under a resource account with the following command:
///         `aptos move create-resource-account-and-publish-package --seed [seed] --address-name mint_nft_v2 --profile default --named-addresses source_addr=default`
///     Sample output is below:
///         aptos move create-resource-account-and-publish-package --seed 2 --address-name mint_nft_v2 --profile default --named-addresses source_addr=default
///         Compiling, may take a little while to download git dependencies...
///         INCLUDING DEPENDENCY AptosFramework
///         INCLUDING DEPENDENCY AptosStdlib
///         INCLUDING DEPENDENCY AptosTokenObjects
///         INCLUDING DEPENDENCY MoveStdlib
///         BUILDING Examples
///         Do you want to publish this package under the resource account's address 495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d? [yes/no] >
///         yes
///         package size 5501 bytes
///         Do you want to submit a transaction for a range of [540000 - 810000] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": "Success"
///         }
///
///     Note the resource account address in the above output: 495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d
///
/// 3. Let's review how we're using the admin account in the code below.
///     a. In struct `ModuleData`, we added two additional fields: `expiration_timestamp` and `minting_enabled`. This will allow us to set and update
///        when this collection will expire, and also enable / disable minting ad-hoc.
///     b. We added two admin functions `set_minting_enabled()` and `set_timestamp()` to update the `expiration_timestamp` and `minting_enabled` fields.
///        In the admin functions, we check if the caller is calling from the valid admin's address. If not, we abort because the caller does not have permission to
///        update the config of this module.
///     c. In `mint_event_ticket()`, we added two assert statements to make sure that the user can only mint token from this collection if minting is enabled and
///        the collection is not expired.
///
/// 4.a Mint an NFT to the nft-receiver account. This will result in an expected failure, because minting hasn't been enabled yet.
///     `aptos move run --function-id [resource account's address]::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver`
///
///     Sample output with an expected failure is below:
///
///     aptos move run --function-id 495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
///     {
///       "Error": "Simulation failed with status: Move abort in 0x495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d::create_nft_with_resource_and_admin_accounts: EMINTING_DISABLED(0x50003): The collection minting is disabled"
///     }
///
/// Running this command fails because minting is disabled in `init_module()`. We will use the admin account to update the flag `minting_enabled` to true and try again.
///
/// 4.b Run the following command from the admin account to update the `minting_enabled` field to true:
///     `aptos move run --function-id [resource account's address]::create_nft_with_resource_and_admin_accounts::set_minting_enabled --args bool:true --profile admin`
///
///     Sample output:
///         aptos move run --function-id 495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d::create_nft_with_resource_and_admin_accounts::set_minting_enabled --args bool:true --profile admin
///         Do you want to submit a transaction for a range of [300 - 400] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0xbec0b1291632e14d41f182d60bc104b82281d2ea5012565d89819506515ebed1",
///             "gas_used": 3,
///             "gas_unit_price": 100,
///             "sender": "9fa360203db4df05b755a481e4d9a6450500f8edd9a64b470d794eea33c1d4c4",
///             "sequence_number": 0,
///             "success": true,
///             "timestamp_us": 1683085645875488,
///             "version": 3702253,
///             "vm_status": "Executed successfully"
///           }
///         }
/// 4.c Mint the NFT again, this time successfully.
///     `aptos move run --function-id 495d6cbe0bfed0b8a2a5c4b429fd7d5ab557020f5a1d486859aab70e4083d67d::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver`
///
///     Sample successful output is below:
///         aptos move run --function-id af7deddd6691d9cf171ab3e20bb50d01711360f6349f811b54332da2628fb376::create_nft_with_resource_and_admin_accounts::mint_event_ticket --profile nft-receiver
///         Do you want to submit a transaction for a range of [52300 - 78400] Octas at a gas unit price of 100 Octas? [yes/no] >
///         yes
///         {
///           "Result": {
///             "transaction_hash": "0xe402be2b609de01c42bd8a528eec4f31fdfd404a90524865a577c94b1cf49b8e",
///             "gas_used": 523,
///             "gas_unit_price": 100,
///             "sender": "e9907369a82cc0d5b93c77e867e36b7e412912ed4825e17b3ca49541888cae67",
///             "sequence_number": 3,
///             "success": true,
///             "timestamp_us": 1683085852885008,
///             "version": 3703876,
///             "vm_status": "Executed successfully"
///           }
///         }
///
///     View the transaction on the explorer here:
///     https://explorer.aptoslabs.com/txn/0xe402be2b609de01c42bd8a528eec4f31fdfd404a90524865a577c94b1cf49b8e?network=devnet
module mint_nft_v2::create_nft_with_resource_and_admin_accounts {
    use std::string;
    use std::bcs;
    use std::object;
    use std::error;

    use std::signer;
    use std::string::String;
    use aptos_framework::account::SignerCapability;
    use aptos_framework::resource_account;
    use aptos_framework::account;
    use aptos_framework::timestamp;

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
        expiration_timestamp: u64,
        minting_enabled: bool,
        collection_object_address: address,
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
            minting_enabled: false,
            expiration_timestamp: 10000000000,
            collection_object_address,
        });
    }

    /// Mint an NFT to the receiver. Note that different from the tutorial in 1-Create-NFT, here we only ask for the receiver's
    /// address. This is because we used resource account to publish this module and stored the resource account's signer
    /// within the `ModuleData`, so we can programmatically sign for transactions instead of manually signing transactions.
    /// See https://aptos.dev/concepts/accounts/#resource-accounts for more details.
    public entry fun mint_event_ticket(receiver: &signer) acquires ModuleData {
        // Mint token to the receiver.
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);

        // Check the config of this module to see if we enable minting tokens from this collection
        assert!(timestamp::now_seconds() < module_data.expiration_timestamp, error::permission_denied(ECOLLECTION_EXPIRED));
        assert!(module_data.minting_enabled, error::permission_denied(EMINTING_DISABLED));

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

        // update "given_to" to the value of the new receiver.
        aptos_token::update_property(
            &resource_signer,
            token_object,
            string::utf8(b"given_to"),
            string::utf8(b"address"),
            bcs::to_bytes(&receiver_address),
        );
    }

    /// Set if minting is enabled for this minting contract.
    public entry fun set_minting_enabled(caller: &signer, minting_enabled: bool) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        // Abort if the caller is not the admin of this module.
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        module_data.minting_enabled = minting_enabled;
    }

    /// Set the expiration timestamp of this minting contract.
    public entry fun set_timestamp(caller: &signer, expiration_timestamp: u64) acquires ModuleData {
        let caller_address = signer::address_of(caller);
        assert!(caller_address == @admin_addr, error::permission_denied(ENOT_AUTHORIZED));
        let module_data = borrow_global_mut<ModuleData>(@mint_nft_v2);
        module_data.expiration_timestamp = expiration_timestamp;
    }
}
