module addr::token_v1 {
    use std::string::{Self, String};
    use std::signer;
    use std::bcs;
    use velor_token::token;
    use velor_token::property_map;
    use velor_token::token_transfers;

    /// The ambassador token collection name
    const COLLECTION_NAME: vector<u8> = b"Ambassador Collection Name";
    /// The ambassador token collection description
    const COLLECTION_DESCRIPTION: vector<u8> = b"Ambassador Collection Description";
    /// The ambassador token collection URI
    const COLLECTION_URI: vector<u8> = b"Ambassador Collection URI";

    const TOKEN_NAME: vector<u8> = b"Token 1";

    public entry fun run(creator: &signer) {
        token::create_collection(
            creator,
            string::utf8(COLLECTION_NAME),
            string::utf8(COLLECTION_DESCRIPTION),
            string::utf8(COLLECTION_URI),
            100,
            vector<bool>[true, true, true],
        );
        let default_keys = vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use"), string::utf8(b"TOKEN_BURNABLE_BY_OWNER")];
        let default_vals = vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5), bcs::to_bytes<bool>(&true)];
        let default_types = vector<String>[string::utf8(b"u64"), string::utf8(b"u64"), string::utf8(b"bool")];
        let mutate_setting = vector<bool>[true, true, true, true, true];

        let amount = 10;
        let token_max = 100;

        token::create_token_script(
            creator,
            string::utf8(COLLECTION_NAME),
            string::utf8(TOKEN_NAME),
            string::utf8(b"Hello, Token"),
            amount,
            token_max,
            string::utf8(b"https://velor.dev"),
            signer::address_of(creator),
            100,
            0,
            mutate_setting,
            default_keys,
            default_vals,
            default_types,
        );
        let token_id = token::create_token_id_raw(
            signer::address_of(creator),
            string::utf8(COLLECTION_NAME),
            string::utf8(TOKEN_NAME),
            0
        );
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use")
        ];
        let new_vals = vector<vector<u8>>[
            bcs::to_bytes<u64>(&1), bcs::to_bytes<u64>(&1)
        ];
        let new_types = vector<String>[
            string::utf8(b"u64"), string::utf8(b"u64")
        ];
        let new_token_id = token::mutate_one_token(
            creator,
            signer::address_of(creator),
            token_id,
            new_keys,
            new_vals,
            new_types,
        );
        let updated_pm = token::get_property_map(signer::address_of(creator), new_token_id);
        property_map::update_property_value(
            &mut updated_pm,
            &string::utf8(b"attack"),
            property_map::create_property_value<u64>(&2),
        );

        let creator_addr = signer::address_of(creator);
        let token_data_id = token::create_token_data_id(signer::address_of(creator), string::utf8(COLLECTION_NAME), string::utf8(TOKEN_NAME));
        token::mutate_tokendata_uri(creator, token_data_id, string::utf8(b"new_uri"));
        token::mutate_tokendata_property(creator, token_data_id, new_keys,
            new_vals,
            new_types);
        token::mutate_tokendata_description(creator, token_data_id, string::utf8(b"new_description"));
        let royalty = token::create_royalty(50, 100, creator_addr);
        token::mutate_tokendata_royalty(creator, token_data_id, royalty);
        token::mutate_tokendata_maximum(creator, token_data_id, 1001);

        token::mutate_collection_description(creator, string::utf8(COLLECTION_NAME), string::utf8(b"new_collection_description"));
        token::mutate_collection_uri(creator, string::utf8(COLLECTION_NAME), string::utf8(b"new_collection_uri"));
        token::mutate_collection_maximum(creator, string::utf8(COLLECTION_NAME), 2002);

        let token_0 = token::withdraw_token(creator, token_id, 1);
        token::deposit_token(creator, token_0);

        token::burn(
            creator,
            signer::address_of(creator),
            string::utf8(COLLECTION_NAME),
            string::utf8(TOKEN_NAME),
            0,
            1
        );

        token_transfers::offer(creator, creator_addr, token_id, 1);
        token_transfers::claim(creator, creator_addr, token_id);

        token_transfers::offer(creator, creator_addr, token_id, 1);
        token_transfers::cancel_offer(creator, creator_addr, token_id);

        token::opt_in_direct_transfer(creator, true);
    }

    #[test(creator = @0xAF, owner = @0xBB)]
    fun test(creator: &signer) {
        use 0x1::account;
        account::create_account_for_test(signer::address_of(creator));
        run(creator);
    }
}
