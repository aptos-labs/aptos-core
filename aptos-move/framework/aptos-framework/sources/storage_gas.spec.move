spec aptos_framework::storage_gas {
    spec fun spec_calculate_gas(max_usage: u64, current_usage: u64, curve: GasCurve): u64;

    spec GasCurve {
        invariant min_gas <= max_gas;
        invariant max_gas <= MAX_U64 / BASIS_POINT_DENOMINATION;
    }

    spec validate_curve {
        pragma opaque;
        let points = curve.points;
        aborts_if exists i in 0..len(points)-1: (
            points[i].x >= points[i+1].x || points[i].y > points[i+1].y
        );
        aborts_if len(points) > 0 && points[0].x == 0;
        aborts_if len(points) > 0 && points[len(points)-1].x == BASIS_POINT_DENOMINATION;
    }

    spec fun storage_gas_config_is_valid(): bool {
        let storage_gas_config = global<StorageGasConfig>(@aptos_framework);
        spec_usage_config_is_validated(storage_gas_config.item_config) &&
            spec_usage_config_is_validated(storage_gas_config.byte_config)
    }

    spec fun spec_usage_config_is_validated(config: UsageGasConfig): bool {
        spec_curve_is_validated(config.read_curve) &&
            spec_curve_is_validated(config.create_curve) &&
            spec_curve_is_validated(config.write_curve)
    }

    spec fun spec_curve_is_validated(curve: GasCurve): bool {
        let points = curve.points;
        (len(points) > 0 ==> points[0].x > 0) &&
            (len(points) > 0 ==> points[len(points)-1].x < BASIS_POINT_DENOMINATION) &&
            (forall i in 0..len(points)-1: (points[i].x < points[i+1].x && points[i].y <= points[i+1].y))
    }

    spec UsageGasConfig {
        invariant target_usage > 0;
        invariant target_usage <= MAX_U64 / BASIS_POINT_DENOMINATION;
    }

    spec Point {
        invariant x <= BASIS_POINT_DENOMINATION;
        invariant y <= BASIS_POINT_DENOMINATION;
    }

    spec calculate_create_gas {
        requires spec_usage_config_is_validated(config);
    }

    spec calculate_read_gas {
        requires spec_usage_config_is_validated(config);
    }

    spec calculate_write_gas {
        requires spec_usage_config_is_validated(config);
    }

    spec calculate_gas {
        pragma opaque;
        requires max_usage > 0;
        requires max_usage <= MAX_U64 / BASIS_POINT_DENOMINATION;
        requires spec_curve_is_validated(curve);

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

    spec module {
        use aptos_std::chain_status;
        // After genesis, `StateStorageUsage` and `GasParameter` exist.
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGasConfig>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==> exists<StorageGas>(@aptos_framework);
        invariant [suspendable] exists<StorageGasConfig>(@aptos_framework) ==> storage_gas_config_is_valid();
    }
}
