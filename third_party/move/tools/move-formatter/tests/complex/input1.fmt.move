module test {
    fun set_incentive_parameters_range_check_inputs(
        integrator_fee_store_tiers_ref: &vector<vector<u64>>
    ) {
        // Assert integrator fee store parameters vector not too long.
        assert!(
            vector::length(integrator_fee_store_tiers_ref) <= MAX_INTEGRATOR_FEE_STORE_TIERS,
            E_TOO_MANY_TIERS
        );
    }
}