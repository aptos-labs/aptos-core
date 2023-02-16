// :!:>module
/// Mock on-chain governance parameters.
module upgrade_and_govern::parameters {

    struct GovernanceParameters has key {
        parameter_1: u64,
        parameter_2: u64
    }

    const GENESIS_PARAMETER_1: u64 = 123;
    const GENESIS_PARAMETER_2: u64 = 456;

    fun init_module(
        upgrade_and_govern: &signer
    ) {
        let governance_parameters = GovernanceParameters{
            parameter_1: GENESIS_PARAMETER_1,
            parameter_2: GENESIS_PARAMETER_2};
        move_to<GovernanceParameters>(
            upgrade_and_govern, governance_parameters);
    }

    public fun get_parameters():
    (u64, u64)
    acquires GovernanceParameters {
        let governance_parameters_ref =
            borrow_global<GovernanceParameters>(@upgrade_and_govern);
        (governance_parameters_ref.parameter_1,
         governance_parameters_ref.parameter_2)
    }

} // <:!:module
