/// This module provides the foundation for transferring of Tokens
module AptosFramework::TokenTransfers {
    use Std::GUID::{Self, ID};
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::Token::{Self, Token};

    struct TokenTransfers<TokenType: copy + drop + store> has key {
        pending_transfers: Table<address, Table<ID, Token<TokenType>>>,
    }

    public fun initialize_token_transfers<TokenType: copy + drop + store>(account: &signer) {
        move_to(
            account,
            TokenTransfers<TokenType> {
                pending_transfers: Table::create<address, Table<ID, Token<TokenType>>>(),
            }
        )
    }

    public(script) fun transfer_to_script<TokenType: copy + drop + store>(
        sender: signer,
        receiver: address,
        creator: address,
        token_creation_num: u64,
        amount: u64,
    ) acquires TokenTransfers {
        let token_id = GUID::create_id(creator, token_creation_num);
        transfer_to<TokenType>(&sender, receiver, &token_id, amount);
    }

    // Make an entry into pending transfers and extract from gallery
    public fun transfer_to<TokenType: copy + drop + store>(
        sender: &signer,
        receiver: address,
        token_id: &ID,
        amount: u64,
    ) acquires TokenTransfers {
        let sender_addr = Signer::address_of(sender);
        let pending_transfers =
            &mut borrow_global_mut<TokenTransfers<TokenType>>(sender_addr).pending_transfers;
        if (!Table::contains_key(pending_transfers, &receiver)) {
            Table::insert(pending_transfers, receiver, Table::create())
        };
        let addr_pending_transfers = Table::borrow_mut(pending_transfers, &receiver);

        let token = Token::withdraw_token<TokenType>(sender, token_id, amount);
        let token_id = Token::token_id(&token);
        if (Table::contains_key(addr_pending_transfers, token_id)) {
            let dst_token = Table::borrow_mut(addr_pending_transfers, token_id);
            Token::merge_token(token, dst_token)
        } else {
            Table::insert(addr_pending_transfers, *token_id, token)
        }
    }

    public(script) fun receive_from_script<TokenType: copy + drop + store>(
        receiver: signer,
        sender: address,
        creator: address,
        token_creation_num: u64,
    ) acquires TokenTransfers {
        let token_id = GUID::create_id(creator, token_creation_num);
        receive_from<TokenType>(&receiver, sender, &token_id);
    }

    // Pull from someone else's pending transfers and insert into our gallery
    public fun receive_from<TokenType: copy + drop + store>(
        receiver: &signer,
        sender: address,
        token_id: &ID,
    ) acquires TokenTransfers {
        let receiver_addr = Signer::address_of(receiver);
        let pending_transfers =
            &mut borrow_global_mut<TokenTransfers<TokenType>>(sender).pending_transfers;
        let pending_tokens = Table::borrow_mut(pending_transfers, &receiver_addr);
        let (_id, token) = Table::remove(pending_tokens, token_id);

        if (Table::count(pending_tokens) == 0) {
            let (_id, real_pending_transfers) = Table::remove(pending_transfers, &receiver_addr);
            Table::destroy_empty(real_pending_transfers)
        };

        Token::deposit_token(receiver, token)
    }

    public(script) fun stop_transfer_to_script<TokenType: copy + drop + store>(
        sender: signer,
        receiver: address,
        creator: address,
        token_creation_num: u64,
    ) acquires TokenTransfers {
        let token_id = GUID::create_id(creator, token_creation_num);
        stop_transfer_to<TokenType>(&sender, receiver, &token_id);
    }

    // Extra from our pending_transfers and return to gallery
    public fun stop_transfer_to<TokenType: copy + drop + store>(
        sender: &signer,
        receiver: address,
        token_id: &ID,
    ) acquires TokenTransfers {
        let sender_addr = Signer::address_of(sender);
        let pending_transfers =
            &mut borrow_global_mut<TokenTransfers<TokenType>>(sender_addr).pending_transfers;
        let pending_tokens = Table::borrow_mut(pending_transfers, &receiver);
        let (_id, token) = Table::remove(pending_tokens, token_id);

        if (Table::count(pending_tokens) == 0) {
            let (_id, real_pending_transfers) = Table::remove(pending_transfers, &receiver);
            Table::destroy_empty(real_pending_transfers)
        };

        Token::deposit_token(sender, token)
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun test_nft(creator: signer, owner: signer) acquires TokenTransfers {
        let token_id = create_token(&creator, 1);
        initialize_token_transfers<u64>(&creator);

        let creator_addr = Signer::address_of(&creator);
        let owner_addr = Signer::address_of(&owner);
        transfer_to<u64>(&creator, owner_addr, &token_id, 1);
        receive_from<u64>(&owner, creator_addr, &token_id);

        initialize_token_transfers<u64>(&owner);
        transfer_to<u64>(&owner, creator_addr, &token_id, 1);
        stop_transfer_to<u64>(&owner, creator_addr, &token_id);
    }

    #[test(creator = @0x1, owner0 = @0x2, owner1 = @0x3)]
    public fun test_editions(
        creator: signer,
        owner0: signer,
        owner1: signer,
    ) acquires TokenTransfers {
        let token_id = create_token(&creator, 2);
        initialize_token_transfers<u64>(&creator);

        let creator_addr = Signer::address_of(&creator);
        let owner0_addr = Signer::address_of(&owner0);
        let owner1_addr = Signer::address_of(&owner1);

        assert!(Table::count(&borrow_global<TokenTransfers<u64>>(creator_addr).pending_transfers) == 0, 0);

        transfer_to<u64>(&creator, owner0_addr, &token_id, 1);
        assert!(Table::count(&borrow_global<TokenTransfers<u64>>(creator_addr).pending_transfers) == 1, 1);
        transfer_to<u64>(&creator, owner1_addr, &token_id, 1);
        assert!(Table::count(&borrow_global<TokenTransfers<u64>>(creator_addr).pending_transfers) == 2, 2);
        receive_from<u64>(&owner0, creator_addr, &token_id);
        assert!(Table::count(&borrow_global<TokenTransfers<u64>>(creator_addr).pending_transfers) == 1, 3);
        receive_from<u64>(&owner1, creator_addr, &token_id);
        assert!(Table::count(&borrow_global<TokenTransfers<u64>>(creator_addr).pending_transfers) == 0, 4);

        initialize_token_transfers<u64>(&owner0);
        transfer_to<u64>(&owner0, owner1_addr, &token_id, 1);
        receive_from<u64>(&owner1, owner0_addr, &token_id);

        initialize_token_transfers<u64>(&owner1);
        transfer_to<u64>(&owner1, creator_addr, &token_id, 1);
        transfer_to<u64>(&owner1, creator_addr, &token_id, 1);
        receive_from<u64>(&creator, owner1_addr, &token_id);
    }

    fun create_token(creator: &signer, amount: u64): ID {
        use Std::ASCII;
        use Std::Option;

        let collection_name = ASCII::string(b"Hello, World");
        Token::create_collection<u64>(
            creator,
            ASCII::string(b"Collection: Hello, World"),
            *&collection_name,
            ASCII::string(b"https://aptos.dev"),
            Option::none(),
        );
        Token::create_token<u64>(
            creator,
            collection_name,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            ASCII::string(b"https://aptos.dev"),
            0,
        )
    }
}
