spec aptos_framework::timestamp {
    spec set_time_has_started {
        use std::signer;
        /// This function can't be verified on its own and has to be verified in the context of Genesis execution.
        ///
        /// After time has started, all invariants guarded by `Timestamp::is_operating` will become activated
        /// and need to hold.
        pragma delegate_invariants_to_caller;
        aborts_if exists<CurrentTimeMicroseconds>(signer::address_of(account));
        include AbortsIfNotGenesis;
        include system_addresses::AbortsIfNotAptosFramework{account};
        ensures is_operating();
    }

    spec update_global_time {
        pragma opaque;
        modifies global<CurrentTimeMicroseconds>(@aptos_framework);

        let now = spec_now_microseconds();
        let post post_now = spec_now_microseconds();

        /// Conditions unique for abstract and concrete version of this function.
        include AbortsIfNotOperating;
        include system_addresses::AbortsIfNotVM;
        ensures post_now == timestamp;

        /// Conditions we only check for the implementation, but do not pass to the caller.
        aborts_if [concrete]
            (if (proposer == @vm_reserved) {
                now != timestamp
            } else  {
                now >= timestamp
            }
        )
        with error::INVALID_ARGUMENT;
    }

    spec now_microseconds {
        pragma opaque;
        include AbortsIfNotOperating;
        ensures result == spec_now_microseconds();
    }

    spec fun spec_now_microseconds(): u64 {
        global<CurrentTimeMicroseconds>(@aptos_framework).microseconds
    }

    spec now_seconds {
        pragma opaque;
        include AbortsIfNotOperating;
        ensures result == spec_now_seconds();
    }
    spec fun spec_now_seconds(): u64 {
        spec_now_microseconds() / MICRO_CONVERSION_FACTOR
    }

    /// Helper schema to specify that a function aborts if not in genesis.
    spec schema AbortsIfNotGenesis {
        aborts_if !is_genesis() with error::INVALID_STATE;
    }

    spec assert_operating {
        pragma opaque = true;
        include AbortsIfNotOperating;
    }

    /// Helper schema to specify that a function aborts if not operating.
    spec schema AbortsIfNotOperating {
        aborts_if !is_operating() with error::INVALID_STATE;
    }

    // ====================
    // Module Specification
    // ====================

    spec module {} // switch documentation context to module level

    spec module {
        /// After genesis, `CurrentTimeMicroseconds` is published forever
        invariant is_operating() ==> exists<CurrentTimeMicroseconds>(@aptos_framework);

        /// After genesis, time progresses monotonically.
        invariant update
            old(is_operating()) ==> old(spec_now_microseconds()) <= spec_now_microseconds();
    }

    spec module {
        /// All functions which do not have an `aborts_if` specification in this module are implicitly declared
        /// to never abort.
        pragma aborts_if_is_strict;
    }
}
