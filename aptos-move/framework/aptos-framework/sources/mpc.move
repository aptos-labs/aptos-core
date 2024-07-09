module aptos_framework::mpc {
    use std::option;
    use std::option::Option;
    use std::vector;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_framework::event::emit;

    struct SharedSecret has store {
        transcript_serialized: vector<u8>,
    }

    struct TaskSpec has copy, drop, store {
        variant: Any,
    }

    struct TaskState has store {
        task: TaskSpec,
        result: Option<vector<u8>>,
    }

    struct TaskRaiseBySecret has copy, drop, store {
        group_element: vector<u8>,
        secret_idx: u64,
    }

    struct State has key {
        shared_secrets: vector<SharedSecret>,
        tasks: vector<TaskState>,
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
