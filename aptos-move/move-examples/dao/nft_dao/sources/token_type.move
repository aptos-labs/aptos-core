// token_type provides a method to tell if a token is NFT, NFT print or a fungible token
// NFT: only one token is allowed to mint from this TokenData. The maximum in TokenData is 1 and the field is immutable
//
// NFT_PRINT: this is a NFT with property_version > 0. Only one token has the same token_id
// But, the maximum of the TokenData is not 1 or the maximum field is mutable at the same time.
// Note: if token maximum is immutable and set to be 1, the token is still a NFT if property_version > 0
//
// FUNGIBLE: a non-decimal token that can have an amount bigger than 1
// NFT DAO only allow globally unqiue token to vote. ONly NFT and NFT_PRINT are globally unqiue token.
module dao_platform::token_type {

    use aptos_token::token::{Self, TokenId};

    //
    // Constants
    //

    // Only one token is allowed to mint from this TokenData. The maximum in token data is 1 and immutable
    const NFT: u64 = 0;

    // This is a NFT with property_version > 0. Only one token has the same token_id
    // But, the maximum of the TokenData is not 1 or the maximum field is mutable at the same time.
    // Note: if token maximum is immutable and set to be 1, the token is still a NFT if property_version > 0
    const NFT_PRINT: u64 = 1;

    // This is a fungible token
    const FUNGIBLE: u64 = 2;

    public fun get_token_type(token_id: TokenId): u64 {
        let token_data_id = token::get_tokendata_id(token_id);
        let (_, _, _, property_version) = token::get_token_id_fields(&token_id);
        let (flag, _, _, _, _) = token::get_tokendata_mutability_config(token_data_id);
        let maximum = token::get_tokendata_maximum(token_data_id);
        if (maximum == 1 && !flag){
            NFT
        } else if (property_version > 0) {
            NFT_PRINT
        } else {
            FUNGIBLE
        }
    }

    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use std::signer;
    #[test_only]
    use aptos_token::token::{create_collection_and_token};
    #[test_only]
    use std::string::String;
    #[test_only]
    use std::string;
    #[test_only]
    use std::bcs;

    #[test(creator = @0xcafe)]
    fun test_token_is_nft(creator: &signer){
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            1,
            4,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        assert!(get_token_type(token_id) == NFT, 1);
    }

    #[test(creator = @0xabc)]
    fun test_token_is_nft_after_mutation(creator: &signer){
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            1,
            4,
            1,
            vector<String>[string::utf8(b"TOKEN_PROPERTY_MUTATBLE")],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let (creator_addr, _, _, _) = token::get_token_id_fields(&token_id);
        let new_token_id = token::mutate_one_token(
            creator,
            creator_addr,
            token_id,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[]
        );
        assert!(get_token_type(new_token_id) == NFT, 1);
    }

    #[test(creator = @0xc)]
    fun test_token_is_nft_print(creator: &signer){
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            2,
            vector<String>[string::utf8(b"TOKEN_PROPERTY_MUTATBLE")],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let (creator_addr, _, _, _) = token::get_token_id_fields(&token_id);
        let new_token_id = token::mutate_one_token(
            creator,
            creator_addr,
            token_id,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[]
        );
        assert!(get_token_type(new_token_id) == NFT_PRINT, 1);
    }

    #[test(creator = @0xb)]
    fun test_token_is_fungible(creator: &signer){
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        assert!(get_token_type(token_id) == FUNGIBLE, 1);
    }

    #[test(creator = @0xa)]
    fun test_token_is_fungible_with_mutatability(creator: &signer){
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            1,
            4,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[true, false, false, false, false],
        );
        assert!(get_token_type(token_id) == FUNGIBLE, 1);
    }
}
