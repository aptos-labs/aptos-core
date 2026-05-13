/// Native position store — framework-level storage for perp / spot position
/// data keyed by `(exchange, account, market)`, with an in-memory
/// residency model, dedicated `position_db` + `position_merkle_db`, and an
/// `AggregatorV2`-bounded per-exchange ceiling.
///
/// Access is gated by `ExchangeCapability`. Any exchange can register,
/// obtain a cap, and read/write positions in its own `exchange` address
/// namespace (the exchange's own owning address — not a synthetic id).
/// Governance can `deny(exchange)` to lock a compromised exchange out
/// without wiping its state.
///
/// See `PLAN_native_position.md` in the repo root for the design rationale.
module aptos_experimental::native_position {
    use std::error;
    use std::signer;

    use aptos_experimental::position_counts;
    use aptos_framework::event;
    use aptos_framework::features;
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};

    #[event]
    struct PositionCreated has drop, store {
        exchange: address,
        account: address,
        market: address,
    }

    #[event]
    struct PositionUpdated has drop, store {
        exchange: address,
        account: address,
        market: address,
    }

    #[event]
    struct PositionRemoved has drop, store {
        exchange: address,
        account: address,
        market: address,
    }

    #[event]
    struct ExchangeRegistered has drop, store {
        exchange: address,
    }

    #[event]
    struct ExchangeDenied has drop, store {
        exchange: address,
    }

    #[event]
    struct ExchangeReenabled has drop, store {
        exchange: address,
    }

    /// Feature `NATIVE_POSITION` is not enabled on this chain.
    const EFEATURE_DISABLED: u64 = 1;
    /// Exchange has not been registered yet.
    const EEXCHANGE_NOT_REGISTERED: u64 = 2;
    /// This exchange address has been disabled by governance.
    const EEXCHANGE_DENIED: u64 = 3;
    /// Position requested does not exist.
    const EPOSITION_NOT_FOUND: u64 = 4;
    /// Attempting to add a position that would cross the per-exchange limit.
    /// Propagated from the position-count aggregator.
    const EPOSITION_LIMIT: u64 = 5;
    /// The supplied capability references an unknown exchange address.
    const EBAD_CAPABILITY: u64 = 6;
    /// `unpack_*` called with a `Position` of the wrong variant.
    const EVARIANT_MISMATCH: u64 = 7;

    // =====================================================================
    // Types
    // =====================================================================

    /// Opaque capability for calling native-position functions in an
    /// exchange's own namespace.
    ///
    /// - `has store`, no `copy`, no `drop`.
    /// - Idempotent: calling `register()` twice with the same signer returns
    ///   a cap with the same `exchange` address.
    /// - A cap is a permission, not a unique authority.  An exchange can
    ///   hold multiple caps for the same `exchange`; all are equally
    ///   valid.  The exchange is responsible for custody.
    struct ExchangeCapability has store {
        exchange: address,
    }

    /// Authoritative registry for known exchange addresses and the
    /// governance deny-list. Lives at `@aptos_framework`.
    ///
    /// - `registered` is the set of addresses that have called
    ///   `register()` at least once. Keeps `register()` idempotent and
    ///   gates per-exchange position-counter allocation.
    /// - `denied` is the set of currently-locked exchange addresses. A
    ///   `deny(addr)` adds to the table, `reenable(addr)` removes.
    ///   Read by every position / collateral write to gate execution.
    ///
    /// Writes here are *rare*: register-once-per-exchange, deny is
    /// governance-only. Read traffic from gating checks is high but
    /// reads of a `Table<address, bool>` cell only conflict with
    /// writes to the same address, so block-STM keeps parallel
    /// position writes from contending across distinct exchanges.
    struct ExchangeRegistry has key {
        registered: Table<address, bool>,
        denied: Table<address, bool>,
    }

    /// Deserialized form of a persisted position. Mirrors the
    /// native-position `NativePosition` Rust enum: one byte variant tag
    /// plus fixed-width fields.
    enum Position has copy, drop, store {
        PerpV1 {
            size: u64,
            is_long: bool,
            entry_px_times_size_sum: u128,
            avg_entry_px: u64,
            user_leverage: u8,
            is_isolated: bool,
            /// Signed funding index at last position update. Matches
            /// etna's `AccumulativeIndex { index: i128 }`.
            funding_index: i128,
            /// Signed accrued funding before the last update. Matches
            /// etna's `unrealized_funding_amount_before_last_update: i64`.
            unrealized_funding_before: i64,
            timestamp: u64,
        },
        SpotV1 {
            size: u64,
            is_long: bool,
            entry_px_times_size_sum: u128,
            avg_entry_px: u64,
            timestamp: u64,
        },
    }

    // =====================================================================
    // Lifecycle
    // =====================================================================

    /// Initialize the `ExchangeRegistry` resource at `@aptos_framework`.
    /// Called once at module publication; for an upgrade migration the
    /// framework signer can invoke this through governance.
    fun init_module(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<ExchangeRegistry>(@aptos_framework)) {
            move_to(framework, ExchangeRegistry {
                registered: table::new(),
                denied: table::new(),
            });
        };
    }

    #[test_only]
    /// Test-only init helper. Production code uses `init_module`.
    public fun init_for_test(framework: &signer) {
        init_module(framework);
    }

    /// Whether `exchange` is currently denied. Public so other
    /// framework modules (e.g. `native_collateral`) can gate their
    /// own writes.
    public fun is_denied(exchange: address): bool acquires ExchangeRegistry {
        let registry = borrow_global<ExchangeRegistry>(@aptos_framework);
        registry.denied.contains(exchange)
    }

    /// Assert that `cap`'s exchange isn't in the denied set. Called
    /// from every write-path entry point (position + collateral).
    public fun assert_cap_active(cap: &ExchangeCapability) acquires ExchangeRegistry {
        let registry = borrow_global<ExchangeRegistry>(@aptos_framework);
        assert!(
            !registry.denied.contains(cap.exchange),
            error::permission_denied(EEXCHANGE_DENIED),
        );
    }

    /// Register an exchange and allocate an `AggregatorV2`-backed
    /// position counter bounded at `initial_max`. The signer's address
    /// itself is the exchange identity — no synthetic id allocation.
    ///
    /// Idempotent per signer: subsequent calls from the same signer
    /// return a cap pointing at the same address; the stored
    /// `initial_max` from the first call sticks; use `update_ceiling`
    /// via governance to tune later.
    public fun register(exchange: &signer, initial_max: u64): ExchangeCapability
    acquires ExchangeRegistry {
        assert!(
            features::is_native_position_enabled(),
            error::permission_denied(EFEATURE_DISABLED),
        );
        let addr = signer::address_of(exchange);
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_framework);
        let freshly_allocated = if (registry.registered.contains(addr)) {
            false
        } else {
            registry.registered.add(addr, true);
            true
        };
        // If this is the first time we've seen `addr`, allocate the
        // counter.  Subsequent calls are no-ops via the existence check.
        let first_register = freshly_allocated && !position_counts::counter_exists(addr);
        if (first_register) {
            position_counts::allocate_counter(addr, initial_max);
            event::emit(ExchangeRegistered { exchange: addr });
        };
        ExchangeCapability { exchange: addr }
    }

    /// Destroy a capability. The underlying registration stays in place
    /// and any other caps for the same exchange remain valid.
    /// Re-registering from the same signer returns the same address.
    public fun unregister(cap: ExchangeCapability) {
        let ExchangeCapability { exchange: _ } = cap;
    }

    /// Governance-only: lock an `exchange` address out. All future
    /// write-path calls that carry a cap with this address abort
    /// `EEXCHANGE_DENIED`. Persisted positions are untouched — this is
    /// a lockout, not a wipe.
    public fun deny(framework: &signer, exchange: address) acquires ExchangeRegistry {
        system_addresses::assert_aptos_framework(framework);
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_framework);
        if (!registry.denied.contains(exchange)) {
            registry.denied.add(exchange, true);
        };
        event::emit(ExchangeDenied { exchange });
    }

    /// Governance-only: re-enable an `exchange` previously locked via
    /// `deny`. Use only if the compromise has been resolved.
    public fun reenable(framework: &signer, exchange: address) acquires ExchangeRegistry {
        system_addresses::assert_aptos_framework(framework);
        let registry = borrow_global_mut<ExchangeRegistry>(@aptos_framework);
        if (registry.denied.contains(exchange)) {
            registry.denied.remove(exchange);
        };
        event::emit(ExchangeReenabled { exchange });
    }

    /// Governance-only: bump or shrink the per-exchange position-count
    /// ceiling. Delegates to `position_counts`.
    public fun update_ceiling(framework: &signer, exchange: address, new_max: u64) {
        position_counts::update_ceiling(framework, exchange, new_max);
    }

    public fun exchange(cap: &ExchangeCapability): address {
        cap.exchange
    }

    // =====================================================================
    // Writes
    // =====================================================================
    //
    // The native-position store is intentionally write-only from Move.
    // Reads of native state happen only on the validator side (Rust),
    // outside Move execution. Move execution must not depend on native
    // state — the authoritative state is whatever the calling module
    // (e.g. etna) chooses to keep in regular Move resources.

    /// Create a brand-new position. Bumps the per-exchange position
    /// counter; aborts `EPOSITION_LIMIT` if the increment would exceed
    /// the configured ceiling. Caller is responsible for ensuring this
    /// is genuinely a new position — a duplicate `create_position` call
    /// for the same `(account, market)` will bump the counter without a
    /// matching storage entry, so the gross-creates-minus-gross-removes
    /// invariant holds only under correct caller usage.
    public fun create_position(
        cap: &ExchangeCapability,
        account: address,
        market: address,
        position: Position,
    ) acquires ExchangeRegistry {
        assert_cap_active(cap);
        position_counts::try_add(cap.exchange, 1);
        native_create_position(cap.exchange, account, market, position);
        event::emit(PositionCreated {
            exchange: cap.exchange,
            account,
            market,
        });
    }

    /// Mutate an existing position's data in place. Does not change
    /// the per-exchange position count.
    public fun update_position(
        cap: &ExchangeCapability,
        account: address,
        market: address,
        position: Position,
    ) acquires ExchangeRegistry {
        assert_cap_active(cap);
        native_update_position(cap.exchange, account, market, position);
        event::emit(PositionUpdated {
            exchange: cap.exchange,
            account,
            market,
        });
    }

    /// Remove a position. Decrements the per-exchange position counter;
    /// aborts on underflow (which indicates caller-side bookkeeping
    /// drift — the counter tracks gross creates minus gross removes).
    public fun remove_position(
        cap: &ExchangeCapability,
        account: address,
        market: address,
    ) acquires ExchangeRegistry {
        assert_cap_active(cap);
        native_remove_position(cap.exchange, account, market);
        position_counts::sub(cap.exchange, 1);
        event::emit(PositionRemoved {
            exchange: cap.exchange,
            account,
            market,
        });
    }

    // =====================================================================
    // Position constructors / accessors
    //
    // Exchange modules outside this crate cannot construct `Position`
    // variants directly (Move struct-literal construction is module-
    // private). These helpers expose the PerpV1 / SpotV1 variants so
    // exchange bridge modules can round-trip their native position
    // representations through the framework layer.
    // =====================================================================

    /// Construct a `Position::PerpV1` with the given field values.
    public fun new_perp_v1(
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        user_leverage: u8,
        is_isolated: bool,
        funding_index: i128,
        unrealized_funding_before: i64,
        timestamp: u64,
    ): Position {
        Position::PerpV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_entry_px,
            user_leverage,
            is_isolated,
            funding_index,
            unrealized_funding_before,
            timestamp,
        }
    }

    /// Construct a `Position::SpotV1` with the given field values.
    public fun new_spot_v1(
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        timestamp: u64,
    ): Position {
        Position::SpotV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_entry_px,
            timestamp,
        }
    }

    /// True iff `pos` is the PerpV1 variant.
    public fun is_perp_v1(pos: &Position): bool {
        match (pos) {
            Position::PerpV1 { .. } => true,
            Position::SpotV1 { .. } => false,
        }
    }

    /// True iff `pos` is the SpotV1 variant.
    public fun is_spot_v1(pos: &Position): bool {
        match (pos) {
            Position::PerpV1 { .. } => false,
            Position::SpotV1 { .. } => true,
        }
    }

    /// Destructure a `Position::PerpV1` into its field tuple. Aborts
    /// `EVARIANT_MISMATCH` if `pos` is not the PerpV1 variant.
    public fun unpack_perp_v1(
        pos: Position,
    ): (u64, bool, u128, u64, u8, bool, i128, i64, u64) {
        match (pos) {
            Position::PerpV1 {
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                user_leverage,
                is_isolated,
                funding_index,
                unrealized_funding_before,
                timestamp,
            } => (
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                user_leverage,
                is_isolated,
                funding_index,
                unrealized_funding_before,
                timestamp,
            ),
            Position::SpotV1 { .. } => abort error::invalid_argument(EVARIANT_MISMATCH),
        }
    }

    /// Destructure a `Position::SpotV1` into its field tuple. Aborts
    /// `EVARIANT_MISMATCH` if `pos` is not the SpotV1 variant.
    public fun unpack_spot_v1(pos: Position): (u64, bool, u128, u64, u64) {
        match (pos) {
            Position::SpotV1 {
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                timestamp,
            } => (
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                timestamp,
            ),
            Position::PerpV1 { .. } => abort error::invalid_argument(EVARIANT_MISMATCH),
        }
    }

    // =====================================================================
    // Native declarations (implemented in position-natives crate)
    //
    // Lifecycle (`register` / `deny` / `reenable`) is now pure Move,
    // backed by the `ExchangeRegistry` resource above. The natives
    // here are write-staging only: they push a position into the
    // off-Move mirror at commit. Reads of native state never happen
    // from Move; that's the validator-side Rust concern.
    // =====================================================================

    native fun native_create_position(
        exchange: address,
        account: address,
        market: address,
        position: Position,
    );
    native fun native_update_position(
        exchange: address,
        account: address,
        market: address,
        position: Position,
    );
    native fun native_remove_position(exchange: address, account: address, market: address);
}
