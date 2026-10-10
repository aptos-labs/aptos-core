/// Minting and token bookkeeping for the NFT marketplace benchmark.
///
/// Tokens are `aptos_token_objects::token` named tokens, so their addresses
/// are derived from the creator, the collection name and the token name, and
/// their data sits in `ObjectGroup`. `TokenIndex` is the package's own group
/// member next to the framework's `Token`: the sale counter a marketplace
/// keeps per token, written on every transfer.
module bench::nftm_token {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_framework::object::{Self, Object, TransferRef};
    use aptos_token_objects::token;
    use bench::nftm_collection;
    use bench::nftm_events;

    /// Only the package address can seed tokens.
    const E_NOT_BENCH: u64 = 1;

    /// Inventory entries an account keeps. Fixed, so a run of any length
    /// leaves the resource the same size and the benchmark measures the
    /// marketplace rather than a growing vector. The ring is a working set,
    /// not a title register: an entry it drops names a token that is still
    /// listed, still owned and still reachable by its derived address.
    const INVENTORY_CAP: u64 = 8;

    /// Ceiling on the tokens one transaction may mint or read, whatever the
    /// caller asks for.
    const MAX_MINT_BATCH: u64 = 32;
    const MAX_READ_BATCH: u64 = 64;

    const TOKEN_PREFIX: vector<u8> = b"Token #";
    const TOKEN_DESCRIPTION: vector<u8> = b"MonoMove benchmark token";
    const TOKEN_URI: vector<u8> = b"https://bench.invalid/token";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The marketplace's per-token record, alongside the framework's `Token`
    /// in the same resource group.
    struct TokenIndex has key {
        collection_index: u64,
        token_index: u64,
        transfers: u64,
        /// A sale moves a token whose owner never signs for it.
        transfer_ref: TransferRef,
    }

    /// Ring of the tokens an account most recently acquired.
    struct Inventory has key {
        tokens: vector<address>,
        cursor: u64,
    }

    // Addresses.

    public fun token_name(token_index: u64): String {
        let name = TOKEN_PREFIX;
        vector::append(&mut name, nftm_collection::decimal(token_index));
        string::utf8(name)
    }

    /// Where the token at `token_index` of a collection lives, whether or not
    /// it has been minted yet.
    public fun token_address(
        collection_index: u64, token_index: u64
    ): address {
        token::create_token_address(
            &nftm_collection::creator_address(),
            &nftm_collection::collection_name(collection_index),
            &token_name(token_index),
        )
    }

    /// Address of a token that exists: `token_index` is folded into the
    /// collection's minted range first.
    public fun resolve(collection_index: u64, token_index: u64): address {
        let collection_index = nftm_collection::wrap_collection(collection_index);
        token_address(
            collection_index,
            nftm_collection::wrap_token(collection_index, token_index),
        )
    }

    public fun token_exists(token_addr: address): bool {
        exists<TokenIndex>(token_addr)
    }

    public fun owner_of(token_addr: address): address {
        object::owner(object::address_to_object<TokenIndex>(token_addr))
    }

    public fun collection_of(token_addr: address): u64 acquires TokenIndex {
        borrow_global<TokenIndex>(token_addr).collection_index
    }

    // Minting.

    /// Mint up to `n` tokens of `collection_index` to `owner`, clamped to the
    /// remaining supply. Returns how many were minted.
    public fun mint_to(
        owner: address, collection_index: u64, n: u64
    ): u64 acquires Inventory {
        let collection_index = nftm_collection::wrap_collection(collection_index);
        if (!nftm_collection::exists_collection(collection_index)) return 0;
        let n = if (n < MAX_MINT_BATCH) n else MAX_MINT_BATCH;
        let (first, taken) = nftm_collection::reserve(collection_index, n);
        let creator = nftm_collection::creator_signer();
        let name = nftm_collection::collection_name(collection_index);
        let i = 0;
        while (i < taken) {
            let token_addr =
                mint_one(&creator, name, collection_index, first + i, owner);
            inventory_push(owner, token_addr);
            nftm_events::emit_mint(collection_index, token_addr, owner);
            i = i + 1;
        };
        taken
    }

    fun mint_one(
        creator: &signer,
        collection_name: String,
        collection_index: u64,
        token_index: u64,
        owner: address,
    ): address {
        let constructor_ref = token::create_named_token(
            creator,
            collection_name,
            string::utf8(TOKEN_DESCRIPTION),
            token_name(token_index),
            option::none(),
            string::utf8(TOKEN_URI),
        );
        let object_signer = object::generate_signer(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::transfer_with_ref(
            object::generate_linear_transfer_ref(&transfer_ref), owner);
        move_to(
            &object_signer,
            TokenIndex {
                collection_index,
                token_index,
                transfers: 0,
                transfer_ref,
            },
        );
        signer::address_of(&object_signer)
    }

    /// Mint one token of `collection_index` to `owner` and hand back its
    /// address, or `@0x0` when the collection has no supply left.
    public fun mint_one_to(
        owner: address, collection_index: u64
    ): address acquires Inventory {
        let collection_index = nftm_collection::wrap_collection(collection_index);
        if (!nftm_collection::exists_collection(collection_index)) return @0x0;
        let (first, taken) = nftm_collection::reserve(collection_index, 1);
        if (taken == 0) return @0x0;
        let token_addr = mint_one(
            &nftm_collection::creator_signer(),
            nftm_collection::collection_name(collection_index),
            collection_index,
            first,
            owner,
        );
        inventory_push(owner, token_addr);
        nftm_events::emit_mint(collection_index, token_addr, owner);
        token_addr
    }

    /// Mint `count` publisher-owned tokens, which every later derived-address
    /// lookup starts out naming.
    public entry fun seed_tokens(
        admin: &signer, collection_index: u64, count: u64
    ) acquires Inventory {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        ensure_inventory(admin);
        mint_to(signer::address_of(admin), collection_index, count);
    }

    // Transfers.

    /// Move a token whose owner did not sign for it, and count the sale on the
    /// package's own group member.
    public fun transfer(token_addr: address, to: address) acquires Inventory, TokenIndex {
        let index = borrow_global_mut<TokenIndex>(token_addr);
        index.transfers = index.transfers + 1;
        let linear = object::generate_linear_transfer_ref(&index.transfer_ref);
        object::transfer_with_ref(linear, to);
        inventory_push(to, token_addr);
    }

    // Inventory.

    public fun ensure_inventory(user: &signer) {
        if (!exists<Inventory>(signer::address_of(user))) {
            move_to(user, Inventory { tokens: vector::empty(), cursor: 0 });
        }
    }

    /// Record a token against its new owner, overwriting the oldest entry once
    /// the ring is full. A token the ring already names does not push, so
    /// buying back what one just sold costs no other entry its slot.
    public fun inventory_push(owner: address, token_addr: address) acquires Inventory {
        if (!exists<Inventory>(owner)) return;
        let inventory = borrow_global_mut<Inventory>(owner);
        let len = vector::length(&inventory.tokens);
        let i = 0;
        while (i < len) {
            if (*vector::borrow(&inventory.tokens, i) == token_addr) return;
            i = i + 1;
        };
        if (len < INVENTORY_CAP) {
            vector::push_back(&mut inventory.tokens, token_addr);
        } else {
            let slot = inventory.cursor % INVENTORY_CAP;
            *vector::borrow_mut(&mut inventory.tokens, slot) = token_addr;
        };
        inventory.cursor = inventory.cursor + 1;
    }

    /// A token the account holds, or `@0x0` when it holds none. The ring can
    /// name a token the account has since sold, so every caller re-checks
    /// ownership.
    public fun inventory_pick(owner: address, hint: u64): address acquires Inventory {
        if (!exists<Inventory>(owner)) return @0x0;
        let tokens = &borrow_global<Inventory>(owner).tokens;
        let len = vector::length(tokens);
        if (len == 0) return @0x0;
        *vector::borrow(tokens, hint % len)
    }

    /// A token the account holds and still owns, or `@0x0`.
    public fun owned_pick(owner: address, hint: u64): address acquires Inventory {
        let token_addr = inventory_pick(owner, hint);
        if (token_addr == @0x0) return @0x0;
        if (!token_exists(token_addr)) return @0x0;
        if (owner_of(token_addr) != owner) return @0x0;
        token_addr
    }

    public fun inventory_cap(): u64 {
        INVENTORY_CAP
    }

    // Mix entry points.

    /// Read `k` tokens' group members without writing anything, starting at a
    /// derived address and walking forward through the seeded range.
    public entry fun bench_read_index(
        _user: &signer, collection_index: u64, token_index: u64, k: u64
    ) acquires TokenIndex {
        read_index(collection_index, token_index, k);
    }

    public fun read_index(
        collection_index: u64, token_index: u64, k: u64
    ): u64 acquires TokenIndex {
        let k = if (k < MAX_READ_BATCH) k else MAX_READ_BATCH;
        let acc = 0;
        let i = 0;
        while (i < k) {
            let token_addr = resolve(collection_index, token_index + i);
            if (token_exists(token_addr)) {
                let index = borrow_global<TokenIndex>(token_addr);
                let object = object::address_to_object<TokenIndex>(token_addr);
                acc = acc + index.token_index + index.transfers
                    + string::length(&token::uri(object));
            };
            i = i + 1;
        };
        acc
    }

    #[view]
    public fun transfers(token_addr: address): u64 acquires TokenIndex {
        borrow_global<TokenIndex>(token_addr).transfers
    }

    #[view]
    public fun token_index_of(token_addr: address): u64 acquires TokenIndex {
        borrow_global<TokenIndex>(token_addr).token_index
    }

    #[view]
    public fun inventory_size(owner: address): u64 acquires Inventory {
        if (!exists<Inventory>(owner)) 0
        else vector::length(&borrow_global<Inventory>(owner).tokens)
    }

    public fun token_object(token_addr: address): Object<TokenIndex> {
        object::address_to_object<TokenIndex>(token_addr)
    }
}
