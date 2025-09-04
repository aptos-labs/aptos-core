spec velor_framework::storage_gas {
    // -----------------
    // Struct invariants
    // -----------------

    spec Point {
        invariant x <= BASIS_POINT_DENOMINATION;
        invariant y <= BASIS_POINT_DENOMINATION;
    }

    spec GasCurve {
        /// Invariant 1: The minimum gas charge does not exceed the maximum gas charge.
        invariant min_gas <= max_gas;
        /// Invariant 2: The maximum gas charge is capped by MAX_U64 scaled down by the basis point denomination.
        invariant max_gas <= MAX_U64 / BASIS_POINT_DENOMINATION;
        /// Invariant 3: The x-coordinate increases monotonically and the y-coordinate increasing strictly monotonically,
        /// that is, the gas-curve is a monotonically increasing function.
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
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The module's initialization guarantees the creation of the StorageGasConfig resource with a precise
    /// configuration, including accurate gas curves for per-item and per-byte operations.
    /// Criticality: Medium
    /// Implementation: The initialize function is responsible for setting up the initial state of the module, ensuring
    /// the fulfillment of the following conditions: (1) the creation of the StorageGasConfig resource, indicating its
    /// existence witqhin the module's context, and (2) the configuration of the StorageGasConfig resource includes the
    /// precise gas curves that define the behavior of per-item and per-byte operations.
    /// Enforcement: Formally verified via [high-level-req-1](initialize). Moreover, the native gas logic has been manually audited.
    ///
    /// No.: 2
    /// Requirement: The gas curve approximates an exponential curve based on a minimum and maximum gas charge.
    /// Criticality: High
    /// Implementation: The validate_points function ensures that the provided vector of points represents a
    /// monotonically non-decreasing curve.
    /// Enforcement: Formally verified via [high-level-req-2](validate_points). Moreover, the configuration logic has been manually audited.
    ///
    /// No.: 3
    /// Requirement: The initialized gas curve structure has values set according to the provided parameters.
    /// Criticality: Low
    /// Implementation: The new_gas_curve function initializes the GasCurve structure with values provided as parameters.
    /// Enforcement: Formally verified via [high-level-req-3](new_gas_curve).
    ///
    /// No.: 4
    /// Requirement: The initialized usage gas configuration structure has values set according to the provided parameters.
    /// Criticality: Low
    /// Implementation: The new_usage_gas_config function initializes the UsageGasConfig structure with values provided
    /// as parameters.
    /// Enforcement: Formally verified via [high-level-req-4](new_usage_gas_config).
    /// </high-level-req>
    ///
    spec module {
        use velor_framework::chain_status;
        pragma verify = true;
        pragma aborts_if_is_strict;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGasConfig>(@velor_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGas>(@velor_framework);
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
        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved).
        include NewGasCurveAbortsIf;
        include ValidatePointsAbortsIf;
        /// [high-level-req-3]
        ensures result == GasCurve {
            min_gas,
            max_gas,
            points
        };
    }

    spec new_usage_gas_config(target_usage: u64, read_curve: GasCurve, create_curve: GasCurve, write_curve: GasCurve): UsageGasConfig {
        aborts_if target_usage == 0;
        aborts_if target_usage > MAX_U64 / BASIS_POINT_DENOMINATION;
        /// [high-level-req-4]
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

    /// Signer address must be @velor_framework and StorageGasConfig exists.
    spec set_config(velor_framework: &signer, config: StorageGasConfig) {
        include system_addresses::AbortsIfNotVelorFramework{ account: velor_framework };
        aborts_if !exists<StorageGasConfig>(@velor_framework);
    }

    /// Signer address must be @velor_framework.
    /// Address @velor_framework does not exist StorageGasConfig and StorageGas before the function call is restricted
    /// and exists after the function is executed.
    spec initialize(velor_framework: &signer) {
        include system_addresses::AbortsIfNotVelorFramework{ account: velor_framework };
        pragma verify_duration_estimate = 120;
        aborts_if exists<StorageGasConfig>(@velor_framework);
        aborts_if exists<StorageGas>(@velor_framework);

        /// [high-level-req-1]
        ensures exists<StorageGasConfig>(@velor_framework);
        ensures exists<StorageGas>(@velor_framework);
    }

    /// A non decreasing curve must ensure that next is greater than cur.
    spec validate_points(points: &vector<Point>) {
        pragma aborts_if_is_strict = false;
        pragma verify = false; // TODO: set because of timeout (property proved).
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

    /// Address @velor_framework must exist StorageGasConfig and StorageGas and StateStorageUsage.
    spec on_reconfig {
        use velor_framework::chain_status;
        requires chain_status::is_operating();
        aborts_if !exists<StorageGasConfig>(@velor_framework);
        aborts_if !exists<StorageGas>(@velor_framework);
        aborts_if !exists<state_storage::StateStorageUsage>(@velor_framework);
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

        /// [high-level-req-2]
        aborts_if exists i in 0..len(points) - 1: (
            points[i].x >= points[i + 1].x || points[i].y > points[i + 1].y
        );
        aborts_if len(points) > 0 && points[0].x == 0;
        aborts_if len(points) > 0 && points[len(points) - 1].x == BASIS_POINT_DENOMINATION;
    }
}
