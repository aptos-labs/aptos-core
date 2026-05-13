/// Framework-level marker that governance sets after a per-exchange
/// off-chain migration sweep confirms no stale `UserPositions`
/// resources remain. Applications (e.g. etna) check for the marker's
/// existence at their Phase 4 cutover as described in
/// `PLAN_native_position.md`.
///
/// Semantics:
/// - One `MigrationComplete` resource per `exchange`, stored at
///   `@aptos_experimental`.
/// - Only `aptos_framework` can call `finalize` — this is a
///   governance action paired with the off-chain sweep.
/// - Once set, the marker is monotonic: cannot be cleared. Rolling
///   back a cutover requires application-side reverse-migration,
///   not marker removal.
module aptos_experimental::migration_complete {
    use std::error;

    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};

    /// The framework-level `MigrationComplete` registry has not been
    /// initialized yet.
    const ENOT_INITIALIZED: u64 = 1;
    /// `finalize` was called more than once for the same `exchange`.
    const EALREADY_FINALIZED: u64 = 2;

    struct CompletionEntry has store, drop {
        finalized_at_version: u64,
    }

    struct MigrationCompleteRegistry has key {
        /// exchange -> finalization entry
        entries: Table<address, CompletionEntry>,
    }

    /// Runs once when the module is published at `@aptos_experimental`.
    fun init_module(experimental: &signer) {
        move_to(
            experimental,
            MigrationCompleteRegistry { entries: table::new() },
        );
    }

    /// Governance-only: mark migration complete for `exchange`.
    /// `finalized_at_version` should be the version at which the
    /// off-chain sweep last observed zero remaining `UserPositions`.
    public fun finalize(
        framework: &signer,
        exchange: address,
        finalized_at_version: u64,
    ) acquires MigrationCompleteRegistry {
        system_addresses::assert_aptos_framework(framework);
        assert!(
            exists<MigrationCompleteRegistry>(@aptos_experimental),
            error::not_found(ENOT_INITIALIZED),
        );
        let registry =
            &mut borrow_global_mut<MigrationCompleteRegistry>(@aptos_experimental).entries;
        assert!(
            !table::contains(registry, exchange),
            error::already_exists(EALREADY_FINALIZED),
        );
        table::add(
            registry,
            exchange,
            CompletionEntry { finalized_at_version },
        );
    }

    /// True if `exchange` has had its migration finalized by
    /// governance. Application-side code calls this before deploying
    /// or executing the no-legacy-path module version.
    public fun is_finalized(exchange: address): bool acquires MigrationCompleteRegistry {
        if (!exists<MigrationCompleteRegistry>(@aptos_experimental)) {
            return false
        };
        let entries = &borrow_global<MigrationCompleteRegistry>(@aptos_experimental).entries;
        table::contains(entries, exchange)
    }

    /// Return the version at which migration was finalized, or 0 if
    /// not yet finalized.
    public fun finalized_at(exchange: address): u64 acquires MigrationCompleteRegistry {
        if (!exists<MigrationCompleteRegistry>(@aptos_experimental)) {
            return 0
        };
        let entries = &borrow_global<MigrationCompleteRegistry>(@aptos_experimental).entries;
        if (!table::contains(entries, exchange)) {
            return 0
        };
        table::borrow(entries, exchange).finalized_at_version
    }
}
