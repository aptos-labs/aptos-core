/// Feed storage for the oracle benchmark, shaped after a Switchboard
/// aggregator: one table item per feed holding the latest report, a trailing
/// price window, and a time-weighted accumulator.
///
/// Every write goes through `write`, which creates a missing feed instead of
/// aborting and caps the values it folds into the accumulator, so no batch of
/// caller-supplied bytes can abort a transaction.
module bench::orc_registry {
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};

    /// Only the package address creates feeds.
    const E_NOT_BENCH: u64 = 1;

    /// Prices kept per feed. The published aggregate is the median over this
    /// window, so it has to stay short enough to sort in a loop.
    const HISTORY_LEN: u64 = 8;

    /// Prices are multiplied by an elapsed time before landing in the TWAP
    /// accumulator, so both factors and the accumulator itself are capped to
    /// keep the arithmetic inside `u128`.
    const PRICE_CAP: u128 = 1_000_000_000_000;
    const DT_CAP: u128 = 1_000_000;
    const TWAP_CAP: u128 = 1_000_000_000_000_000_000_000_000_000_000;

    /// Feed ids callers are held to. Leaving the top of the range unused means
    /// a base plus a batch length cannot overflow.
    const FEED_ID_SPACE: u64 = 4294967296;

    const MAX_U64: u64 = 18446744073709551615;

    struct Feed has store {
        price: u128,
        /// Median over the trailing price window at the last update.
        aggregate: u128,
        conf: u64,
        ts: u64,
        updates: u64,
        twap_num: u128,
        twap_den: u128,
        history: vector<u128>,
    }

    struct Registry has key {
        feeds: Table<u64, Feed>,
        num_feeds: u64,
    }

    /// Append `count` feeds. Callers chunk this, since a whole feed set in one
    /// transaction runs past the per-transaction execution limit.
    public entry fun create_feeds(admin: &signer, count: u64) acquires Registry {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Registry>(@bench)) {
            move_to(admin, Registry { feeds: table::new(), num_feeds: 0 });
        };
        let registry = borrow_global_mut<Registry>(@bench);
        let i = 0;
        while (i < count) {
            let feed_id = registry.num_feeds;
            if (!table::contains(&registry.feeds, feed_id)) {
                table::add(&mut registry.feeds, feed_id, empty_feed());
            };
            registry.num_feeds = registry.num_feeds + 1;
            i = i + 1;
        }
    }

    fun empty_feed(): Feed {
        Feed {
            price: 0,
            aggregate: 0,
            conf: 0,
            ts: 0,
            updates: 0,
            twap_num: 0,
            twap_den: 0,
            history: vector::empty(),
        }
    }

    public fun history_len(): u64 {
        HISTORY_LEN
    }

    public fun feed_id_space(): u64 {
        FEED_ID_SPACE
    }

    #[view]
    public fun is_initialized(): bool {
        exists<Registry>(@bench)
    }

    #[view]
    public fun num_feeds(): u64 acquires Registry {
        if (!exists<Registry>(@bench)) return 0;
        borrow_global<Registry>(@bench).num_feeds
    }

    #[view]
    public fun contains(feed_id: u64): bool acquires Registry {
        if (!exists<Registry>(@bench)) return false;
        table::contains(&borrow_global<Registry>(@bench).feeds, feed_id)
    }

    /// Trailing price window of `feed_id`, empty when there is no such feed.
    public fun history(feed_id: u64): vector<u128> acquires Registry {
        if (!exists<Registry>(@bench)) return vector::empty();
        let feeds = &borrow_global<Registry>(@bench).feeds;
        if (!table::contains(feeds, feed_id)) return vector::empty();
        table::borrow(feeds, feed_id).history
    }

    /// `(price, aggregate, conf, ts, updates)`, all zero when there is no such
    /// feed.
    public fun read(feed_id: u64): (u128, u128, u64, u64, u64) acquires Registry {
        if (!exists<Registry>(@bench)) return (0, 0, 0, 0, 0);
        let feeds = &borrow_global<Registry>(@bench).feeds;
        if (!table::contains(feeds, feed_id)) return (0, 0, 0, 0, 0);
        let feed = table::borrow(feeds, feed_id);
        (feed.price, feed.aggregate, feed.conf, feed.ts, feed.updates)
    }

    #[view]
    /// Time-weighted average price, zero before the second update of a feed.
    public fun twap(feed_id: u64): u128 acquires Registry {
        if (!exists<Registry>(@bench)) return 0;
        let feeds = &borrow_global<Registry>(@bench).feeds;
        if (!table::contains(feeds, feed_id)) return 0;
        let feed = table::borrow(feeds, feed_id);
        if (feed.twap_den == 0) 0 else feed.twap_num / feed.twap_den
    }

    /// Record a report against `feed_id`, creating the feed when it is past
    /// the ones the publisher made.
    public fun write(
        feed_id: u64, price: u128, aggregate: u128, conf: u64, ts: u64
    ) acquires Registry {
        // There is nothing to write before the publisher creates the registry.
        // Returning here keeps every mix entry point free of aborts.
        if (!exists<Registry>(@bench)) return;
        let price = price % PRICE_CAP;
        let registry = borrow_global_mut<Registry>(@bench);
        if (!table::contains(&registry.feeds, feed_id)) {
            table::add(&mut registry.feeds, feed_id, empty_feed());
            if (feed_id >= registry.num_feeds && feed_id < MAX_U64) {
                registry.num_feeds = feed_id + 1;
            };
        };
        let feed = table::borrow_mut(&mut registry.feeds, feed_id);
        let dt = if (ts > feed.ts) (((ts - feed.ts) as u128) % DT_CAP) + 1 else 1;
        // Halving both terms keeps the ratio and puts a ceiling on the
        // accumulator, so a long run cannot overflow it.
        if (feed.twap_num > TWAP_CAP) {
            feed.twap_num = feed.twap_num / 2;
            feed.twap_den = feed.twap_den / 2;
        };
        feed.twap_num = feed.twap_num + price * dt;
        feed.twap_den = feed.twap_den + dt;
        feed.price = price;
        feed.aggregate = aggregate % PRICE_CAP;
        feed.conf = conf;
        feed.ts = ts;
        feed.updates = feed.updates + 1;
        vector::push_back(&mut feed.history, price);
        if (vector::length(&feed.history) > HISTORY_LEN) {
            vector::remove(&mut feed.history, 0);
        }
    }
}
