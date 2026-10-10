/// Collateralized debt positions, shaped after Liquity's troves and Thala's
/// CDP on Aptos.
///
/// A vault is a resource at a named object under its owner, and it carries its
/// own neighbour links. `cdp_sorted` therefore learns the address of the next
/// vault only by reading the current one, which is the chain of sequentially
/// dependent resource reads this package exists to measure.
///
/// # Ratios
///
/// The nominal collateral ratio is collateral per unit of debt and does not
/// involve price, so it is stable across a price move and is what the list
/// sorts on. The collateral ratio in basis points does involve price and is
/// what liquidation tests against the minimum.
module bench::cdp_vault {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_framework::object::{Self, ExtendRef};

    /// Only the package address configures the protocol.
    const E_NOT_BENCH: u64 = 1;
    /// Nothing was ever created at this vault address.
    const E_NO_VAULT: u64 = 2;

    /// Basis point denominator.
    const BPS: u64 = 10000;
    /// Fixed point scale of the nominal collateral ratio.
    const NICR_SCALE: u64 = 1000000000;
    /// Reported for a vault with no debt, which is healthier and sorts later
    /// than any vault that has some.
    const MAX_RATIO: u64 = 18446744073709551615;

    /// Ceiling on a vault's collateral. Growth is clamped to it so the fixed
    /// point products below cannot overflow however long the mix runs.
    const MAX_COLL: u64 = 1000000;

    /// Used before the publisher has configured the protocol.
    const DEFAULT_MCR_BPS: u64 = 11000;

    struct Vault has key {
        coll: u64,
        debt: u64,
        next: address,
        prev: address,
    }

    /// Lives at the vault's object address and is never removed, so a closed
    /// vault can be reopened at the same address without a second named
    /// object creation.
    struct VaultRefs has key {
        extend_ref: ExtendRef,
    }

    struct Config has key {
        mcr_bps: u64,
    }

    public entry fun initialize(admin: &signer, mcr_bps: u64) acquires Config {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Config>(@bench)) {
            move_to(admin, Config { mcr_bps });
        } else {
            borrow_global_mut<Config>(@bench).mcr_bps = mcr_bps;
        }
    }

    public fun mcr_bps(): u64 acquires Config {
        if (!exists<Config>(@bench)) {
            DEFAULT_MCR_BPS
        } else {
            borrow_global<Config>(@bench).mcr_bps
        }
    }

    // Addressing.

    public fun vault_seed(index: u64): vector<u8> {
        let seed = b"cdp_vault";
        vector::append(&mut seed, bcs::to_bytes(&index));
        seed
    }

    public fun vault_address(owner: address, index: u64): address {
        object::create_object_address(&owner, vault_seed(index))
    }

    public fun exists_vault(addr: address): bool {
        exists<Vault>(addr)
    }

    // Lifecycle.

    /// Create `owner`'s vault number `index`, unlinked. The caller puts it in
    /// the sorted list.
    public fun create(
        owner: &signer, index: u64, coll: u64, debt: u64
    ): address {
        let constructor_ref =
            object::create_named_object(owner, vault_seed(index));
        let vault_signer = object::generate_signer(&constructor_ref);
        move_to(
            &vault_signer,
            Vault { coll: clamp_coll(coll), debt, next: @0x0, prev: @0x0 },
        );
        move_to(
            &vault_signer,
            VaultRefs {
                extend_ref: object::generate_extend_ref(&constructor_ref),
            },
        );
        signer::address_of(&vault_signer)
    }

    /// Take the `Vault` away, leaving the `VaultRefs` behind.
    public fun destroy(addr: address) acquires Vault {
        if (!exists<Vault>(addr)) return;
        let Vault { coll: _, debt: _, next: _, prev: _ } = move_from<Vault>(addr);
    }

    /// Put a `Vault` back at an address a previous `destroy` emptied.
    public fun recreate(addr: address, coll: u64, debt: u64) acquires VaultRefs {
        assert!(exists<VaultRefs>(addr), E_NO_VAULT);
        let vault_signer = object::generate_signer_for_extending(
            &borrow_global<VaultRefs>(addr).extend_ref);
        move_to(
            &vault_signer,
            Vault { coll: clamp_coll(coll), debt, next: @0x0, prev: @0x0 },
        );
    }

    // Field access. Every list hop goes through `next_of`, so the address of
    // the next node comes only from the contents of the current one.

    public fun next_of(addr: address): address acquires Vault {
        if (!exists<Vault>(addr)) @0x0 else borrow_global<Vault>(addr).next
    }

    public fun prev_of(addr: address): address acquires Vault {
        if (!exists<Vault>(addr)) @0x0 else borrow_global<Vault>(addr).prev
    }

    public fun set_next(addr: address, next: address) acquires Vault {
        if (!exists<Vault>(addr)) return;
        borrow_global_mut<Vault>(addr).next = next;
    }

    public fun set_prev(addr: address, prev: address) acquires Vault {
        if (!exists<Vault>(addr)) return;
        borrow_global_mut<Vault>(addr).prev = prev;
    }

    public fun set_links(
        addr: address, next: address, prev: address
    ) acquires Vault {
        if (!exists<Vault>(addr)) return;
        let vault = borrow_global_mut<Vault>(addr);
        vault.next = next;
        vault.prev = prev;
    }

    public fun coll_debt_of(addr: address): (u64, u64) acquires Vault {
        if (!exists<Vault>(addr)) return (0, 0);
        let vault = borrow_global<Vault>(addr);
        (vault.coll, vault.debt)
    }

    public fun set_coll_debt(
        addr: address, coll: u64, debt: u64
    ) acquires Vault {
        if (!exists<Vault>(addr)) return;
        let vault = borrow_global_mut<Vault>(addr);
        vault.coll = clamp_coll(coll);
        vault.debt = debt;
    }

    public fun nicr_of(addr: address): u64 acquires Vault {
        let (coll, debt) = coll_debt_of(addr);
        nicr(coll, debt)
    }

    public fun icr_of(addr: address, price: u64): u64 acquires Vault {
        let (coll, debt) = coll_debt_of(addr);
        icr_bps(coll, debt, price)
    }

    // Ratio math.

    /// Collateral per unit of debt, scaled. Independent of price, so a price
    /// move never invalidates the list's order.
    public fun nicr(coll: u64, debt: u64): u64 {
        if (debt == 0) MAX_RATIO else clamp_coll(coll) * NICR_SCALE / debt
    }

    /// Collateral ratio in basis points at `price`.
    public fun icr_bps(coll: u64, debt: u64, price: u64): u64 {
        if (debt == 0) MAX_RATIO else clamp_coll(coll) * price * BPS / debt
    }

    /// Largest debt `coll` carries at `price` without falling below
    /// `target_bps`. This is how every caller clamps a debt increase instead
    /// of rejecting it.
    public fun max_debt_at(coll: u64, price: u64, target_bps: u64): u64 {
        if (target_bps == 0) 0 else clamp_coll(coll) * price * BPS / target_bps
    }

    public fun clamp_coll(coll: u64): u64 {
        if (coll > MAX_COLL) MAX_COLL else coll
    }

    public fun max_coll(): u64 { MAX_COLL }

    public fun max_ratio(): u64 { MAX_RATIO }

    public fun bps(): u64 { BPS }
}
