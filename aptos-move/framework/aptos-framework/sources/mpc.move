module aptos_framework::mpc {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_framework::event::emit;
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    struct SharedSecretState has store {
        transcript_for_cur_epoch: Option<vector<u8>>,
        transcript_for_next_epoch: Option<vector<u8>>,
    }

    struct TaskSpec has copy, drop, store {
        variant: Any,
    }

    struct TaskRaiseBySecret has copy, drop, store {
        group_element: vector<u8>,
        secret_idx: u64,
    }

    struct TaskState has store {
        task: TaskSpec,
        result: Option<vector<u8>>,
    }

    struct State has key {
        shared_secrets: vector<SharedSecretState>,
        /// tasks[0] should always be `raise_by_secret(GENERATOR)`
        tasks: vector<TaskState>,
    }

    #[event]
    struct EpochSwitchStart {

    }

    #[event]
    struct NewTaskEvent has drop, store {
        task_idx: u64,
        task_spec: TaskSpec,
    }

    #[event]
    struct TaskCompletedEvent has drop, store {
        task_idx: u64,
        result: Option<vector<u8>>,
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
        }
    }

    public fun on_async_reconfig_start() {
        if (exists<FeatureEnabledFlag>(@aptos_framework)) {
            //mpc todo: emit an event to trigger validator components.
        }
    }

    public(friend) fun ready_for_next_epoch(): bool acquires State {
        if (!exists<FeatureEnabledFlag>(@aptos_framework)) {
            return true
        };

        if (!exists<State>(@aptos_framework)) {
            return false
        };

        let state = borrow_global<State>(@aptos_framework);
        let num_secrets = vector::length(&state.shared_secrets);
        if (num_secrets == 0) {
            return false
        };

        let secret_state = vector::borrow(&state.shared_secrets, 0);
        let maybe_trx = &secret_state.transcript_for_next_epoch;
        if (option::is_none(maybe_trx)) {
            return false
        };

        true
    }

    public(friend) fun on_new_epoch(_framework: &signer) {
        //mpc todo: should clean up any in-progress session states.
    }


    public fun raise_by_secret(group_element: vector<u8>, secret_idx: u64): u64 acquires State {
        let task_spec = TaskSpec {
            variant: copyable_any::pack(TaskRaiseBySecret {
                group_element,
                secret_idx
            }),
        };

        let task_state = TaskState {
            task: task_spec,
            result: option::none(),
        };
        let task_list = &mut borrow_global_mut<State>(@aptos_framework).tasks;
        let task_idx = vector::length(task_list);
        vector::push_back(task_list, task_state);

        let event = NewTaskEvent {
            task_idx,
            task_spec
        };
        emit(event);

        task_idx
    }

    /// When a MPC task is done, this is invoked by validator transactions.
    fun fulfill_task(task_idx: u64, result: Option<vector<u8>>) acquires State {
        vector::borrow_mut(&mut borrow_global_mut<State>(@aptos_framework).tasks, task_idx).result = result;
        let event = TaskCompletedEvent {
            task_idx,
            result,
        };
        emit(event);
    }

    /// Used by user contract to get the result.
    public fun get_result(task_idx: u64): Option<vector<u8>> acquires State {
        vector::borrow(&mut borrow_global_mut<State>(@aptos_framework).tasks, task_idx).result
    }
}
