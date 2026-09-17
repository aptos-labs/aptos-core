/// Doubly linked list of vaults ordered by nominal collateral ratio, in the
/// shape Liquity's `SortedTroves` uses.
///
/// Only the head, the tail and the length live in one place. Every other link
/// is a field of the vault resource it belongs to, so moving one position
/// along the list costs one dependent resource read. A caller supplies a hint
/// node to start from, and every walk is capped, so one transaction does a
/// bounded amount of work even on a long list.
module bench::cdp_sorted {
    use std::signer;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use bench::cdp_assets;
    use bench::cdp_oracle;
    use bench::cdp_vault;

    /// Only the package address seeds the list.
    const E_NOT_BENCH: u64 = 1;

    const COLL_SYMBOL: vector<u8> = b"CDPC";
    const STABLE_SYMBOL: vector<u8> = b"CDPS";

    /// Every account opens one vault, at this index under its own address.
    const USER_VAULT_INDEX: u64 = 0;

    /// Nodes a seeding insert may walk. Seeded vaults arrive in ascending
    /// ratio order and start from the tail hint, so one hop is enough and the
    /// cap never binds.
    const SEED_WALK_CAP: u64 = 32;
    /// Nodes any other insert may walk before placing the vault where it got
    /// to. An approximate position costs nothing here and a full walk would
    /// make the transaction's cost depend on the list length.
    const INSERT_WALK_CAP: u64 = 64;

    const CHECKSUM_MOD: u64 = 1000000007;

    /// Faucet amounts at onboarding. Generous, so no account runs short of
    /// either asset however long the mix runs.
    const FAUCET_COLL: u64 = 1000000000;
    const FAUCET_STABLE: u64 = 1000000000000;

    /// Ratio band the seeded filler vaults span, in basis points against the
    /// oracle's reference price. Every one of them is safe at the top of the
    /// price band and a candidate near the bottom, so a sweep finds work only
    /// while the price is low, and how far down the band it has to be before
    /// a given vault qualifies varies across the list.
    const SEED_ICR_MIN_BPS: u64 = 11500;
    const SEED_ICR_STEP_BPS: u64 = 5;
    /// Collateral of a seeded vault varies over this many values, so the list
    /// is not uniform even though its ratios are.
    const SEED_COLL_BASE: u64 = 100;
    const SEED_COLL_SPREAD: u64 = 97;

    /// Position a `bench_close_reopen` gives an account that never onboarded.
    const DEFAULT_COLL: u64 = 150;
    const DEFAULT_ICR_BPS: u64 = 25000;

    /// Floor on a vault's collateral, so a shrinking adjust cannot reach zero
    /// and make every ratio degenerate.
    const MIN_COLL: u64 = 1;
    /// Ceilings on one adjust step, so a caller-supplied delta cannot
    /// overflow the position it is applied to.
    const MAX_STEP_COLL: u64 = 1000;
    const MAX_STEP_DEBT: u64 = 1000000;

    struct SortedVaults has key {
        head: address,
        tail: address,
        len: u64,
    }

    // Publisher-signed setup.

    /// Create `count` publisher-owned filler vaults from `start`, so the list
    /// has length before any account onboards. Chunked by the caller, since a
    /// whole list in one transaction runs past the execution limit.
    public entry fun seed_vaults(
        admin: &signer, start: u64, count: u64
    ) acquires SortedVaults {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        ensure_list(admin);
        let price = cdp_oracle::reference_price();
        let i = 0;
        while (i < count) {
            let index = start + i;
            let addr = cdp_vault::vault_address(@bench, index);
            if (!cdp_vault::exists_vault(addr)) {
                let coll = SEED_COLL_BASE + index % SEED_COLL_SPREAD;
                let target = SEED_ICR_MIN_BPS + index * SEED_ICR_STEP_BPS;
                cdp_vault::create(
                    admin,
                    index,
                    coll,
                    cdp_vault::max_debt_at(coll, price, target),
                );
                insert(addr, tail(), SEED_WALK_CAP);
            };
            i = i + 1;
        }
    }

    // Mix entry points.

    /// Fund an account on both assets and give it a vault in the list.
    /// Running twice resizes the vault rather than creating a second one.
    public entry fun bench_onboard(
        user: &signer, coll: u64, icr_bps: u64, hint: address
    ) acquires SortedVaults {
        let owner = signer::address_of(user);
        cdp_assets::faucet(coll_asset(), owner, FAUCET_COLL);
        cdp_assets::faucet(stable_asset(), owner, FAUCET_STABLE);
        let coll = cdp_vault::clamp_coll(
            if (coll < MIN_COLL) MIN_COLL else coll);
        let debt = cdp_vault::max_debt_at(
            coll, cdp_oracle::reference_price(), floor_target(icr_bps));
        let addr = cdp_vault::vault_address(owner, USER_VAULT_INDEX);
        if (cdp_vault::exists_vault(addr)) {
            reposition(addr, coll, debt, hint);
        } else {
            cdp_vault::create(user, USER_VAULT_INDEX, coll, debt);
            insert(addr, hint, INSERT_WALK_CAP);
        }
    }

    /// The measured kernel: `limit` sequentially dependent resource reads from
    /// the hint node onward.
    public entry fun bench_walk(
        _user: &signer, hint: address, limit: u64
    ) acquires SortedVaults {
        walk_checksum(hint, limit);
    }

    /// Move both legs of the caller's position and re-sort it. The debt leg
    /// is clamped to whatever puts the vault on the minimum ratio at the
    /// reference price, so an oversized request lands on the boundary instead
    /// of aborting. Clamping against the reference rather than the spot price
    /// is what lets an adjust produce a candidate: at the spot price the
    /// clamped vault would sit exactly on the minimum and be immune, whereas
    /// against the reference it is a candidate over the lower half of the
    /// band.
    public entry fun bench_adjust(
        user: &signer, d_coll: u64, d_debt: u64, grow: bool, hint: address
    ) acquires SortedVaults {
        let addr = cdp_vault::vault_address(
            signer::address_of(user), USER_VAULT_INDEX);
        if (!cdp_vault::exists_vault(addr)) return;
        let d_coll = if (d_coll > MAX_STEP_COLL) MAX_STEP_COLL else d_coll;
        let d_debt = if (d_debt > MAX_STEP_DEBT) MAX_STEP_DEBT else d_debt;
        let (coll, debt) = cdp_vault::coll_debt_of(addr);
        let new_coll = if (grow) {
            cdp_vault::clamp_coll(coll + d_coll)
        } else if (coll > d_coll + MIN_COLL) {
            coll - d_coll
        } else {
            MIN_COLL
        };
        let want = if (grow) {
            debt + d_debt
        } else if (debt > d_debt) {
            debt - d_debt
        } else {
            0
        };
        let ceiling = cdp_vault::max_debt_at(
            new_coll, cdp_oracle::reference_price(), cdp_vault::mcr_bps());
        let new_debt = if (want > ceiling) ceiling else want;
        reposition(addr, new_coll, new_debt, hint);
    }

    /// Delete the caller's vault resource and put it back at the same address
    /// through the extend ref the close left behind, which is why reopening
    /// never collides with the original named object.
    public entry fun bench_close_reopen(
        user: &signer, hint: address
    ) acquires SortedVaults {
        let addr = cdp_vault::vault_address(
            signer::address_of(user), USER_VAULT_INDEX);
        if (!cdp_vault::exists_vault(addr)) {
            bench_onboard(user, DEFAULT_COLL, DEFAULT_ICR_BPS, hint);
            return
        };
        let (coll, debt) = cdp_vault::coll_debt_of(addr);
        remove(addr);
        cdp_vault::destroy(addr);
        cdp_vault::recreate(addr, coll, debt);
        insert(addr, hint, INSERT_WALK_CAP);
    }

    // List operations.

    /// Place `addr` by walking from `hint` toward its sort position. The walk
    /// stops after `cap` hops and leaves the vault where it got to.
    public fun insert(
        addr: address, hint: address, cap: u64
    ) acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) return;
        if (!cdp_vault::exists_vault(addr) || in_list(addr)) return;
        let nicr = cdp_vault::nicr_of(addr);
        let head = head();
        if (head == @0x0) {
            cdp_vault::set_links(addr, @0x0, @0x0);
            let list = borrow_global_mut<SortedVaults>(@bench);
            list.head = addr;
            list.tail = addr;
            list.len = 1;
            return
        };
        let start =
            if (hint == @0x0 || hint == addr || !in_list(hint)) head else hint;
        if (nicr >= cdp_vault::nicr_of(start)) {
            let cur = start;
            let next = cdp_vault::next_of(cur);
            let steps = 0;
            while (
                next != @0x0 && steps < cap && cdp_vault::nicr_of(next) <= nicr
            ) {
                cur = next;
                next = cdp_vault::next_of(cur);
                steps = steps + 1;
            };
            link_after(cur, addr);
        } else {
            let cur = start;
            let prev = cdp_vault::prev_of(cur);
            let steps = 0;
            while (
                prev != @0x0 && steps < cap && cdp_vault::nicr_of(prev) > nicr
            ) {
                cur = prev;
                prev = cdp_vault::prev_of(cur);
                steps = steps + 1;
            };
            link_before(cur, addr);
        }
    }

    public fun remove(addr: address) acquires SortedVaults {
        if (!exists<SortedVaults>(@bench) || !in_list(addr)) return;
        let next = cdp_vault::next_of(addr);
        let prev = cdp_vault::prev_of(addr);
        if (prev == @0x0) {
            borrow_global_mut<SortedVaults>(@bench).head = next;
        } else {
            cdp_vault::set_next(prev, next);
        };
        if (next == @0x0) {
            borrow_global_mut<SortedVaults>(@bench).tail = prev;
        } else {
            cdp_vault::set_prev(next, prev);
        };
        cdp_vault::set_links(addr, @0x0, @0x0);
        let list = borrow_global_mut<SortedVaults>(@bench);
        list.len = list.len - 1;
    }

    /// Fold the list from `hint` into a checksum. `limit` is clamped to the
    /// length and the walk stops at the end marker, so neither an oversized
    /// limit nor a stale hint can run past the list.
    public fun walk_checksum(
        hint: address, limit: u64
    ): u64 acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) return 0;
        let len = len();
        let limit = if (limit > len) len else limit;
        let cur = if (hint == @0x0 || !in_list(hint)) head() else hint;
        let acc = 0;
        let n = 0;
        while (cur != @0x0 && n < limit) {
            let (coll, debt) = cdp_vault::coll_debt_of(cur);
            acc = (acc * 31 + coll + debt % CHECKSUM_MOD) % CHECKSUM_MOD;
            cur = cdp_vault::next_of(cur);
            n = n + 1;
        };
        acc
    }

    /// A vault is linked when it is the head or has a neighbour. Removal
    /// clears both links, so a detached vault answers false.
    public fun in_list(addr: address): bool acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) return false;
        if (!cdp_vault::exists_vault(addr)) return false;
        addr == head()
            || cdp_vault::next_of(addr) != @0x0
            || cdp_vault::prev_of(addr) != @0x0
    }

    fun link_after(cur: address, addr: address) acquires SortedVaults {
        let next = cdp_vault::next_of(cur);
        cdp_vault::set_links(addr, next, cur);
        cdp_vault::set_next(cur, addr);
        if (next == @0x0) {
            borrow_global_mut<SortedVaults>(@bench).tail = addr;
        } else {
            cdp_vault::set_prev(next, addr);
        };
        let list = borrow_global_mut<SortedVaults>(@bench);
        list.len = list.len + 1;
    }

    fun link_before(cur: address, addr: address) acquires SortedVaults {
        let prev = cdp_vault::prev_of(cur);
        cdp_vault::set_links(addr, cur, prev);
        cdp_vault::set_prev(cur, addr);
        if (prev == @0x0) {
            borrow_global_mut<SortedVaults>(@bench).head = addr;
        } else {
            cdp_vault::set_next(prev, addr);
        };
        let list = borrow_global_mut<SortedVaults>(@bench);
        list.len = list.len + 1;
    }

    fun reposition(
        addr: address, coll: u64, debt: u64, hint: address
    ) acquires SortedVaults {
        remove(addr);
        cdp_vault::set_coll_debt(addr, coll, debt);
        insert(addr, hint, INSERT_WALK_CAP);
    }

    fun ensure_list(admin: &signer) {
        if (!exists<SortedVaults>(@bench)) {
            move_to(admin, SortedVaults { head: @0x0, tail: @0x0, len: 0 });
        }
    }

    /// A vault must stay at or above the minimum ratio, so a request below it
    /// is raised rather than rejected.
    fun floor_target(icr_bps: u64): u64 {
        let mcr = cdp_vault::mcr_bps();
        if (icr_bps < mcr) mcr else icr_bps
    }

    // Accessors.

    public fun head(): address acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) {
            @0x0
        } else {
            borrow_global<SortedVaults>(@bench).head
        }
    }

    public fun tail(): address acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) {
            @0x0
        } else {
            borrow_global<SortedVaults>(@bench).tail
        }
    }

    #[view]
    public fun len(): u64 acquires SortedVaults {
        if (!exists<SortedVaults>(@bench)) {
            0
        } else {
            borrow_global<SortedVaults>(@bench).len
        }
    }

    public fun user_vault_index(): u64 { USER_VAULT_INDEX }

    public fun insert_walk_cap(): u64 { INSERT_WALK_CAP }

    public fun coll_asset(): Object<Metadata> {
        cdp_assets::asset(@bench, COLL_SYMBOL)
    }

    public fun stable_asset(): Object<Metadata> {
        cdp_assets::asset(@bench, STABLE_SYMBOL)
    }
}
