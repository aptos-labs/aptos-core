module aptos_framework::mpc {
    use std::option;
    use std::option::Option;
    use std::string::utf8;
    use std::vector;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_std::debug;
    use aptos_framework::event::emit;
    use aptos_framework::next_validator_set;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;

    friend aptos_framework::reconfiguration_with_dkg;

    struct SharedSecretState has copy, drop, store {
        transcript_for_cur_epoch: Option<vector<u8>>,
        transcript_for_next_epoch: Option<vector<u8>>,
    }

    struct TaskSpec has copy, drop, store {
        group_element: vector<u8>,
        secret_idx: u64,
    }

    struct TaskState has copy, drop, store {
        task: TaskSpec,
        result: Option<vector<u8>>,
    }

    struct State has copy, drop, key, store {
        /// Currently only has 1 secret: the main secret.
        shared_secrets: vector<SharedSecretState>,
        /// The user request queue.
        /// mpc todo: scale with Table/BigVector.
        tasks: vector<TaskState>,
    }

    #[event]
    struct MPCEvent has drop, store {
        variant: Any,
    }

    struct MPCEventReconfigStart has copy, drop, store {
        epoch: u64,
        next_validator_set: vector<ValidatorConsensusInfo>,
    }

    struct MPCEventStateUpdated has copy, drop, store {
        epoch: u64,
        new_state: State,
    }

    /// This resource exists under 0x1 iff MPC is enabled.
    struct FeatureEnabledFlag has key {}

    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<State>(@aptos_framework)) {
            let state = State {
                shared_secrets: vector[],
                tasks: vector[],
            };
            move_to(framework, state);
            move_to(framework, FeatureEnabledFlag {}); //mpc todo: this needs to be pulled out as part of mpc_config, just like randomness_config.
        }
    }

    public fun on_async_reconfig_start() {
        if (exists<FeatureEnabledFlag>(@aptos_framework)) {
            debug::print(&utf8(b"0722 - emitting mpc event"));
            let event = MPCEventReconfigStart {
                epoch: reconfiguration::current_epoch(),
                next_validator_set: next_validator_set::load(),
            };
            emit(MPCEvent { variant: copyable_any::pack(event)});
        }
    }

    public(friend) fun ready_for_next_epoch(): bool acquires State {
        if (!exists<FeatureEnabledFlag>(@aptos_framework)) {
            debug::print(&utf8(b"0722 - mpc ready 0"));
            return true
        };

        if (!exists<State>(@aptos_framework)) {
            debug::print(&utf8(b"0722 - mpc not ready 1"));
            return false
        };

        let state = borrow_global<State>(@aptos_framework);
        let num_secrets = vector::length(&state.shared_secrets);
        if (num_secrets == 0) {
            debug::print(&utf8(b"0722 - mpc not ready 2"));
            return false
        };

        let secret_state = vector::borrow(&state.shared_secrets, 0);
        let maybe_trx = &secret_state.transcript_for_next_epoch;
        if (option::is_none(maybe_trx)) {
            debug::print(&utf8(b"0722 - mpc not ready 3"));
            return false
        };

        debug::print(&utf8(b"0722 - mpc ready 4"));
        true
    }

    public(friend) fun on_new_epoch(_framework: &signer) acquires State {
        //mpc todo: should clean up any in-progress session states.
        let state = borrow_global_mut<State>(@aptos_framework);
        let main_secret_state = vector::borrow_mut(&mut state.shared_secrets, 0);
        let trx = option::extract(&mut main_secret_state.transcript_for_next_epoch);
        main_secret_state.transcript_for_cur_epoch = option::some(trx);
    }


    public fun raise_by_secret(group_element: vector<u8>, secret_idx: u64): u64 acquires State {
        //mpc todo: validate group_element
        let task_spec = TaskSpec {
            group_element,
            secret_idx
        };

        let task_state = TaskState {
            task: task_spec,
            result: option::none(),
        };
        let state = borrow_global_mut<State>(@aptos_framework);
        let task_idx = vector::length(&state.tasks);
        vector::push_back(&mut state.tasks, task_state);

        let event = MPCEventStateUpdated {
            epoch: reconfiguration::current_epoch(),
            new_state: *state,
        };
        emit(MPCEvent { variant: copyable_any::pack(event)});

        task_idx
    }

    /// Used by user contracts to get the result.
    public fun get_result(task_idx: u64): Option<vector<u8>> acquires State {
        vector::borrow(&mut borrow_global_mut<State>(@aptos_framework).tasks, task_idx).result
    }

    /// When a MPC task is done, this is invoked by validator transactions.
    fun publish_reconfig_work_result(trx: vector<u8>) acquires State {
        debug::print(&utf8(b"0720 - publish_reconfig_work_result: begin"));
        let state = borrow_global_mut<State>(@aptos_framework);
        let secret_state = vector::borrow_mut(&mut state.shared_secrets, 0);
        if (option::is_none(&secret_state.transcript_for_next_epoch)) {
            debug::print(&utf8(b"0720 - publish_reconfig_work_result: apply"));
            secret_state.transcript_for_next_epoch = option::some(trx);
        };
        debug::print(&utf8(b"0720 - publish_reconfig_work_result: end"));
    }

    fun publish_task_result(idx: u64, result: vector<u8>) acquires State {
        debug::print(&utf8(b"0720 - publish_task_result: begin"));
        let state = borrow_global_mut<State>(@aptos_framework);
        let task_state = vector::borrow_mut(&mut state.tasks, idx);
        if (option::is_none(&task_state.result)) {
            debug::print(&utf8(b"0720 - publish_task_result: apply"));
            task_state.result = option::some(result);
        };
        debug::print(&utf8(b"0720 - publish_task_result: end"));
    }
}
