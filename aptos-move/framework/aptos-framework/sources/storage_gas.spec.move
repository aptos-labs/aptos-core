spec aptos_framework::storage_gas {
    // -----------------
    // Struct invariants
    // -----------------

    spec Point {
        invariant x <= BASIS_POINT_DENOMINATION;
        invariant y <= BASIS_POINT_DENOMINATION;
    }

    spec GasCurve {
        invariant min_gas <= max_gas;
        invariant max_gas <= MAX_U64 / BASIS_POINT_DENOMINATION;
        invariant (len(points) > 0 ==> points[0].x > 0)
            && (len(points) > 0 ==> points[len(points) - 1].x < BASIS_POINT_DENOMINATION)
            && (forall i in 0..len(points) - 1: (points[i].x < points[i + 1].x && points[i].y <= points[i + 1].y));
    }

    spec UsageGasConfig {
        invariant target_usage > 0;
        invariant target_usage <= MAX_U64 / BASIS_POINT_DENOMINATION;
    }


    // -----------------
    // Global invariants
    // -----------------

    spec module {
        use aptos_std::chain_status;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGasConfig>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGas>(@aptos_framework);
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec validate_points {
        pragma opaque;
        aborts_if [abstract] exists i in 0..len(points) - 1: (
            points[i].x >= points[i + 1].x || points[i].y > points[i + 1].y
        );
        aborts_if [abstract] len(points) > 0 && points[0].x == 0;
        aborts_if [abstract]  len(points) > 0 && points[len(points) - 1].x == BASIS_POINT_DENOMINATION;
    }

    spec calculate_gas {
        pragma opaque;
        requires max_usage > 0;
        requires max_usage <= MAX_U64 / BASIS_POINT_DENOMINATION;
        aborts_if false;
        ensures [abstract] result == spec_calculate_gas(max_usage, current_usage, curve);
    }

    spec interpolate {
        pragma opaque;
        requires x0 < x1;
        requires y0 <= y1;
        requires x0 <= x && x <= x1;
        requires x1 * y1 <= MAX_U64;
        aborts_if false;
        ensures y0 <= result && result <= y1;
    }

    spec on_reconfig {
        use aptos_std::chain_status;
        requires chain_status::is_operating();
        aborts_if false;
    }


    // ---------------------------------
    // Spec helper functions and schemas
    // ---------------------------------

    spec fun spec_calculate_gas(max_usage: u64, current_usage: u64, curve: GasCurve): u64;
}
