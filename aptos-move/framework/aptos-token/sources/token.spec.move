spec aptos_token::token {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    /// The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
    /// The length of the uri is up to MAX_URI_LENGTH;
    spec create_collection_script (
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        maximum: u64,
        mutate_setting: vector<bool>,
    ) {
        // TODO: `create_collection` cannot cover all aborts.
        pragma aborts_if_is_partial;
        include CreateCollectionAbortsIf;
    }

    /// the length of 'mutate_setting' should maore than five.
    /// The creator of the TokenDataId is signer.
    /// The token_data_id should exist in the creator's collections..
    /// The sum of supply and mint Token is less than maximum.
    spec create_token_script(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        balance: u64,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        mutate_setting: vector<bool>,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ) {
        // TODO: Complex abort condition in "create_tokendata".
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        let token_data_id = spec_create_tokendata(addr, collection, name);
        let creator_addr = token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if token_data_id.creator != addr;
        // aborts_if !table::spec_contains(all_token_data, token_data_id);
        // aborts_if token_data.maximum > 0 && token_data.supply + balance > token_data.maximum;
        aborts_if !exists<Collections>(creator_addr);
        aborts_if balance <= 0;
        include CreateTokenMutabilityConfigAbortsIf;

        include CreateTokenMutabilityConfigAbortsIf;
    }

    spec fun spec_create_tokendata(
        creator: address,
        collection: String,
        name: String): TokenDataId {
        TokenDataId { creator, collection, name }
    }

    /// only creator of the tokendata can mint tokens
    spec mint_script(
        account: &signer,
        token_data_address: address,
        collection: String,
        name: String,
        amount: u64,
    ) {
        //TODO: Complex abort condition in mint_token.
        pragma aborts_if_is_partial;
        let token_data_id = spec_create_token_data_id(
            token_data_address,
            collection,
            name,
        );
        let addr = signer::address_of(account);
        let creator_addr = token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if token_data_id.creator != signer::address_of(account);

        include CreateTokenDataIdAbortsIf{
        creator: token_data_address,
        collection,
        name
        };

        include MintTokenAbortsIf {
        token_data_id
        };
    }

    /// The signer is creator.
    spec mutate_token_properties(
        account: &signer,
        token_owner: address,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        amount: u64,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) {
        //TODO: Abort condition is complex in mutate_one_token function.
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        aborts_if addr != creator;
        include CreateTokenDataIdAbortsIf {
            creator,
            collection: collection_name,
            name: token_name
        };
    }

    spec direct_transfer_script (
        sender: &signer,
        receiver: &signer,
        creators_address: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) {
        // TODO: Unknown error message in direct_transfer function.
        pragma aborts_if_is_partial;
        include CreateTokenDataIdAbortsIf{
            creator: creators_address,
            collection,
            name
        };
    }

    spec opt_in_direct_transfer(account: &signer, opt_in: bool) {
        // TODO: Unknown abort condition.
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        let account_addr = global<account::Account>(addr);
        // aborts_if !exists<TokenStore>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<TokenStore>(addr) && account_addr.guid_creation_num + 4 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<TokenStore>(addr) && account_addr.guid_creation_num + 4 > MAX_U64;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account_addr.guid_creation_num + 9 > account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account_addr.guid_creation_num + 9 > MAX_U64;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
    }

    spec transfer_with_opt_in(
        from: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        to: address,
        amount: u64,
    ) {
        //TODO: Abort condition is complex because of transfer function.
        pragma aborts_if_is_partial;
        include CreateTokenDataIdAbortsIf{
            creator,
            collection: collection_name,
            name: token_name
        };
    }

    spec burn_by_creator(
        creator: &signer,
        owner: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) {
        use aptos_std::simple_map;
        //TODO: Abort condition is complex because of the read_bool in the property_map module.
        pragma aborts_if_is_partial;
        let creator_address = signer::address_of(creator);
        let token_id = spec_create_token_id_raw(creator_address, collection, name, property_version);
        let creator_addr = token_id.token_data_id.creator;
        let collections = borrow_global_mut<Collections>(creator_address);
        let token_data = table::spec_get(
            collections.token_data,
            token_id.token_data_id,
        );
        aborts_if amount <= 0;
        aborts_if !exists<Collections>(creator_addr);
        aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);
        aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_CREATOR));
    }

    /// The token_data_id should exist in token_data.
    spec burn(
        owner: &signer,
        creators_address: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64
    ) {
        use aptos_std::simple_map;
        //TODO: Abort condition is complex because of the read_bool in the property_map module.
        pragma aborts_if_is_partial;
        let token_id = spec_create_token_id_raw(creators_address, collection, name, property_version);
        let creator_addr = token_id.token_data_id.creator;
        let collections = borrow_global_mut<Collections>(creator_addr);
        let token_data = table::spec_get(
            collections.token_data,
            token_id.token_data_id,
        );
        include CreateTokenDataIdAbortsIf {
        creator: creators_address
        };
        aborts_if amount <= 0;
        aborts_if !exists<Collections>(creator_addr);
        aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);
        aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_OWNER));
        aborts_if !string::spec_internal_check_utf8(BURNABLE_BY_OWNER);

    }

    spec fun spec_create_token_id_raw(
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ): TokenId {
        let token_data_id = TokenDataId { creator, collection, name };
        TokenId {
            token_data_id,
            property_version
        }
    }

    /// The description of Collection is mutable.
    spec mutate_collection_description(creator: &signer, collection_name: String, description: String) {
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let collection_data = table::spec_get(global<Collections>(addr).collection_data, collection_name);
        include AssertCollectionExistsAbortsIf {
            creator_address: addr,
            collection_name
        };
        aborts_if !collection_data.mutability_config.description;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// The uri of Collection is mutable.
    spec mutate_collection_uri(creator: &signer, collection_name: String, uri: String) {
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let collection_data = table::spec_get(global<Collections>(addr).collection_data, collection_name);
        aborts_if len(uri.bytes) > MAX_URI_LENGTH;
        include AssertCollectionExistsAbortsIf {
            creator_address: addr,
            collection_name
        };
        aborts_if !collection_data.mutability_config.uri;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// Cannot change maximum from 0 and cannot change maximum to 0.
    /// The maximum should more than suply.
    /// The maxium of Collection is mutable.
    spec mutate_collection_maximum(creator: &signer, collection_name: String, maximum: u64) {
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let collection_data = table::spec_get(global<Collections>(addr).collection_data, collection_name);
        include AssertCollectionExistsAbortsIf {
            creator_address: addr,
            collection_name
        };
        aborts_if collection_data.maximum == 0 || maximum == 0;
        aborts_if maximum < collection_data.supply;
        aborts_if !collection_data.mutability_config.maximum;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// Cannot change maximum from 0 and cannot change maximum to 0.
    /// The maximum should more than suply.
    /// The token maximum is mutable
    spec mutate_tokendata_maximum(creator: &signer, token_data_id: TokenDataId, maximum: u64) {
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let all_token_data = global<Collections>(token_data_id.creator).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        include AssertTokendataExistsAbortsIf;
        aborts_if token_data.maximum == 0 || maximum == 0;
        aborts_if maximum < token_data.supply;
        aborts_if !token_data.mutability_config.maximum;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// The length of uri should less than MAX_URI_LENGTH.
    /// The  creator of token_data_id should exist in Collections.
    /// The token uri is mutable
    spec mutate_tokendata_uri(
        creator: &signer,
        token_data_id: TokenDataId,
        uri: String
    ) {
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let all_token_data = global<Collections>(token_data_id.creator).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        include AssertTokendataExistsAbortsIf;
        aborts_if len(uri.bytes) > MAX_URI_LENGTH;
        aborts_if !token_data.mutability_config.uri;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

   /// The token royalty is mutable
    spec mutate_tokendata_royalty(creator: &signer, token_data_id: TokenDataId, royalty: Royalty) {
        include AssertTokendataExistsAbortsIf;
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let all_token_data = global<Collections>(token_data_id.creator).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if !token_data.mutability_config.royalty;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// The token description is mutable
    spec mutate_tokendata_description(creator: &signer, token_data_id: TokenDataId, description: String) {
        include AssertTokendataExistsAbortsIf;
        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let all_token_data = global<Collections>(token_data_id.creator).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if !token_data.mutability_config.description;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<token_event_store::TokenEventStoreV1>(addr) && account.guid_creation_num + 9 > MAX_U64;
    }

    /// The property map is mutable
    spec mutate_tokendata_property(
        creator: &signer,
        token_data_id: TokenDataId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) {
        // TODO: Can't handle abort in loop.
        pragma aborts_if_is_partial;
        let all_token_data = global<Collections>(token_data_id.creator).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        include AssertTokendataExistsAbortsIf;
        aborts_if len(keys) != len(values);
        aborts_if len(keys) != len(types);
        aborts_if !token_data.mutability_config.properties;
    }

    /// The signer is creator.
    /// The token_data_id should exist in token_data.
    /// The property map is mutable.
    spec mutate_one_token(
        account: &signer,
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ): TokenId {
        use aptos_std::simple_map;
        //TODO: Abort condition is complex because of the read_bool funtion in the property_map module.
        pragma aborts_if_is_partial;
        let creator = token_id.token_data_id.creator;
        let addr = signer::address_of(account);
        let all_token_data = global<Collections>(creator).token_data;
        let token_data = table::spec_get(all_token_data, token_id.token_data_id);
        aborts_if addr != creator;
        aborts_if !exists<Collections>(creator);
        aborts_if !table::spec_contains(all_token_data, token_id.token_data_id);
        aborts_if !token_data.mutability_config.properties && !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(TOKEN_PROPERTY_MUTABLE));
    }

    spec create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): Royalty {
        include CreateRoyaltyAbortsIf;
    }

    /// The royalty_points_numerator should less than royalty_points_denominator.
    spec schema CreateRoyaltyAbortsIf {
        royalty_points_numerator: u64;
        royalty_points_denominator: u64;
        payee_address: address;
        aborts_if royalty_points_numerator > royalty_points_denominator;
    }

    spec deposit_token(account: &signer, token: Token) {
        // TODO: boogie error: invalid type for argument 1 in application of $ResourceExists: $1_account_Account (expected: int)
        pragma verify = false;
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        include !exists<TokenStore>(account_addr) ==> InitializeTokenStore;
        let token_id = token.id;
        let token_amount = token.amount;
        include DirectDepositAbortsIf;
    }

    /// The token can direct_transfer.
    spec direct_deposit_with_opt_in(account_addr: address, token: Token) {
        let opt_in_transfer = global<TokenStore>(account_addr).direct_transfer;
        aborts_if !exists<TokenStore>(account_addr);
        aborts_if !opt_in_transfer;
        let token_id = token.id;
        let token_amount = token.amount;
        include DirectDepositAbortsIf;
    }

    /// Cannot withdraw 0 tokens.
    /// Make sure the account has sufficient tokens to withdraw.
    spec direct_transfer(
        sender: &signer,
        receiver: &signer,
        token_id: TokenId,
        amount: u64,
    ) {
        //TODO: Unable to get thef value of token.
        pragma verify = false;
    }

    spec initialize_token_store(account: &signer) {
        include InitializeTokenStore;
    }

    spec schema InitializeTokenStore {
        account: signer;

        let addr = signer::address_of(account);
        let account_addr = global<account::Account>(addr);
        // aborts_if !exists<TokenStore>(addr) && !exists<account::Account>(addr);
        // aborts_if !exists<TokenStore>(addr) && account_addr.guid_creation_num + 4 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<TokenStore>(addr) && account_addr.guid_creation_num + 4 > MAX_U64;
    }

    spec merge(dst_token: &mut Token, source_token: Token) {
        aborts_if dst_token.id != source_token.id;
        aborts_if dst_token.amount + source_token.amount > MAX_U64;
    }

    spec split(dst_token: &mut Token, amount: u64): Token {
        aborts_if dst_token.id.property_version != 0;
        aborts_if dst_token.amount <= amount;
        aborts_if amount <= 0;
    }

    spec transfer(
        from: &signer,
        id: TokenId,
        to: address,
        amount: u64,
    ) {
        let opt_in_transfer = global<TokenStore>(to).direct_transfer;
        let account_addr = signer::address_of(from);
        aborts_if !opt_in_transfer;
        // TODO: Unable to get token value through spec fun.
        pragma aborts_if_is_partial;
        include WithdrawWithEventInternalAbortsIf;
    }

    spec withdraw_with_capability(
        withdraw_proof: WithdrawCapability,
    ): Token {
        let now_seconds = global<timestamp::CurrentTimeMicroseconds>(@aptos_framework).microseconds;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR > withdraw_proof.expiration_sec;
        include WithdrawWithEventInternalAbortsIf{
        account_addr: withdraw_proof.token_owner,
        id: withdraw_proof.token_id,
        amount: withdraw_proof.amount};
    }

    spec partial_withdraw_with_capability(
        withdraw_proof: WithdrawCapability,
        withdraw_amount: u64,
    ): (Token, Option<WithdrawCapability>) {
        let now_seconds = global<timestamp::CurrentTimeMicroseconds>(@aptos_framework).microseconds;
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR > withdraw_proof.expiration_sec;
        aborts_if withdraw_amount > withdraw_proof.amount;
        include WithdrawWithEventInternalAbortsIf{
            account_addr: withdraw_proof.token_owner,
            id: withdraw_proof.token_id,
            amount: withdraw_amount
        };
    }

    /// Cannot withdraw 0 tokens.
    /// Make sure the account has sufficient tokens to withdraw.
    spec withdraw_token(
        account: &signer,
        id: TokenId,
        amount: u64,
    ): Token {
        let account_addr = signer::address_of(account);
        include WithdrawWithEventInternalAbortsIf;
    }

    /// The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
    /// The length of the uri is up to MAX_URI_LENGTH;
    /// The collection_data should not exist before you create it.
    spec create_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        maximum: u64,
        mutate_setting: vector<bool>
    ) {
        // TODO: Complex abort condition.
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(creator);
        aborts_if len(name.bytes) > 128;
        aborts_if len(uri.bytes) > 512;
        include CreateCollectionAbortsIf;
    }

    spec schema CreateCollectionAbortsIf {
        creator: signer;
        name: String;
        description: String;
        uri: String;
        maximum: u64;
        mutate_setting: vector<bool>;

        let addr = signer::address_of(creator);
        let account = global<account::Account>(addr);
        let collection = global<Collections>(addr);
        let b = !exists<Collections>(addr);
        let collection_data = global<Collections>(addr).collection_data;
        // TODO: The collection_data should not exist before you create it.
        // aborts_if table::spec_contains(collection_data, name);
        // aborts_if b && !exists<account::Account>(addr);
        // aborts_if len(name.bytes) > MAX_COLLECTION_NAME_LENGTH;
        // aborts_if len(uri.bytes) > MAX_URI_LENGTH;
        // aborts_if b && account.guid_creation_num + 3 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if b && account.guid_creation_num + 3 > MAX_U64;
        include CreateCollectionMutabilityConfigAbortsIf;
    }

    spec check_collection_exists(creator: address, name: String): bool {
        aborts_if !exists<Collections>(creator);
    }

    /// The length of collection should less than MAX_COLLECTION_NAME_LENGTH
    /// The length of name should less than MAX_NFT_NAME_LENGTH
    spec check_tokendata_exists(creator: address, collection_name: String, token_name: String): bool {
        aborts_if !exists<Collections>(creator);
        include CreateTokenDataIdAbortsIf {
            creator,
            collection: collection_name,
            name: token_name
        };
    }

    /// The length of collection should less than MAX_COLLECTION_NAME_LENGTH
    /// The length of name should less than MAX_NFT_NAME_LENGTH
    spec create_tokendata(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        token_mutate_config: TokenMutabilityConfig,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ): TokenDataId {
        // TODO: Complex abort condition in "roperty_map::new".
        pragma verify = false;
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        let collections = global<Collections>(account_addr);
        let token_data_id = spec_create_token_data_id(account_addr, collection, name);
        let Collection = table::spec_get(collections.collection_data, token_data_id.collection);
        let length = len(property_keys);
        aborts_if len(name.bytes) > MAX_NFT_NAME_LENGTH;
        aborts_if len(collection.bytes) > MAX_COLLECTION_NAME_LENGTH;
        aborts_if len(uri.bytes) > MAX_URI_LENGTH;
        aborts_if royalty_points_numerator > royalty_points_denominator;
        aborts_if !exists<Collections>(account_addr);
        include CreateTokenDataIdAbortsIf {
            creator: account_addr,
            collection,
            name
        };
        aborts_if !table::spec_contains(collections.collection_data, collection);
        aborts_if table::spec_contains(collections.token_data, token_data_id);
        aborts_if Collection.maximum > 0 && Collection.supply + 1 > MAX_U64;
        aborts_if Collection.maximum > 0 && Collection.maximum < Collection.supply + 1;
        include CreateRoyaltyAbortsIf {
            payee_address: royalty_payee_address
        };
        aborts_if length > property_map::MAX_PROPERTY_MAP_SIZE;
        aborts_if length != len(property_values);
        aborts_if length != len(property_types);
    }

    spec fun spec_create_token_data_id(
        creator: address,
        collection: String,
        name: String,
    ): TokenDataId {
        TokenDataId { creator, collection, name }
    }

    spec get_collection_supply(creator_address: address, collection_name: String): Option<u64> {
        include AssertCollectionExistsAbortsIf;
    }

    spec get_collection_description(creator_address: address, collection_name: String): String {
        include AssertCollectionExistsAbortsIf;
    }

    spec get_collection_uri(creator_address: address, collection_name: String): String {
        include AssertCollectionExistsAbortsIf;
    }

    spec get_collection_maximum(creator_address: address, collection_name: String): u64 {
        include AssertCollectionExistsAbortsIf;
    }

    spec get_token_supply(creator_address: address, token_data_id: TokenDataId): Option<u64> {
        aborts_if !exists<Collections>(creator_address);
        let all_token_data = global<Collections>(creator_address).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_tokendata_largest_property_version(creator_address: address, token_data_id: TokenDataId): u64 {
        aborts_if !exists<Collections>(creator_address);
        let all_token_data = global<Collections>(creator_address).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    /// The length of 'mutate_setting' should more than five.
    /// The mutate_setting shuold have a value.
    spec create_token_mutability_config(mutate_setting: &vector<bool>): TokenMutabilityConfig  {
        include CreateTokenMutabilityConfigAbortsIf;
    }

    spec schema CreateTokenMutabilityConfigAbortsIf {
        mutate_setting: vector<bool>;
        aborts_if len(mutate_setting) < 5;
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_MAX_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_URI_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_ROYALTY_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_DESCRIPTION_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_PROPERTY_MUTABLE_IND]);
    }

    spec create_collection_mutability_config {
        include CreateCollectionMutabilityConfigAbortsIf;
    }

    spec schema CreateCollectionMutabilityConfigAbortsIf {
        mutate_setting: vector<bool>;
        aborts_if len(mutate_setting) < 3;
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_DESCRIPTION_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_URI_MUTABLE_IND]);
        aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_MAX_MUTABLE_IND]);
    }

    /// The creator of the TokenDataId is signer.
    /// The token_data_id should exist in the creator's collections..
    /// The sum of supply and the amount of mint Token is less than maximum.
    spec mint_token(
        account: &signer,
        token_data_id: TokenDataId,
        amount: u64,
    ): TokenId {
        //TODO: Cannot get the value of Token for deposit_token function.
        // pragma aborts_if_is_partial;
        pragma verify = false;
        // include MintTokenAbortsIf;
        // let addr = signer::address_of(account);
        // let creator_addr = token_data_id.creator;
        // aborts_if token_data_id.creator != addr;
        // aborts_if !table::spec_contains(all_token_data, token_data_id);
        // let token_data = table::spec_get(all_token_data, token_data_id);
        // let all_token_data = global<Collections>(creator_addr).token_data;
        // aborts_if token_data.maximum > 0 ==> token_data.supply + amount > token_data.maximum;
    }

    spec schema MintTokenAbortsIf {
        account: signer;
        token_data_id: TokenDataId;
        amount: u64;

        let addr = signer::address_of(account);
        let creator_addr = token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if token_data_id.creator != addr;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
        aborts_if token_data.maximum > 0 && token_data.supply + amount > token_data.maximum;
        aborts_if !exists<Collections>(creator_addr);
        aborts_if amount <= 0;
        include InitializeTokenStore;

        let token_id = create_token_id(token_data_id, 0);
        // aborts_if !exists<TokenStore>(addr);
        // let token_store = global<TokenStore>(addr);
        // let recipient_token = table::spec_get(token_store.tokens, token_id);
        // let b = table::spec_contains(token_store.tokens, token_id);
        // aborts_if amount <= 0;
        // aborts_if b && recipient_token.id != token_id;
        // aborts_if b && recipient_token.amount + amount > MAX_U64;
    }

    spec mint_token_to(
        account: &signer,
        receiver: address,
        token_data_id: TokenDataId,
        amount: u64,
    ) {
        let addr = signer::address_of(account);
        let opt_in_transfer = global<TokenStore>(receiver).direct_transfer;
        let creator_addr = token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        let token_data = table::spec_get(all_token_data, token_data_id);
        aborts_if !exists<TokenStore>(receiver);
        aborts_if !opt_in_transfer;
        aborts_if token_data_id.creator != addr;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
        aborts_if token_data.maximum > 0 && token_data.supply + amount > token_data.maximum;
        aborts_if amount <= 0;
        aborts_if !exists<Collections>(creator_addr);

        let token_id = create_token_id(token_data_id, 0);

        include DirectDepositAbortsIf {
            account_addr: receiver,
            token_id,
            token_amount: amount,
        };
    }

    /// The length of collection should less than MAX_COLLECTION_NAME_LENGTH
    /// The length of name should less than MAX_NFT_NAME_LENGTH
    spec create_token_data_id(
        creator: address,
        collection: String,
        name: String,
    ): TokenDataId {
        include CreateTokenDataIdAbortsIf;
    }

    spec schema CreateTokenDataIdAbortsIf {
        creator: address;
        collection: String;
        name: String;
        aborts_if len(collection.bytes) > MAX_COLLECTION_NAME_LENGTH;
        aborts_if len(name.bytes) > MAX_NFT_NAME_LENGTH;
    }

    /// The length of collection should less than MAX_COLLECTION_NAME_LENGTH
    /// The length of name should less than MAX_NFT_NAME_LENGTH
    spec create_token_id_raw(
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ): TokenId {
        include CreateTokenDataIdAbortsIf;
    }

    spec fun spec_balance_of(owner: address, id: TokenId): u64 {
        let token_store = borrow_global<TokenStore>(owner);
        if (!exists<TokenStore>(owner)) {
            0
        }
        else if (table::spec_contains(token_store.tokens, id)) {
            table::spec_get(token_store.tokens, id).amount
        } else {
            0
        }
    }

    spec get_royalty(token_id: TokenId): Royalty {
        include GetTokendataRoyaltyAbortsIf {
            token_data_id: token_id.token_data_id
        };
    }

    spec get_property_map(owner: address, token_id: TokenId): PropertyMap {
        let creator_addr = token_id.token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        aborts_if spec_balance_of(owner, token_id) <= 0;
        aborts_if token_id.property_version == 0 && !table::spec_contains(all_token_data, token_id.token_data_id);
        aborts_if token_id.property_version == 0 && !exists<Collections>(creator_addr);
    }

    spec get_tokendata_maximum(token_data_id: TokenDataId): u64 {
        let creator_address = token_data_id.creator;
        aborts_if !exists<Collections>(creator_address);
        let all_token_data = global<Collections>(creator_address).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_tokendata_uri(creator: address, token_data_id: TokenDataId): String {
        aborts_if !exists<Collections>(creator);
        let all_token_data = global<Collections>(creator).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_tokendata_description(token_data_id: TokenDataId): String {
        let creator_address = token_data_id.creator;
        aborts_if !exists<Collections>(creator_address);
        let all_token_data = global<Collections>(creator_address).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_tokendata_royalty(token_data_id: TokenDataId): Royalty {
        include GetTokendataRoyaltyAbortsIf;
    }

    spec schema GetTokendataRoyaltyAbortsIf {
        token_data_id: TokenDataId;
        let creator_address = token_data_id.creator;
        let all_token_data = global<Collections>(creator_address).token_data;
        aborts_if !exists<Collections>(creator_address);
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_tokendata_mutability_config(token_data_id: TokenDataId): TokenMutabilityConfig {
        let creator_addr = token_data_id.creator;
        let all_token_data = global<Collections>(creator_addr).token_data;
        aborts_if !exists<Collections>(creator_addr);
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec get_collection_mutability_config(
        creator: address,
        collection_name: String
    ): CollectionMutabilityConfig {
        let all_collection_data = global<Collections>(creator).collection_data;
        aborts_if !exists<Collections>(creator);
        aborts_if !table::spec_contains(all_collection_data, collection_name);
    }

    spec withdraw_with_event_internal(
        account_addr: address,
        id: TokenId,
        amount: u64,
    ): Token {
        include WithdrawWithEventInternalAbortsIf;
    }

    spec schema WithdrawWithEventInternalAbortsIf {
        account_addr: address;
        id: TokenId;
        amount: u64;
        let tokens = global<TokenStore>(account_addr).tokens;
        aborts_if amount <= 0;
        aborts_if spec_balance_of(account_addr, id) < amount;
        aborts_if !exists<TokenStore>(account_addr);
        aborts_if !table::spec_contains(tokens, id);
    }

    spec update_token_property_internal (
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) {
        //TODO: Abort in `property_map::update_property_map` loop cannot be handled
        pragma aborts_if_is_partial;
        let tokens = global<TokenStore>(token_owner).tokens;
        aborts_if !exists<TokenStore>(token_owner);
        aborts_if !table::spec_contains(tokens, token_id);
    }

    spec direct_deposit(account_addr: address, token: Token) {
        let token_id = token.id;
        let token_amount = token.amount;
        include DirectDepositAbortsIf;
    }

    spec schema DirectDepositAbortsIf {
        account_addr: address;
        token_id: TokenId;
        token_amount: u64;
        let token_store = global<TokenStore>(account_addr);
        let recipient_token = table::spec_get(token_store.tokens, token_id);
        let b = table::spec_contains(token_store.tokens, token_id);
        aborts_if token_amount <= 0;
        aborts_if !exists<TokenStore>(account_addr);
        aborts_if b && recipient_token.id != token_id;
        aborts_if b && recipient_token.amount + token_amount > MAX_U64;
    }

    /// The collection_name should exist in collection_data of the creator_address's Collections.
    spec assert_collection_exists(creator_address: address, collection_name: String) {
        include AssertCollectionExistsAbortsIf;
    }

    spec schema AssertCollectionExistsAbortsIf {
        creator_address: address;
        collection_name: String;
        let all_collection_data = global<Collections>(creator_address).collection_data;
        aborts_if !exists<Collections>(creator_address);
        aborts_if !table::spec_contains(all_collection_data, collection_name);
    }

    /// The creator of token_data_id should be signer.
    /// The  creator of token_data_id exists in Collections.
    /// The token_data_id is in the all_token_data.
    spec assert_tokendata_exists(creator: &signer, token_data_id: TokenDataId) {
        include AssertTokendataExistsAbortsIf;
    }

    spec schema AssertTokendataExistsAbortsIf {
        creator: signer;
        token_data_id: TokenDataId;
        let creator_addr = token_data_id.creator;
        let addr = signer::address_of(creator);
        aborts_if addr != creator_addr;
        aborts_if !exists<Collections>(creator_addr);
        let all_token_data = global<Collections>(creator_addr).token_data;
        aborts_if !table::spec_contains(all_token_data, token_data_id);
    }

    spec assert_non_standard_reserved_property(keys: &vector<String>) {
        // TODO: Can't handle abort in loop.
        pragma verify = false;
    }

    /// Deprecated function
    spec initialize_token_script(_account: &signer) {
        pragma verify = false;
    }

    /// Deprecated function
    spec initialize_token(_account: &signer, _token_id: TokenId) {
        pragma verify = false;
    }
}
