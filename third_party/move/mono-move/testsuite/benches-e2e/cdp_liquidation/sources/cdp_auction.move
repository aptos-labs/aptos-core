/// Collateral auction, in the shape MakerDAO and Thala use to sell what a
/// liquidation seized.
///
/// Lots are chained by id through a table, so buying a run of them is a second
/// sequence of dependent reads alongside the vault list, over table items
/// rather than resources. A step puts back as many lots as it takes, so the
/// chain never drains and the branch is never a no-op.
module bench::cdp_auction {
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use aptos_framework::primary_fungible_store;
    use bench::cdp_assets;

    /// Only the package address opens the auction.
    const E_NOT_BENCH: u64 = 1;

    const COLL_SYMBOL: vector<u8> = b"CDPC";
    const STABLE_SYMBOL: vector<u8> = b"CDPS";

    /// Collateral and asking price of a replenished lot.
    const LOT_COLL: u64 = 50;
    const LOT_PRICE: u64 = 100000;

    struct Lot has store, drop {
        coll: u64,
        price: u64,
        /// Id of the lot behind this one, or zero at the end of the chain.
        next: u64,
    }

    struct Auction has key {
        lots: Table<u64, Lot>,
        head: u64,
        tail: u64,
        len: u64,
        /// Ids count from one, so zero can mark the end of the chain.
        next_id: u64,
        proceeds: u64,
    }

    /// Admin-only. Creates the auction on the first call and adds `lots` to
    /// whatever is already chained.
    public entry fun initialize(admin: &signer, lots: u64) acquires Auction {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Auction>(@bench)) {
            move_to(admin, Auction {
                lots: table::new(),
                head: 0,
                tail: 0,
                len: 0,
                next_id: 1,
                proceeds: 0,
            });
        };
        let book = borrow_global_mut<Auction>(@bench);
        let i = 0;
        while (i < lots) {
            push_lot(book, LOT_COLL, LOT_PRICE);
            i = i + 1;
        }
    }

    /// Take up to `lots` off the head of the chain and put the same number
    /// back on the tail. `lots` is clamped to the chain length, and the walk
    /// stops at the end marker, so an oversized request buys what there is.
    public entry fun bench_auction_step(
        user: &signer, lots: u64
    ) acquires Auction {
        if (!exists<Auction>(@bench)) return;
        let owner = signer::address_of(user);
        let book = borrow_global_mut<Auction>(@bench);
        let take = if (lots > book.len) book.len else lots;
        let cur = book.head;
        let bought = 0;
        let coll = 0;
        let paid = 0;
        while (cur != 0 && bought < take) {
            // The id of the next lot comes only from the one just taken.
            let lot = table::remove(&mut book.lots, cur);
            coll = coll + lot.coll;
            paid = paid + lot.price;
            cur = lot.next;
            bought = bought + 1;
        };
        book.head = cur;
        if (cur == 0) {
            book.tail = 0;
        };
        book.len = book.len - bought;
        book.proceeds = book.proceeds + paid;
        let i = 0;
        while (i < bought) {
            push_lot(book, LOT_COLL, LOT_PRICE);
            i = i + 1;
        };
        // The buyer pays for what it took, so the proceeds are backed by the
        // auction's stable balance rather than being a bare counter. The
        // payment is faucet-backed, like every other stake here, so a buyer
        // never runs short however long the mix runs.
        if (paid > 0) {
            let stable = stable_asset();
            cdp_assets::faucet(stable, owner, paid);
            primary_fungible_store::transfer(user, stable, @bench, paid);
        };
        if (coll > 0) {
            cdp_assets::faucet(coll_asset(), owner, coll);
        }
    }

    fun push_lot(book: &mut Auction, coll: u64, price: u64) {
        let id = book.next_id;
        book.next_id = id + 1;
        table::add(&mut book.lots, id, Lot { coll, price, next: 0 });
        if (book.tail == 0) {
            book.head = id;
        } else {
            table::borrow_mut(&mut book.lots, book.tail).next = id;
        };
        book.tail = id;
        book.len = book.len + 1;
    }

    fun coll_asset(): Object<Metadata> {
        cdp_assets::asset(@bench, COLL_SYMBOL)
    }

    fun stable_asset(): Object<Metadata> {
        cdp_assets::asset(@bench, STABLE_SYMBOL)
    }

    #[view]
    /// Lots on the chain and stable the auction has taken in.
    public fun auction_state(): (u64, u64) acquires Auction {
        if (!exists<Auction>(@bench)) return (0, 0);
        let book = borrow_global<Auction>(@bench);
        (book.len, book.proceeds)
    }
}
