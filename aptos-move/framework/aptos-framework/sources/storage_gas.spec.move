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
        use aptos_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGasConfig>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGas>(@aptos_framework);
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec base_8192_exponential_curve(min_gas: u64, max_gas: u64): GasCurve {
        include NewGasCurveAbortsIf;
    }

    spec new_point(x: u64, y: u64): Point {
        aborts_if x > BASIS_POINT_DENOMINATION || y > BASIS_POINT_DENOMINATION;

        ensures result.x == x;
        ensures result.y == y;
    }

    /// A non decreasing curve must ensure that next is greater than cur.
    spec new_gas_curve(min_gas: u64, max_gas: u64, points: vector<Point>): GasCurve {
        include NewGasCurveAbortsIf;
        include ValidatePointsAbortsIf;
        ensures result == GasCurve {
            min_gas,
            max_gas,
            points
        };
    }

    spec new_usage_gas_config(target_usage: u64, read_curve: GasCurve, create_curve: GasCurve, write_curve: GasCurve): UsageGasConfig {
        aborts_if target_usage == 0;
        aborts_if target_usage > MAX_U64 / BASIS_POINT_DENOMINATION;
        ensures result == UsageGasConfig {
            target_usage,
            read_curve,
            create_curve,
            write_curve,
        };
    }

    spec new_storage_gas_config(item_config: UsageGasConfig, byte_config: UsageGasConfig): StorageGasConfig {
        aborts_if false;

        ensures result.item_config == item_config;
        ensures result.byte_config == byte_config;
    }

    /// Signer address must be @aptos_framework and StorageGasConfig exists.
    spec set_config(aptos_framework: &signer, config: StorageGasConfig) {
        include system_addresses::AbortsIfNotAptosFramework{ account: aptos_framework };
        aborts_if !exists<StorageGasConfig>(@aptos_framework);
    }

    /// Signer address must be @aptos_framework.
    /// Address @aptos_framework does not exist StorageGasConfig and StorageGas before the function call is restricted
    /// and exists after the function is executed.
    spec initialize(aptos_framework: &signer) {
        include system_addresses::AbortsIfNotAptosFramework{ account: aptos_framework };
        aborts_if exists<StorageGasConfig>(@aptos_framework);
        aborts_if exists<StorageGas>(@aptos_framework);

        ensures exists<StorageGasConfig>(@aptos_framework);
        ensures exists<StorageGas>(@aptos_framework);
    }

    /// A non decreasing curve must ensure that next is greater than cur.
    spec validate_points(points: &vector<Point>) {
        pragma aborts_if_is_strict = false;
        pragma verify = false; // TODO: Disabled. Investigate why this fails.
        pragma opaque;
        include ValidatePointsAbortsIf;
    }

    spec calculate_gas(max_usage: u64, current_usage: u64, curve: &GasCurve): u64 {
        pragma opaque;
        // Not verified when verify_duration_estimate > vc_timeout
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved).
        requires max_usage > 0;
        requires max_usage <= MAX_U64 / BASIS_POINT_DENOMINATION;
        aborts_if false;
        ensures [abstract] result == spec_calculate_gas(max_usage, current_usage, curve);
    }

    spec interpolate(x0: u64, x1: u64, y0: u64, y1: u64, x: u64): u64 {
        pragma opaque;
        pragma intrinsic;

        aborts_if false;
    }

    /// Address @aptos_framework must exist StorageGasConfig and StorageGas and StateStorageUsage.
    spec on_reconfig {
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
        aborts_if !exists<StorageGasConfig>(@aptos_framework);
        aborts_if !exists<StorageGas>(@aptos_framework);
        aborts_if !exists<state_storage::StateStorageUsage>(@aptos_framework);
    }


    // ---------------------------------
    // Spec helper functions and schemas
    // ---------------------------------

    spec fun spec_calculate_gas(max_usage: u64, current_usage: u64, curve: GasCurve): u64;

    spec schema NewGasCurveAbortsIf {
        min_gas: u64;
        max_gas: u64;

        aborts_if max_gas < min_gas;
        aborts_if max_gas > MAX_U64 / BASIS_POINT_DENOMINATION;
    }

    /// A non decreasing curve must ensure that next is greater than cur.
    spec schema ValidatePointsAbortsIf {
        points: vector<Point>;

        aborts_if exists i in 0..len(points) - 1: (
            points[i].x >= points[i + 1].x || points[i].y > points[i + 1].y
        );
        aborts_if len(points) > 0 && points[0].x == 0;
        aborts_if len(points) > 0 && points[len(points) - 1].x == BASIS_POINT_DENOMINATION;
    }
}
