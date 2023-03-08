// :!:>script
script {
    use upgrade_and_govern::parameters;
    use upgrade_and_govern::transfer;

    const PARAMETER_1: u64 = 300;
    const PARAMETER_2: u64 = 200;

    fun set_and_transfer(
        upgrade_and_govern: &signer,
        to_1: address,
        to_2: address
    ) {
        parameters::set_parameters(
            upgrade_and_govern, PARAMETER_1, PARAMETER_2);
        transfer::transfer_octas(upgrade_and_govern, to_1, to_2);
    }
} // <:!:script
