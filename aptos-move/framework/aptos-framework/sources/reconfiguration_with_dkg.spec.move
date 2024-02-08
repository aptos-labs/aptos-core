spec aptos_framework::reconfiguration_with_dkg {
    spec module {
        pragma verify = false;
    }

    // spec try_start() {
    //     include dkg::spec_in_progress() ==> reconfiguration_state::TryMarkAsInProgressAbortsIf;
    //     aborts_if dkg::spec_in_progress() && !exists<Configuration>(@aptos_framework);
    // }

}
