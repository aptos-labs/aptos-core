/// Collection registry for the NFT marketplace benchmark, shaped after a
/// Topaz-style launchpad: the package owns the collections through one object,
/// and any account may mint from them.
///
/// Collections are `aptos_token_objects::collection` fixed-supply collections
/// with a royalty, so token data lands in `ObjectGroup` and token addresses
/// are derived rather than stored. The package keeps its own mint counter
/// beside the framework's supply aggregator, because a benchmark mint has to
/// clamp at the cap instead of aborting there.
module bench::nftm_collection {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_token_objects::collection::{Self, Collection};
    use aptos_token_objects::royalty;

    /// Only the package address can configure the registry.
    const E_NOT_BENCH: u64 = 1;
    /// No collection with the given index.
    const E_NO_COLLECTION: u64 = 2;
    /// Registry has not been initialized.
    const E_NO_REGISTRY: u64 = 3;

    /// Seed of the object that creates and therefore owns every collection.
    const CREATOR_SEED: vector<u8> = b"nftm_creator";

    /// Denominator royalties are quoted against.
    const BPS_DENOMINATOR: u64 = 10000;

    const COLLECTION_PREFIX: vector<u8> = b"NFTM Collection ";
    const COLLECTION_DESCRIPTION: vector<u8> = b"MonoMove benchmark collection";
    const COLLECTION_URI: vector<u8> = b"https://bench.invalid/collection";

    struct CollectionInfo has store {
        addr: address,
        name: String,
        mutator_ref: collection::MutatorRef,
        /// Tokens minted from this collection, and the index the next one
        /// takes. Indices below this are exactly the ones a derived-address
        /// lookup may name.
        minted: u64,
        supply_cap: u64,
    }

    struct Registry has key {
        creator_ref: ExtendRef,
        collections: Table<u64, CollectionInfo>,
        n_collections: u64,
    }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let constructor_ref = object::create_named_object(admin, CREATOR_SEED);
        move_to(
            admin,
            Registry {
                creator_ref: object::generate_extend_ref(&constructor_ref),
                collections: table::new(),
                n_collections: 0,
            },
        );
    }

    public entry fun create_collection(
        admin: &signer, index: u64, supply_cap: u64, royalty_bps: u64
    ) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(exists<Registry>(@bench), E_NO_REGISTRY);
        let registry = borrow_global_mut<Registry>(@bench);
        let creator =
            object::generate_signer_for_extending(&registry.creator_ref);
        let name = collection_name_of(index);
        // `royalty::create` refuses a royalty above 100%, and the benchmark's
        // setup stage may not abort.
        let royalty_bps =
            if (royalty_bps > BPS_DENOMINATOR) BPS_DENOMINATOR else royalty_bps;
        // A fixed collection of zero supply is refused the same way.
        let supply_cap = if (supply_cap == 0) 1 else supply_cap;
        let constructor_ref = collection::create_fixed_collection(
            &creator,
            string::utf8(COLLECTION_DESCRIPTION),
            supply_cap,
            name,
            option::some(
                royalty::create(royalty_bps, BPS_DENOMINATOR, @bench)),
            string::utf8(COLLECTION_URI),
        );
        table::add(
            &mut registry.collections,
            index,
            CollectionInfo {
                addr: collection::create_collection_address(
                    &signer::address_of(&creator), &name),
                name,
                mutator_ref: collection::generate_mutator_ref(&constructor_ref),
                minted: 0,
                supply_cap,
            },
        );
        if (index >= registry.n_collections) {
            registry.n_collections = index + 1;
        }
    }

    /// Collections are owned by an object the package controls, so a mint
    /// needs that object's signer. Any account may take it on purpose:
    /// routing every mint through the admin would serialize the whole account
    /// pool on the admin's sequence number.
    public fun creator_signer(): signer acquires Registry {
        assert!(exists<Registry>(@bench), E_NO_REGISTRY);
        object::generate_signer_for_extending(
            &borrow_global<Registry>(@bench).creator_ref)
    }

    public fun creator_address(): address {
        object::create_object_address(&@bench, CREATOR_SEED)
    }

    /// Take up to `n` token indices from the collection's remaining supply.
    /// Returns the first index taken and how many were actually available, so
    /// a mint at the cap does nothing instead of aborting.
    public fun reserve(index: u64, n: u64): (u64, u64) acquires Registry {
        assert!(exists<Registry>(@bench), E_NO_REGISTRY);
        let registry = borrow_global_mut<Registry>(@bench);
        assert!(table::contains(&registry.collections, index), E_NO_COLLECTION);
        let info = table::borrow_mut(&mut registry.collections, index);
        let remaining = info.supply_cap - info.minted;
        let taken = if (n < remaining) n else remaining;
        let first = info.minted;
        info.minted = info.minted + taken;
        (first, taken)
    }

    /// Fold `token_index` into the minted range, so an index past it names a
    /// token that exists rather than one that does not. Every token ever
    /// minted stays inside this range, so none of them becomes unreachable.
    public fun wrap_token(index: u64, token_index: u64): u64 acquires Registry {
        let minted = minted(index);
        if (minted == 0) 0 else token_index % minted
    }

    public fun wrap_collection(index: u64): u64 acquires Registry {
        let n = n_collections();
        if (n == 0) 0 else index % n
    }

    /// Update a collection's URI. Admin-only because it writes a resource
    /// every mint of the collection reads.
    public entry fun set_collection_uri(
        admin: &signer, index: u64, uri: vector<u8>
    ) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(exists<Registry>(@bench), E_NO_REGISTRY);
        let registry = borrow_global<Registry>(@bench);
        assert!(table::contains(&registry.collections, index), E_NO_COLLECTION);
        let info = table::borrow(&registry.collections, index);
        collection::set_uri(&info.mutator_ref, string::utf8(uri));
    }

    /// Everything about a collection that is worth copying out: a reference
    /// into a global cannot leave the function that borrowed it.
    fun info_of(index: u64): (address, String, u64, u64) acquires Registry {
        assert!(exists<Registry>(@bench), E_NO_REGISTRY);
        let registry = borrow_global<Registry>(@bench);
        assert!(
            table::contains(&registry.collections, index), E_NO_COLLECTION);
        let info = table::borrow(&registry.collections, index);
        (info.addr, info.name, info.minted, info.supply_cap)
    }

    public fun collection_name_of(index: u64): String {
        let name = COLLECTION_PREFIX;
        vector::append(&mut name, decimal(index));
        string::utf8(name)
    }

    /// Decimal digits of `n`. Names are built here rather than through
    /// `string_utils`, so that no formatting native sits on the mint path.
    public fun decimal(n: u64): vector<u8> {
        if (n == 0) return b"0";
        let digits = vector::empty<u8>();
        while (n > 0) {
            vector::push_back(&mut digits, ((48 + n % 10) as u8));
            n = n / 10;
        };
        vector::reverse(&mut digits);
        digits
    }

    public fun exists_collection(index: u64): bool acquires Registry {
        exists<Registry>(@bench)
            && table::contains(
                &borrow_global<Registry>(@bench).collections, index)
    }

    #[view]
    public fun n_collections(): u64 acquires Registry {
        if (!exists<Registry>(@bench)) 0
        else borrow_global<Registry>(@bench).n_collections
    }

    #[view]
    public fun collection_address(index: u64): address acquires Registry {
        let (addr, _, _, _) = info_of(index);
        addr
    }

    public fun collection_object(index: u64): Object<Collection> acquires Registry {
        object::address_to_object<Collection>(collection_address(index))
    }

    #[view]
    public fun collection_name(index: u64): String acquires Registry {
        let (_, name, _, _) = info_of(index);
        name
    }

    #[view]
    public fun minted(index: u64): u64 acquires Registry {
        let (_, _, minted, _) = info_of(index);
        minted
    }

    #[view]
    public fun supply_cap(index: u64): u64 acquires Registry {
        let (_, _, _, supply_cap) = info_of(index);
        supply_cap
    }

    #[view]
    public fun remaining_supply(index: u64): u64 acquires Registry {
        let (_, _, minted, supply_cap) = info_of(index);
        supply_cap - minted
    }
}
