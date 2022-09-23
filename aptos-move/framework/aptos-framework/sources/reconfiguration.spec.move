spec aptos_framework::reconfiguration {
    spec module {
        // After genesis, `Configuration` exists.
        invariant [suspendable] chain_status::is_operating() ==> exists<Configuration>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==>
            (timestamp::spec_now_microseconds() >= last_reconfiguration_time());
    }

    spec reconfigure {
        aborts_if false;
    }
}
