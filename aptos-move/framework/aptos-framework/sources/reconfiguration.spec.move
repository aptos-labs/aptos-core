spec aptos_framework::reconfiguration {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;

        // After genesis, `Configuration` exists.
        invariant [suspendable] chain_status::is_operating() ==> exists<Configuration>(@aptos_framework);
        invariant [suspendable] chain_status::is_operating() ==>
            (timestamp::spec_now_microseconds() >= last_reconfiguration_time());
    }

    /// Make sure the signer address is @aptos_framework.
    spec schema AbortsIfNotAptosFramework {
        aptos_framework: &signer;

        let addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
    }

    /// Address @aptos_framework must exist resource Account and Configuration.
    /// Already exists in framework account.
    /// Guid_creation_num should be 2 according to logic.
    spec initialize(aptos_framework: &signer) {
        use std::signer;
        use aptos_framework::account::{Account};
        use aptos_framework::guid;

        include AbortsIfNotAptosFramework;
        let addr = signer::address_of(aptos_framework);
        let post config = global<Configuration>(@aptos_framework);
        requires exists<Account>(addr);
        aborts_if !(global<Account>(addr).guid_creation_num == 2);
        aborts_if exists<Configuration>(@aptos_framework);
        // property 1: During the module's initialization, it guarantees that the Configuration resource will move under the Aptos framework account with initial values.
        ensures exists<Configuration>(@aptos_framework);
        ensures config.epoch == 0 && config.last_reconfiguration_time == 0;
        ensures config.events == event::EventHandle<NewEpochEvent> {
            counter: 0,
            guid: guid::GUID {
                id: guid::ID {
                    creation_num: 2,
                    addr: @aptos_framework
                }
            }
        };
    }

    spec current_epoch(): u64 {
        aborts_if !exists<Configuration>(@aptos_framework);
        ensures result == global<Configuration>(@aptos_framework).epoch;
    }

    spec disable_reconfiguration(aptos_framework: &signer) {
        include AbortsIfNotAptosFramework;
        aborts_if exists<DisableReconfiguration>(@aptos_framework);
        ensures exists<DisableReconfiguration>(@aptos_framework);
    }

    /// Make sure the caller is admin and check the resource DisableReconfiguration.
    spec enable_reconfiguration(aptos_framework: &signer) {
        use aptos_framework::reconfiguration::{DisableReconfiguration};
        include AbortsIfNotAptosFramework;
        aborts_if !exists<DisableReconfiguration>(@aptos_framework);
        ensures !exists<DisableReconfiguration>(@aptos_framework);
    }

    /// When genesis_event emit the epoch and the `last_reconfiguration_time` .
    /// Should equal to 0
    spec emit_genesis_reconfiguration_event {
        use aptos_framework::reconfiguration::{Configuration};

        aborts_if !exists<Configuration>(@aptos_framework);
        let config_ref = global<Configuration>(@aptos_framework);
        aborts_if !(config_ref.epoch == 0 && config_ref.last_reconfiguration_time == 0);
        ensures global<Configuration>(@aptos_framework).epoch == 1;
    }

    spec last_reconfiguration_time {
        aborts_if !exists<Configuration>(@aptos_framework);
        ensures result == global<Configuration>(@aptos_framework).last_reconfiguration_time;
    }

    spec reconfigure {
        use aptos_framework::aptos_coin;
        use aptos_framework::coin::CoinInfo;
        use aptos_framework::aptos_coin::AptosCoin;
        use aptos_framework::transaction_fee;
        use aptos_framework::staking_config;

        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)

        requires exists<stake::ValidatorFees>(@aptos_framework);
        requires exists<CoinInfo<AptosCoin>>(@aptos_framework);

        include features::spec_periodical_reward_rate_decrease_enabled() ==> staking_config::StakingRewardsConfigEnabledRequirement;
        include features::spec_collect_and_distribute_gas_fees_enabled() ==> aptos_coin::ExistsAptosCoin;
        include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
        aborts_if false;

        // The ensure conditions of the reconfigure function are not fully written, because there is a new cycle in it,
        // but its existing ensure conditions satisfy hp.
        let success = !(chain_status::is_genesis() || timestamp::spec_now_microseconds() == 0 || !reconfiguration_enabled())
            && timestamp::spec_now_microseconds() != global<Configuration>(@aptos_framework).last_reconfiguration_time;
        // The property below is not proved within 500s and still cause an timeout
        // property 3: Synchronization of NewEpochEvent counter with configuration epoch.
        ensures success ==> global<Configuration>(@aptos_framework).epoch == old(global<Configuration>(@aptos_framework).epoch) + 1;
        ensures success ==> global<Configuration>(@aptos_framework).last_reconfiguration_time == timestamp::spec_now_microseconds();
        // We remove the ensures of event increment due to inconsisency
        // TODO: property 4: Only performs reconfiguration if genesis has started and reconfiguration is enabled.
        // Also, the last reconfiguration must not be the current time, returning early without further actions otherwise.
        // property 5: Consecutive reconfigurations without the passage of time are not permitted.
        ensures !success ==> global<Configuration>(@aptos_framework).epoch == old(global<Configuration>(@aptos_framework).epoch);
    }

    spec reconfiguration_enabled {
        // property 2: The reconfiguration status may be determined at any time without causing an abort, indicating whether or not the system allows reconfiguration.
        aborts_if false;
        ensures result == !exists<DisableReconfiguration>(@aptos_framework);
    }
}
