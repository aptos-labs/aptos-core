module nft::mint {
    use aptos_framework::account;
    use aptos_framework::resource_account;
    use aptos_token::token::{Self, Token, TokenId};

    use std::string::{Self, String};
    use std::vector;

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
    }

    fun init_module(origin: &signer) {
        let (resource, signer_cap) = resource_account::create_resource_account(origin, vector::empty(), vector::empty());
        let collection_name = string::utf8(b"Aptos Example");
        let description = string::utf8(b"This is an Aptos NFT Minting Example");
        let collection_uri = string::utf8(b"aptos.dev");
        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        token::create_collection(&resource, collection_name, description, collection_uri, maximum_supply, mutate_setting);
    }

    fun mint_NFT(nft_receiver: &signer, creator_address: address) acquires ModuleData {
        let module_data = borrow_global_mut<ModuleData>(creator_address);
        let resource_signer = account::create_signer_with_capability(&module_data.resource_signer_cap);

    }
}

// TODO use mint_to
// TODO collection maximum sets to 0, token maximum sets to 0