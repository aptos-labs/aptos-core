spec velor_framework::timestamp {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: There should only exist one global wall clock and it should be created during genesis.
    /// Criticality: High
    /// Implementation: The function set_time_has_started is only called by genesis::initialize and ensures that no
    /// other resources of this type exist by only assigning it to a predefined account.
    /// Enforcement: Formally verified via [high-level-req-1](module).
    ///
    /// No.: 2
    /// Requirement: The global wall clock resource should only be owned by the Velor framework.
    /// Criticality: High
    /// Implementation: The function set_time_has_started ensures that only the velor_framework account can possess the
    /// CurrentTimeMicroseconds resource using the assert_velor_framework function.
    /// Enforcement: Formally verified via [high-level-req-2](module).
    ///
    /// No.: 3
    /// Requirement: The clock time should only be updated by the VM account.
    /// Criticality: High
    /// Implementation: The update_global_time function asserts that the transaction signer is the vm_reserved account.
    /// Enforcement: Formally verified via [high-level-req-3](UpdateGlobalTimeAbortsIf).
    ///
    /// No.: 4
    /// Requirement: The clock time should increase with every update as agreed through consensus and proposed by the
    /// current epoch's validator.
    /// Criticality: High
    /// Implementation: The update_global_time function asserts that the new timestamp is greater than the current
    /// timestamp.
    /// Enforcement: Formally verified via [high-level-req-4](UpdateGlobalTimeAbortsIf).
    /// </high-level-req>
    ///
    spec module {
        use velor_framework::chain_status;
        /// [high-level-req-1]
        /// [high-level-req-2]
        invariant [suspendable] chain_status::is_operating() ==> exists<CurrentTimeMicroseconds>(@velor_framework);
    }

    spec update_global_time {
        use velor_framework::chain_status;
        requires chain_status::is_operating();
        include UpdateGlobalTimeAbortsIf;
        ensures (proposer != @vm_reserved) ==> (spec_now_microseconds() == timestamp);
    }

    spec schema UpdateGlobalTimeAbortsIf {
        account: signer;
        proposer: address;
        timestamp: u64;
        /// [high-level-req-3]
        aborts_if !system_addresses::is_vm(account);
        /// [high-level-req-4]
        aborts_if (proposer == @vm_reserved) && (spec_now_microseconds() != timestamp);
        aborts_if (proposer != @vm_reserved) && (spec_now_microseconds() >= timestamp);
    }

    spec fun spec_now_microseconds(): u64 {
        global<CurrentTimeMicroseconds>(@velor_framework).microseconds
    }

    spec fun spec_now_seconds(): u64 {
        spec_now_microseconds() / MICRO_CONVERSION_FACTOR
    }
}
