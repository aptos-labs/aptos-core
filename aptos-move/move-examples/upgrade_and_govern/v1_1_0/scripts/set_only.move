// :!:>script
script {
    use upgrade_and_govern::parameters;

    const PARAMETER_1: u64 = 500;
    const PARAMETER_2: u64 = 700;

    fun set_only(
        upgrade_and_govern: &signer,
    ) {
        parameters::set_parameters(
            upgrade_and_govern, PARAMETER_1, PARAMETER_2);
    }
} // <:!:script
